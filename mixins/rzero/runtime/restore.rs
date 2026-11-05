use super::super::{
    Dim, Game,
};
use super::persistence::{
    hydrate_slot_from_disk_on_server, load_checkpoint_from_disk, load_slot_from_disk,
    restore_disk_files_for_slot, restore_disk_files_for_slot_opts, restore_disk_files_on_rollback,
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
                    if let (Some(u1), Some(u2)) = (snap_sec.blocks.uniform(), live_sec.blocks.uniform()) {
                        if u1 == u2 {
                            continue;
                        }
                    }
                    if snap_sec.blocks == live_sec.blocks {
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
        for (&(cx, cz), live_chunk) in old_chunks.iter() {
            if !snap.chunks.contains_key(&(cx, cz)) {
                if let Some(disk_chunk) =
                    game.save.load(cx, cz, game.pack.assets.registry, game.res.biomes())
                {
                    let min_y = disk_chunk.min_y();
                    for (s_idx, disk_sec) in disk_chunk.sections.iter().enumerate() {
                        let Some(live_sec) = live_chunk.sections.get(s_idx) else {
                            continue;
                        };
                        if disk_sec.blocks == live_sec.blocks {
                            continue;
                        }
                        let y_base = min_y + (s_idx as i32) * 16;
                        let x_base = cx * 16;
                        let z_base = cz * 16;
                        for sy in 0..16 {
                            for sz in 0..16 {
                                for sx in 0..16 {
                                    let idx = ((sy << 8) | (sz << 4) | sx) as usize;
                                    let want = disk_sec.get(idx);
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
        }

        bench_add_chunks(snap.chunks.len());
        bench_add_blocks_changed(changed_coords.len());

        if policy.blocks {
            game.world.block_log_held.clear();
            for (wx, wy, wz, want) in changed_coords {
                let _ = game.world.set(wx, wy, wz, BlockStateId(want));
            }
        }
        bench_accum(Phase::ChunkBlocks, t_blocks);

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
        let mut be_positions: HashSet<(i32, i32, i32)> =
            game.world.block_entities.keys().copied().collect();
        be_positions.extend(snap.block_entities.keys().copied());
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
        bench_accum(Phase::ChunkBeLoad, t_containers);
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
        bench_add_mobs(snap.mobs.len());
        game.mobs.clone_from(&snap.mobs);
        if policy.dropped_items {
            game.dropped.clone_from(&snap.dropped);
        }
        if policy.experience_orbs {
            game.orbs.clone_from(&snap.orbs);
        }
        game.creatures.projectiles.clone_from(&snap.projectiles);
        game.boats.clone_from(&snap.boats);
        game.carts.clone_from(&snap.carts);
        game.tnts.clone_from(&snap.tnts);
        game.falling = snap.falling.iter().map(FallingSnapshot::to_live).collect::<Vec<_>>().into();
        game.paintings.clone_from(&snap.paintings);
        game.item_frames.clone_from(&snap.item_frames);
        game.armor_stands.clone_from(&snap.armor_stands);
        game.cushions.clone_from(&snap.cushions);
        game.others.clone_from(&snap.others);
        game.crystals.clone_from(&snap.crystals);
        game.dragon = snap
            .dragon
            .as_ref()
            .and_then(|d| d.to_live(game.level.seed));
        game.knot_ids.clone_from(&snap.knot_ids);
        game.persist.uuids.clone_from(&snap.persist_uuids);
        game.persist.raw.clone_from(&snap.persist_raw);
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

    bench_accum(Phase::SpawnNbt, t_entities);
    let entities_ms = (std::time::Instant::now() - t_entities).as_secs_f64() * 1000.0;
    (blocks_ms, containers_ms, entities_ms)
}

pub fn apply_client_level_rollback(game: &mut Game<'_>, slot: &str) {
    let t_start = std::time::Instant::now();
    let _ = game.stop_remote_mining();

    let client_snap = match get_client_slot_snapshot_clone(slot) {
        Some(s) => s,
        None => {
            let (px, pz) = game
                .player
                .as_ref()
                .map(|p| ((p.position.x as i32) >> 4, (p.position.z as i32) >> 4))
                .unwrap_or((0, 0));

            let mut candidate_chunks = game.world.modified.clone();
            for dz in -3..=3 {
                for dx in -3..=3 {
                    candidate_chunks.insert((px + dx, pz + dz));
                }
            }

            let mut diffs = Vec::new();
            for (cx, cz) in candidate_chunks {
                if let Some(live_chunk) = game.world.chunks.get(&(cx, cz)) {
                    if let Some(disk_chunk) = game.save.load(cx, cz, game.pack.assets.registry, game.res.biomes()) {
                        let min_y = disk_chunk.min_y();
                        for (s_idx, disk_sec) in disk_chunk.sections.iter().enumerate() {
                            let Some(live_sec) = live_chunk.sections.get(s_idx) else {
                                continue;
                            };
                            if disk_sec.blocks == live_sec.blocks {
                                continue;
                            }
                            let y_base = min_y + (s_idx as i32) * 16;
                            let x_base = cx * 16;
                            let z_base = cz * 16;
                            for sy in 0..16 {
                                for sz in 0..16 {
                                    for sx in 0..16 {
                                        let idx = ((sy << 8) | (sz << 4) | sx) as usize;
                                        let want = disk_sec.get(idx);
                                        let current = live_sec.get(idx);
                                        if current != want {
                                            diffs.push((x_base + sx, y_base + sy, z_base + sz, current, want));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            let dirty_before = game.world.dirty_sections.len();
            for &(x, y, z, _, want) in &diffs {
                let _ = game.world.set(x, y, z, BlockStateId(want));
            }
            let dirty_marked = game.world.dirty_sections.len().saturating_sub(dirty_before);
            let elapsed_ms = (t_start.elapsed().as_nanos() as f64) / 1_000_000.0;

            let mut report = format!(
                "[RZero][Client-Spark] === CLIENT ROLLBACK [{slot}]: {elapsed_ms:.2} ms ===\n\
                 |-- Source: Disk Fallback (RAM Cache MISS)\n\
                 |-- Blocks Changed ({}):",
                diffs.len()
            );
            for &(x, y, z, old_id, new_id) in diffs.iter().take(10) {
                let old_name = game.res.facts().get(BlockStateId(old_id)).name.clone();
                let new_name = game.res.facts().get(BlockStateId(new_id)).name.clone();
                report.push_str(&format!("\n|   |-- [{x}, {y}, {z}]: {old_name} ({old_id}) -> {new_name} ({new_id})"));
            }
            if diffs.len() > 10 {
                report.push_str(&format!("\n|   |-- ... and {} more", diffs.len() - 10));
            }
            report.push_str(&format!("\n|-- Dirty Sections Marked: {dirty_marked}"));
            eprintln!("{report}");
            log::info!("{report}");
            append_to_rzero_log(&game.world_root, &report);
            return;
        }
    };

    if game.dim != client_snap.dim {
        let elapsed_ms = (t_start.elapsed().as_nanos() as f64) / 1_000_000.0;
        let report = format!(
            "[RZero][Client-Spark] === CLIENT ROLLBACK [{slot}]: {elapsed_ms:.2} ms ===\n\
             |-- Status: Dimension Mismatch (game: {}, snap: {})\n\
             |-- Blocks Changed: 0",
            game.dim.id(), client_snap.dim.id()
        );
        eprintln!("{report}");
        log::info!("{report}");
        append_to_rzero_log(&game.world_root, &report);
        return;
    }

    let old_chunks = game.world.chunks.clone();
    let chunks_total = client_snap.chunks.len();
    let mut chunks_skipped_ptr = 0usize;
    let mut chunks_evaluated = 0usize;
    let mut sections_total = 0usize;
    let mut sections_skipped_ptr = 0usize;
    let mut sections_skipped_uniform = 0usize;
    let mut sections_skipped_eq = 0usize;
    let mut sections_evaluated = 0usize;
    let mut changed_coords = Vec::new();

    let (px, pz) = game
        .player
        .as_ref()
        .map(|p| ((p.position.x as i32) >> 4, (p.position.z as i32) >> 4))
        .unwrap_or((0, 0));

    let mut chunk_keys: Vec<(i32, i32)> = client_snap.chunks.keys().copied().collect();
    chunk_keys.sort_unstable_by_key(|&(cx, cz)| (cx - px).abs().max((cz - pz).abs()));

    let t_scan = std::time::Instant::now();
    for &(cx, cz) in &chunk_keys {
        let snap_chunk = &client_snap.chunks[&(cx, cz)];
        if let Some(live_chunk) = old_chunks.get(&(cx, cz)) {
            if Arc::ptr_eq(live_chunk, snap_chunk) {
                chunks_skipped_ptr += 1;
                continue;
            }
            chunks_evaluated += 1;
            let min_y = snap_chunk.min_y();
            for (s_idx, snap_sec) in snap_chunk.sections.iter().enumerate() {
                sections_total += 1;
                let Some(live_sec) = live_chunk.sections.get(s_idx) else {
                    continue;
                };
                if Arc::ptr_eq(live_sec, snap_sec) {
                    sections_skipped_ptr += 1;
                    continue;
                }
                if let (Some(u1), Some(u2)) = (snap_sec.blocks.uniform(), live_sec.blocks.uniform()) {
                    if u1 == u2 {
                        sections_skipped_uniform += 1;
                        continue;
                    }
                }
                if snap_sec.blocks == live_sec.blocks {
                    sections_skipped_eq += 1;
                    continue;
                }
                sections_evaluated += 1;

                let y_base = min_y + (s_idx as i32) * 16;
                let x_base = cx * 16;
                let z_base = cz * 16;
                for sy in 0..16 {
                    for sz in 0..16 {
                        for sx in 0..16 {
                            let idx = ((sy << 8) | (sz << 4) | sx) as usize;
                            let want = snap_sec.get(idx);
                            let current = live_sec.get(idx);
                            if current != want {
                                changed_coords.push((
                                    x_base + sx,
                                    y_base + sy,
                                    z_base + sz,
                                    current,
                                    want,
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    let scan_ms = (t_scan.elapsed().as_nanos() as f64) / 1_000_000.0;

    let t_set = std::time::Instant::now();
    let dirty_before = game.world.dirty_sections.len();
    for &(wx, wy, wz, _, want) in &changed_coords {
        let _ = game.world.set(wx, wy, wz, BlockStateId(want));
    }
    let dirty_marked = game.world.dirty_sections.len().saturating_sub(dirty_before);
    let set_ms = (t_set.elapsed().as_nanos() as f64) / 1_000_000.0;
    let elapsed_ms = (t_start.elapsed().as_nanos() as f64) / 1_000_000.0;

    sync_client_epoch();

    let mut report = format!(
        "[RZero][Client-Spark] === CLIENT ROLLBACK [{slot}]: {elapsed_ms:.2} ms ===\n\
         |-- Phase 1 (Cache Lookup): {:.2} ms [RAM HIT | dim: {}]\n\
         |-- Phase 2 (Chunks & Sections Scan): {scan_ms:.2} ms\n\
         |   |-- Chunks: {chunks_total} total | {chunks_skipped_ptr} ptr_eq | {chunks_evaluated} eval (nearest-first)\n\
         |   |-- Sections: {sections_total} in eval | {sections_skipped_ptr} ptr_eq | {sections_skipped_uniform} uniform | {sections_skipped_eq} content_eq | {sections_evaluated} eval\n\
         |-- Phase 3 (Block World Sets): {set_ms:.2} ms (blocks changed: {})\n\
         |-- Phase 4 (Mesher Invalidation): {dirty_marked} dirty sections marked",
        (t_scan.duration_since(t_start).as_nanos() as f64) / 1_000_000.0,
        game.dim.id(),
        changed_coords.len()
    );
    if !changed_coords.is_empty() {
        report.push_str("\n|-- Changed Blocks Detail:");
        for &(x, y, z, old_id, new_id) in changed_coords.iter().take(10) {
            let old_name = game.res.facts().get(BlockStateId(old_id)).name.clone();
            let new_name = game.res.facts().get(BlockStateId(new_id)).name.clone();
            report.push_str(&format!("\n|   |-- [{x}, {y}, {z}]: {old_name} ({old_id}) -> {new_name} ({new_id})"));
        }
        if changed_coords.len() > 10 {
            report.push_str(&format!("\n|   |-- ... and {} more", changed_coords.len() - 10));
        }
    }
    eprintln!("{report}");
    log::info!("{report}");
    append_to_rzero_log(&game.world_root, &report);
}

pub fn rollback_checkpoint_on_server(game: &mut Game<'_>) -> bool {
    let slot = with_state(|st| st.current_slot.clone());
    rollback_slot_checkpoint_on_server(game, &slot)
}

pub fn rollback_slot_checkpoint_on_server(game: &mut Game<'_>, slot: &str) -> bool {
    let t_start = std::time::Instant::now();
    bench_begin(&format!("ROLLBACK RESTORE [{}]", slot), game.world.game_time);

    let t_flush = std::time::Instant::now();
    let _ = game.save.flush();
    for parked in game.levels.parked.values() {
        let _ = parked.save.flush();
    }
    game.world.loaded_entities.clear();
    bench_accum(Phase::FlushSave, t_flush);

    let t_hydrate = std::time::Instant::now();
    let mut snap = match get_slot_snapshot_clone(slot) {
        Some(s) => s,
        None => match hydrate_slot_from_disk_on_server(game, slot) {
            Some(s) => s,
            None => return false,
        },
    };

    let target_dim = snap
        .guests
        .values()
        .next()
        .map(|g| Dim::from_id(&g.dimension))
        .unwrap_or_else(|| {
            snap.levels.keys().copied().next().unwrap_or(Dim::Overworld)
        });

    if !snap.levels.contains_key(&target_dim) {
        if let Some(hydrated) = hydrate_slot_from_disk_on_server(game, slot) {
            snap = hydrated;
        }
    }
    bench_accum(Phase::HydrateSnapshot, t_hydrate);

    let need_force_disk = !snap.disk_restored;
    let is_hot_rollback = slot == "default"
        && game.dim == target_dim
        && snap.levels.contains_key(&game.dim)
        && !need_force_disk;
    let t_disk = std::time::Instant::now();
    let disk_files_changed =
        restore_disk_files_for_slot_opts(&game.world_root, slot, need_force_disk);
    bench_accum(Phase::RestoreDiskFiles, t_disk);

    let t_savedir = std::time::Instant::now();
    if need_force_disk || !is_hot_rollback || disk_files_changed > 0 {
        let new_save = horizon_client::save::SaveDir::new(
            game.save.root.clone(),
            game.save.shape,
        );
        if let Some(tables) = game.save.tables.get() {
            let _ = new_save.tables.set(Arc::clone(tables));
        } else {
            let tables = Arc::new(horizon_client::save::SaveTables::new(&game.pack.assets));
            let _ = new_save.tables.set(tables);
        }
        game.save = Arc::new(new_save);
    }
    bench_accum(Phase::SaveDirStreaming, t_savedir);

    bench_add_players(snap.guests.len());
    let policy = &snap.policy;

    let t_globals = std::time::Instant::now();
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
        game.ui.clock.rate = snap.time_rate;
        game.ui.clock.paused = snap.clock_frozen;
    }
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
    if policy.world_scoreboard {
        game.score.board = snap.scoreboard.clone();
    }
    bench_accum(Phase::ServerGlobals, t_globals);

    let t_bosses = std::time::Instant::now();
    if policy.server_bossbars {
        let bars = BossBars::unpack(&snap.bossbars);
        game.bosses = Default::default();
        game.bosses.bars = bars;
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
    bench_accum(Phase::Bosses, t_bosses);

    let t_mca = std::time::Instant::now();
    let subfolders = ["region", "entities", "poi"];
    let dims = [
        "dimensions/minecraft/overworld",
        "dimensions/minecraft/the_nether",
        "dimensions/minecraft/the_end",
    ];
    let mut deleted_region_files = false;
    for dim_path in dims {
        for sub in subfolders {
            let rdir = game.world_root.join(dim_path).join(sub);
            if let Ok(entries) = fs::read_dir(rdir) {
                for e in entries.flatten() {
                    let p = e.path();
                    if p.is_file() && !snap.region_files.contains(&p) {
                        let _ = fs::remove_file(&p);
                        deleted_region_files = true;
                    }
                }
            }
        }
    }
    bench_accum(Phase::FutureMca, t_mca);

    if target_dim != game.dim || deleted_region_files || disk_files_changed > 0 || !is_hot_rollback {
        restore_offline_files(&snap.offline_player_data, policy.player_stats);
        game.streaming.wait(std::time::Duration::from_millis(2));
        if let Ok((fresh_streaming, fresh_save, _)) = super::super::world_loading::make_streaming(
            &game.res,
            &game.pack,
            &game.options,
            &game.settings,
            &game.world_root,
            target_dim,
            game.world.shape,
            game.level.seed,
            &game.level.enabled_features,
        ) {
            if let Some(tables) = game.save.tables.get() {
                let _ = fresh_save.tables.set(Arc::clone(tables));
            } else {
                let tables = Arc::new(horizon_client::save::SaveTables::new(&game.pack.assets));
                let _ = fresh_save.tables.set(tables);
            }
            game.streaming = fresh_streaming;
            game.save = fresh_save;
        }
    }

    bench_accum(Phase::ServerGlobals, t_globals);
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
    } else {
        let (px, pz) = game
            .player
            .as_ref()
            .map(|p| ((p.position.x as i32) >> 4, (p.position.z as i32) >> 4))
            .or_else(|| {
                game.lan
                    .as_ref()
                    .and_then(|h| h.hub.guests.first())
                    .map(|g| ((g.position[0] as i32) >> 4, (g.position[2] as i32) >> 4))
            })
            .or_else(|| {
                snap.guests.values().next().map(|g| ((g.position[0] as i32) >> 4, (g.position[2] as i32) >> 4))
            })
            .unwrap_or((0, 0));

        let mut candidate_chunks = game.world.modified.clone();
        for dz in -3..=3 {
            for dx in -3..=3 {
                candidate_chunks.insert((px + dx, pz + dz));
            }
        }

        for (cx, cz) in candidate_chunks {
            if let Some(live_chunk) = game.world.chunks.get(&(cx, cz)) {
                if let Some(disk_chunk) = game.save.load(cx, cz, game.pack.assets.registry, game.res.biomes()) {
                    let min_y = disk_chunk.min_y();
                    let mut chunk_diff = false;
                    for (s_idx, disk_sec) in disk_chunk.sections.iter().enumerate() {
                        let Some(live_sec) = live_chunk.sections.get(s_idx) else {
                            continue;
                        };
                        if disk_sec.blocks == live_sec.blocks {
                            continue;
                        }
                        chunk_diff = true;
                        let y_base = min_y + (s_idx as i32) * 16;
                        let x_base = cx * 16;
                        let z_base = cz * 16;
                        for sy in 0..16 {
                            for sz in 0..16 {
                                for sx in 0..16 {
                                    let idx = ((sy << 8) | (sz << 4) | sx) as usize;
                                    let want = disk_sec.get(idx);
                                    if live_sec.get(idx) != want {
                                        let pos = (x_base + sx, y_base + sy, z_base + sz);
                                        if let Some(log) = game.world.block_log.as_mut() {
                                            log.push(pos);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if chunk_diff {
                        game.world.chunks.insert((cx, cz), Arc::new(disk_chunk));
                    }
                }
            }
        }
        game.world.modified.clear();
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
                g.boarding_cooldown = gs.boarding_cooldown;
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
                    velocity: [gs.velocity.x, gs.velocity.y, gs.velocity.z],
                    food_exhaustion: gs.exhaustion,
                    food_tick_timer: 0,
                    ticks_frozen: gs.ticks_frozen,
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

                if let Some(active_snap_chunks) = snap.levels.get(&game.dim).map(|l| &l.chunks) {
                    let forget: Vec<_> = g.sent_chunks.iter().copied().collect();
                    for (x, z) in forget {
                        let in_snap = active_snap_chunks.contains_key(&(x, z));
                        if !in_snap {
                            g.sent_chunks.remove(&(x, z));
                            g.send(&C::ForgetLevelChunk(play::ForgetLevelChunk { pos: [x, z] }));
                        }
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
                            vehicle: None,
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
        host.hub.rzero_reset_tracked();
        host.hub.synced.clear();
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

    bench_accum(Phase::PlayersOnline, t_player);
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

    let t_cs = std::time::Instant::now();
    if need_force_disk || !snap.levels.contains_key(&target_dim) {
        let mut updated = (*snap).clone();
        updated.disk_restored = true;
        if !updated.levels.contains_key(&target_dim) {
            updated
                .levels
                .insert(target_dim, capture_level_snapshot(game, target_dim));
        }
        set_slot_snapshot(slot, Arc::new(updated));
    }

    let client_snap = ClientBlockSnapshot {
        dim: game.dim,
        chunks: game.world.chunks.clone(),
    };
    set_client_slot_snapshot(slot, client_snap);
    bump_rollback_epoch();
    bench_accum(Phase::ClientSnapUpdate, t_cs);

    let bench_report = bench_end_and_log();
    log::info!("{}", bench_report);
    append_to_rzero_log(&game.world_root, &bench_report);

    if let Ok(mut tr) = loop_tracker().write() {
        tr.on_rollback(snap.game_time);
    }

    true
}
