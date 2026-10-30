use crate::{
    RZeroConfig,
    adaptive::AdaptiveSaveEngine,
    anchor::AnchorSelector,
    api::DevHooks,
    checkpoint::{
        CheckpointManager,
        data::{CheckpointData, EntitySnapshot, MobRamLive, PlayerData},
    },
    command::CommandRZeroBase,
    engine::EntityIdCounter,
    log_info,
    metrics::PlayerMetrics,
    runtime::{MobRamCache, PathRestore, RZeroRuntime, RestoreQueues, SnapshotRegistry},
    serial::{RZBlob, RZBlobEncoder},
    util::RZBenchmark,
};
use std::path::Path;

pub struct ChunkEvents;

impl ChunkEvents {
    const WINDOW_TICKS: i64 = 20;

    pub fn on_chunk_load(dim: &str, chunk_key: i64, sections_count: usize, now_tick: i64) {
        SnapshotRegistry::loaded_chunks_insert(dim, chunk_key);
        SnapshotRegistry::with_snapshot_mut(dim, |snapshot| {
            if let Some(scope) = &snapshot.scope
                && !scope.includes(dim, chunk_key)
            {
                return;
            }
            if snapshot.pending_block_rollbacks.remove(&chunk_key) {
                if snapshot.section_snapshots.contains_key(&chunk_key) {
                    RestoreQueues::enqueue_chunk_ready_for_restore(dim, chunk_key);
                }
                RestoreQueues::set_rollback_window(dim, chunk_key, now_tick + Self::WINDOW_TICKS);
                return;
            }
            let is_restoring = RZeroRuntime::with(|rt| rt.is_restoring);
            if !is_restoring && !snapshot.section_snapshots.contains_key(&chunk_key) {
                snapshot
                    .section_snapshots
                    .insert(chunk_key, vec![None; sections_count]);
                RestoreQueues::set_capture_window(dim, chunk_key, now_tick + Self::WINDOW_TICKS);
            }
        });
    }

    pub fn on_chunk_unload(dim: &str, chunk_key: i64) {
        SnapshotRegistry::loaded_chunks_remove(dim, chunk_key);
    }
}

pub struct CommandEvents;

impl CommandEvents {
    pub fn on_command(
        command: &str,
        requester: Option<u128>,
        current_dim: &str,
        current_tick: i64,
    ) -> Option<Vec<String>> {
        CommandRZeroBase::execute(command, requester, current_dim, current_tick)
    }
}

pub struct EntityEvents;

impl EntityEvents {
    pub fn on_entity_join(
        dim: &str,
        entity: EntitySnapshot,
        now_tick: i64,
    ) -> Option<MobRamLive> {
        let mut applied_ram = None;
        let policy = RZeroRuntime::with(|rt| rt.checkpoint_policy());
        if policy.rollback.entities.mob_ram_cache {
            applied_ram = MobRamCache::take_on_join(dim, entity.uuid);
        }

        if !policy.rollback.entities.presence {
            return applied_ram;
        }
        if SnapshotRegistry::is_entity_allowed(entity.uuid) {
            return applied_ram;
        }
        if RestoreQueues::has_rollback_window(dim, entity.chunk_key) {
            RestoreQueues::enqueue_doomed_entity(entity.uuid);
            return applied_ram;
        }
        if RZeroRuntime::with(|rt| rt.is_restoring) {
            return applied_ram;
        }
        let window_end = RestoreQueues::get_capture_window(dim, entity.chunk_key);
        if window_end < 0 || now_tick > window_end {
            return applied_ram;
        }

        SnapshotRegistry::with_snapshot_mut(dim, |active| {
            let uuid = entity.uuid;
            let chunk_key = entity.chunk_key;
            active.entities.push(entity.clone());
            active
                .entities_by_chunk
                .entry(chunk_key)
                .or_default()
                .push(entity);
            SnapshotRegistry::allow_entity(uuid);
        });

        applied_ram
    }
}

