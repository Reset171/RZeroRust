use super::super::{
    Dim, Game,
};
use super::persistence::{
    load_checkpoint_from_disk, load_slot_from_disk, restore_disk_files_for_slot,
    restore_disk_files_on_rollback,
};
use super::snapshots::*;
use super::types::*;
use horizon_client::{
    bossbar::BossBars,
    lan::guest::SavedBody,
};
use horizon_core::{
    geometry::Vec3,
    position::{BlockPos, ChunkPos},
};
use horizon_protocol::{
    packets::{
        play::{self, Clientbound as C},
        recipes,
    },
};
use horizon_world::registry::BlockStateId;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    sync::Arc,
};

pub fn restore_offline_files(saved: &HashMap<PathBuf, Vec<u8>>, restore_stats: bool) {
    for (path, bytes) in saved {
        let is_stat = path.to_string_lossy().contains("stats");
        if is_stat && !restore_stats {
            continue;
        }
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(path, bytes);
    }
}

pub fn apply_level_snapshot_in_place(
    game: &mut Game<'_>,
    dim: Dim,
    snap: &LevelSnapshot,
    policy: &RollbackPolicy,
) -> (f64, f64, f64) {
    let old_chunks = game.world.chunks.clone();

    let t_blocks = std::time::Instant::now();
    if policy.blocks || policy.biomes {
        let mut changed_coords = Vec::new();
        for (&(cx, cz), snap_chunk) in snap.chunks.iter() {
            if let Some(live_chunk) = old_chunks.get(&(cx, cz)) {
                if Arc::ptr_eq(live_chunk, snap_chunk) {
                    continue;
                }
                let min_y = snap_chunk.min_y();
                for (s_idx, snap_sec) in snap_chunk.sections.iter().enumerate() {
                    let Some(live_sec) = live_chunk.sections.get(s_idx) else {
                        continue;
                    };
                    if Arc::ptr_eq(live_sec, snap_sec) {
                        continue;
                    }
                    let y_base = min_y + (s_idx as i32) * 16;
                    let x_base = cx * 16;
                    let z_base = cz * 16;
                    for sy in 0..16 {
                        for sz in 0..16 {
                            for sx in 0..16 {
                                let idx = ((sy << 8) | (sz << 4) | sx) as usize;
                                let want = snap_sec.get(idx);
                                if live_sec.get(idx) != want {
                                    changed_coords.push((
                                        x_base + sx,
                                        y_base + sy,
                                        z_base + sz,
                                        want,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        if policy.blocks {
            for (wx, wy, wz, want) in changed_coords {
                let _ = game.world.set(wx, wy, wz, BlockStateId(want));
            }
        }

        if policy.blocks && policy.biomes {
            game.world.chunks = Default::default();
            for (&k, v) in &snap.chunks {
                game.world.chunks.insert(k, v.clone());
            }
            game.world.inhabited = snap.inhabited.clone();
        } else {
            game.world.chunks = Default::default();
            for (&(cx, cz), snap_chunk) in &snap.chunks {
                if let Some(live_chunk) = old_chunks.get(&(cx, cz)) {
                    let mut c = (**live_chunk).clone();
                    for (s_idx, snap_sec) in snap_chunk.sections.iter().enumerate() {
                        if let Some(sec_arc) = c.sections.get_mut(s_idx) {
                            let sec = Arc::make_mut(sec_arc);
                            if policy.blocks {
                                sec.blocks = snap_sec.blocks.clone();
                            }
                            if policy.biomes {
                                sec.biomes = snap_sec.biomes.clone();
                            }
                        }
                    }
                    game.world.chunks.insert((cx, cz), Arc::new(c));
                } else {
                    game.world.chunks.insert((cx, cz), snap_chunk.clone());
                }
            }
        }
    }

    let blocks_ms = (std::time::Instant::now() - t_blocks).as_secs_f64() * 1000.0;
    let t_containers = std::time::Instant::now();
    if policy.block_entities {
        let mut be_positions: Vec<(i32, i32, i32)> =
            game.world.block_entities.keys().copied().collect();
        for &pos in snap.block_entities.keys() {
            if !be_positions.contains(&pos) {
                be_positions.push(pos);
            }
        }
        game.world.block_entities.clear();
        for (&pos, be) in &snap.block_entities {
            game.world.block_entities.insert(pos, be.clone());
        }
        game.world.block_names = snap.block_names.clone();
        game.hoppers = snap.hoppers.clone();
        if let Some(log) = game.world.block_log.as_mut() {
            for pos in be_positions {
                log.push(pos);
            }
        }
    }
    let containers_ms = (std::time::Instant::now() - t_containers).as_secs_f64() * 1000.0;
    let t_entities = std::time::Instant::now();

    if policy.block_ticks || policy.fluid_ticks {
        let mut all_chunk_keys: HashSet<(i32, i32)> = old_chunks.keys().copied().collect();
        all_chunk_keys.extend(snap.chunks.keys().copied());
        for (cx, cz) in all_chunk_keys {
            let cpos = ChunkPos::new(cx, cz);
            let (old_b, old_f) = game.block_ticks.remove_chunk(cpos);
            if let Some((snap_b, snap_f)) = snap.chunk_ticks.get(&(cx, cz)) {
                let b_to_add = if policy.block_ticks {
                    snap_b.clone()
                } else {
                    old_b.unwrap_or_default()
                };
                let f_to_add = if policy.fluid_ticks {
                    snap_f.clone()
                } else {
                    old_f.unwrap_or_default()
                };
                let _ = game.block_ticks.add_chunk(cpos, b_to_add, f_to_add);
            }
        }
        game.block_ticks.random_tick_value = snap.random_tick_value;
        game.block_ticks.comparator_outputs = snap.comparator_outputs.clone();
        game.block_ticks.torch_toggles = snap.torch_toggles.clone();
    }

    if policy.block_events {
        game.block_ticks.moving_pistons = snap.moving_pistons.clone();
        game.block_ticks.piston_pushes = snap.piston_pushes.clone();
        game.block_ticks.block_events = snap.block_events.clone();
    }

    if policy.world_level_rng {
        game.world.random = snap.random.clone();
    }

    if policy.server_forced_chunks {
        game.streaming.forced = snap.forced_chunks.clone();
    }

    if policy.entities_presence {
        game.mobs = snap.mobs.clone();
        if policy.dropped_items {
            game.dropped = snap.dropped.clone();
        }
        if policy.experience_orbs {
            game.orbs = snap.orbs.clone();
        }
        game.creatures.projectiles = snap.projectiles.clone();
        game.boats = snap.boats.clone();
        game.carts = snap.carts.clone();
        game.tnts = snap.tnts.clone();
        game.falling = snap.falling.iter().map(FallingSnapshot::to_live).collect();
        game.paintings = snap.paintings.clone();
        game.item_frames = snap.item_frames.clone();
        game.armor_stands = snap.armor_stands.clone();
        game.cushions = snap.cushions.clone();
        game.others = snap.others.clone();
        game.crystals = snap.crystals.clone();
        game.dragon = snap
            .dragon
            .as_ref()
            .and_then(|d| d.to_live(game.level.seed));
        game.knot_ids = snap.knot_ids.clone();
        game.persist.uuids = snap.persist_uuids.clone();
        game.persist.raw = snap.persist_raw.clone();
    }

    if policy.world_raids {
        let path = raids_dat_path(&game.world_root, dim);
        match &snap.raids_dat {
            Some(bytes) => {
                if let Some(parent) = path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = fs::write(&path, bytes);
            }
            None => {
                let _ = fs::remove_file(&path);
            }
        }
        let mut fresh_raiding = Default::default();
        game.raiding.swap_level(&mut fresh_raiding);
    }

    game.sculk.known.clear();
    game.sculk.known_revision = None;
    game.world.loaded_entities.clear();
    game.world.modified.clear();

    game.last_hurt_by_player = snap.last_hurt_by_player.clone();
    game.love_cause = snap.love_cause.clone();
    game.died.clear();
    game.combat = Default::default();
    game.guest_combat.clear();
    game.stabbed.clear();
    game.kinetic_feedback.clear();
    game.brushing = Default::default();
    game.lid_openers.clear();
    game.be_anims = Default::default();
    game.lan_songs.clear();
    game.lan_cushion_breaks.clear();
    game.lan_global_events.clear();

    game.tridents.clear();
    game.rockets.clear();
    game.bolts.clear();
    game.dragon_fireballs.clear();
    game.breath_clouds.clear();
    game.ominous_spawners.clear();
    game.spill = Default::default();
    game.pending = Default::default();
    game.particles.clear();
    game.pickups.clear();
    game.destroying.clear();
    game.mining = None;
    game.using = None;
    game.lan_events.clear();
    game.lan_block_events.clear();
    game.lan_entity_events.clear();
    game.lan_critical_hits.clear();

    game.potions.clear_thrown();
    game.potions.clouds.clear();
    game.close_container();
    game.station = None;
    game.container = None;

    let entities_ms = (std::time::Instant::now() - t_entities).as_secs_f64() * 1000.0;
    (blocks_ms, containers_ms, entities_ms)
}

pub fn rollback_checkpoint_on_server(game: &mut Game<'_>) -> bool {
    let slot = with_state(|st| st.current_slot.clone());
    rollback_slot_checkpoint_on_server(game, &slot)
}

pub fn rollback_slot_checkpoint_on_server(game: &mut Game<'_>, slot: &str) -> bool {
    let t_start = std::time::Instant::now();
    let t_globals = std::time::Instant::now();
    let _ = game.save.flush();
    game.world.modified.clear();
    game.world.loaded_entities.clear();
    let snap = match get_slot_snapshot_clone(slot) {
        Some(s) => s,
        None => match load_slot_from_disk(&game.world_root, slot) {
            Some(s) => {
                set_slot_snapshot(slot, s.clone());
                s
            }
            None => return false,
        },
    };
    restore_disk_files_for_slot(&game.world_root, slot);
    let policy = &snap.policy;

    if policy.world_time {
        game.world.game_time = snap.game_time;
        game.world.day_time = snap.day_time;
    }
    if policy.world_weather {
        game.world.weather = snap.weather;
    }
    if policy.server_game_rules {
        game.ui.rules = snap.rules.clone();
        game.world.health_rules = snap.health_rules;
        game.sync_rules();
        game.clock.set_rate(snap.clock_rate);
        game.clock.set_frozen(snap.clock_frozen);
        game.ui.clock.rate = snap.clock_rate;
        game.ui.clock.paused = snap.clock_frozen;
    }
    game.portals.travel = None;
    game.portals.player.cooldown = 80;
    game.server_trips.clear();
    if policy.world_difficulty {
        game.ui.difficulty = snap.difficulty;
        game.level.set_difficulty(snap.difficulty);
        game.level.difficulty_locked = snap.difficulty_locked;
        game.sync_rules();
    }
    if policy.world_spawn {
        game.level.world_spawn = snap.world_spawn;
        game.level.world_spawn_angle = snap.world_spawn_angle;
    }
    if policy.server_world_border {
        game.border = snap.border.clone();
    }
    if policy.server_bossbars {
        let bars = BossBars::unpack(&snap.bossbars);
        game.bosses = Default::default();
        game.bosses.bars = bars;
    }
    if policy.world_scoreboard {
        game.score.board = snap.scoreboard.clone();
    }
    if policy.world_dragon_fight {
        let fight_path = fight_dat_path(&game.world_root);
        match &snap.dragon_fight_dat {
            Some(bytes) => {
                if let Some(parent) = fight_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = fs::write(&fight_path, bytes);
            }
            None => {
                let _ = fs::remove_file(&fight_path);
            }
        }
        game.fight = Default::default();
    }

    game.sculk.tracker = snap.sculk_tracker;
    game.sculk.darkness = snap.sculk_darkness;
    game.next_entity = snap.next_entity;
    game.rand_sequences = snap.rand_sequences.clone();

    let trader_path = trader_dat_path(&game.world_root);
    match &snap.trader_dat {
        Some(bytes) => {
            if let Some(parent) = trader_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(&trader_path, bytes);
        }
        None => {
            let _ = fs::remove_file(&trader_path);
        }
    }
    game.creatures.reset_trader();

    restore_offline_files(&snap.offline_player_data, policy.player_stats);

    game.streaming.wait(std::time::Duration::from_millis(200));
    let _ = game.save.flush();

    let subfolders = ["region", "entities", "poi"];
    let dims = [
        "dimensions/minecraft/overworld",
        "dimensions/minecraft/the_nether",
        "dimensions/minecraft/the_end",
    ];
    for dim_path in dims {
        for sub in subfolders {
            let rdir = game.world_root.join(dim_path).join(sub);
            if let Ok(entries) = fs::read_dir(rdir) {
                for e in entries.flatten() {
                    let p = e.path();
                    if p.is_file() && !snap.region_files.contains(&p) {
                        let _ = fs::remove_file(&p);
                    }
                }
            }
        }
    }

    if let Ok((fresh_streaming, fresh_save, _)) = super::super::world_loading::make_streaming(
        &game.res,
        &game.pack,
        &game.options,
        &game.settings,
        &game.world_root,
        game.dim,
        game.world.shape,
        game.level.seed,
        &game.level.enabled_features,
    ) {
        game.streaming = fresh_streaming;
        game.save = fresh_save;
    } else {
        game.save = Arc::new(horizon_client::save::SaveDir::new(
            game.save.root.clone(),
            game.save.shape,
        ));
    }

    let target_dim = snap
        .guests
        .values()
        .next()
        .map(|g| Dim::from_id(&g.dimension))
        .unwrap_or_else(|| {
            snap.levels.keys().copied().next().unwrap_or(Dim::Overworld)
        });

    let globals_ms = (std::time::Instant::now() - t_globals).as_secs_f64() * 1000.0;
    if game.dim != target_dim {
        let _ = game.enter_level(None, target_dim);
    }

    // If Nether or End was opened after the checkpoint, drop it from parked memory
    let saved_dims: HashSet<Dim> = snap.levels.keys().copied().collect();
    game.levels.parked.retain(|dim, _| saved_dims.contains(dim));

    let other_dims: Vec<Dim> = snap
        .levels
        .keys()
        .copied()
        .filter(|&d| d != target_dim)
        .collect();

    let mut acc_blocks_ms = 0.0;
    let mut acc_containers_ms = 0.0;
    let mut acc_entities_ms = 0.0;

    for dim in other_dims {
        if let Some(level_snap) = snap.levels.get(&dim) {
            if game.enter_level(None, dim).is_ok() {
                let (b, c, e) = apply_level_snapshot_in_place(game, dim, level_snap, policy);
                acc_blocks_ms += b;
                acc_containers_ms += c;
                acc_entities_ms += e;
            }
        }
    }
    if game.dim != target_dim {
        let _ = game.enter_level(None, target_dim);
    }

    if let Some(level_snap) = snap.levels.get(&target_dim) {
        let (b, c, e) = apply_level_snapshot_in_place(game, target_dim, level_snap, policy);
        acc_blocks_ms += b;
        acc_containers_ms += c;
        acc_entities_ms += e;
    }

    let t_player = std::time::Instant::now();

    if let Some(mut host) = game.lan.take() {
        if policy.server_world_border {
            host.border_sent = None;
        }
        if policy.entities_presence {
            host.mob_players = snap.mob_players.clone();
        }
        for gi in 0..host.hub.guests.len() {
            if !host.hub.guests[gi].placed {
                continue;
            }
            let uuid = host.hub.guests[gi].uuid;
            let conn = host.hub.guests[gi].conn;
            let tracked = host.hub.guests[gi].tracker.ids();
            if !tracked.is_empty() {
                host.hub.guests[gi].send(&C::RemoveEntities(play::RemoveEntities { ids: tracked }));
                host.hub.guests[gi].tracker.clear();
            }

            host.hub.guests[gi].send(&C::ContainerClose(play::ContainerClose { container: 0 }));
            host.hub.close_container(conn);

            if let Some(gs) = snap.guests.get(&uuid) {
                let ent_id = host.hub.guests[gi].entity_id as i32;
                let dim_changed = policy.player_position && host.hub.guests[gi].dimension != gs.dimension;
                {
                    let g = &mut host.hub.guests[gi];
                if policy.player_position {
                    g.dimension = gs.dimension.clone();
                    g.position = gs.position;
                    g.previous_position = gs.position;
                    g.first_good = gs.position;
                    g.yaw = gs.yaw;
                    g.pitch = gs.pitch;
                    g.head_yaw = gs.head_yaw;
                    g.on_ground = gs.on_ground;
                }
                if policy.player_abilities {
                    g.game_mode = gs.game_mode;
                    g.flying = gs.flying;
                }
                if policy.player_motion {
                    g.sprinting = gs.sprinting;
                    g.sneaking = gs.sneaking;
                    g.fall_flying = gs.fall_flying;
                }
                g.sleeping = gs.sleeping;
                g.bed = gs.bed;
                g.sleep_timer = gs.sleep_timer;

                let prev_vehicle = g.vehicle;
                g.vehicle = gs.vehicle;
                g.vehicle_good = gs.vehicle_good;
                if prev_vehicle != gs.vehicle {
                    if let Some(old_v) = prev_vehicle {
                        g.send(&C::SetPassengers(play::SetPassengers {
                            vehicle: old_v as i32,
                            passengers: Vec::new(),
                        }));
                    }
                    if let Some(new_v) = gs.vehicle {
                        g.send(&C::SetPassengers(play::SetPassengers {
                            vehicle: new_v as i32,
                            passengers: vec![g.entity_id],
                        }));
                    }
                }

                if policy.player_spawn_point {
                    g.spawn = gs.spawn;
                    g.respawn_elsewhere = gs.respawn_elsewhere.clone();
                }
                g.last_death = gs.last_death.clone();
                if policy.player_health {
                    g.health = gs.health.max(1.0);
                    g.saved_absorption = Some(gs.absorption);
                }
                if policy.player_hunger {
                    g.food = gs.food;
                    g.saturation = gs.saturation;
                }
                if policy.player_experience {
                    g.xp_level = gs.xp_level;
                    g.xp_progress = gs.xp_progress;
                    g.xp_total = gs.xp_total;
                    g.xp_seed = gs.xp_seed;
                }
                if policy.player_inventory {
                    g.inventory = gs.inventory.clone();
                    g.armor = gs.armor.clone();
                    g.offhand = gs.offhand.clone();
                    g.selected = gs.selected;
                    g.craft_grid = gs.craft_grid.clone();
                    g.craft_result = gs.craft_result.clone();
                    g.carried = gs.carried.clone();
                }
                if policy.player_ender_chest {
                    g.ender_items = gs.ender_items.clone();
                }
                if policy.player_potion_effects {
                    g.effects.list = gs.effects.clone();
                }
                if policy.player_recipe_book {
                    g.recipe_book = gs.recipe_book.clone();
                    g.send(&C::RecipeBookSettings(recipes::RecipeBookSettings {
                        books: std::array::from_fn(|idx| {
                            (g.recipe_book.open[idx], g.recipe_book.filtering[idx])
                        }),
                    }));
                }
                g.advancements = gs.advancements.clone();
                g.advancements_visible.clear();
                g.advancements_changed.clear();
                g.dead = false;
                g.death_time = 0;
                g.sent_health = None;
                g.sent_xp = None;
                g.awaiting_teleport = None;
                g.send(&C::SetHealth(play::SetHealth {
                    health: g.health,
                    food: g.food,
                    saturation: g.saturation,
                }));
                g.send(&C::GameEvent(play::GameEvent {
                    event: 3,
                    param: gs.game_mode as f32,
                }));
                g.send(&C::PlayerAbilities(play::PlayerAbilities {
                    flags: g.ability_flags(),
                    flying_speed: 0.05,
                    walking_speed: 0.1,
                }));
                g.seen_credits = gs.seen_credits;
                g.saved = gs.saved.clone();
                g.loaded_body = Some(SavedBody {
                    fire_ticks: gs.fire_ticks,
                    air: gs.air,
                    fall_distance: gs.fall_distance,
                });
                if let Some(m) = g.motion.as_mut() {
                    let is_invuln = gs.game_mode == 1 || gs.game_mode == 3;
                    let is_spectator = gs.game_mode == 3;
                    m.health.dead = false;
                    m.invulnerable = is_invuln;
                    m.may_fly = is_invuln;
                    m.spectator = is_spectator;
                    m.flying = gs.flying;
                    m.health.hurt.no_action_time = 0;
                    m.health.hurt.cooldown = 0;
                    m.health.hurt.last_hurt = 0.0;
                    m.health.hurt.hurt_duration = 0;
                    m.health.hurt.hurt_time = 0;
                    m.health.hurt.last_source = None;
                    m.health.hurt.last_stamp = 0;
                    m.health.hurt_animation_time = 0;
                    m.health.hurt_dir = 0.0;
                    m.health.indicated_dir = None;
                    if policy.player_position {
                        m.position = Vec3::new(gs.position[0], gs.position[1], gs.position[2]);
                        m.yaw = gs.yaw;
                        m.pitch = gs.pitch;
                        m.ground.on_ground = gs.on_ground;
                    }
                    if policy.player_motion {
                        m.velocity = gs.velocity;
                        m.fall_distance = gs.fall_distance;
                    }
                    if policy.player_health {
                        m.fire_ticks = gs.fire_ticks;
                        m.air = gs.air;
                        m.ticks_frozen = gs.ticks_frozen;
                        let _ = m.set_health(gs.health.max(1.0), false);
                        let _ = m.internal_set_absorption(gs.absorption);
                        m.health.hurt_animation_time = 0;
                    }
                    if policy.player_hunger {
                        m.health.food.food = gs.food;
                        m.health.food.saturation = gs.saturation;
                        m.health.food.exhaustion = gs.exhaustion;
                    }
                }
                if policy.player_inventory {
                    g.send(&C::SetHeldSlot(play::SetHeldSlot {
                        slot: gs.selected as i32,
                    }));
                }

                let active_snap_chunks = snap.levels.get(&game.dim).map(|l| &l.chunks);
                let forget: Vec<_> = g.sent_chunks.iter().copied().collect();
                for (x, z) in forget {
                    let in_snap = active_snap_chunks.is_some_and(|c| c.contains_key(&(x, z)));
                    if !in_snap {
                        g.sent_chunks.remove(&(x, z));
                        g.send(&C::ForgetLevelChunk(play::ForgetLevelChunk { pos: [x, z] }));
                    }
                }

                let dragon_bar_id = horizon_client::bossbar::stable_uuid("minecraft:ender_dragon");
                g.send(&C::BossEvent(play::BossEvent {
                    id: dragon_bar_id,
                    action: play::BossAction::Remove,
                }));
            } // конец заимствования g

            if let Some(play_state) = host.play.get_mut(&conn) {
                if policy.player_cooldowns {
                    play_state.shield_cooldown = gs.shield_cooldown;
                    play_state.cooldowns = gs.cooldowns.clone();
                }
                if policy.player_stats {
                    play_state.stats.values = gs.stats.clone();
                    play_state.stats.mark_all_dirty();
                }
            }

            if policy.player_abilities {
                host.hub.set_game_mode(conn, gs.game_mode, &game.world, Some(game.collision));
            }
            if policy.player_position {
                if dim_changed {
                    let target_dim = Dim::from_id(&gs.dimension);
                    game.level_teleports
                        .push(super::super::server_travel::LevelTeleport {
                            conn,
                            dim: target_dim,
                            at: gs.position,
                            rotation: Some([gs.yaw, gs.pitch]),
                        });
                } else {
                    host.hub.teleport(gi, Some(gs.position));
                }
            }
            if policy.player_inventory {
                host.hub.send_inventory(gi);
            }
            if policy.player_potion_effects {
                host.hub.send_active_effects(gi);
            }
            if let Some(hs) = &gs.hook {
                game.spawn_remote_hook(
                    hs.id,
                    Vec3::new(hs.position[0], hs.position[1], hs.position[2]),
                    Vec3::ZERO,
                    ent_id,
                );
            } else if let Some(h) = game.hook.take() {
                game.remove_remote_hook(h.id());
            }

            host.hub.flush_progress(conn);
            } else if !policy.preserve_new_player_inventory {
                let g = &mut host.hub.guests[gi];
                g.inventory = vec![None; 36];
                g.armor = [None, None, None, None];
                g.offhand = None;
                g.ender_items = vec![None; 27];
                g.carried = None;
                host.hub.send_inventory(gi);
            }
        }
        if !game.level_teleports.is_empty() {
            game.run_level_teleports(&mut host);
        }
        game.lan = Some(host);
    }

    if let Some(gs) = snap.guests.values().next() {
        let is_invuln = gs.game_mode == 1 || gs.game_mode == 3;
        let is_spectator = gs.game_mode == 3;
        game.set_mode_id(gs.game_mode);
        if let Some(p) = game.player.as_mut() {
            let _ = p.set_health(gs.health.max(1.0), false);
            p.health.dead = false;
            p.invulnerable = is_invuln;
            p.may_fly = is_invuln;
            p.spectator = is_spectator;
            p.flying = gs.flying;
            p.fall_distance = 0.0;
            p.ticks_frozen = gs.ticks_frozen;
            p.health.hurt.no_action_time = 0;
            p.health.hurt.cooldown = 0;
            p.health.hurt.last_hurt = 0.0;
            p.health.hurt.hurt_duration = 0;
            p.health.hurt.hurt_time = 0;
            p.health.hurt.last_source = None;
            p.health.hurt.last_stamp = 0;
            p.health.hurt_animation_time = 0;
            p.health.hurt_dir = 0.0;
            p.health.indicated_dir = None;
        }
    }

    game.ui.death_time = 0;
    game.ui.died_at = None;
    game.ui.death_confirm = None;
    game.screen = super::super::Screen::Playing;
    game.boss_overlay.reset();
    if policy.server_bossbars {
        game.publish_boss_bars();
    }

    let player_ms = (std::time::Instant::now() - t_player).as_secs_f64() * 1000.0;
    let total_ms = (std::time::Instant::now() - t_start).as_secs_f64() * 1000.0;

    with_state_mut(|st| {
        st.last_save_tick = snap.game_time;
        st.pending_death_rollback = None;
        st.current_intensity = 0.0;
        st.peak_intensity = 0.0;
        st.relax_ticks = 0;
        st.last_benchmark = Some(RollbackBenchmark {
            total_ms,
            globals_ms,
            blocks_ms: acc_blocks_ms,
            containers_ms: acc_containers_ms,
            entities_ms: acc_entities_ms,
            player_ms,
        });
    });
    true
}
