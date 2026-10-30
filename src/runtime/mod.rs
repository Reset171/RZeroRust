use crate::{
    ModGameRules,
    anchor::{AnchorSelector, RZeroAnchorSettings, RollbackCooldown},
    checkpoint::data::{AdaptiveState, CheckpointData, MobRamLive, OpenMenuSnapshot, PlayerData},
    config::{RZeroCheckpointPolicy, RZeroSettings},
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Mutex, OnceLock},
};

#[derive(Clone, Debug, PartialEq)]
pub struct PathRestore {
    pub mob_uuid: u128,
    pub target: [i32; 3],
    pub speed: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkResend {
    pub dim: String,
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub due_tick: i32,
}

pub struct RZeroRuntime {
    pub is_restoring: bool,
    pub is_headless_ticking: bool,
    pub was_restored_this_tick: bool,
    pub is_verify_run: bool,
    pub rzerochash_enabled: bool,
    pub shuffling_counter: i64,
    pub settings: RZeroSettings,
    pub active_checkpoint_policy: RZeroCheckpointPolicy,
    pub anchor_settings: RZeroAnchorSettings,
    pub game_rules: ModGameRules,
    pub adaptive_state: AdaptiveState,
    pub cooldown: RollbackCooldown,
    pub online_players: Vec<u128>,
}

impl Default for RZeroRuntime {
    fn default() -> Self {
        let settings = RZeroSettings::default();
        let active_checkpoint_policy = settings.checkpoint_policy.clone();
        let anchor_settings = settings.anchor.clone();
        Self {
            is_restoring: false,
            is_headless_ticking: false,
            was_restored_this_tick: false,
            is_verify_run: false,
            rzerochash_enabled: settings.rzerochash_enabled,
            shuffling_counter: 0,
            settings,
            active_checkpoint_policy,
            anchor_settings,
            game_rules: ModGameRules::default(),
            adaptive_state: AdaptiveState {
                last_boss_death_tick: -9999,
                ..AdaptiveState::default()
            },
            cooldown: RollbackCooldown::default(),
            online_players: Vec::new(),
        }
    }
}

static RUNTIME: OnceLock<Mutex<RZeroRuntime>> = OnceLock::new();

fn runtime_lock() -> &'static Mutex<RZeroRuntime> {
    RUNTIME.get_or_init(|| Mutex::new(RZeroRuntime::default()))
}

impl RZeroRuntime {
    pub fn with<R>(f: impl FnOnce(&RZeroRuntime) -> R) -> R {
        let guard = runtime_lock().lock().unwrap_or_else(|e| e.into_inner());
        f(&guard)
    }

    pub fn with_mut<R>(f: impl FnOnce(&mut RZeroRuntime) -> R) -> R {
        let mut guard = runtime_lock().lock().unwrap_or_else(|e| e.into_inner());
        f(&mut guard)
    }

    pub fn set_settings(&mut self, new_settings: RZeroSettings) {
        let sanitized = new_settings.sanitize();
        self.active_checkpoint_policy = sanitized.checkpoint_policy.clone();
        self.rzerochash_enabled = sanitized.rzerochash_enabled;
        self.anchor_settings = sanitized.anchor.clone();
        self.settings = sanitized;
    }

    pub fn set_anchor_settings(&mut self, new_anchor: RZeroAnchorSettings) {
        let sanitized = new_anchor.sanitize();
        self.settings = self.settings.clone().with_anchor(sanitized.clone());
        self.anchor_settings = sanitized;
    }

    pub fn set_active_checkpoint_policy(&mut self, policy: RZeroCheckpointPolicy) {
        self.active_checkpoint_policy = policy.sanitize();
    }

    pub fn reset_active_checkpoint_policy_to_config(&mut self) {
        self.active_checkpoint_policy = self.settings.checkpoint_policy.clone();
    }

    pub fn checkpoint_policy(&self) -> RZeroCheckpointPolicy {
        self.active_checkpoint_policy.clone()
    }
}

#[derive(Default)]
struct RegistryState {
    active_snapshots: HashMap<String, CheckpointData>,
    allowed_snapshot_entities: HashSet<u128>,
    loaded_chunks: HashMap<String, HashSet<i64>>,
}

static SNAPSHOT_REGISTRY: OnceLock<Mutex<RegistryState>> = OnceLock::new();

