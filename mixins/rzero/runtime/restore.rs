use super::super::{
    Dim, Game,
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
) {
    let old_chunks = game.world.chunks.clone();

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
}

pub fn rollback_checkpoint_on_server(game: &mut Game<'_>) -> bool {
    let Some(snap) = get_snapshot_clone() else {
        return false;
    };
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
    }
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
        game.bosses.bars = BossBars::unpack(&snap.bossbars);
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

    restore_offline_files(&snap.offline_player_data, policy.player_stats);

    let start_dim = game.dim;
    let mut dims: Vec<Dim> = snap.levels.keys().copied().collect();
    if !dims.contains(&start_dim) {
        dims.push(start_dim);
    }

    for dim in dims {
        let Some(level_snap) = snap.levels.get(&dim) else {
            continue;
        };
        if dim == game.dim {
            apply_level_snapshot_in_place(game, dim, level_snap, policy);
        } else if game.enter_level(None, dim).is_ok() {
            apply_level_snapshot_in_place(game, dim, level_snap, policy);
        }
    }
    if game.dim != start_dim {
        let _ = game.enter_level(None, start_dim);
    }

    if let Some(mut host) = game.lan.take() {
        if policy.server_world_border {
            host.border_sent = None;
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

            if let Some(gs) = snap.guests.get(&uuid) {
                let g = &mut host.hub.guests[gi];
                let dim_changed = policy.player_position && g.dimension != gs.dimension;
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
                    g.carried = None;
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
                g.seen_credits = gs.seen_credits;
                g.saved = gs.saved.clone();
                g.loaded_body = Some(SavedBody {
                    fire_ticks: gs.fire_ticks,
                    air: gs.air,
                    fall_distance: gs.fall_distance,
                });
                if let Some(m) = g.motion.as_mut() {
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
                    host.hub.set_game_mode(conn, gs.game_mode);
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

    if policy.server_bossbars {
        game.publish_boss_bars();
    }

    with_state_mut(|st| {
        st.last_save_tick = snap.game_time;
        st.pending_death_rollback = None;
        st.current_intensity = 0.0;
        st.peak_intensity = 0.0;
        st.relax_ticks = 0;
    });
    true
}