pub struct PlayerEvents;

impl PlayerEvents {
    pub fn on_player_join(
        player_uuid: u128,
        dim: &str,
        game_time: i64,
        is_safe: bool,
    ) -> Option<PlayerData> {
        RZeroRuntime::with_mut(|rt| {
            if !rt.online_players.contains(&player_uuid) {
                rt.online_players.push(player_uuid);
            }
        });

        let auto_save = RZeroRuntime::with(|rt| rt.game_rules.auto_save);
        let offline_rollback = RestoreQueues::take_pending_offline_rollback(player_uuid);
        if !auto_save {
            return offline_rollback;
        }

        let has_checkpoint = SnapshotRegistry::has_checkpoint();
        PlayerMetrics::set_last_respawn_or_checkpoint_tick(player_uuid, game_time);

        if !has_checkpoint && is_safe {
            CheckpointManager::set_checkpoint(dim, Some(player_uuid), game_time);
            AdaptiveSaveEngine::reset_auto_save_timer(game_time, game_time);
            log_info("Initial checkpoint created on player join.");
        }
        offline_rollback
    }

    pub fn on_player_leave(player_uuid: u128) {
        RZeroRuntime::with_mut(|rt| {
            rt.online_players.retain(|&id| id != player_uuid);
        });
    }

    pub fn on_right_click_block(player_uuid: u128) {
        PlayerMetrics::increment_blocks_placed(player_uuid);
    }

    pub fn on_right_click_consumable(player_uuid: u128) {
        PlayerMetrics::increment_items_burned(player_uuid);
    }
}

pub struct ServerLifecycleEvents;

impl ServerLifecycleEvents {
    pub fn on_server_started(boot_tick: i64) {
        Self::on_server_started_in(Path::new("."), boot_tick);
    }

    pub fn on_server_started_in(world_root: &Path, boot_tick: i64) {
        RZeroConfig::load();
        log_info("Survival classifier initialized.");
        DevHooks::fire_server_started(boot_tick);
        SnapshotRegistry::clear();
        MobRamCache::clear();
        RestoreQueues::clear_for_restore();
        RZeroRuntime::with_mut(|rt| {
            rt.reset_active_checkpoint_policy_to_config();
            if rt.adaptive_state.current_interval == 0 && rt.adaptive_state.last_save_tick == 0 {
                rt.adaptive_state.last_save_tick = boot_tick;
            }
        });

        let mut restored_origin = false;
        for dim in [
            "minecraft:overworld",
            "minecraft:the_nether",
            "minecraft:the_end",
        ] {
            let Some(data) = CheckpointData::load_from_disk(world_root, dim) else {
                continue;
            };
            if data.anchor_id.is_none() && data.anchor_ids.is_empty() {
                continue;
            }
            if data.policy.rollback.entities.presence {
                for es in &data.entities {
                    SnapshotRegistry::allow_entity(es.uuid);
                }
            }
            let seeded = if data.policy.rollback.entities.mob_ram_cache {
                MobRamCache::seed_from(dim, &data, boot_tick)
            } else {
                0
            };
            log_info(&format!(
                "Loaded timeline for dimension {dim}: pending chunks={}, seeded mob RAM={seeded}",
                data.pending_block_rollbacks.len()
            ));
            if !restored_origin
                && let Some(as_state) = data.adaptive_state
            {
                let policy = data.policy.clone();
                RZeroRuntime::with_mut(|rt| rt.set_active_checkpoint_policy(policy.clone()));
                AdaptiveSaveEngine::restore_adaptive_state(Some(as_state));
                if policy.rollback.entities.entity_id {
                    EntityIdCounter::set(as_state.entity_id_counter);
                }
                restored_origin = true;
            }
            SnapshotRegistry::put_active_snapshot(dim.to_owned(), data);
        }
    }

    pub fn on_server_stopping() {
        RZBlobEncoder::shutdown();
        RZBlob::clear_intern_pool();
    }
}

