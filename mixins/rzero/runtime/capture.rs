use super::super::{
    Dim, Game,
};
use super::persistence::{save_checkpoint_to_disk, save_slot_to_disk};
use super::snapshots::*;
use super::types::*;
use horizon_client::{
    chunk::Chunk,
    cooldowns::Cooldowns,
    fast_hash::FxHashMap,
    lan::guest::SavedBody,
};
use horizon_core::{
    geometry::Vec3,
    position::ChunkPos,
    ticks::ChunkTicks,
};
use horizon_world::level::Ticks;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

pub fn capture_chunk_ticks(
    chunks: &FxHashMap<(i32, i32), Arc<Chunk>>,
    ticks: &Ticks,
) -> HashMap<(i32, i32), (ChunkTicks, ChunkTicks)> {
    let mut out = HashMap::new();
    for &(cx, cz) in chunks.keys() {
        let cpos = ChunkPos::new(cx, cz);
        let b = ticks.blocks.chunk(cpos).cloned().unwrap_or_default();
        let f = ticks.fluids.chunk(cpos).cloned().unwrap_or_default();
        out.insert((cx, cz), (b, f));
    }
    out
}

pub fn backup_offline_files(world_root: &Path) -> HashMap<PathBuf, Vec<u8>> {
    let mut map = HashMap::new();
    fn walk_dir(dir: &Path, map: &mut HashMap<PathBuf, Vec<u8>>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    walk_dir(&p, map);
                } else if p.is_file() {
                    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if ext == "dat" || ext == "json" {
                        if let Ok(bytes) = fs::read(&p) {
                            map.insert(p, bytes);
                        }
                    }
                }
            }
        }
    }
    walk_dir(&world_root.join("data"), &mut map);
    walk_dir(&world_root.join("dimensions"), &mut map);
    walk_dir(&world_root.join("players"), &mut map);
    map
}

pub fn capture_level_snapshot(game: &Game<'_>, dim: Dim) -> LevelSnapshot {
    let raids_dat = fs::read(raids_dat_path(&game.world_root, dim)).ok();
    LevelSnapshot {
        chunks: game.world.chunks.clone(),
        block_entities: (&*game.world.block_entities).clone(),
        block_names: game.world.block_names.clone(),
        inhabited: game.world.inhabited.clone(),
        random: game.world.random.clone(),
        forced_chunks: game.streaming.forced.clone(),
        chunk_ticks: capture_chunk_ticks(&game.world.chunks, &game.block_ticks),
        random_tick_value: game.block_ticks.random_tick_value,
        comparator_outputs: game.block_ticks.comparator_outputs.clone(),
        torch_toggles: game.block_ticks.torch_toggles.clone(),
        moving_pistons: game.block_ticks.moving_pistons.clone(),
        piston_pushes: game.block_ticks.piston_pushes.clone(),
        block_events: game.block_ticks.block_events.clone(),
        hoppers: game.hoppers.clone(),
        mobs: game.mobs.clone(),
        dropped: game.dropped.clone(),
        orbs: game.orbs.clone(),
        projectiles: game.creatures.projectiles.clone(),
        boats: game.boats.clone(),
        carts: game.carts.clone(),
        tnts: game.tnts.clone(),
        falling: game.falling.iter().map(FallingSnapshot::from_live).collect(),
        paintings: game.paintings.clone(),
        item_frames: game.item_frames.clone(),
        armor_stands: game.armor_stands.clone(),
        cushions: game.cushions.clone(),
        others: game.others.clone(),
        crystals: game.crystals.clone(),
        dragon: game.dragon.as_ref().map(DragonSnapshot::from_live),
        knot_ids: game.knot_ids.clone(),
        persist_uuids: game.persist.uuids.clone(),
        persist_raw: game.persist.raw.clone(),
        raids_dat,
        last_hurt_by_player: game.last_hurt_by_player.clone(),
        love_cause: game.love_cause.clone(),
    }
}

