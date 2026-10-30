use super::{
    capture::{EntityCapture, PoiCapture},
    data::{
        BlockEntityEntry, BlockEventEntry, CheckpointData, ChunkScope, EntityRamSnapshot,
        EntitySnapshot, OfflinePlayerFiles, PlayerData, ScheduledTickCodec, WorldSnapshot,
    },
};
use crate::{
    adaptive::AdaptiveSaveEngine,
    api::DevHooks,
    block::{BIOME_SECTION_SIZE, SECTION_SIZE},
    config::RZeroCheckpointPolicy,
    engine::EntityIdCounter,
    log_info, log_warn,
    runtime::{MobRamCache, PathRestore, RZeroRuntime, RestoreQueues, SnapshotRegistry},
    util::{Phase, RZBenchmark},
};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

static IS_RESTORING_CHUNK: AtomicBool = AtomicBool::new(false);

pub struct RollbackCoordinator;

impl RollbackCoordinator {
    const ENTITY_ROLLBACK_WINDOW: i64 = 20;

    pub fn restore_checkpoint(_requester: Option<u128>, current_tick: i64) -> bool {
        log_info("Initiating timeline rollback...");
        RZBenchmark::begin("ROLLBACK RESTORE", current_tick as i32);

        let target = SnapshotRegistry::find_timeline_origin();
        let Some((_dim, target_data)) = target else {
            log_warn("Rollback aborted: No timeline origin found in any dimension.");
            RZBenchmark::end_and_log();
            return false;
        };

        let policy = target_data.policy.clone();
        RZeroRuntime::with_mut(|rt| {
            rt.set_active_checkpoint_policy(policy.clone());
            rt.is_restoring = true;
            rt.was_restored_this_tick = true;
        });

        AdaptiveSaveEngine::restore_adaptive_state(target_data.adaptive_state);
        if policy.rollback.entities.entity_id
            && let Some(as_state) = target_data.adaptive_state
        {
            let max_snap = SnapshotRegistry::max_entity_id().max(as_state.entity_id_counter);
            EntityIdCounter::set(max_snap + 16);
        }

        DevHooks::fire_profile_start(current_tick, "restore");

        let t0 = Instant::now();
        SnapshotRegistry::clear_allowed_entities();
        MobRamCache::clear();
        SnapshotRegistry::populate_allowed_entities_from_snapshots();
        RestoreQueues::clear_for_restore();
        RZBenchmark::accum(Phase::Prepare, t0.elapsed().as_nanos() as u64);

        let (processed, queued) = SnapshotRegistry::schedule_chunk_rollbacks(
            current_tick + Self::ENTITY_ROLLBACK_WINDOW,
        );

        AdaptiveSaveEngine::reset_auto_save_timer(current_tick, current_tick);
        DevHooks::fire_profile_end(current_tick, "restore", processed, queued);
        DevHooks::fire_checkpoint_loaded(current_tick);

        RZeroRuntime::with_mut(|rt| {
            rt.is_restoring = false;
        });
        RZBenchmark::end_and_log();
        true
    }

    pub fn flush_sync_pending_restores() -> Vec<(String, i64)> {
        let mut flushed = Vec::new();
        while let Some((dim, chunk_key)) = RestoreQueues::poll_chunk_ready_for_restore() {
            flushed.push((dim.clone(), chunk_key));
            RestoreQueues::enqueue_chunk_pending_entity_restore(&dim, chunk_key);
        }
        while let Some((dim, chunk_key)) = RestoreQueues::poll_chunk_pending_entity_restore() {
            if !flushed.contains(&(dim.clone(), chunk_key)) {
                flushed.push((dim, chunk_key));
            }
        }
        flushed
    }
}

pub struct ChunkRestorer;

pub struct ChunkRestoreOutput {
    pub changed_blocks: Vec<(i32, i32, i32)>,
    pub biome_changed: bool,
    pub restored_block_entities: Vec<BlockEntityEntry>,
    pub restored_block_ticks: Vec<([i32; 3], String, i64, i32, i64)>,
    pub restored_fluid_ticks: Vec<([i32; 3], String, i64, i32, i64)>,
    pub replayed_block_events: Vec<BlockEventEntry>,
    pub inhabited_time: Option<i64>,
    pub enqueue_chunk_resend: bool,
}