pub struct ServerTickEvents;

impl ServerTickEvents {
    const ENTITY_ROLLBACK_WINDOW: i64 = 20;
    const PATH_RESTORE_BUDGET: usize = 32;

    pub fn on_server_tick_post(current_tick: i64) {
        RZeroRuntime::with_mut(|rt| {
            rt.was_restored_this_tick = false;
        });
        DevHooks::fire_server_tick_post(current_tick);

        RZBenchmark::tick(current_tick as i32);
        if RestoreQueues::all_queues_empty() || RZBenchmark::is_timed_out() {
            RZBenchmark::end_and_log();
        }

        MobRamCache::evict_expired(current_tick);
        SnapshotRegistry::promote_loaded_chunks();
        let _ = RestoreQueues::drain_doomed_entities();
        Self::drain_block_restore_queue(current_tick);
        Self::drain_entity_restore_queue();
        Self::drain_path_restores();
        while RestoreQueues::poll_chunk_resend(current_tick as i32).is_some() {}
        let _ = RestoreQueues::take_pending_rides();

        if let Some(dead_player) = RestoreQueues::take_pending_death_rollback() {
            CheckpointManager::restore_checkpoint(Some(dead_player), current_tick);
        }
        RestoreQueues::expire_windows(current_tick);
        Self::tick_interval_autosave(current_tick);
    }

    pub fn drain_block_restore_queue(current_tick: i64) -> Vec<(String, i64)> {
        let mut restored = Vec::new();
        while let Some((dim, chunk_key)) = RestoreQueues::poll_chunk_ready_for_restore() {
            RestoreQueues::enqueue_chunk_pending_entity_restore(&dim, chunk_key);
            RestoreQueues::set_rollback_window(
                &dim,
                chunk_key,
                current_tick + Self::ENTITY_ROLLBACK_WINDOW,
            );
            restored.push((dim, chunk_key));
        }
        restored
    }

    pub fn drain_entity_restore_queue() -> Vec<(String, i64)> {
        let mut drained = Vec::new();
        while let Some((dim, chunk_key)) = RestoreQueues::poll_chunk_pending_entity_restore() {
            drained.push((dim, chunk_key));
        }
        drained
    }

    pub fn drain_path_restores() -> Vec<PathRestore> {
        let mut out = Vec::with_capacity(Self::PATH_RESTORE_BUDGET);
        for _ in 0..Self::PATH_RESTORE_BUDGET {
            let Some(p) = RestoreQueues::poll_path_restore() else {
                break;
            };
            out.push(p);
        }
        out
    }

    fn tick_interval_autosave(current_tick: i64) {
        if !SnapshotRegistry::has_checkpoint() {
            return;
        }
        let (rules, settings, online, mut state) = RZeroRuntime::with(|rt| {
            (
                rt.game_rules.clone(),
                rt.anchor_settings.clone(),
                rt.online_players.clone(),
                rt.adaptive_state,
            )
        });
        if !rules.auto_save || rules.use_adaptive_mode {
            return;
        }
        let anchors = AnchorSelector::resolve_anchors(&settings, &online, current_tick, None);
        let Some(&primary) = anchors.first() else {
            return;
        };
        if state.current_interval == 0 {
            state.current_interval = AdaptiveSaveEngine::compute_interval(&rules, current_tick);
            state.last_save_tick = current_tick;
            RZeroRuntime::with_mut(|rt| rt.adaptive_state = state);
            return;
        }
        if current_tick - state.last_save_tick >= state.current_interval {
            CheckpointManager::set_checkpoint("minecraft:overworld", Some(primary), current_tick);
            let next_interval = AdaptiveSaveEngine::compute_interval(&rules, current_tick);
            RZeroRuntime::with_mut(|rt| {
                rt.adaptive_state.last_save_tick = current_tick;
                rt.adaptive_state.current_interval = next_interval;
            });
        }
    }
}