pub fn capture_parked_snapshot(world_root: &Path, dim: Dim, p: &super::Parked) -> LevelSnapshot {
    let raids_dat = fs::read(raids_dat_path(world_root, dim)).ok();
    LevelSnapshot {
        chunks: p.world.chunks.clone(),
        block_entities: (&*p.world.block_entities).clone(),
        block_names: p.world.block_names.clone(),
        inhabited: p.world.inhabited.clone(),
        random: p.world.random.clone(),
        forced_chunks: p.streaming.forced.clone(),
        chunk_ticks: capture_chunk_ticks(&p.world.chunks, &p.block_ticks),
        random_tick_value: p.block_ticks.random_tick_value,
        comparator_outputs: p.block_ticks.comparator_outputs.clone(),
        torch_toggles: p.block_ticks.torch_toggles.clone(),
        moving_pistons: p.block_ticks.moving_pistons.clone(),
        piston_pushes: p.block_ticks.piston_pushes.clone(),
        block_events: p.block_ticks.block_events.clone(),
        hoppers: p.hoppers.clone(),
        mobs: p.mobs.clone(),
        dropped: p.dropped.clone(),
        orbs: p.orbs.clone(),
        projectiles: p.projectiles.clone(),
        boats: p.boats.clone(),
        carts: p.carts.clone(),
        tnts: p.tnts.clone(),
        falling: p.falling.iter().map(FallingSnapshot::from_live).collect(),
        paintings: p.paintings.clone(),
        item_frames: p.item_frames.clone(),
        armor_stands: p.armor_stands.clone(),
        cushions: p.cushions.clone(),
        others: p.others.clone(),
        crystals: p.crystals.clone(),
        dragon: p.dragon.as_ref().map(DragonSnapshot::from_live),
        knot_ids: p.knot_ids.clone(),
        persist_uuids: p.persist.uuids.clone(),
        persist_raw: p.persist.raw.clone(),
        raids_dat,
        last_hurt_by_player: HashMap::new(),
        love_cause: HashMap::new(),
    }
}

pub fn create_checkpoint_on_server(game: &mut Game<'_>, requester: Option<u128>) {
    create_slot_checkpoint_on_server(game, requester, "default");
}