fn registry_lock() -> &'static Mutex<RegistryState> {
    SNAPSHOT_REGISTRY.get_or_init(|| Mutex::new(RegistryState::default()))
}

pub struct SnapshotRegistry;

impl SnapshotRegistry {
    pub fn has_checkpoint() -> bool {
        let guard = registry_lock().lock().unwrap_or_else(|e| e.into_inner());
        guard
            .active_snapshots
            .values()
            .any(|d| d.anchor_id.is_some() || !d.anchor_ids.is_empty())
    }

    pub fn is_anchor_in_snapshots(candidate: u128) -> bool {
        let guard = registry_lock().lock().unwrap_or_else(|e| e.into_inner());
        guard
            .active_snapshots
            .values()
            .any(|d| d.is_anchor(candidate))
    }

    pub fn put_active_snapshot(dim: String, data: CheckpointData) {
        let mut guard = registry_lock().lock().unwrap_or_else(|e| e.into_inner());
        guard.active_snapshots.insert(dim, data);
    }

    pub fn get_active_snapshot(dim: &str) -> Option<CheckpointData> {
        let guard = registry_lock().lock().unwrap_or_else(|e| e.into_inner());
        guard.active_snapshots.get(dim).cloned()
    }

    pub fn find_timeline_origin() -> Option<(String, CheckpointData)> {
        let guard = registry_lock().lock().unwrap_or_else(|e| e.into_inner());
        let mut keys: Vec<&String> = guard.active_snapshots.keys().collect();
        keys.sort();
        for k in keys {
            if let Some(d) = guard.active_snapshots.get(k)
                && d.adaptive_state.is_some()
            {
                return Some((k.clone(), d.clone()));
            }
        }
        None
    }

    pub fn with_snapshot_mut<R>(dim: &str, f: impl FnOnce(&mut CheckpointData) -> R) -> Option<R> {
        let mut guard = registry_lock().lock().unwrap_or_else(|e| e.into_inner());
        guard.active_snapshots.get_mut(dim).map(f)
    }

    pub fn schedule_chunk_rollbacks(deadline: i64) -> (usize, usize) {
        let mut guard = registry_lock().lock().unwrap_or_else(|e| e.into_inner());
        let loaded_by_dim = guard.loaded_chunks.clone();
        let mut processed = 0;
        let mut queued = 0;
        let mut dims: Vec<String> = guard.active_snapshots.keys().cloned().collect();
        dims.sort();

        for dim in dims {
            let loaded = loaded_by_dim.get(&dim);
            let Some(data) = guard.active_snapshots.get_mut(&dim) else {
                continue;
            };
            data.pending_block_rollbacks.clear();
            let mut chunk_keys: Vec<i64> = data.section_snapshots.keys().copied().collect();
            chunk_keys.sort_unstable();
            for chunk_key in chunk_keys {
                if loaded.is_some_and(|set| set.contains(&chunk_key)) {
                    processed += 1;
                    RestoreQueues::enqueue_chunk_pending_entity_restore(&dim, chunk_key);
                } else {
                    queued += 1;
                    data.pending_block_rollbacks.insert(chunk_key);
                }
                RestoreQueues::set_rollback_window(&dim, chunk_key, deadline);
            }
        }
        (processed, queued)
    }

    pub fn promote_loaded_chunks() -> Vec<(String, i64)> {
        let mut guard = registry_lock().lock().unwrap_or_else(|e| e.into_inner());
        let loaded_by_dim = guard.loaded_chunks.clone();
        let mut promoted = Vec::new();
        for (dim, data) in &mut guard.active_snapshots {
            if data.pending_block_rollbacks.is_empty() {
                continue;
            }
            let Some(loaded) = loaded_by_dim.get(dim) else {
                continue;
            };
            let pending: Vec<i64> = data.pending_block_rollbacks.iter().copied().collect();
            for chunk_key in pending {
                if loaded.contains(&chunk_key) {
                    data.pending_block_rollbacks.remove(&chunk_key);
                    RestoreQueues::enqueue_chunk_ready_for_restore(dim, chunk_key);
                    promoted.push((dim.clone(), chunk_key));
                }
            }
        }
        promoted
    }

