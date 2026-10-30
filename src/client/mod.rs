use crate::{
    checkpoint::ChunkScope,
    log_info,
    network::{RollbackChatPacket, SyncAnchorSettingsPacket},
    runtime::RZeroRuntime,
};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct KeyBindings {
    pub save_key: &'static str,
    pub load_key: &'static str,
    pub anchor_menu_key: &'static str,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            save_key: "key.rzero.save",
            load_key: "key.rzero.load",
            anchor_menu_key: "key.rzero.anchor_menu",
        }
    }
}

#[derive(Clone, Debug)]
pub struct SpatialGrid<T> {
    pub center_x: i32,
    pub center_z: i32,
    pub radius: i32,
    width: usize,
    data: Vec<Option<T>>,
}

impl<T: Clone> SpatialGrid<T> {
    pub fn new(center_x: i32, center_z: i32, radius: i32) -> Self {
        let r = radius.max(0);
        let width = (2 * r + 1) as usize;
        Self {
            center_x,
            center_z,
            radius: r,
            width,
            data: vec![None; width * width],
        }
    }

    pub fn get(&self, x: i32, z: i32) -> Option<&T> {
        let lx = x - self.center_x + self.radius;
        let lz = z - self.center_z + self.radius;
        if lx < 0 || lz < 0 || lx as usize >= self.width || lz as usize >= self.width {
            return None;
        }
        self.data[lx as usize + lz as usize * self.width].as_ref()
    }

    pub fn set(&mut self, x: i32, z: i32, value: Option<T>) {
        let lx = x - self.center_x + self.radius;
        let lz = z - self.center_z + self.radius;
        if lx < 0 || lz < 0 || lx as usize >= self.width || lz as usize >= self.width {
            return;
        }
        self.data[lx as usize + lz as usize * self.width] = value;
    }

    pub fn width(&self) -> usize {
        self.width
    }
}

#[derive(Clone, Debug)]
pub struct SpatialSectionGrid<T> {
    pub center_x: i32,
    pub center_z: i32,
    pub radius: i32,
    pub min_y: i32,
    pub max_y: i32,
    width: usize,
    height: usize,
    plane_stride: usize,
    data: Vec<Option<T>>,
}

impl<T: Clone> SpatialSectionGrid<T> {
    pub fn new(center_x: i32, center_z: i32, radius: i32, min_y: i32, max_y: i32) -> Self {
        let r = radius.max(0);
        let width = (2 * r + 1) as usize;
        let height = (max_y - min_y).max(1) as usize;
        let plane_stride = width * width;
        Self {
            center_x,
            center_z,
            radius: r,
            min_y,
            max_y,
            width,
            height,
            plane_stride,
            data: vec![None; plane_stride * height],
        }
    }

    pub fn get(&self, x: i32, y: i32, z: i32) -> Option<&T> {
        let lx = x - self.center_x + self.radius;
        let lz = z - self.center_z + self.radius;
        let ly = y - self.min_y;
        if lx < 0
            || lz < 0
            || ly < 0
            || lx as usize >= self.width
            || lz as usize >= self.width
            || ly as usize >= self.height
        {
            return None;
        }
        self.data[lx as usize + lz as usize * self.width + ly as usize * self.plane_stride].as_ref()
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, value: Option<T>) {
        let lx = x - self.center_x + self.radius;
        let lz = z - self.center_z + self.radius;
        let ly = y - self.min_y;
        if lx < 0
            || lz < 0
            || ly < 0
            || lx as usize >= self.width
            || lz as usize >= self.width
            || ly as usize >= self.height
        {
            return;
        }
        self.data[lx as usize + lz as usize * self.width + ly as usize * self.plane_stride] = value;
    }

