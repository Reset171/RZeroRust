use crate::{block::SectionSnapshot, config::RZeroCheckpointPolicy, serial::RZBlob};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AdaptiveState {
    pub last_save_tick: i64,
    pub current_interval: i64,
    pub current_intensity: f64,
    pub peak_intensity: f64,
    pub relax_ticks: i32,
    pub last_boss_death_tick: i64,
    pub entity_id_counter: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkScope {
    pub dim: String,
    pub min_chunk_x: i32,
    pub min_chunk_z: i32,
    pub max_chunk_x: i32,
    pub max_chunk_z: i32,
}

impl ChunkScope {
    pub fn around(dim: &str, anchor_x: i32, anchor_z: i32, chunk_radius: i32) -> Self {
        let r = chunk_radius.max(0);
        let cx = anchor_x >> 4;
        let cz = anchor_z >> 4;
        Self {
            dim: dim.to_owned(),
            min_chunk_x: cx - r,
            min_chunk_z: cz - r,
            max_chunk_x: cx + r,
            max_chunk_z: cz + r,
        }
    }

    pub fn pack_chunk(x: i32, z: i32) -> i64 {
        (x as i64 & 0xFFFF_FFFF) | ((z as i64 & 0xFFFF_FFFF) << 32)
    }

    pub fn unpack_chunk(key: i64) -> (i32, i32) {
        (key as i32, (key >> 32) as i32)
    }

    pub fn includes(&self, test_dim: &str, chunk_key: i64) -> bool {
        if self.dim != test_dim {
            return false;
        }
        let (cx, cz) = Self::unpack_chunk(chunk_key);
        cx >= self.min_chunk_x
            && cx <= self.max_chunk_x
            && cz >= self.min_chunk_z
            && cz <= self.max_chunk_z
    }

    pub fn covers_dim(&self, test_dim: &str) -> bool {
        self.dim == test_dim
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub day_time: i64,
    pub game_time: i64,
    pub is_raining: bool,
    pub is_thundering: bool,
    pub rain_time: i32,
    pub thunder_time: i32,
    pub clear_weather_time: i32,
    pub rng_state: Vec<i64>,
    pub difficulty: String,
    pub difficulty_locked: bool,
    pub spawn_x: i32,
    pub spawn_y: i32,
    pub spawn_z: i32,
    pub spawn_angle: f32,
    pub has_world_spawn: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VillageSiegeSnapshot {
    pub has_setup_siege: bool,
    pub zombies_to_spawn: i32,
    pub next_spawn_time: i32,
    pub spawn_x: i32,
    pub spawn_y: i32,
    pub spawn_z: i32,
    pub state: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CustomSpawnersSnapshot {
    pub patrol_next_tick: Option<i32>,
    pub phantom_next_tick: Option<i32>,
    pub cat_next_tick: Option<i32>,
    pub village_siege: Option<VillageSiegeSnapshot>,
    pub trader_tick_delay: Option<i32>,
    pub trader_spawn_delay: Option<i32>,
    pub trader_spawn_chance: Option<i32>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ServerGlobalsSnapshot {
    pub bossbars: Option<RZBlob>,
    pub world_border: Option<RZBlob>,
    pub random_sequences: Option<RZBlob>,
    pub game_rules: HashMap<String, String>,
    pub forced_chunks: HashMap<String, Vec<i64>>,
    pub saved_data_snapshots: HashMap<String, HashMap<String, RZBlob>>,
    pub server_tick_count: i32,
    pub shuffling_counter: i64,
    pub level_random_state: Vec<i64>,
    pub custom_spawners: HashMap<String, CustomSpawnersSnapshot>,
    pub scheduled_events: Vec<RZBlob>,
    pub tick_rate: f32,
    pub is_frozen: bool,
    pub has_tick_rate: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EntityRamSnapshot {
    pub memories: HashMap<String, Option<RZBlob>>,
    pub has_look_control: bool,
    pub look_x: f64,
    pub look_y: f64,
    pub look_z: f64,
    pub look_y_max_rot_speed: f32,
    pub look_x_max_rot_angle: f32,
    pub look_at_cooldown: i32,
    pub has_move_control: bool,
    pub move_x: f64,
    pub move_y: f64,
    pub move_z: f64,
    pub move_speed_modifier: f64,
    pub move_strafe_forwards: f32,
    pub move_strafe_right: f32,
    pub move_operation: String,
    pub walk_dist_o: f32,
    pub walk_dist: f32,
    pub move_dist: f32,
    pub fly_dist: f32,
    pub next_step: f32,
    pub y_head_rot: f32,
    pub y_head_rot_o: f32,
    pub y_body_rot: f32,
    pub y_body_rot_o: f32,
    pub ambient_sound_time: i32,
    pub has_bat_target: bool,
    pub bat_target: [i32; 3],
    pub has_slime_jump_delay: bool,
    pub slime_jump_delay: i32,
    pub has_armadillo_state_ticks: bool,
    pub armadillo_in_state_ticks: i64,
    pub has_dragon_state: bool,
    pub dragon_pos_pointer: i32,
    pub dragon_o_flap_time: f32,
    pub dragon_flap_time: f32,
    pub dragon_y_rot_a: f32,
    pub dragon_in_wall: bool,
    pub dragon_positions: Vec<[f64; 3]>,
    pub cloned_brain_blob: Option<RZBlob>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EntitySnapshot {
    pub entity_id: i32,
    pub uuid: u128,
    pub kind: String,
    pub blob: RZBlob,
    pub chunk_key: i64,
    pub pos: [f64; 3],
    pub vel: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub head_yaw: f32,
    pub on_ground: bool,
    pub health: f32,
    pub target_uuid: Option<u128>,
    pub rng_state: Vec<i64>,
    pub tick_count: i32,
    pub has_path: bool,
    pub path_target: [i32; 3],
    pub path_speed: f64,
    pub has_living_timers: bool,
    pub invulnerable_time: i32,
    pub hurt_time: i32,
    pub death_time: i32,
    pub last_hurt: f32,
    pub attack_strength_ticker: i32,
    pub no_action_time: i32,
    pub no_jump_delay: i32,
    pub swing_time: i32,
    pub swinging: bool,
    pub use_item_remaining: i32,
    pub last_hurt_by_mob_timestamp: i32,
    pub portal_cooldown: i32,
    pub forced_age_timer: i32,
}

impl Default for EntitySnapshot {
    fn default() -> Self {
        Self {
            entity_id: 0,
            uuid: 0,
            kind: String::new(),
            blob: RZBlob::empty(),
            chunk_key: 0,
            pos: [0.0; 3],
            vel: [0.0; 3],
            yaw: 0.0,
            pitch: 0.0,
            head_yaw: 0.0,
            on_ground: false,
            health: 20.0,
            target_uuid: None,
            rng_state: Vec::new(),
            tick_count: 0,
            has_path: false,
            path_target: [0; 3],
            path_speed: 1.0,
            has_living_timers: false,
            invulnerable_time: 0,
            hurt_time: 0,
            death_time: 0,
            last_hurt: 0.0,
            attack_strength_ticker: 0,
            no_action_time: 0,
            no_jump_delay: 0,
            swing_time: 0,
            swinging: false,
            use_item_remaining: 0,
            last_hurt_by_mob_timestamp: 0,
            portal_cooldown: 0,
            forced_age_timer: 0,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MobRamLive {
    pub ram: EntityRamSnapshot,
    pub target_uuid: Option<u128>,
    pub has_path: bool,
    pub path_target: [i32; 3],
    pub path_speed: f64,
    pub rng_state: Vec<i64>,
    pub tick_count: i32,
    pub has_living_timers: bool,
    pub invulnerable_time: i32,
    pub hurt_time: i32,
    pub death_time: i32,
    pub last_hurt: f32,
    pub attack_strength_ticker: i32,
    pub no_action_time: i32,
    pub no_jump_delay: i32,
    pub swing_time: i32,
    pub swinging: bool,
    pub use_item_remaining: i32,
    pub last_hurt_by_mob_timestamp: i32,
    pub portal_cooldown: i32,
    pub captured_at_tick: i64,
}

impl MobRamLive {
    pub fn from_snapshots(ram: Option<&EntityRamSnapshot>, entity: Option<&EntitySnapshot>) -> Self {
        let mut live = Self::default();
        if let Some(r) = ram {
            live.ram = r.clone();
        }
        if let Some(e) = entity {
            live.target_uuid = e.target_uuid;
            live.has_path = e.has_path;
            live.path_target = e.path_target;
            live.path_speed = e.path_speed;
            live.rng_state = e.rng_state.clone();
            live.tick_count = e.tick_count;
            live.has_living_timers = e.has_living_timers;
            live.invulnerable_time = e.invulnerable_time;
            live.hurt_time = e.hurt_time;
            live.death_time = e.death_time;
            live.last_hurt = e.last_hurt;
            live.attack_strength_ticker = e.attack_strength_ticker;
            live.no_action_time = e.no_action_time;
            live.no_jump_delay = e.no_jump_delay;
            live.swing_time = e.swing_time;
            live.swinging = e.swinging;
            live.use_item_remaining = e.use_item_remaining;
            live.last_hurt_by_mob_timestamp = e.last_hurt_by_mob_timestamp;
            live.portal_cooldown = e.portal_cooldown;
        }
        live
    }

    pub fn apply_to_snapshot(&self, snap: &mut EntitySnapshot) {
        snap.target_uuid = self.target_uuid;
        snap.has_path = self.has_path;
        snap.path_target = self.path_target;
        snap.path_speed = self.path_speed;
        if !self.rng_state.is_empty() {
            snap.rng_state = self.rng_state.clone();
        }
        snap.tick_count = self.tick_count;
        if self.has_living_timers {
            snap.has_living_timers = true;
            snap.invulnerable_time = self.invulnerable_time;
            snap.hurt_time = self.hurt_time;
            snap.death_time = self.death_time;
            snap.last_hurt = self.last_hurt;
            snap.attack_strength_ticker = self.attack_strength_ticker;
            snap.no_action_time = self.no_action_time;
            snap.no_jump_delay = self.no_jump_delay;
            snap.swing_time = self.swing_time;
            snap.swinging = self.swinging;
            snap.use_item_remaining = self.use_item_remaining;
            snap.last_hurt_by_mob_timestamp = self.last_hurt_by_mob_timestamp;
            snap.portal_cooldown = self.portal_cooldown;
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenMenuSnapshot {
    pub menu_type_id: Option<String>,
    pub container_id: i32,
    pub title_json: Option<String>,
    pub anchor_block_pos: Option<[i32; 3]>,
    pub anchor_entity_uuid: Option<u128>,
    pub slots: Vec<RZBlob>,
    pub data_slots: Vec<i32>,
    pub carried_blob: Option<RZBlob>,
    pub craft_matrix: Vec<RZBlob>,
    pub creative_tab: i32,
    #[serde(skip)]
    pub menu_restore_attempts: i32,
    #[serde(skip)]
    pub enqueued_at_tick: i32,
}

impl OpenMenuSnapshot {
    pub fn is_anchor_ready(
        &self,
        has_entity: impl Fn(u128) -> bool,
        has_block_provider: impl Fn([i32; 3]) -> bool,
    ) -> bool {
        if let Some(uuid) = self.anchor_entity_uuid {
            return has_entity(uuid);
        }
        if let Some(pos) = self.anchor_block_pos {
            return has_block_provider(pos);
        }
        true
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdvancementSnapshot {
    pub completed: HashMap<String, Vec<String>>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CooldownEntry {
    pub item_id: String,
    pub remaining: i32,
    pub total: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PlayerData {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub motion_x: f64,
    pub motion_y: f64,
    pub motion_z: f64,
    pub y_rot: f32,
    pub x_rot: f32,
    pub fall_distance: f32,
    pub tick_count: i32,
    pub health: f32,
    pub absorption_amount: f32,
    pub invulnerable_time: i32,
    pub hurt_time: i32,
    pub death_time: i32,
    pub hurt_by_timestamp: i32,
    pub last_hurt: f32,
    pub attack_strength_ticker: i32,
    pub no_action_time: i32,
    pub no_jump_delay: i32,
    pub swing_time: i32,
    pub swinging: bool,
    pub swinging_arm: Option<String>,
    pub use_item: Option<RZBlob>,
    pub use_item_remaining: i32,
    pub arrow_count: i32,
    pub stinger_count: i32,
    pub air_supply: i32,
    pub fire_ticks: i32,
    pub portal_cooldown: i32,
    pub has_visual_fire: bool,
    pub glowing_tag: bool,
    pub hunger: i32,
    pub food_saturation_level: f32,
    pub food_exhaustion_level: f32,
    pub food_tick_timer: i32,
    pub experience_level: i32,
    pub experience_progress: f32,
    pub total_experience: i32,
    pub enchantment_seed: i32,
    pub score: i32,
    pub sleep_counter: i16,
    pub abilities_blob: Option<RZBlob>,
    pub shoulder_left: Option<RZBlob>,
    pub shoulder_right: Option<RZBlob>,
    pub last_death_dimension: Option<String>,
    pub last_death_pos: Option<[i32; 3]>,
    pub current_impulse_impact_pos: Option<[f64; 3]>,
    pub ignore_fall_damage_from_current_impulse: bool,
    pub current_impulse_context_reset_grace_time: i32,
    pub warden_spawn_tracker: Option<RZBlob>,
    pub entered_nether_position: Option<[f64; 3]>,
    pub starting_to_fall_position: Option<[f64; 3]>,
    pub entered_lava_on_vehicle_position: Option<[f64; 3]>,
    pub levitation_start_pos: Option<[f64; 3]>,
    pub levitation_start_time: i32,
    pub seen_credits: bool,
    pub spawn_extra_particles_on_fall: bool,
    pub raid_omen_position: Option<[i32; 3]>,
    pub respawn_angle: f32,
    pub recipe_book_blob: Option<RZBlob>,
    pub game_mode: String,
    pub dimension: String,
    pub spawn_block: Option<[i32; 3]>,
    pub spawn_dimension: Option<String>,
    pub spawn_forced: bool,
    pub inventory: Vec<RZBlob>,
    pub ender_chest_inventory: Vec<RZBlob>,
    pub selected_slot: usize,
    pub potion_effects: Vec<RZBlob>,
    pub vehicle_uuid: Option<u128>,
    pub rng_state: Vec<i64>,
    pub item_cooldowns: Vec<CooldownEntry>,
    pub advancements: AdvancementSnapshot,
    pub open_menu: Option<OpenMenuSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockEntityEntry {
    pub pos: [i32; 3],
    pub blob: RZBlob,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduledTickEntry {
    pub pos: [i32; 3],
    pub id: String,
    pub delay: i64,
    pub priority: i32,
    pub sub_tick_order: i64,
}

pub struct ScheduledTickCodec;

impl ScheduledTickCodec {
    pub fn encode(
        ticks: &[([i32; 3], String, i64, i32, i64)],
        game_time: i64,
    ) -> Vec<ScheduledTickEntry> {
        ticks
            .iter()
            .map(|(pos, id, trigger_tick, priority, sub)| ScheduledTickEntry {
                pos: *pos,
                id: id.clone(),
                delay: *trigger_tick - game_time,
                priority: *priority,
                sub_tick_order: *sub,
            })
            .collect()
    }

    pub fn decode(
        entries: &[ScheduledTickEntry],
        game_time: i64,
    ) -> Vec<([i32; 3], String, i64, i32, i64)> {
        entries
            .iter()
            .map(|e| {
                (
                    e.pos,
                    e.id.clone(),
                    game_time + e.delay,
                    e.priority,
                    e.sub_tick_order,
                )
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockEventEntry {
    pub pos: [i32; 3],
    pub block: String,
    pub param_a: i32,
    pub param_b: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoiEntry {
    pub pos: [i32; 3],
    pub poi_type: String,
    pub free_tickets: i32,
}

pub struct OfflinePlayerFiles;

impl OfflinePlayerFiles {
    pub fn player_data_dir(world_root: &Path) -> PathBuf {
        let horizon_dir = world_root.join("players").join("data");
        if horizon_dir.is_dir() {
            horizon_dir
        } else {
            world_root.join("playerdata")
        }
    }

    pub fn player_stats_dir(world_root: &Path) -> PathBuf {
        world_root.join("stats")
    }

    pub fn backup_into(world_root: &Path, target: &mut HashMap<u128, Vec<u8>>) {
        let dir = Self::player_data_dir(world_root);
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("dat") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if let Some(uuid) = Self::parse_uuid(stem)
                && let Ok(bytes) = fs::read(&path)
            {
                target.insert(uuid, bytes);
            }
        }
    }

    pub fn restore_from(world_root: &Path, saved: &HashMap<u128, Vec<u8>>) {
        if saved.is_empty() {
            return;
        }
        let dir = Self::player_data_dir(world_root);
        let _ = fs::create_dir_all(&dir);
        for (&uuid, bytes) in saved {
            let path = dir.join(format!("{}.dat", Self::format_uuid(uuid)));
            let _ = fs::write(path, bytes);
        }
    }

    pub fn backup_stats_into(world_root: &Path, target: &mut HashMap<u128, String>) {
        let dir = Self::player_stats_dir(world_root);
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if let Some(uuid) = Self::parse_uuid(stem)
                && let Ok(text) = fs::read_to_string(&path)
            {
                target.insert(uuid, text);
            }
        }
    }

    pub fn restore_stats_from(world_root: &Path, saved: &HashMap<u128, String>) {
        if saved.is_empty() {
            return;
        }
        let dir = Self::player_stats_dir(world_root);
        let _ = fs::create_dir_all(&dir);
        for (&uuid, json) in saved {
            let path = dir.join(format!("{}.json", Self::format_uuid(uuid)));
            let _ = fs::write(path, json);
        }
    }

    pub fn parse_uuid(s: &str) -> Option<u128> {
        let hex: String = s.chars().filter(|c| *c != '-').collect();
        if hex.len() != 32 {
            return None;
        }
        u128::from_str_radix(&hex, 16).ok()
    }

    pub fn format_uuid(u: u128) -> String {
        format!(
            "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
            (u >> 96) as u32,
            ((u >> 80) & 0xFFFF) as u16,
            ((u >> 64) & 0xFFFF) as u16,
            ((u >> 48) & 0xFFFF) as u16,
            (u & 0xFFFF_FFFF_FFFF) as u64,
        )
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PendingSpawnLedger {
    pub chunk_last_epoch: HashMap<i64, i64>,
    pub monster_last_epoch: HashMap<i64, i64>,
}

impl PendingSpawnLedger {
    pub const MAX_CATCHUP_EPOCHS: i64 = 2;

    pub fn catch_up_from(&mut self, chunk_key: i64, now_epoch: i64) -> i64 {
        let last = self.chunk_last_epoch.insert(chunk_key, now_epoch);
        let Some(last_epoch) = last else {
            return now_epoch + 1;
        };
        if last_epoch >= now_epoch {
            return now_epoch + 1;
        }
        let first = last_epoch + 1;
        let capped = now_epoch - Self::MAX_CATCHUP_EPOCHS + 1;
        first.max(capped)
    }

    pub fn monster_owes_replay(&mut self, chunk_key: i64, day_epoch: i64) -> bool {
        if self.monster_last_epoch.get(&chunk_key).copied() == Some(day_epoch) {
            return false;
        }
        self.monster_last_epoch.insert(chunk_key, day_epoch);
        true
    }

    pub fn clear_monster_progress(&mut self) {
        self.monster_last_epoch.clear();
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CheckpointData {
    pub anchor_id: Option<u128>,
    pub anchor_ids: HashSet<u128>,
    pub policy: RZeroCheckpointPolicy,
    pub players_data: HashMap<u128, PlayerData>,
    pub raw_players_nbt: HashMap<u128, Vec<u8>>,
    pub raw_player_stats: HashMap<u128, String>,
    pub world_state: Option<WorldSnapshot>,
    pub adaptive_state: Option<AdaptiveState>,
    pub raids_blob: Option<RZBlob>,
    pub scoreboard_blob: Option<RZBlob>,
    pub dragon_fight_blob: Option<RZBlob>,
    pub server_globals: Option<ServerGlobalsSnapshot>,
    pub scope: Option<ChunkScope>,
    pub chunk_pois: HashMap<i64, Vec<PoiEntry>>,
    pub entities: Vec<EntitySnapshot>,
    pub entities_by_chunk: HashMap<i64, Vec<EntitySnapshot>>,
    pub entity_ram_snapshots: HashMap<u128, EntityRamSnapshot>,
    pub section_snapshots: HashMap<i64, Vec<Option<SectionSnapshot>>>,
    pub chunk_block_entities: HashMap<i64, Vec<BlockEntityEntry>>,
    pub chunk_block_ticks: HashMap<i64, Vec<ScheduledTickEntry>>,
    pub chunk_fluid_ticks: HashMap<i64, Vec<ScheduledTickEntry>>,
    pub block_events: Vec<BlockEventEntry>,
    pub chunk_inhabited_time: HashMap<i64, i64>,
    pub pending_block_rollbacks: HashSet<i64>,
}

impl CheckpointData {
    pub fn is_anchor(&self, candidate: u128) -> bool {
        if self.anchor_ids.is_empty() {
            self.anchor_id == Some(candidate)
        } else {
            self.anchor_ids.contains(&candidate)
        }
    }

    pub fn get_entities_for_chunk(&self, chunk_key: i64) -> &[EntitySnapshot] {
        self.entities_by_chunk
            .get(&chunk_key)
            .map_or(&[], Vec::as_slice)
    }

    pub fn save_to_bytes(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(self).map_err(|e| e.to_string())
    }

    pub fn load_from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let mut data: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if data.anchor_ids.is_empty()
            && let Some(id) = data.anchor_id
        {
            data.anchor_ids.insert(id);
        }
        data.entities_by_chunk.clear();
        for es in &data.entities {
            data.entities_by_chunk
                .entry(es.chunk_key)
                .or_default()
                .push(es.clone());
        }
        Ok(data)
    }

    pub fn data_file_path(world_root: &Path, dim: &str) -> PathBuf {
        let dim_path = dim.rsplit(':').next().unwrap_or(dim);
        world_root
            .join("data")
            .join(format!("rzero_data_{dim_path}.json"))
    }

    pub fn save_to_disk(&self, world_root: &Path, dim: &str) -> Result<(), String> {
        let path = Self::data_file_path(world_root, dim);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let bytes = self.save_to_bytes()?;
        let staging = path.with_extension("json.staging");
        fs::write(&staging, bytes).map_err(|e| {
            let _ = fs::remove_file(&staging);
            e.to_string()
        })?;
        fs::rename(&staging, &path).map_err(|e| {
            let _ = fs::remove_file(&staging);
            e.to_string()
        })
    }

    pub fn load_from_disk(world_root: &Path, dim: &str) -> Option<Self> {
        let path = Self::data_file_path(world_root, dim);
        let bytes = fs::read(path).ok()?;
        Self::load_from_bytes(&bytes).ok()
    }
}