    pub fn max_entity_id() -> i32 {
        let guard = registry_lock().lock().unwrap_or_else(|e| e.into_inner());
        guard
            .active_snapshots
            .values()
            .flat_map(|d| d.entities.iter().map(|e| e.entity_id))
            .max()
            .unwrap_or(0)
    }

    pub fn allow_entity(uuid: u128) {
        let mut guard = registry_lock().lock().unwrap_or_else(|e| e.into_inner());
        guard.allowed_snapshot_entities.insert(uuid);
    }

    pub fn is_entity_allowed(uuid: u128) -> bool {
        let guard = registry_lock().lock().unwrap_or_else(|e| e.into_inner());
        guard.allowed_snapshot_entities.contains(&uuid)
    }

    pub fn clear_allowed_entities() {
        let mut guard = registry_lock().lock().unwrap_or_else(|e| e.into_inner());
        guard.allowed_snapshot_entities.clear();
    }

    pub fn populate_allowed_entities_from_snapshots() {
        let mut guard = registry_lock().lock().unwrap_or_else(|e| e.into_inner());
        let mut allowed = HashSet::new();
        for d in guard.active_snapshots.values() {
            if d.policy.rollback.entities.presence {
                for e in &d.entities {
                    allowed.insert(e.uuid);
                }
            }
        }
        guard.allowed_snapshot_entities = allowed;
    }

    pub fn loaded_chunks_insert(dim: &str, chunk_key: i64) {
        let mut guard = registry_lock().lock().unwrap_or_else(|e| e.into_inner());
        guard
            .loaded_chunks
            .entry(dim.to_owned())
            .or_default()
            .insert(chunk_key);
    }

    pub fn loaded_chunks_remove(dim: &str, chunk_key: i64) {
        let mut guard = registry_lock().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(set) = guard.loaded_chunks.get_mut(dim) {
            set.remove(&chunk_key);
        }
    }

    pub fn sync_active_snapshot_anchors(game_time: i64) {
        let (settings, online) = RZeroRuntime::with(|rt| {
            (rt.anchor_settings.clone(), rt.online_players.clone())
        });
        let anchors = AnchorSelector::resolve_anchors(&settings, &online, game_time, None);
        let primary = anchors.first().copied();
        let mut guard = registry_lock().lock().unwrap_or_else(|e| e.into_inner());
        for data in guard.active_snapshots.values_mut() {
            data.anchor_id = primary;
            data.anchor_ids = anchors.iter().copied().collect();
        }
    }

    pub fn clear() {
        let mut guard = registry_lock().lock().unwrap_or_else(|e| e.into_inner());
        guard.active_snapshots.clear();
        guard.allowed_snapshot_entities.clear();
    }
}

#[derive(Default)]
struct QueuesState {
    chunks_ready_for_restore: VecDeque<(String, i64)>,
    chunks_pending_entity_restore: VecDeque<(String, i64)>,
    pending_path_restores: VecDeque<PathRestore>,
    pending_chunk_resends: VecDeque<ChunkResend>,
    queued_resend_keys: HashSet<(String, i64)>,
    doomed_entities: VecDeque<u128>,
    pending_rides: HashMap<u128, u128>,
    pending_offline_rollbacks: HashMap<u128, PlayerData>,
    pending_menu_restores: HashMap<u128, OpenMenuSnapshot>,
    pending_entity_rollbacks: HashMap<String, HashMap<i64, i64>>,
    chunk_capture_windows: HashMap<String, HashMap<i64, i64>>,
    pending_death_rollback: Option<u128>,
}

static RESTORE_QUEUES: OnceLock<Mutex<QueuesState>> = OnceLock::new();

fn queues_lock() -> &'static Mutex<QueuesState> {
    RESTORE_QUEUES.get_or_init(|| Mutex::new(QueuesState::default()))
}

pub struct RestoreQueues;

impl RestoreQueues {
    pub fn set_pending_death_rollback(player: u128) {
        let mut q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        q.pending_death_rollback = Some(player);
    }

    pub fn take_pending_death_rollback() -> Option<u128> {
        let mut q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        q.pending_death_rollback.take()
    }

    pub fn clear_pending_death_rollback() {
        let mut q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        q.pending_death_rollback = None;
    }