impl ChunkRestorer {
    pub const RESEND_CHUNK_THRESHOLD: usize = 64;

    pub fn is_restoring_chunk() -> bool {
        IS_RESTORING_CHUNK.load(Ordering::SeqCst)
    }

    pub fn apply(data: &CheckpointData, chunk_key: i64) -> usize {
        data.section_snapshots
            .get(&chunk_key)
            .map_or(0, |s| s.iter().filter(|opt| opt.is_some()).count())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn apply_to_live(
        data: &CheckpointData,
        dim: &str,
        chunk_key: i64,
        min_section_y: i32,
        game_time: i64,
        is_during_load: bool,
        live_sections: &mut [([u32; SECTION_SIZE], [u32; BIOME_SECTION_SIZE])],
        clear_pois: impl FnMut(),
        add_poi: impl FnMut([i32; 3], &str, i32),
    ) -> ChunkRestoreOutput {
        let policy = &data.policy.rollback;
        let (cx, cz) = ChunkScope::unpack_chunk(chunk_key);
        let x_base = cx << 4;
        let z_base = cz << 4;

        let inhabited_time = data.chunk_inhabited_time.get(&chunk_key).copied();
        let mut changed_blocks = Vec::new();
        let mut biome_changed = false;

        if (policy.blocks || policy.biomes)
            && let Some(saved_sections) = data.section_snapshots.get(&chunk_key)
        {
            let t0 = Instant::now();
            IS_RESTORING_CHUNK.store(true, Ordering::SeqCst);
            for (idx, snap_opt) in saved_sections.iter().enumerate() {
                let Some(snap) = snap_opt else {
                    continue;
                };
                let Some((live_blocks, live_biomes)) = live_sections.get_mut(idx) else {
                    continue;
                };
                let y_base = (min_section_y + idx as i32) << 4;
                snap.apply_diff_to(
                    live_blocks,
                    live_biomes,
                    x_base,
                    y_base,
                    z_base,
                    policy.blocks,
                    policy.biomes,
                    &mut changed_blocks,
                    &mut biome_changed,
                );
            }
            IS_RESTORING_CHUNK.store(false, Ordering::SeqCst);
            RZBenchmark::accum(Phase::ChunkBlocks, t0.elapsed().as_nanos() as u64);
        }

        let restored_block_entities = if policy.block_entities {
            data.chunk_block_entities
                .get(&chunk_key)
                .cloned()
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        let restored_block_ticks = if policy.block_ticks {
            data.chunk_block_ticks
                .get(&chunk_key)
                .map(|t| ScheduledTickCodec::decode(t, game_time))
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        let restored_fluid_ticks = if policy.fluid_ticks {
            data.chunk_fluid_ticks
                .get(&chunk_key)
                .map(|t| ScheduledTickCodec::decode(t, game_time))
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        if !is_during_load
            && policy.pois
            && let Some(saved_pois) = data.chunk_pois.get(&chunk_key)
        {
            let t_poi = Instant::now();
            PoiCapture::restore(saved_pois, clear_pois, add_poi);
            RZBenchmark::accum(Phase::ChunkPois, t_poi.elapsed().as_nanos() as u64);
        }

        let replayed_block_events = if policy.block_events {
            data.block_events
                .iter()
                .filter(|ev| (ev.pos[0] >> 4) == cx && (ev.pos[2] >> 4) == cz)
                .cloned()
                .collect()
        } else {
            Vec::new()
        };

        let enqueue_chunk_resend =
            !is_during_load && changed_blocks.len() > Self::RESEND_CHUNK_THRESHOLD;
        if enqueue_chunk_resend {
            RestoreQueues::enqueue_chunk_resend(dim, cx, cz, (game_time as i32) + 1);
        }

        ChunkRestoreOutput {
            changed_blocks,
            biome_changed,
            restored_block_entities,
            restored_block_ticks,
            restored_fluid_ticks,
            replayed_block_events,
            inhabited_time,
            enqueue_chunk_resend,
        }
    }

    pub fn max_sub_tick_order(data: &CheckpointData) -> i64 {
        let b_max = data
            .chunk_block_ticks
            .values()
            .flat_map(|list| list.iter().map(|t| t.sub_tick_order))
            .max()
            .unwrap_or(-1);
        let f_max = data
            .chunk_fluid_ticks
            .values()
            .flat_map(|list| list.iter().map(|t| t.sub_tick_order))
            .max()
            .unwrap_or(-1);
        b_max.max(f_max)
    }
}

pub struct EntityRestorer;

impl EntityRestorer {
    pub fn spawn_entities_for_chunk(
        data: &CheckpointData,
        chunk_key: i64,
        live_entities_in_chunk: &[(u128, String)],
    ) -> (Vec<u128>, Vec<EntitySnapshot>) {
        let policy = &data.policy.rollback.entities;
        if !policy.presence {
            return (Vec::new(), Vec::new());
        }

        let t0 = Instant::now();
        let mut to_remove = Vec::new();
        for (uuid, kind) in live_entities_in_chunk {
            if !policy.dropped_items && kind == "minecraft:item" {
                continue;
            }
            if !policy.experience_orbs && kind == "minecraft:experience_orb" {
                continue;
            }
            if !SnapshotRegistry::is_entity_allowed(*uuid) {
                to_remove.push(*uuid);
            }
        }
        RZBenchmark::accum(Phase::RemoveForeign, t0.elapsed().as_nanos() as u64);

        let t1 = Instant::now();
        let mut spawned = Vec::new();
        for snap in data.get_entities_for_chunk(chunk_key) {
            if !policy.dropped_items && snap.kind == "minecraft:item" {
                continue;
            }
            if !policy.experience_orbs && snap.kind == "minecraft:experience_orb" {
                continue;
            }
            let mut restored = snap.clone();
            if !policy.rng_state {
                restored.rng_state.clear();
            }
            if !policy.living_timers {
                restored.has_living_timers = false;
            }
            if !policy.target {
                restored.target_uuid = None;
            }
            if policy.brain_ram
                && let Some(ram) = data.entity_ram_snapshots.get(&restored.uuid)
            {
                BrainRestorer::restore(ram, &mut restored);
                EntityCapture::apply_mob_ram(ram, &mut restored);
            }
            if policy.navigation && restored.has_path {
                RestoreQueues::enqueue_path_restore(PathRestore {
                    mob_uuid: restored.uuid,
                    target: restored.path_target,
                    speed: if restored.path_speed > 0.0 {
                        restored.path_speed
                    } else {
                        1.0
                    },
                });
            }
            spawned.push(restored);
        }
        RZBenchmark::accum(Phase::SpawnNbt, t1.elapsed().as_nanos() as u64);

        (to_remove, spawned)
    }
}

pub struct BrainRestorer;

impl BrainRestorer {
    pub fn is_complex_brain_mob(kind: &str) -> bool {
        matches!(
            kind,
            "minecraft:villager" | "minecraft:piglin" | "minecraft:piglin_brute"
        )
    }

    pub fn restore(ram: &EntityRamSnapshot, snap: &mut EntitySnapshot) -> bool {
        let t0 = Instant::now();
        let used_deep_clone =
            Self::is_complex_brain_mob(&snap.kind) && ram.cloned_brain_blob.is_some();
        if used_deep_clone {
            snap.has_path = false;
        }
        RZBenchmark::accum(Phase::BrainRelink, t0.elapsed().as_nanos() as u64);
        used_deep_clone
    }
}

pub struct PlayerRestorer;

impl PlayerRestorer {
    pub fn restore_all(
        data: &CheckpointData,
        online_players: &[u128],
        current_tick: i32,
    ) -> (HashMap<u128, PlayerData>, Vec<u128>) {
        RestoreQueues::clear_pending_offline_rollbacks();
        let online_set: HashSet<u128> = online_players.iter().copied().collect();
        let mut restored_online = HashMap::new();

        for (&uuid, pd) in &data.players_data {
            if online_set.contains(&uuid) {
                let t0 = Instant::now();
                restored_online.insert(uuid, pd.clone());
                RestoreQueues::remove_pending_ride(uuid);
                if let Some(veh) = pd.vehicle_uuid {
                    RestoreQueues::set_pending_ride(uuid, veh);
                }
                if data.policy.rollback.players.open_menu
                    && let Some(menu) = &pd.open_menu
                    && menu.menu_type_id.is_some()
                {
                    let mut m = menu.clone();
                    m.enqueued_at_tick = current_tick;
                    RestoreQueues::enqueue_menu_restore(uuid, m);
                }
                RZBenchmark::accum(Phase::PlayersOnline, t0.elapsed().as_nanos() as u64);
            } else {
                RestoreQueues::put_pending_offline_rollback(uuid, pd.clone());
            }
        }

        let mut reset_new_players = Vec::new();
        if !data.policy.rollback.players.preserve_new_player_inventory {
            for &uuid in online_players {
                if !data.players_data.contains_key(&uuid)
                    && !data.raw_players_nbt.contains_key(&uuid)
                {
                    reset_new_players.push(uuid);
                }
            }
        }

        (restored_online, reset_new_players)
    }

    pub fn restore_offline_files(
        world_root: &Path,
        data: &CheckpointData,
        policy: &RZeroCheckpointPolicy,
    ) {
        let t0 = Instant::now();
        OfflinePlayerFiles::restore_from(world_root, &data.raw_players_nbt);
        if policy.rollback.players.stats {
            OfflinePlayerFiles::restore_stats_from(world_root, &data.raw_player_stats);
        }
        RZBenchmark::accum(Phase::OfflineFiles, t0.elapsed().as_nanos() as u64);
    }
}

pub struct WorldStateRestorer;

pub struct WorldRestoreSummary {
    pub day_time: Option<i64>,
    pub game_time: Option<i64>,
    pub weather: Option<(bool, bool, i32, i32, i32)>,
    pub rng_state: Option<Vec<i64>>,
    pub difficulty: Option<(String, bool)>,
    pub world_spawn: Option<([i32; 3], f32)>,
    pub next_sub_tick_count: Option<i64>,
}

impl WorldStateRestorer {
    pub fn restore(data: &CheckpointData, is_overworld: bool) -> WorldRestoreSummary {
        let policy = &data.policy.rollback.world;
        let Some(ws): Option<&WorldSnapshot> = data.world_state.as_ref() else {
            return WorldRestoreSummary {
                day_time: None,
                game_time: None,
                weather: None,
                rng_state: None,
                difficulty: None,
                world_spawn: None,
                next_sub_tick_count: None,
            };
        };

        let (day_time, game_time) = if policy.time {
            (Some(ws.day_time), Some(ws.game_time))
        } else {
            (None, None)
        };

        let weather = if policy.weather {
            Some((
                ws.is_raining,
                ws.is_thundering,
                ws.rain_time,
                ws.thunder_time,
                ws.clear_weather_time,
            ))
        } else {
            None
        };

        let rng_state = if policy.level_rng && !ws.rng_state.is_empty() {
            Some(ws.rng_state.clone())
        } else {
            None
        };

        let difficulty = if policy.difficulty && !ws.difficulty.is_empty() {
            Some((ws.difficulty.clone(), ws.difficulty_locked))
        } else {
            None
        };

        let world_spawn = if policy.world_spawn && ws.has_world_spawn && is_overworld {
            Some(([ws.spawn_x, ws.spawn_y, ws.spawn_z], ws.spawn_angle))
        } else {
            None
        };

        let next_sub_tick_count =
            if data.policy.rollback.block_ticks || data.policy.rollback.fluid_ticks {
                let max_sub = ChunkRestorer::max_sub_tick_order(data);
                (max_sub >= 0).then_some(max_sub + 1)
            } else {
                None
            };

        WorldRestoreSummary {
            day_time,
            game_time,
            weather,
            rng_state,
            difficulty,
            world_spawn,
            next_sub_tick_count,
        }
    }
}