    pub fn clear_column(&mut self, x: i32, z: i32) {
        let lx = x - self.center_x + self.radius;
        let lz = z - self.center_z + self.radius;
        if lx < 0 || lz < 0 || lx as usize >= self.width || lz as usize >= self.width {
            return;
        }
        let base = lx as usize + lz as usize * self.width;
        for ly in 0..self.height {
            self.data[base + ly * self.plane_stride] = None;
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct RZeroFakeChunk {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub min_section: i32,
    pub max_section: i32,
    pub block_light: Vec<Option<Vec<u8>>>,
    pub sky_light: Vec<Option<Vec<u8>>>,
}

#[derive(Clone, Debug, Default)]
pub struct GeometryArena {
    pub capacity: u64,
    pub used: u64,
}

impl GeometryArena {
    pub fn remaining(&self) -> u64 {
        self.capacity.saturating_sub(self.used)
    }

    pub fn allocate(&mut self, bytes: u64) -> bool {
        if bytes == 0 {
            return false;
        }
        self.capacity = bytes;
        self.used = 0;
        true
    }

    pub fn push(&mut self, size: u64) -> Option<u64> {
        if size == 0 || size > self.remaining() {
            return None;
        }
        let offset = self.used;
        self.used += size;
        Some(offset)
    }

    pub fn reset(&mut self, capacity: u64) {
        self.capacity = capacity;
        self.used = 0;
    }
}

#[derive(Clone, Debug, Default)]
pub struct SectionMeshRecord {
    pub origin_key: i64,
    pub byte_size: u64,
    pub block_entity_positions: Vec<i64>,
}

#[derive(Clone, Debug, Default)]
pub struct RZeroMeshCache {
    pub arena: GeometryArena,
    pub section_records: HashMap<i64, SectionMeshRecord>,
    pub snapshot_dimension: Option<String>,
}

impl RZeroMeshCache {
    pub fn has_snapshot(&self) -> bool {
        !self.section_records.is_empty() && self.snapshot_dimension.is_some()
    }

    pub fn has_record(&self, origin_key: i64) -> bool {
        self.section_records.contains_key(&origin_key)
    }

    pub fn restore(&self) -> usize {
        if !self.has_snapshot() {
            return 0;
        }
        self.section_records.len()
    }

    pub fn clear(&mut self) {
        self.section_records.clear();
        self.arena.reset(0);
        self.snapshot_dimension = None;
    }
}

#[derive(Clone, Debug)]
pub struct SnapshotEntityState {
    pub uuid: u128,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub y_rot: f32,
    pub x_rot: f32,
    pub y_head_rot: f32,
    pub y_body_rot: f32,
}

#[derive(Clone, Debug)]
pub struct RZeroClientCache {
    pub enabled: bool,
    pub in_rollback: bool,
    pub capture_pending: bool,
    pub capture_attempts: i32,
    pub pending_section_refresh: bool,
    pub pending_refresh_attempts: i32,
    pub is_inter_dimensional_rollback: bool,
    pub inter_dimensional_arm_tick: i64,
    pub sync_occlusion_requested: bool,
    pub snapshot_dimension: Option<String>,
    pub snapshot_center_x: i32,
    pub snapshot_center_z: i32,
    pub snapshot_radius: i32,
    pub session_expires_at_tick: i64,
    pub snapshot_grid: Option<SpatialGrid<RZeroFakeChunk>>,
    pub active_grid: Option<SpatialGrid<RZeroFakeChunk>>,
    pub active_fake_count: usize,
    pub snapshot_entities: HashMap<u128, SnapshotEntityState>,
    pub spawned_fake_entity_ids: HashMap<u128, i32>,
    pub fake_entity_by_chunk: HashMap<i64, Vec<u128>>,
    pub mesh_cache: RZeroMeshCache,
    next_fake_id: i32,
}

impl Default for RZeroClientCache {
    fn default() -> Self {
        Self {
            enabled: true,
            in_rollback: false,
            capture_pending: false,
            capture_attempts: 0,
            pending_section_refresh: false,
            pending_refresh_attempts: 0,
            is_inter_dimensional_rollback: false,
            inter_dimensional_arm_tick: -1,
            sync_occlusion_requested: false,
            snapshot_dimension: None,
            snapshot_center_x: 0,
            snapshot_center_z: 0,
            snapshot_radius: 0,
            session_expires_at_tick: -1,
            snapshot_grid: None,
            active_grid: None,
            active_fake_count: 0,
            snapshot_entities: HashMap::new(),
            spawned_fake_entity_ids: HashMap::new(),
            fake_entity_by_chunk: HashMap::new(),
            mesh_cache: RZeroMeshCache::default(),
            next_fake_id: i32::MIN / 2,
        }
    }
}

impl RZeroClientCache {
    pub const SESSION_TTL_TICKS: i64 = 200;
    pub const LEAVE_BUFFER_CHUNKS: i32 = 4;
    pub const MAX_CAPTURE_ATTEMPTS: i32 = 200;
    pub const MAX_REFRESH_ATTEMPTS: i32 = 200;

    pub fn set_enabled(&mut self, enabled: bool) {
        if self.enabled == enabled {
            return;
        }
        self.enabled = enabled;
        if !enabled {
            self.clear();
        }
    }

    pub fn request_capture(&mut self) {
        if !self.enabled {
            return;
        }
        self.clear();
        self.capture_pending = true;
        self.capture_attempts = 0;
    }

    pub fn arm_inter_dimensional_rollback(&mut self, game_time: i64) {
        self.is_inter_dimensional_rollback = true;
        self.inter_dimensional_arm_tick = game_time;
    }

    pub fn clear_inter_dimensional_rollback(&mut self) {
        self.is_inter_dimensional_rollback = false;
        self.inter_dimensional_arm_tick = -1;
    }

    pub fn request_sync_occlusion(&mut self) {
        self.sync_occlusion_requested = true;
    }

    pub fn consume_sync_occlusion_request(&mut self) -> bool {
        std::mem::replace(&mut self.sync_occlusion_requested, false)
    }

    pub fn capture(
        &mut self,
        dim: &str,
        center_x: i32,
        center_z: i32,
        radius: i32,
        chunks: Vec<RZeroFakeChunk>,
        entities: Vec<SnapshotEntityState>,
    ) -> bool {
        if !self.enabled || chunks.is_empty() {
            return false;
        }
        let r = radius.max(2);
        let mut grid = SpatialGrid::new(center_x, center_z, r);
        for chunk in chunks {
            grid.set(chunk.chunk_x, chunk.chunk_z, Some(chunk));
        }
        self.snapshot_grid = Some(grid);
        self.snapshot_entities.clear();
        for ent in entities {
            let cx = (ent.x.floor() as i32) >> 4;
            let cz = (ent.z.floor() as i32) >> 4;
            if (cx - center_x).abs() <= r && (cz - center_z).abs() <= r {
                self.snapshot_entities.insert(ent.uuid, ent);
            }
        }
        self.snapshot_dimension = Some(dim.to_owned());
        self.snapshot_center_x = center_x;
        self.snapshot_center_z = center_z;
        self.snapshot_radius = r;
        self.capture_pending = false;
        true
    }

    pub fn inject(&mut self, current_dim: &str, game_time: i64) -> usize {
        if !self.enabled {
            return 0;
        }
        if self.snapshot_dimension.as_deref() != Some(current_dim) {
            return 0;
        }
        let Some(snapshot) = self.snapshot_grid.clone() else {
            return 0;
        };

        self.detach_active();

        let r = snapshot.radius;
        let mut active = SpatialGrid::new(snapshot.center_x, snapshot.center_z, r);
        let mut attached = 0;
        for dx in -r..=r {
            for dz in -r..=r {
                let cx = snapshot.center_x + dx;
                let cz = snapshot.center_z + dz;
                if let Some(chunk) = snapshot.get(cx, cz) {
                    active.set(cx, cz, Some(chunk.clone()));
                    attached += 1;
                }
            }
        }

        self.active_grid = Some(active);
        self.active_fake_count = attached;
        self.pending_section_refresh = true;
        self.pending_refresh_attempts = 0;

        let mut ordered: Vec<&SnapshotEntityState> = self.snapshot_entities.values().collect();
        ordered.sort_by_key(|s| s.uuid);
        for state in ordered {
            self.next_fake_id += 1;
            let id = self.next_fake_id;
            self.spawned_fake_entity_ids.insert(state.uuid, id);
            let chunk_key =
                ChunkScope::pack_chunk((state.x.floor() as i32) >> 4, (state.z.floor() as i32) >> 4);
            self.fake_entity_by_chunk
                .entry(chunk_key)
                .or_default()
                .push(state.uuid);
        }

        self.refresh_rollback_flag();
        self.session_expires_at_tick = game_time + Self::SESSION_TTL_TICKS;
        attached
    }

    pub fn tick_pending_refresh(&mut self, player_chunk_x: i32, player_chunk_z: i32) -> usize {
        if !self.enabled || !self.pending_section_refresh {
            return 0;
        }
        let dx = (player_chunk_x - self.snapshot_center_x).abs();
        let dz = (player_chunk_z - self.snapshot_center_z).abs();
        if dx > self.snapshot_radius || dz > self.snapshot_radius {
            self.pending_refresh_attempts += 1;
            if self.pending_refresh_attempts >= Self::MAX_REFRESH_ATTEMPTS {
                self.pending_section_refresh = false;
                self.pending_refresh_attempts = 0;
            }
            return 0;
        }

        self.pending_section_refresh = false;
        self.pending_refresh_attempts = 0;
        self.request_sync_occlusion();
        self.mesh_cache.restore()
    }

    pub fn tick_session(
        &mut self,
        current_dim: &str,
        player_chunk_x: i32,
        player_chunk_z: i32,
        now_tick: i64,
    ) {
        if !self.enabled || !self.in_rollback || self.pending_section_refresh {
            return;
        }
        if self.snapshot_dimension.as_deref() != Some(current_dim) {
            return;
        }
        if self.session_expires_at_tick > 0 && now_tick >= self.session_expires_at_tick {
            self.session_expires_at_tick = -1;
            self.detach_active();
            return;
        }
        let dx = (player_chunk_x - self.snapshot_center_x).abs();
        let dz = (player_chunk_z - self.snapshot_center_z).abs();
        let limit = self.snapshot_radius + Self::LEAVE_BUFFER_CHUNKS;
        if dx > limit || dz > limit {
            self.session_expires_at_tick = -1;
            self.detach_active();
        }
    }

    pub fn on_real_chunk_arrived(&mut self, x: i32, z: i32) -> Vec<i32> {
        let mut removed_fake_ids = Vec::new();
        if let Some(grid) = &mut self.active_grid
            && grid.get(x, z).is_some()
        {
            grid.set(x, z, None);
            self.active_fake_count = self.active_fake_count.saturating_sub(1);
        }
        let key = ChunkScope::pack_chunk(x, z);
        if let Some(uuids) = self.fake_entity_by_chunk.remove(&key) {
            for uuid in uuids {
                if let Some(id) = self.spawned_fake_entity_ids.remove(&uuid) {
                    removed_fake_ids.push(id);
                }
            }
        }
        self.refresh_rollback_flag();
        removed_fake_ids
    }

    pub fn on_real_entity_added(&mut self, uuid: u128) -> Option<i32> {
        let removed = self.spawned_fake_entity_ids.remove(&uuid);
        self.refresh_rollback_flag();
        removed
    }

    fn refresh_rollback_flag(&mut self) {
        self.in_rollback = self.active_fake_count > 0 || !self.spawned_fake_entity_ids.is_empty();
    }

    pub fn detach_active(&mut self) {
        self.active_grid = None;
        self.active_fake_count = 0;
        self.spawned_fake_entity_ids.clear();
        self.fake_entity_by_chunk.clear();
        self.refresh_rollback_flag();
    }

    pub fn clear(&mut self) {
        self.detach_active();
        self.in_rollback = false;
        self.capture_pending = false;
        self.capture_attempts = 0;
        self.pending_section_refresh = false;
        self.pending_refresh_attempts = 0;
        self.is_inter_dimensional_rollback = false;
        self.inter_dimensional_arm_tick = -1;
        self.snapshot_dimension = None;
        self.snapshot_grid = None;
        self.snapshot_entities.clear();
        self.mesh_cache.clear();
    }
}

#[derive(Clone, Debug, Default)]
pub struct RZeroClient {
    pub saved_camera_type: Option<u8>,
    pub saved_sounds: HashSet<String>,
    pub skip_equip_animation_ticks: i32,
    pub snap_player_rotation_ticks: i32,
    pub saved_creative_tab: Option<usize>,
    pub saved_screen_kind: Option<String>,
    pub saved_recipe_book_category: Option<String>,
    pub saved_search_text: Option<String>,
    pub marked_chat_len: Option<usize>,
    pub cache: RZeroClientCache,
}

impl RZeroClient {
    pub fn handle_mark_chat(
        &mut self,
        camera_type: u8,
        screen_kind: Option<String>,
        chat_len: usize,
        playing_sounds: impl IntoIterator<Item = String>,
    ) {
        let policy = RZeroRuntime::with(|rt| rt.settings.client_restore);
        self.marked_chat_len = policy.restore_chat_state.then_some(chat_len);
        self.cache.request_capture();
        self.saved_camera_type = policy.camera_type.then_some(camera_type);
        self.saved_sounds.clear();
        if policy.stop_future_sounds {
            self.saved_sounds.extend(playing_sounds);
        }
        self.saved_screen_kind = if policy.restore_inventory_screen {
            screen_kind
        } else {
            None
        };
    }

    pub fn handle_rollback_chat(&mut self, payload: &RollbackChatPacket, current_dim: &str) {
        RZeroVisualRollbackManager::execute(&mut self.cache, current_dim, payload);
        let policy = RZeroRuntime::with(|rt| rt.settings.client_restore);
        self.skip_equip_animation_ticks = if policy.skip_equip_animation { 5 } else { 0 };
        self.snap_player_rotation_ticks = if policy.snap_player_rotation { 5 } else { 0 };
    }

    pub fn handle_rzerochash_toggle(&mut self, enabled: bool) {
        self.cache.set_enabled(enabled);
    }

    pub fn handle_sync_anchor_settings(&mut self, payload: &SyncAnchorSettingsPacket) {
        RZeroRuntime::with_mut(|rt| rt.set_anchor_settings(payload.settings.clone()));
    }

    pub fn client_tick(
        &mut self,
        current_dim: &str,
        player_chunk_x: i32,
        player_chunk_z: i32,
        now_tick: i64,
    ) {
        if self.skip_equip_animation_ticks > 0 {
            self.skip_equip_animation_ticks -= 1;
        }
        if self.snap_player_rotation_ticks > 0 {
            self.snap_player_rotation_ticks -= 1;
        }
        if self.cache.pending_section_refresh {
            self.cache
                .tick_pending_refresh(player_chunk_x, player_chunk_z);
        }
        self.cache
            .tick_session(current_dim, player_chunk_x, player_chunk_z, now_tick);
    }
}

pub struct RZeroVisualRollbackManager;

impl RZeroVisualRollbackManager {
    pub fn execute(
        cache: &mut RZeroClientCache,
        current_dim: &str,
        payload: &RollbackChatPacket,
    ) -> bool {
        if cache.snapshot_dimension.as_deref() != Some(current_dim) {
            return false;
        }
        cache.clear_inter_dimensional_rollback();
        let injected = cache.inject(current_dim, payload.game_time.max(0));
        cache.request_sync_occlusion();
        cache.pending_section_refresh = false;
        cache.pending_refresh_attempts = 0;
        log_info(&format!(
            "[cache] Visual rollback executed in {current_dim}: {injected} fake chunks injected"
        ));
        injected > 0
    }
}