pub fn create_slot_checkpoint_on_server(game: &mut Game<'_>, requester: Option<u128>, slot: &str) {
    let _ = game.save.flush();
    game.save_raids();
    game.save_fight();
    game.save_wandering_trader();

    let mut guests = HashMap::new();
    let mut online = Vec::new();
    if let Some(host) = &game.lan {
        for g in host.hub.guests.iter().filter(|g| g.placed) {
            online.push(g.uuid);
            let motion = g.motion.as_deref();
            let (velocity, absorption, exhaustion, fire_ticks, air, fall_distance, ticks_frozen) = match motion {
                Some(m) => (
                    m.velocity,
                    m.health.absorption(),
                    m.health.food.exhaustion,
                    m.fire_ticks,
                    m.air,
                    m.fall_distance,
                    m.ticks_frozen,
                ),
                None => {
                    let body = g.loaded_body.unwrap_or(SavedBody {
                        fire_ticks: -20,
                        air: 300,
                        fall_distance: 0.0,
                    });
                    (
                        Vec3::ZERO,
                        g.saved_absorption.unwrap_or(0.0),
                        0.0,
                        body.fire_ticks,
                        body.air,
                        body.fall_distance,
                        0,
                    )
                }
            };
            let (shield_cooldown, cooldowns, stats, hook) = match host.play.get(&g.conn) {
                Some(p) => (
                    p.shield_cooldown,
                    p.cooldowns.clone(),
                    p.stats.values.clone(),
                    p.hook.as_ref().map(|h| HookSnapshot {
                        id: h.id(),
                        position: [h.position.x, h.position.y, h.position.z],
                    }),
                ),
                None => (0, Cooldowns::default(), BTreeMap::new(), None),
            };
            guests.insert(
                g.uuid,
                GuestSnapshot {
                    uuid: g.uuid,
                    dimension: g.dimension.clone(),
                    position: g.position,
                    velocity,
                    yaw: g.yaw,
                    pitch: g.pitch,
                    head_yaw: g.head_yaw,
                    on_ground: g.on_ground,
                    game_mode: g.game_mode,
                    flying: g.flying,
                    sprinting: g.sprinting,
                    sneaking: g.sneaking,
                    fall_flying: g.fall_flying,
                    sleeping: g.sleeping,
                    bed: g.bed,
                    sleep_timer: g.sleep_timer,
                    vehicle: g.vehicle,
                    vehicle_good: g.vehicle_good,
                    spawn: g.spawn,
                    respawn_elsewhere: g.respawn_elsewhere.clone(),
                    last_death: g.last_death.clone(),
                    health: g.health,
                    absorption,
                    food: g.food,
                    saturation: g.saturation,
                    exhaustion,
                    fire_ticks,
                    air,
                    ticks_frozen,
                    fall_distance,
                    xp_level: g.xp_level,
                    xp_progress: g.xp_progress,
                    xp_total: g.xp_total,
                    xp_seed: g.xp_seed,
                    inventory: g.inventory.clone(),
                    armor: g.armor.clone(),
                    offhand: g.offhand.clone(),
                    carried: g.carried.clone(),
                    ender_items: g.ender_items.clone(),
                    selected: g.selected,
                    craft_grid: g.craft_grid.clone(),
                    craft_result: g.craft_result.clone(),
                    effects: g.effects.list.clone(),
                    shield_cooldown,
                    cooldowns,
                    recipe_book: g.recipe_book.clone(),
                    advancements: g.advancements.clone(),
                    stats,
                    seen_credits: g.seen_credits,
                    saved: g.saved.clone(),
                    hook,
                },
            );
        }
    }

    let mut levels = HashMap::new();
    levels.insert(game.dim, capture_level_snapshot(game, game.dim));
    for (&dim, parked) in &game.levels.parked {
        levels.insert(
            dim,
            capture_parked_snapshot(&game.world_root, dim, parked),
        );
    }

    let now = game.world.game_time;
    bench_begin(&format!("CHECKPOINT CAPTURE [{}]", slot), now);
    bench_add_players(online.len());
    let (anchor_set, policy) = with_state_mut(|st| {
        let anchors: HashSet<u128> = resolve_anchors(&st.anchor, &online, now, requester)
            .into_iter()
            .collect();
        st.anchor_ids = anchors.clone();
        st.has_checkpoint = true;
        st.last_save_tick = now;
        st.current_intensity = 0.0;
        st.peak_intensity = 0.0;
        st.relax_ticks = 0;
        (anchors, st.policy)
    });

    let dragon_fight_dat = fs::read(fight_dat_path(&game.world_root)).ok();
    let trader_dat = fs::read(trader_dat_path(&game.world_root)).ok();
    let offline_player_data = backup_offline_files(&game.world_root);

    let mut region_files = HashSet::new();
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
                    if p.is_file() {
                        region_files.insert(p);
                    }
                }
            }
        }
    }

    let timeline = Arc::new(TimelineSnapshot {
        anchor_ids: anchor_set,
        policy,
        game_time: game.world.game_time,
        day_time: game.world.day_time,
        weather: game.world.weather,
        health_rules: game.world.health_rules,
        rules: game.ui.rules.clone(),
        border: game.border.clone(),
        bossbars: game.bosses.bars.pack(),
        sculk_tracker: game.sculk.tracker,
        sculk_darkness: game.sculk.darkness,
        dragon_fight_dat,
        trader_dat,
        rand_sequences: game.rand_sequences.clone(),
        region_files,
        difficulty: game.ui.difficulty,
        difficulty_locked: game.level.difficulty_locked,
        clock_rate: game.clock.rate(),
        clock_frozen: game.clock.frozen(),
        time_rate: game.ui.clock.rate,
        world_spawn: game.level.world_spawn,
        world_spawn_angle: game.level.world_spawn_angle,
        next_entity: game.next_entity,
        scoreboard: game.score.board.clone(),
        guests,
        offline_player_data,
        levels,
        mob_players: game.lan.as_ref().map(|h| h.mob_players.clone()).unwrap_or_default(),
    });

    let _ = game.save.flush();
    save_slot_to_disk(&game.world_root, slot, &timeline);
    set_slot_snapshot(slot, timeline);

    let bench_report = bench_end_and_log();
    log::info!("{}", bench_report);
    append_to_rzero_log(&game.world_root, &bench_report);
}
