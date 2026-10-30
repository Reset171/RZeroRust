use super::super::Dim;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

pub const MOD_ID: &str = "rzero";
pub const RBD_SOUND_ID: &str = "rzero:rbd_sound";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnchorMode {
    Fixed,
    Multi,
    Everyone,
    Rotating,
}

impl AnchorMode {
    pub fn id(self) -> &'static str {
        match self {
            Self::Fixed => "fixed",
            Self::Multi => "multi",
            Self::Everyone => "everyone",
            Self::Rotating => "rotating",
        }
    }

    pub fn by_id(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "multi" => Self::Multi,
            "everyone" => Self::Everyone,
            "rotating" => Self::Rotating,
            _ => Self::Fixed,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Fixed => Self::Multi,
            Self::Multi => Self::Everyone,
            Self::Everyone => Self::Rotating,
            Self::Rotating => Self::Fixed,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AnchorSettings {
    pub mode: AnchorMode,
    pub rotation_seconds: i32,
    pub rollback_cooldown_seconds: i32,
    pub pinned: Vec<u128>,
}

impl Default for AnchorSettings {
    fn default() -> Self {
        Self {
            mode: AnchorMode::Fixed,
            rotation_seconds: 300,
            rollback_cooldown_seconds: 0,
            pinned: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RollbackPolicy {
    pub blocks: bool,
    pub biomes: bool,
    pub block_entities: bool,
    pub block_ticks: bool,
    pub fluid_ticks: bool,
    pub block_events: bool,
    pub entities_presence: bool,
    pub dropped_items: bool,
    pub experience_orbs: bool,
    pub player_position: bool,
    pub player_motion: bool,
    pub player_health: bool,
    pub player_hunger: bool,
    pub player_experience: bool,
    pub player_inventory: bool,
    pub player_ender_chest: bool,
    pub player_potion_effects: bool,
    pub player_cooldowns: bool,
    pub player_abilities: bool,
    pub player_recipe_book: bool,
    pub player_spawn_point: bool,
    pub player_stats: bool,
    pub preserve_new_player_inventory: bool,
    pub world_time: bool,
    pub world_weather: bool,
    pub world_raids: bool,
    pub world_dragon_fight: bool,
    pub world_scoreboard: bool,
    pub world_level_rng: bool,
    pub world_difficulty: bool,
    pub world_spawn: bool,
    pub server_bossbars: bool,
    pub server_forced_chunks: bool,
    pub server_world_border: bool,
    pub server_game_rules: bool,
}

impl Default for RollbackPolicy {
    fn default() -> Self {
        Self {
            blocks: true,
            biomes: true,
            block_entities: true,
            block_ticks: true,
            fluid_ticks: true,
            block_events: true,
            entities_presence: true,
            dropped_items: true,
            experience_orbs: true,
            player_position: true,
            player_motion: true,
            player_health: true,
            player_hunger: true,
            player_experience: true,
            player_inventory: true,
            player_ender_chest: true,
            player_potion_effects: true,
            player_cooldowns: true,
            player_abilities: true,
            player_recipe_book: true,
            player_spawn_point: true,
            player_stats: true,
            preserve_new_player_inventory: false,
            world_time: true,
            world_weather: true,
            world_raids: true,
            world_dragon_fight: true,
            world_scoreboard: true,
            world_level_rng: true,
            world_difficulty: true,
            world_spawn: true,
            server_bossbars: true,
            server_forced_chunks: true,
            server_world_border: true,
            server_game_rules: true,
        }
    }
}

pub struct RZeroState {
    pub auto_save: bool,
    pub adaptive_mode: bool,
    pub use_random_interval: bool,
    pub fixed_interval_sec: i32,
    pub random_min_sec: i32,
    pub random_max_sec: i32,
    pub adaptive_interval_sec: i32,
    pub min_save_gap_sec: i32,
    pub relax_time_sec: i32,
    pub post_combat_save_pct: i32,
    pub save_on_boss_defeat: bool,
    pub play_rollback_sound: bool,
    pub logs: bool,
    pub policy: RollbackPolicy,
    pub anchor: AnchorSettings,
    pub anchor_ids: HashSet<u128>,
    pub has_checkpoint: bool,
    pub last_save_tick: i64,
    pub current_interval_ticks: i64,
    pub last_rollback_tick: Option<i64>,
    pub last_boss_death_tick: i64,
    pub current_intensity: f64,
    pub peak_intensity: f64,
    pub relax_ticks: i32,
    pub pending_death_rollback: Option<u128>,
    pub pending_save_request: bool,
    pub pending_load_request: bool,
}

impl Default for RZeroState {
    fn default() -> Self {
        Self {
            auto_save: true,
            adaptive_mode: false,
            use_random_interval: false,
            fixed_interval_sec: 300,
            random_min_sec: 300,
            random_max_sec: 600,
            adaptive_interval_sec: 300,
            min_save_gap_sec: 15,
            relax_time_sec: 10,
            post_combat_save_pct: 100,
            save_on_boss_defeat: true,
            play_rollback_sound: true,
            logs: true,
            policy: RollbackPolicy::default(),
            anchor: AnchorSettings::default(),
            anchor_ids: HashSet::new(),
            has_checkpoint: false,
            last_save_tick: 0,
            current_interval_ticks: 6000,
            last_rollback_tick: None,
            last_boss_death_tick: -9999,
            current_intensity: 0.0,
            peak_intensity: 0.0,
            relax_ticks: 0,
            pending_death_rollback: None,
            pending_save_request: false,
            pending_load_request: false,
        }
    }
}

static STATE: OnceLock<Mutex<RZeroState>> = OnceLock::new();

fn state_lock() -> &'static Mutex<RZeroState> {
    STATE.get_or_init(|| Mutex::new(RZeroState::default()))
}

pub fn with_state<R>(f: impl FnOnce(&RZeroState) -> R) -> R {
    let guard = state_lock().lock().unwrap_or_else(|e| e.into_inner());
    f(&guard)
}

pub fn with_state_mut<R>(f: impl FnOnce(&mut RZeroState) -> R) -> R {
    let mut guard = state_lock().lock().unwrap_or_else(|e| e.into_inner());
    f(&mut guard)
}

pub fn raids_dat_path(world_root: &Path, dim: Dim) -> PathBuf {
    world_root.join(dim.folder()).join("data").join("raids.dat")
}

pub fn fight_dat_path(world_root: &Path) -> PathBuf {
    world_root.join("data").join("ender_dragon_fight.dat")
}

pub fn resolve_anchors(
    settings: &AnchorSettings,
    online: &[u128],
    game_time: i64,
    requester: Option<u128>,
) -> Vec<u128> {
    match settings.mode {
        AnchorMode::Fixed => {
            if let Some(req) = requester {
                vec![req]
            } else if !settings.pinned.is_empty() {
                settings.pinned.clone()
            } else if let Some(&first) = online.first() {
                vec![first]
            } else {
                Vec::new()
            }
        }
        AnchorMode::Multi => {
            if !settings.pinned.is_empty() {
                settings.pinned.clone()
            } else {
                online.to_vec()
            }
        }
        AnchorMode::Everyone => online.to_vec(),
        AnchorMode::Rotating => {
            if online.is_empty() {
                return Vec::new();
            }
            let rotation_ticks = i64::from(settings.rotation_seconds.max(5)) * 20;
            let index = ((game_time / rotation_ticks).max(0) as usize) % online.len();
            vec![online[index]]
        }
    }
}