    pub fn enqueue_chunk_ready_for_restore(dim: &str, chunk_key: i64) {
        let mut q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        q.chunks_ready_for_restore
            .push_back((dim.to_owned(), chunk_key));
    }

    pub fn poll_chunk_ready_for_restore() -> Option<(String, i64)> {
        let mut q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        q.chunks_ready_for_restore.pop_front()
    }

    pub fn enqueue_chunk_pending_entity_restore(dim: &str, chunk_key: i64) {
        let mut q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        q.chunks_pending_entity_restore
            .push_back((dim.to_owned(), chunk_key));
    }

    pub fn poll_chunk_pending_entity_restore() -> Option<(String, i64)> {
        let mut q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        q.chunks_pending_entity_restore.pop_front()
    }

    pub fn enqueue_path_restore(restore: PathRestore) {
        let mut q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        q.pending_path_restores.push_back(restore);
    }

    pub fn poll_path_restore() -> Option<PathRestore> {
        let mut q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        q.pending_path_restores.pop_front()
    }

    pub fn enqueue_chunk_resend(dim: &str, chunk_x: i32, chunk_z: i32, due_tick: i32) {
        let key = crate::checkpoint::ChunkScope::pack_chunk(chunk_x, chunk_z);
        let mut q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        if q.queued_resend_keys.insert((dim.to_owned(), key)) {
            q.pending_chunk_resends.push_back(ChunkResend {
                dim: dim.to_owned(),
                chunk_x,
                chunk_z,
                due_tick,
            });
        }
    }

    pub fn poll_chunk_resend(current_tick: i32) -> Option<ChunkResend> {
        let mut q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        if q.pending_chunk_resends
            .front()
            .is_none_or(|r| r.due_tick > current_tick)
        {
            return None;
        }
        let resend = q.pending_chunk_resends.pop_front()?;
        let key = crate::checkpoint::ChunkScope::pack_chunk(resend.chunk_x, resend.chunk_z);
        q.queued_resend_keys.remove(&(resend.dim.clone(), key));
        Some(resend)
    }

    pub fn enqueue_menu_restore(player: u128, snapshot: OpenMenuSnapshot) {
        let mut q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        q.pending_menu_restores.insert(player, snapshot);
    }

    pub fn with_menu_restores_mut(f: impl FnOnce(&mut HashMap<u128, OpenMenuSnapshot>)) {
        let mut q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        f(&mut q.pending_menu_restores);
    }

    pub fn set_pending_ride(player: u128, vehicle: u128) {
        let mut q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        q.pending_rides.insert(player, vehicle);
    }

    pub fn remove_pending_ride(player: u128) {
        let mut q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        q.pending_rides.remove(&player);
    }

    pub fn take_pending_rides() -> Vec<(u128, u128)> {
        let mut q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        let mut list: Vec<(u128, u128)> = q.pending_rides.iter().map(|(&p, &v)| (p, v)).collect();
        list.sort_by_key(|(p, _)| *p);
        q.pending_rides.clear();
        list
    }

    pub fn put_pending_offline_rollback(player: u128, data: PlayerData) {
        let mut q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        q.pending_offline_rollbacks.insert(player, data);
    }

    pub fn take_pending_offline_rollback(player: u128) -> Option<PlayerData> {
        let mut q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        q.pending_offline_rollbacks.remove(&player)
    }

    pub fn clear_pending_offline_rollbacks() {
        let mut q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        q.pending_offline_rollbacks.clear();
    }

    pub fn enqueue_doomed_entity(uuid: u128) {
        let mut q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        q.doomed_entities.push_back(uuid);
    }

    pub fn drain_doomed_entities() -> Vec<u128> {
        let mut q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        q.doomed_entities.drain(..).collect()
    }

    pub fn set_rollback_window(dim: &str, chunk_key: i64, deadline: i64) {
        let mut q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        q.pending_entity_rollbacks
            .entry(dim.to_owned())
            .or_default()
            .insert(chunk_key, deadline);
    }

    pub fn has_rollback_window(dim: &str, chunk_key: i64) -> bool {
        let q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        q.pending_entity_rollbacks
            .get(dim)
            .is_some_and(|m| m.contains_key(&chunk_key))
    }

    pub fn set_capture_window(dim: &str, chunk_key: i64, deadline: i64) {
        let mut q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        q.chunk_capture_windows
            .entry(dim.to_owned())
            .or_default()
            .insert(chunk_key, deadline);
    }

    pub fn get_capture_window(dim: &str, chunk_key: i64) -> i64 {
        let q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        q.chunk_capture_windows
            .get(dim)
            .and_then(|m| m.get(&chunk_key).copied())
            .unwrap_or(-1)
    }

    pub fn clear_capture_windows() {
        let mut q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        q.chunk_capture_windows.clear();
    }

    pub fn clear_for_restore() {
        let mut q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        q.pending_entity_rollbacks.clear();
        q.chunks_ready_for_restore.clear();
        q.chunks_pending_entity_restore.clear();
        q.pending_path_restores.clear();
        q.pending_chunk_resends.clear();
        q.queued_resend_keys.clear();
        q.pending_menu_restores.clear();
    }

    pub fn all_queues_empty() -> bool {
        let q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        q.chunks_ready_for_restore.is_empty()
            && q.chunks_pending_entity_restore.is_empty()
            && q.pending_path_restores.is_empty()
            && q.pending_chunk_resends.is_empty()
            && q.doomed_entities.is_empty()
            && q.pending_menu_restores.is_empty()
    }

    pub fn expire_windows(current_tick: i64) {
        let mut q = queues_lock().lock().unwrap_or_else(|e| e.into_inner());
        for inner in q.pending_entity_rollbacks.values_mut() {
            inner.retain(|_, &mut deadline| current_tick < deadline);
        }
        for inner in q.chunk_capture_windows.values_mut() {
            inner.retain(|_, &mut deadline| current_tick < deadline);
        }
    }
}

static MOB_RAM_CACHE: OnceLock<Mutex<HashMap<String, HashMap<u128, MobRamLive>>>> = OnceLock::new();

fn mob_ram_lock() -> &'static Mutex<HashMap<String, HashMap<u128, MobRamLive>>> {
    MOB_RAM_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub struct MobRamCache;

impl MobRamCache {
    const TTL_TICKS: i64 = 24000;

    pub fn clear() {
        let mut cache = mob_ram_lock().lock().unwrap_or_else(|e| e.into_inner());
        cache.clear();
    }

    pub fn put_on_unload(dim: &str, uuid: u128, mut live: MobRamLive, current_tick: i64) {
        live.captured_at_tick = current_tick;
        let mut cache = mob_ram_lock().lock().unwrap_or_else(|e| e.into_inner());
        cache.entry(dim.to_owned()).or_default().insert(uuid, live);
    }

    pub fn evict_expired(current_tick: i64) {
        if (current_tick & 0xFF) != 0 {
            return;
        }
        let cutoff = current_tick - Self::TTL_TICKS;
        let mut cache = mob_ram_lock().lock().unwrap_or_else(|e| e.into_inner());
        for dim_map in cache.values_mut() {
            dim_map.retain(|_, live| live.captured_at_tick >= cutoff);
        }
    }

    pub fn seed_from(dim: &str, data: &CheckpointData, boot_tick: i64) -> usize {
        let mut by_uuid = HashMap::new();
        for es in &data.entities {
            by_uuid.insert(es.uuid, es);
        }
        let mut dim_cache = HashMap::new();
        for (uuid, ram) in &data.entity_ram_snapshots {
            let mut live = MobRamLive::from_snapshots(Some(ram), by_uuid.get(uuid).copied());
            live.captured_at_tick = boot_tick;
            dim_cache.insert(*uuid, live);
        }
        for es in &data.entities {
            if dim_cache.contains_key(&es.uuid) {
                continue;
            }
            let mut live = MobRamLive::from_snapshots(None, Some(es));
            live.captured_at_tick = boot_tick;
            dim_cache.insert(es.uuid, live);
        }
        let count = dim_cache.len();
        if !dim_cache.is_empty() {
            let mut cache = mob_ram_lock().lock().unwrap_or_else(|e| e.into_inner());
            cache.insert(dim.to_owned(), dim_cache);
        }
        count
    }

    pub fn take_on_join(dim: &str, uuid: u128) -> Option<MobRamLive> {
        let mut cache = mob_ram_lock().lock().unwrap_or_else(|e| e.into_inner());
        cache.get_mut(dim)?.remove(&uuid)
    }
}