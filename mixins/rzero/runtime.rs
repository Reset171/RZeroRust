// Auto-generated from mixins/rzero/runtime/*.rs. DO NOT EDIT DIRECTLY.
// To modify, edit files in mixins/rzero/runtime/ and run cargo build.

use super::super::{
    Dim, ExperienceOrb, FallingBlock, Game, ItemEntity, Mob,
    boats::Boat,
    commands::OtherEntity,
    dragon::{Crystal, Dragon},
    lan::LanHost,
    machines::HopperState,
    minecarts::Minecart,
    native_hooks::NativeMod,
    native_lifecycle::WorldPhase,
    tnt::PrimedTnt,
};
use horizon_client::{
    advancements::Progress,
    blocks::Fluid,
    bossbar::{self, BossBars},
    chunk::Chunk,
    containers::BlockEntity,
    cooldowns::Cooldowns,
    effects::EffectInstance,
    entities::{ArmorStand, Cushion, ItemFrame, Painting},
    falling_block::Damage as FallingDamage,
    fast_hash::FxHashMap,
    gamerules::GameRules,
    items::ItemStack,
    lan::guest::SavedBody,
    mobs::Species,
    projectiles::Projectile,
    recipe_book::Settings as RecipeBookSettings,
    save::{item_from_nbt, read_dat, write_dat, Level},
    scoreboard::Scoreboard,
    sculk::WardenSpawnTracker,
    text::Text,
    world::{HealthRules, Weather},
    worldborder::WorldBorder,
};
use horizon_core::{
    geometry::Vec3,
    position::{BlockPos, ChunkPos},
    random::{LegacyRandom, RandomSource},
    ticks::ChunkTicks,
};
use horizon_protocol::{
    packets::{
        play::{self, Clientbound as C},
        recipes,
    },
    text::literal,
};
use horizon_world::{
    level::{BlockEventData, MovingPiston, Ticks},
    nbt::Tag,
    registry::BlockStateId,
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    fs::{self, OpenOptions},
    io::{Read, Write},
    ops::ControlFlow,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, OnceLock, RwLock,
    },
    time::Instant,
};

// ============================================================================
// Submodule: types.rs
// ============================================================================


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

impl RollbackPolicy {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "blocks": self.blocks,
            "biomes": self.biomes,
            "block_entities": self.block_entities,
            "block_ticks": self.block_ticks,
            "fluid_ticks": self.fluid_ticks,
            "block_events": self.block_events,
            "entities_presence": self.entities_presence,
            "dropped_items": self.dropped_items,
            "experience_orbs": self.experience_orbs,
            "player_position": self.player_position,
            "player_motion": self.player_motion,
            "player_health": self.player_health,
            "player_hunger": self.player_hunger,
            "player_experience": self.player_experience,
            "player_inventory": self.player_inventory,
            "player_ender_chest": self.player_ender_chest,
            "player_potion_effects": self.player_potion_effects,
            "player_cooldowns": self.player_cooldowns,
            "player_abilities": self.player_abilities,
            "player_recipe_book": self.player_recipe_book,
            "player_spawn_point": self.player_spawn_point,
            "player_stats": self.player_stats,
            "preserve_new_player_inventory": self.preserve_new_player_inventory,
            "world_time": self.world_time,
            "world_weather": self.world_weather,
            "world_raids": self.world_raids,
            "world_dragon_fight": self.world_dragon_fight,
            "world_scoreboard": self.world_scoreboard,
            "world_level_rng": self.world_level_rng,
            "world_difficulty": self.world_difficulty,
            "world_spawn": self.world_spawn,
            "server_bossbars": self.server_bossbars,
            "server_forced_chunks": self.server_forced_chunks,
            "server_world_border": self.server_world_border,
            "server_game_rules": self.server_game_rules,
        })
    }

    pub fn set_field(&mut self, key: &str, val: bool) -> bool {
        let norm = key.trim().to_ascii_lowercase().replace('.', "_");
        match norm.as_str() {
            "blocks" => self.blocks = val,
            "biomes" => self.biomes = val,
            "block_entities" => self.block_entities = val,
            "block_ticks" => self.block_ticks = val,
            "fluid_ticks" => self.fluid_ticks = val,
            "block_events" => self.block_events = val,
            "entities_presence" | "entities" => self.entities_presence = val,
            "dropped_items" => self.dropped_items = val,
            "experience_orbs" => self.experience_orbs = val,
            "player_position" | "position" => self.player_position = val,
            "player_motion" | "motion" => self.player_motion = val,
            "player_health" | "health" => self.player_health = val,
            "player_hunger" | "hunger" => self.player_hunger = val,
            "player_experience" | "experience" | "xp" => self.player_experience = val,
            "player_inventory" | "inventory" => self.player_inventory = val,
            "player_ender_chest" | "ender_chest" => self.player_ender_chest = val,
            "player_potion_effects" | "effects" => self.player_potion_effects = val,
            "player_cooldowns" | "cooldowns" => self.player_cooldowns = val,
            "player_abilities" | "abilities" => self.player_abilities = val,
            "player_recipe_book" | "recipe_book" => self.player_recipe_book = val,
            "player_spawn_point" | "spawn_point" => self.player_spawn_point = val,
            "player_stats" | "stats" => self.player_stats = val,
            "preserve_new_player_inventory" => self.preserve_new_player_inventory = val,
            "world_time" | "time" => self.world_time = val,
            "world_weather" | "weather" => self.world_weather = val,
            "world_raids" | "raids" => self.world_raids = val,
            "world_dragon_fight" | "dragon_fight" => self.world_dragon_fight = val,
            "world_scoreboard" | "scoreboard" => self.world_scoreboard = val,
            "world_level_rng" | "level_rng" | "rng" => self.world_level_rng = val,
            "world_difficulty" | "difficulty" => self.world_difficulty = val,
            "world_spawn" | "spawn" => self.world_spawn = val,
            "server_bossbars" | "bossbars" => self.server_bossbars = val,
            "server_forced_chunks" | "forced_chunks" => self.server_forced_chunks = val,
            "server_world_border" | "world_border" | "border" => self.server_world_border = val,
            "server_game_rules" | "game_rules" | "rules" => self.server_game_rules = val,
            _ => return false,
        }
        true
    }
}

#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RollbackBenchmark {
    pub total_ms: f64,
    pub globals_ms: f64,
    pub blocks_ms: f64,
    pub containers_ms: f64,
    pub entities_ms: f64,
    pub player_ms: f64,
}

pub struct RZeroState {
    pub current_slot: String,
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
    pub last_benchmark: Option<RollbackBenchmark>,
    pub f3_loop_hitboxes: bool,
}

impl Default for RZeroState {
    fn default() -> Self {
        Self {
            current_slot: "default".to_owned(),
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
            last_benchmark: None,
            f3_loop_hitboxes: true,
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
    let dim_folder = match dim {
        Dim::Overworld => "minecraft/overworld",
        Dim::Nether => "minecraft/the_nether",
        Dim::End => "minecraft/the_end",
    };
    world_root
        .join("dimensions")
        .join(dim_folder)
        .join("data/minecraft/raids.dat")
}

pub fn fight_dat_path(world_root: &Path) -> PathBuf {
    world_root.join("data/minecraft/ender_dragon_fight.dat")
}

pub fn trader_dat_path(world_root: &Path) -> PathBuf {
    world_root.join("data/minecraft/wandering_trader.dat")
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

// ============================================================================
// Submodule: snapshots.rs
// ============================================================================


#[derive(Clone, Debug)]
pub struct FallingSnapshot {
    pub join: horizon_client::join_seq::JoinSeq,
    pub id: u32,
    pub state: BlockStateId,
    pub start_pos: BlockPos,
    pub position: Vec3,
    pub previous: Vec3,
    pub velocity: Vec3,
    pub age: i32,
    pub no_gravity: bool,
    pub portal: horizon_client::nether_portal::PortalState,
    pub drop_item: bool,
    pub fall_distance: f64,
    pub stuck: Vec3,
    pub damage: FallingDamage,
    pub saved: Option<Box<([i32; 4], Tag)>>,
}

impl FallingSnapshot {
    pub fn from_live(f: &FallingBlock) -> Self {
        Self {
            join: f.join,
            id: f.id,
            state: f.state,
            start_pos: f.start_pos,
            position: f.position,
            previous: f.previous,
            velocity: f.velocity,
            age: f.age,
            no_gravity: f.no_gravity,
            portal: f.portal.clone(),
            drop_item: f.drop_item,
            fall_distance: f.fall_distance,
            stuck: f.stuck,
            damage: f.damage,
            saved: f.saved.clone(),
        }
    }

    pub fn to_live(&self) -> FallingBlock {
        FallingBlock {
            join: self.join,
            id: self.id,
            state: self.state,
            start_pos: self.start_pos,
            position: self.position,
            previous: self.previous,
            velocity: self.velocity,
            age: self.age,
            no_gravity: self.no_gravity,
            portal: self.portal.clone(),
            drop_item: self.drop_item,
            fall_distance: self.fall_distance,
            stuck: self.stuck,
            damage: self.damage,
            saved: self.saved.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DragonSnapshot {
    pub id: u32,
    pub nbt: Tag,
    pub previous: Vec3,
    pub previous_yaw: f32,
    pub hurt_time: i32,
    pub flap: f32,
    pub old_flap: f32,
    pub parts: [(Vec3, Vec3); 8],
}

impl DragonSnapshot {
    pub fn from_live(d: &Dragon) -> Self {
        Self {
            id: d.id,
            nbt: d.to_nbt([0; 4], None),
            previous: d.previous,
            previous_yaw: d.previous_yaw,
            hurt_time: d.hurt_time,
            flap: d.flap,
            old_flap: d.old_flap,
            parts: d.parts,
        }
    }

    pub fn to_live(&self, seed: i64) -> Option<Dragon> {
        let mut d = Dragon::from_nbt(&self.nbt, seed)?;
        d.id = self.id;
        d.previous = self.previous;
        d.previous_yaw = self.previous_yaw;
        d.hurt_time = self.hurt_time;
        d.flap = self.flap;
        d.old_flap = self.old_flap;
        d.parts = self.parts;
        Some(d)
    }
}

#[derive(Clone)]
pub struct GuestSnapshot {
    pub uuid: u128,
    pub dimension: String,
    pub position: [f64; 3],
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub head_yaw: f32,
    pub on_ground: bool,
    pub game_mode: i32,
    pub flying: bool,
    pub sprinting: bool,
    pub sneaking: bool,
    pub fall_flying: bool,
    pub sleeping: bool,
    pub bed: Option<[i32; 3]>,
    pub sleep_timer: u32,
    pub vehicle: Option<u32>,
    pub vehicle_good: Option<[f64; 3]>,
    pub spawn: Option<[f64; 3]>,
    pub respawn_elsewhere: Option<(String, [i32; 3], f32, bool)>,
    pub last_death: Option<(String, [i32; 3])>,
    pub health: f32,
    pub absorption: f32,
    pub food: i32,
    pub saturation: f32,
    pub exhaustion: f32,
    pub fire_ticks: i32,
    pub air: i32,
    pub ticks_frozen: i32,
    pub fall_distance: f64,
    pub xp_level: i32,
    pub xp_progress: f32,
    pub xp_total: i32,
    pub xp_seed: i32,
    pub inventory: Vec<Option<ItemStack>>,
    pub armor: [Option<ItemStack>; 4],
    pub offhand: Option<ItemStack>,
    pub carried: Option<ItemStack>,
    pub ender_items: Vec<Option<ItemStack>>,
    pub selected: usize,
    pub craft_grid: [Option<ItemStack>; 4],
    pub craft_result: Option<ItemStack>,
    pub effects: Vec<EffectInstance>,
    pub shield_cooldown: u32,
    pub cooldowns: Cooldowns,
    pub recipe_book: RecipeBookSettings,
    pub advancements: Progress,
    pub stats: BTreeMap<String, BTreeMap<String, i32>>,
    pub seen_credits: bool,
    pub saved: Option<Tag>,
    pub hook: Option<HookSnapshot>,
}

#[derive(Clone, Debug)]
pub struct HookSnapshot {
    pub id: u32,
    pub position: [f64; 3],
}

#[derive(Clone)]
pub struct LevelSnapshot {
    pub chunks: FxHashMap<(i32, i32), Arc<Chunk>>,
    pub block_entities: HashMap<(i32, i32, i32), BlockEntity>,
    pub block_names: FxHashMap<(i32, i32, i32), String>,
    pub inhabited: FxHashMap<(i32, i32), i64>,
    pub random: LegacyRandom,
    pub forced_chunks: HashSet<(i32, i32)>,
    pub chunk_ticks: HashMap<(i32, i32), (ChunkTicks, ChunkTicks)>,
    pub random_tick_value: i32,
    pub comparator_outputs: HashMap<BlockPos, i32>,
    pub torch_toggles: Vec<(BlockPos, i64)>,
    pub moving_pistons: Vec<(BlockPos, MovingPiston)>,
    pub piston_pushes: Vec<(BlockPos, MovingPiston, f32)>,
    pub block_events: Vec<BlockEventData>,
    pub hoppers: HashMap<(i32, i32, i32), HopperState>,
    pub mobs: horizon_client::join_seq::JoinList<Mob>,
    pub dropped: horizon_client::join_seq::JoinList<ItemEntity>,
    pub orbs: horizon_client::join_seq::JoinList<ExperienceOrb>,
    pub projectiles: horizon_client::join_seq::JoinList<horizon_client::projectiles::Projectile>,
    pub boats: horizon_client::join_seq::JoinList<Boat>,
    pub carts: horizon_client::join_seq::JoinList<Minecart>,
    pub tnts: horizon_client::join_seq::JoinList<PrimedTnt>,
    pub falling: Vec<FallingSnapshot>,
    pub paintings: Vec<Painting>,
    pub item_frames: Vec<ItemFrame>,
    pub armor_stands: horizon_client::join_seq::JoinList<ArmorStand>,
    pub cushions: Vec<Cushion>,
    pub others: Vec<OtherEntity>,
    pub crystals: Vec<Crystal>,
    pub dragon: Option<DragonSnapshot>,
    pub knot_ids: HashMap<[i32; 3], u32>,
    pub persist_uuids: HashMap<u32, [i32; 4]>,
    pub persist_raw: HashMap<u32, Tag>,
    pub raids_dat: Option<Vec<u8>>,
    pub last_hurt_by_player: HashMap<u32, Option<u32>>,
    pub love_cause: HashMap<u32, Option<u32>>,
}

#[derive(Clone)]
pub struct TimelineSnapshot {
    pub anchor_ids: HashSet<u128>,
    pub policy: RollbackPolicy,
    pub game_time: i64,
    pub day_time: i64,
    pub weather: Weather,
    pub health_rules: HealthRules,
    pub rules: GameRules,
    pub border: WorldBorder,
    pub bossbars: Tag,
    pub sculk_tracker: WardenSpawnTracker,
    pub sculk_darkness: (f32, f32),
    pub dragon_fight_dat: Option<Vec<u8>>,
    pub trader_dat: Option<Vec<u8>>,
    pub rand_sequences: horizon_client::random_sequences::RandomSequences,
    pub region_files: HashSet<PathBuf>,
    pub difficulty: u8,
    pub difficulty_locked: bool,
    pub clock_rate: f32,
    pub clock_frozen: bool,
    pub time_rate: f32,
    pub world_spawn: Option<[i32; 3]>,
    pub world_spawn_angle: [f32; 2],
    pub next_entity: u32,
    pub scoreboard: Scoreboard,
    pub guests: HashMap<u128, GuestSnapshot>,
    pub offline_player_data: HashMap<PathBuf, Vec<u8>>,
    pub levels: HashMap<Dim, LevelSnapshot>,
    pub mob_players: FxHashMap<u32, u32>,
}

#[derive(Clone)]
pub struct ClientBlockSnapshot {
    pub dim: Dim,
    pub chunks: FxHashMap<(i32, i32), Arc<Chunk>>,
}

static CLIENT_SNAPSHOT: OnceLock<RwLock<Option<ClientBlockSnapshot>>> = OnceLock::new();
static CLIENT_SLOTS: OnceLock<RwLock<FxHashMap<String, ClientBlockSnapshot>>> = OnceLock::new();

fn client_snapshot_lock() -> &'static RwLock<Option<ClientBlockSnapshot>> {
    CLIENT_SNAPSHOT.get_or_init(|| RwLock::new(None))
}

fn client_slots_lock() -> &'static RwLock<FxHashMap<String, ClientBlockSnapshot>> {
    CLIENT_SLOTS.get_or_init(|| RwLock::new(FxHashMap::default()))
}

pub fn get_client_snapshot_clone() -> Option<ClientBlockSnapshot> {
    client_snapshot_lock().read().ok()?.clone()
}

pub fn set_client_snapshot(snap: Option<ClientBlockSnapshot>) {
    if let Ok(mut lock) = client_snapshot_lock().write() {
        *lock = snap;
    }
}

pub fn get_client_slot_snapshot_clone(slot: &str) -> Option<ClientBlockSnapshot> {
    if slot == "default" {
        return get_client_snapshot_clone();
    }
    client_slots_lock().read().ok()?.get(slot).cloned()
}

pub fn set_client_slot_snapshot(slot: &str, snap: ClientBlockSnapshot) {
    if slot == "default" {
        set_client_snapshot(Some(snap));
        return;
    }
    if let Ok(mut lock) = client_slots_lock().write() {
        lock.insert(slot.to_owned(), snap);
    }
}


pub static ROLLBACK_EPOCH: AtomicU64 = AtomicU64::new(0);

pub fn bump_rollback_epoch() {
    ROLLBACK_EPOCH.fetch_add(1, Ordering::SeqCst);
}

pub fn get_rollback_epoch() -> u64 {
    ROLLBACK_EPOCH.load(Ordering::SeqCst)
}

pub fn clear_client_snapshots() {
    set_client_snapshot(None);
    if let Ok(mut lock) = client_slots_lock().write() {
        lock.clear();
    }
}

std::thread_local! {
    pub static SNAPSHOT: RefCell<Option<Arc<TimelineSnapshot>>> = const { RefCell::new(None) };
    pub static SLOTS: RefCell<HashMap<String, Arc<TimelineSnapshot>>> = RefCell::new(HashMap::new());
}

pub fn get_snapshot_clone() -> Option<Arc<TimelineSnapshot>> {
    SNAPSHOT.with(|cell| cell.borrow().as_ref().map(Arc::clone))
}

pub fn set_snapshot(snap: Option<Arc<TimelineSnapshot>>) {
    SNAPSHOT.with(|cell| *cell.borrow_mut() = snap);
}

pub fn clear_snapshot() {
    SNAPSHOT.with(|cell| *cell.borrow_mut() = None);
}

pub fn get_slot_snapshot_clone(slot: &str) -> Option<Arc<TimelineSnapshot>> {
    if slot == "default" {
        return get_snapshot_clone();
    }
    SLOTS.with(|cell| cell.borrow().get(slot).map(Arc::clone))
}

pub fn set_slot_snapshot(slot: &str, snap: Arc<TimelineSnapshot>) {
    if slot == "default" {
        set_snapshot(Some(snap));
        return;
    }
    SLOTS.with(|cell| {
        cell.borrow_mut().insert(slot.to_owned(), snap);
    });
}

pub fn list_memory_slots() -> Vec<String> {
    SLOTS.with(|cell| cell.borrow().keys().cloned().collect())
}

pub fn delete_slot_snapshot(slot: &str) -> bool {
    if slot == "default" {
        let existed = SNAPSHOT.with(|cell| cell.borrow().is_some());
        clear_snapshot();
        return existed;
    }
    SLOTS.with(|cell| cell.borrow_mut().remove(slot).is_some())
}

// ============================================================================
// Submodule: benchmark.rs
// ============================================================================

// 1:1 Rust port of RZBenchmark.java for nanosecond phase profiling of Rollback and Capture.


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Prepare = 0,
    PlayersOnline = 1,
    OfflineFiles = 2,
    WorldState = 3,
    ServerGlobals = 4,
    Scoreboard = 5,
    ChunkBlocks = 6,
    ChunkBeClear = 7,
    ChunkBeLoad = 8,
    ChunkTicks = 9,
    ChunkPois = 10,
    ChunkPush = 11,
    RemoveForeign = 12,
    SpawnNbt = 13,
    ApplyMobRam = 14,
    BrainRelink = 15,
    BrainDeepClone = 16,
    RemoveSnapshotEntities = 17,
    DropStaleEvents = 18,
    MenuReopen = 19,
    CapturePlayers = 20,
    CaptureGlobals = 21,
    CaptureRaids = 22,
    CaptureWorldState = 23,
    CaptureChunks = 24,
    CaptureBe = 25,
    CaptureTicksPois = 26,
    CaptureEntities = 27,
}

const PHASE_COUNT: usize = 28;

#[derive(Clone, Debug)]
pub struct BenchmarkData {
    pub open: bool,
    pub title: String,
    pub start_time: Option<Instant>,
    pub start_tick: i64,
    pub deferred_ticks: i64,
    pub nanos: [u64; PHASE_COUNT],
    pub counts: [u32; PHASE_COUNT],
    pub blocks_changed: usize,
    pub chunks: usize,
    pub mobs: usize,
    pub players: usize,
    pub last_report: String,
}

impl Default for BenchmarkData {
    fn default() -> Self {
        Self {
            open: false,
            title: String::new(),
            start_time: None,
            start_tick: 0,
            deferred_ticks: 0,
            nanos: [0; PHASE_COUNT],
            counts: [0; PHASE_COUNT],
            blocks_changed: 0,
            chunks: 0,
            mobs: 0,
            players: 0,
            last_report: String::new(),
        }
    }
}

std::thread_local! {
    static BENCHMARK: RefCell<BenchmarkData> = RefCell::new(BenchmarkData::default());
}

pub fn bench_begin(title: &str, current_tick: i64) {
    BENCHMARK.with(|cell| {
        let mut b = cell.borrow_mut();
        b.open = true;
        b.title = title.to_owned();
        b.start_time = Some(Instant::now());
        b.start_tick = current_tick;
        b.deferred_ticks = 0;
        b.nanos = [0; PHASE_COUNT];
        b.counts = [0; PHASE_COUNT];
        b.blocks_changed = 0;
        b.chunks = 0;
        b.mobs = 0;
        b.players = 0;
    });
}

pub fn bench_accum(phase: Phase, phase_start: Instant) {
    let elapsed = phase_start.elapsed().as_nanos() as u64;
    BENCHMARK.with(|cell| {
        let mut b = cell.borrow_mut();
        if b.open {
            let idx = phase as usize;
            if idx < PHASE_COUNT {
                b.nanos[idx] += elapsed;
                b.counts[idx] += 1;
            }
        }
    });
}

pub fn bench_add_blocks_changed(n: usize) {
    BENCHMARK.with(|cell| {
        let mut b = cell.borrow_mut();
        if b.open {
            b.blocks_changed += n;
        }
    });
}

pub fn bench_add_chunks(n: usize) {
    BENCHMARK.with(|cell| {
        let mut b = cell.borrow_mut();
        if b.open {
            b.chunks += n;
        }
    });
}

pub fn bench_add_mobs(n: usize) {
    BENCHMARK.with(|cell| {
        let mut b = cell.borrow_mut();
        if b.open {
            b.mobs += n;
        }
    });
}

pub fn bench_add_players(n: usize) {
    BENCHMARK.with(|cell| {
        let mut b = cell.borrow_mut();
        if b.open {
            b.players += n;
        }
    });
}

pub fn bench_end_and_log() -> String {
    BENCHMARK.with(|cell| {
        let mut b = cell.borrow_mut();
        if !b.open {
            return b.last_report.clone();
        }
        b.open = false;

        let total_nanos = b.start_time.map_or(0, |t| t.elapsed().as_nanos() as u64);
        let is_rollback = b.title.starts_with("ROLLBACK");

        let top_level = if is_rollback {
            &[
                Phase::Prepare, Phase::PlayersOnline, Phase::OfflineFiles, Phase::WorldState,
                Phase::ServerGlobals, Phase::Scoreboard, Phase::ChunkBlocks, Phase::ChunkBeClear,
                Phase::ChunkBeLoad, Phase::ChunkTicks, Phase::ChunkPois, Phase::ChunkPush,
                Phase::RemoveForeign, Phase::SpawnNbt, Phase::BrainRelink, Phase::ApplyMobRam,
                Phase::RemoveSnapshotEntities, Phase::DropStaleEvents, Phase::MenuReopen,
            ][..]
        } else {
            &[
                Phase::CapturePlayers, Phase::CaptureGlobals, Phase::CaptureRaids,
                Phase::CaptureWorldState, Phase::CaptureChunks, Phase::CaptureBe,
                Phase::CaptureTicksPois, Phase::CaptureEntities,
            ][..]
        };

        let work_nanos: u64 = top_level.iter().map(|&p| b.nanos[p as usize]).sum();

        let mut out = String::with_capacity(1024);
        out.push_str(&format!(
            "[RZero][Benchmark] === {}: {:.2} ms (wall) | work {:.2} ms ===",
            b.title,
            total_nanos as f64 / 1_000_000.0,
            work_nanos as f64 / 1_000_000.0
        ));

        let nanos = b.nanos;
        let get_ms = |p: Phase| nanos[p as usize] as f64 / 1_000_000.0;
        let sum_ms = |phases: &[Phase]| -> f64 {
            phases.iter().map(|&p| nanos[p as usize] as f64 / 1_000_000.0).sum()
        };

        if is_rollback {
            let prep = get_ms(Phase::Prepare);
            if prep > 0.0 {
                out.push_str(&format!("\n|-- Prepare: {:.2} ms", prep));
            }

            let players_work = sum_ms(&[Phase::PlayersOnline, Phase::OfflineFiles]);
            if players_work > 0.0 || b.players > 0 {
                out.push_str(&format!("\n|-- Players ({}): {:.2} ms", b.players, players_work));
                let off = get_ms(Phase::OfflineFiles);
                if off > 0.0 {
                    out.push_str(&format!("\n|   |-- Offline Files (.dat): {:.2} ms", off));
                }
            }

            let world = sum_ms(&[Phase::WorldState, Phase::ServerGlobals, Phase::Scoreboard]);
            if world > 0.0 {
                out.push_str(&format!("\n|-- World & Globals: {:.2} ms", world));
                let ws = get_ms(Phase::WorldState);
                if ws > 0.0 { out.push_str(&format!("\n|   |-- WorldState: {:.2} ms", ws)); }
                let sg = get_ms(Phase::ServerGlobals);
                if sg > 0.0 { out.push_str(&format!("\n|   |-- ServerGlobals: {:.2} ms", sg)); }
                let sc = get_ms(Phase::Scoreboard);
                if sc > 0.0 { out.push_str(&format!("\n|   |-- Scoreboard: {:.2} ms", sc)); }
            }

            let chunk_sum = sum_ms(&[
                Phase::ChunkBlocks, Phase::ChunkBeClear, Phase::ChunkBeLoad,
                Phase::ChunkTicks, Phase::ChunkPois, Phase::ChunkPush,
            ]);
            if chunk_sum > 0.0 || b.chunks > 0 {
                out.push_str(&format!(
                    "\n|-- Chunks ({}, blocks changed {}): {:.2} ms",
                    b.chunks, b.blocks_changed, chunk_sum
                ));
                let cb = get_ms(Phase::ChunkBlocks);
                if cb > 0.0 { out.push_str(&format!("\n|   |-- Blocks (applyDiffTo): {:.2} ms", cb)); }
                let bec = get_ms(Phase::ChunkBeClear);
                if bec > 0.0 { out.push_str(&format!("\n|   |-- Block Entities (clear): {:.2} ms", bec)); }
                let bel = get_ms(Phase::ChunkBeLoad);
                if bel > 0.0 { out.push_str(&format!("\n|   |-- Block Entities (load): {:.2} ms", bel)); }
                let ct = get_ms(Phase::ChunkTicks);
                if ct > 0.0 { out.push_str(&format!("\n|   |-- Scheduled Ticks: {:.2} ms", ct)); }
                let poi = get_ms(Phase::ChunkPois);
                if poi > 0.0 { out.push_str(&format!("\n|   |-- POIs: {:.2} ms", poi)); }
                let cp = get_ms(Phase::ChunkPush);
                if cp > 0.0 { out.push_str(&format!("\n|   |-- PushUpdates: {:.2} ms", cp)); }
            }

            let entity_sum = sum_ms(&[
                Phase::RemoveForeign, Phase::SpawnNbt, Phase::BrainRelink, Phase::ApplyMobRam,
            ]);
            if entity_sum > 0.0 || b.mobs > 0 {
                out.push_str(&format!("\n|-- Entities ({}): {:.2} ms", b.mobs, entity_sum));
                let rf = get_ms(Phase::RemoveForeign);
                if rf > 0.0 { out.push_str(&format!("\n|   |-- Remove Foreign: {:.2} ms", rf)); }
                let sn = get_ms(Phase::SpawnNbt);
                if sn > 0.0 { out.push_str(&format!("\n|   |-- Spawn & NBT: {:.2} ms", sn)); }
                let br = get_ms(Phase::BrainRelink);
                if br > 0.0 { out.push_str(&format!("\n|   |-- Brain Relink: {:.2} ms", br)); }
                let mr = get_ms(Phase::ApplyMobRam);
                if mr > 0.0 { out.push_str(&format!("\n|   |-- ApplyMobRam: {:.2} ms", mr)); }
            }
        } else {
            out.push_str(&format!("\n|-- Globals: {:.2} ms", get_ms(Phase::CaptureGlobals)));
            out.push_str(&format!("\n|-- Raids & Dragon: {:.2} ms", get_ms(Phase::CaptureRaids)));
            out.push_str(&format!("\n|-- World State: {:.2} ms", get_ms(Phase::CaptureWorldState)));
            let chunks_ms = get_ms(Phase::CaptureChunks);
            out.push_str(&format!("\n|-- Chunks ({}): {:.2} ms", b.chunks, chunks_ms));
            let cbe = get_ms(Phase::CaptureBe);
            if cbe > 0.0 { out.push_str(&format!("\n|   |-- Block Entities: {:.2} ms", cbe)); }
            let ce = get_ms(Phase::CaptureEntities);
            if ce > 0.0 { out.push_str(&format!("\n|   |-- Entities: {:.2} ms", ce)); }
        }

        b.last_report = out.clone();
        out
    })
}

pub fn bench_get_last_report() -> String {
    BENCHMARK.with(|cell| cell.borrow().last_report.clone())
}

pub fn append_to_rzero_log(world_root: &std::path::Path, text: &str) {

    // 1. Instance logs/rzero.log (alongside latest.log)
    if let Some(instance_dir) = world_root.parent().and_then(|p| p.parent()) {
        let logs_dir = instance_dir.join("logs");
        if logs_dir.is_dir() {
            let log_file = logs_dir.join("rzero.log");
            if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(log_file) {
                let _ = writeln!(f, "{}", text);
            }
        }
    }

    // 2. World-specific saves/<World>/rzero/benchmark.log
    let rzero_dir = world_root.join("rzero");
    let _ = std::fs::create_dir_all(&rzero_dir);
    let world_log = rzero_dir.join("benchmark.log");
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(world_log) {
        let _ = writeln!(f, "{}", text);
    }
}

// ============================================================================
// Submodule: adaptive.rs
// ============================================================================


pub fn is_guest_safe(game: &Game<'_>, g: &horizon_client::lan::guest::Guest) -> bool {
    if !g.placed || g.dead || g.health <= 1.0 {
        return false;
    }
    if g.effects.list.iter().any(|e| e.id == "levitation") {
        return false;
    }
    let (fire_ticks, fall_distance) = match g.motion.as_deref() {
        Some(m) => (m.fire_ticks, m.fall_distance as f32),
        None => (0, 0.0),
    };
    if fire_ticks > 0 {
        return false;
    }
    let bx = g.position[0].floor() as i32;
    let by = g.position[1].floor() as i32;
    let bz = g.position[2].floor() as i32;
    let facts = game.res.facts();
    let here_fluid = facts.get(game.world.block(bx, by, bz)).fluid;
    if here_fluid == Fluid::Lava {
        return false;
    }
    let pos_vec = Vec3::new(g.position[0], g.position[1], g.position[2]);
    let imminent_tnt = game
        .tnts
        .iter()
        .any(|t| t.fuse <= 60 && t.position.distance_squared(pos_vec) <= 64.0);
    let imminent_creeper = game.mobs.iter().any(|m| {
        m.alive()
            && m.species() == Species::Creeper
            && (m.extra.counter > 0 || m.extra.flag)
            && m.position.distance_squared(pos_vec) <= 49.0
    });
    if imminent_tnt || imminent_creeper {
        return false;
    }
    if !g.on_ground && !g.flying && !g.creative() && !g.spectator() {
        let min_y = game.world.shape.min_y;
        let mut probe_y = by;
        let mut found_ground = false;
        let mut expected_damage = 0.0f32;
        for _ in 0..256 {
            probe_y -= 1;
            if probe_y < min_y {
                break;
            }
            let st = game.world.block(bx, probe_y, bz);
            let f = facts.get(st);
            if facts.is_air(st) && f.fluid == Fluid::None {
                continue;
            }
            if f.fluid == Fluid::Lava {
                return false;
            }
            found_ground = true;
            if f.fluid == Fluid::None {
                let total_fall = fall_distance + (g.position[1] as f32 - probe_y as f32);
                expected_damage = total_fall - 3.0;
            }
            break;
        }
        if !found_ground || expected_damage >= g.health {
            return false;
        }
    }
    true
}

pub fn pack_pos(x: i32, y: i32, z: i32) -> i64 {
    ((x as i64 & 0x3FF_FFFF) << 38) | ((z as i64 & 0x3FF_FFFF) << 12) | (y as i64 & 0xFFF)
}

pub fn unpack_pos(packed: i64) -> (i32, i32, i32) {
    let x = (packed >> 38) as i32;
    let z = ((packed << 26) >> 38) as i32;
    let y = ((packed << 52) >> 52) as i32;
    (x, y, z)
}

pub fn flood_escape_space(
    game: &Game<'_>,
    start: [i32; 3],
    blocked: Option<&HashSet<i64>>,
    allow_drops: bool,
) -> (usize, usize) {
    const MAX_VISITED: usize = 200;
    const MAX_DIST: i32 = 6;
    const DX: [i32; 4] = [0, 0, -1, 1];
    const DZ: [i32; 4] = [-1, 1, 0, 0];

    let min_dy = if allow_drops { -3 } else { -2 };
    let facts = game.res.facts();
    let mut visited = HashSet::with_capacity(MAX_VISITED);
    let mut queue = VecDeque::with_capacity(MAX_VISITED);
    let start_long = pack_pos(start[0], start[1], start[2]);
    visited.insert(start_long);
    queue.push_back(start_long);
    let mut cliffs = 0usize;

    while let Some(curr) = queue.pop_front() {
        if visited.len() >= MAX_VISITED {
            break;
        }
        let (cx, cy, cz) = unpack_pos(curr);
        for dir in 0..4 {
            let nx = cx + DX[dir];
            let nz = cz + DZ[dir];
            let mut target = None;
            let mut is_cliff = false;
            for dy in (min_dy..=1).rev() {
                let ny = cy + dy;
                let st_f = facts.get(game.world.block(nx, ny, nz));
                let fl_f = facts.get(game.world.block(nx, ny - 1, nz));
                let state_blocks = st_f.full_collision || st_f.legacy_solid;
                let floor_blocks = fl_f.full_collision || fl_f.legacy_solid;
                let floor_fluid = fl_f.fluid != Fluid::None;
                let floor_lava = fl_f.fluid == Fluid::Lava;
                let standable = if allow_drops {
                    floor_blocks || floor_fluid
                } else {
                    floor_blocks
                };
                if !state_blocks && standable {
                    target = Some((nx, ny, nz));
                    if allow_drops && (dy <= min_dy || floor_lava) {
                        is_cliff = true;
                    }
                    break;
                }
            }
            if is_cliff {
                cliffs += 1;
            }
            let Some((tx, ty, tz)) = target else {
                continue;
            };
            let t_long = pack_pos(tx, ty, tz);
            if visited.contains(&t_long) {
                continue;
            }
            if blocked.is_some_and(|b| b.contains(&t_long)) {
                continue;
            }
            let dist = (tx - start[0]).abs() + (ty - start[1]).abs() + (tz - start[2]).abs();
            if dist <= MAX_DIST {
                visited.insert(t_long);
                queue.push_back(t_long);
            }
        }
    }
    (visited.len(), cliffs)
}

pub fn calculate_guest_threat(game: &Game<'_>, g: &horizon_client::lan::guest::Guest) -> f64 {
    if !is_guest_safe(game, g) {
        return -1.0;
    }
    let hp_pct = (g.health / 20.0).clamp(0.0, 1.0);
    let mut env_threat = f64::from(1.0 - hp_pct) * 60.0;
    if g.food < 6 {
        env_threat += 15.0;
    }
    let air = g.motion.as_deref().map_or(300, |m| m.air);
    if air < 150 {
        let air_pct = air as f32 / 300.0;
        env_threat += f64::from(0.5 - air_pct) * 80.0;
    }
    if g.effects
        .list
        .iter()
        .any(|e| matches!(e.id, "poison" | "wither"))
    {
        env_threat += 40.0;
    }

    let p_vec = Vec3::new(g.position[0], g.position[1], g.position[2]);
    let mut mobs_tracking = 0;
    let mut mobs_attacking = 0;
    let mut mob_cells = HashSet::new();
    for m in &game.mobs {
        if !m.alive() || !m.hostile() {
            continue;
        }
        let d2 = m.position.distance_squared(p_vec);
        if d2 > 256.0 {
            continue;
        }
        if m.ai.target.is_some() || d2 < 16.0 {
            mobs_tracking += 1;
            if d2 < 9.0 || m.ai.aggressive {
                mobs_attacking += 1;
            }
            let mx = m.position.x.floor() as i32;
            let my = m.position.y.floor() as i32;
            let mz = m.position.z.floor() as i32;
            mob_cells.insert(pack_pos(mx, my, mz));
        }
    }

    let mob_threat = (f64::from(mobs_tracking) * 5.0 + f64::from(mobs_attacking) * 25.0).min(60.0);
    if mob_threat == 0.0 {
        env_threat *= 0.5;
    }
    let mut total_threat = (env_threat + mob_threat).min(100.0);
    if mobs_tracking > 0 {
        let start = [
            g.position[0].floor() as i32,
            g.position[1].floor() as i32,
            g.position[2].floor() as i32,
        ];
        let (free_space, cliffs) = flood_escape_space(game, start, None, true);
        let (safe_space, _) = flood_escape_space(game, start, Some(&mob_cells), false);
        if free_space > 0 {
            let cornered = 1.0 - (safe_space as f64 / free_space as f64);
            total_threat = (total_threat + cornered * 15.0 + (cliffs.min(4) as f64) * 2.5).min(100.0);
        }
    }
    total_threat
}

pub fn tick_autosave_on_server(game: &mut Game<'_>) {
    let now = game.world.game_time;

    let boss_just_died = game.dragon.as_ref().is_some_and(|d| d.death_time == 1)
        || game.mobs.iter().any(|m| {
            m.species() == Species::Wither && !m.alive() && m.death_time == 1
        });
    if boss_just_died {
        let should_boss_save = with_state_mut(|st| {
            if st.save_on_boss_defeat && now - st.last_boss_death_tick > 100 {
                st.last_boss_death_tick = now;
                true
            } else {
                false
            }
        });
        if should_boss_save {
            let safe = game
                .lan
                .as_ref()
                .is_some_and(|h| h.hub.guests.iter().any(|g| is_guest_safe(game, g)));
            if safe {
                create_checkpoint_on_server(game, None);
                if with_state(|st| st.logs) {
                    game.system_message(Text::literal(
                        "[RZero] Boss defeat checkpoint created.",
                    ));
                }
                return;
            }
        }
    }

    let (auto_save, adaptive_mode) = with_state(|st| (st.auto_save, st.adaptive_mode));
    if !auto_save {
        return;
    }

    if !adaptive_mode {
        let due = with_state(|st| now - st.last_save_tick >= st.current_interval_ticks.max(100));
        if !due {
            return;
        }
        let any_safe = game
            .lan
            .as_ref()
            .is_some_and(|h| h.hub.guests.iter().any(|g| is_guest_safe(game, g)));
        if any_safe {
            create_checkpoint_on_server(game, None);
            with_state_mut(|st| {
                st.current_interval_ticks = if st.use_random_interval {
                    let min = st.random_min_sec.max(5);
                    let max = st.random_max_sec.max(min + 1);
                    let mut rng = LegacyRandom::new(now);
                    let extra = rng.next_int_bound((max - min) + 1).unwrap_or(0);
                    i64::from(min + extra) * 20
                } else {
                    i64::from(st.fixed_interval_sec.max(5)) * 20
                };
            });
            if with_state(|st| st.logs) {
                game.system_message(Text::literal("[RZero] Auto-saved checkpoint."));
            }
        }
        return;
    }

    let Some(host) = &game.lan else {
        return;
    };
    let online: Vec<u128> = host
        .hub
        .guests
        .iter()
        .filter(|g| g.placed)
        .map(|g| g.uuid)
        .collect();
    let anchors = with_state(|st| resolve_anchors(&st.anchor, &online, now, None));
    let mut max_threat = 0.0f64;
    let mut any_anchor_here = false;
    for g in host
        .hub
        .guests
        .iter()
        .filter(|g| g.placed && anchors.contains(&g.uuid) && g.dimension == game.dim.id())
    {
        any_anchor_here = true;
        let threat = calculate_guest_threat(game, g);
        if threat < 0.0 {
            with_state_mut(|st| st.relax_ticks = 0);
            return;
        }
        if threat > max_threat {
            max_threat = threat;
        }
    }
    if !any_anchor_here {
        return;
    }

    let mut rng = LegacyRandom::new(now ^ 0x5DEECE66D);
    let rand_f = rng.next_float();
    let should_save = with_state_mut(|st| {
        let since_last = now - st.last_save_tick;
        if max_threat > st.current_intensity {
            st.current_intensity += (max_threat - st.current_intensity) * 0.2;
            st.relax_ticks = 0;
        } else {
            st.current_intensity -= (st.current_intensity - max_threat) * 0.02;
        }
        if st.current_intensity > st.peak_intensity {
            st.peak_intensity = st.current_intensity;
        }

        let relax_req = (st.relax_time_sec * 20).max(20);
        let min_gap = (i64::from(st.min_save_gap_sec) * 20).max(100);
        let chance = st.post_combat_save_pct as f32 / 100.0;

        let mut trigger = false;
        if st.peak_intensity > 35.0 {
            if st.current_intensity < st.peak_intensity * 0.5 && max_threat <= 20.0 {
                st.relax_ticks += 1;
                if st.relax_ticks > relax_req {
                    if rand_f < chance {
                        trigger = true;
                    } else {
                        st.peak_intensity = 0.0;
                        st.current_intensity = 0.0;
                        st.relax_ticks = 0;
                    }
                }
            } else {
                st.relax_ticks = 0;
            }
        } else {
            let tranquil_ticks = i64::from(st.adaptive_interval_sec.max(60)) * 20;
            if since_last >= tranquil_ticks && max_threat <= 10.0 {
                trigger = true;
            }
        }
        trigger && since_last > min_gap
    });

    if should_save {
        create_checkpoint_on_server(game, None);
        if with_state(|st| st.logs) {
            game.system_message(Text::literal("[RZero] Adaptive checkpoint created."));
        }
    }
}

// ============================================================================
// Submodule: capture.rs
// ============================================================================


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
                        velocity: [0.0, 0.0, 0.0],
                        food_exhaustion: 0.0,
                        food_tick_timer: 0,
                        ticks_frozen: 0,
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

    let _ = game.save_game_with(true);
    save_slot_to_disk(&game.world_root, slot, &timeline);
    set_slot_snapshot(slot, timeline);

    let client_snap = ClientBlockSnapshot {
        dim: game.dim,
        chunks: game.world.chunks.clone(),
    };
    set_client_slot_snapshot(slot, client_snap);

    if let Ok(mut tr) = loop_tracker().write() {
        tr.on_checkpoint(game.world.game_time);
    }

    let bench_report = bench_end_and_log();
    log::info!("{}", bench_report);
    append_to_rzero_log(&game.world_root, &bench_report);
}

// ============================================================================
// Submodule: restore.rs
// ============================================================================


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
        bench_add_chunks(snap.chunks.len());
        bench_add_blocks_changed(changed_coords.len());

        if policy.blocks {
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
                                        if live_sec.get(idx) != want {
                                            diffs.push((x_base + sx, y_base + sy, z_base + sz, want));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            for (x, y, z, want) in diffs {
                let _ = game.world.set(x, y, z, BlockStateId(want));
            }
            return;
        }
    };

    if game.dim != client_snap.dim {
        return;
    }

    let old_chunks = game.world.chunks.clone();
    let mut changed_coords = Vec::new();

    for (&(cx, cz), snap_chunk) in client_snap.chunks.iter() {
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

    for (wx, wy, wz, want) in changed_coords {
        let _ = game.world.set(wx, wy, wz, BlockStateId(want));
    }
}

pub fn rollback_checkpoint_on_server(game: &mut Game<'_>) -> bool {
    let slot = with_state(|st| st.current_slot.clone());
    rollback_slot_checkpoint_on_server(game, &slot)
}

pub fn rollback_slot_checkpoint_on_server(game: &mut Game<'_>, slot: &str) -> bool {
    let t_start = std::time::Instant::now();
    let t_globals = std::time::Instant::now();
    game.world.loaded_entities.clear();
    let snap = match get_slot_snapshot_clone(slot) {
        Some(s) => s,
        None => match load_slot_from_disk(&game.world_root, slot) {
            Some(s) => {
                let arc = Arc::new(s);
                set_slot_snapshot(slot, Arc::clone(&arc));
                arc
            }
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

    let is_hot_rollback = slot == "default"
        && game.dim == target_dim
        && snap.levels.contains_key(&game.dim);
    let disk_files_changed = restore_disk_files_for_slot(&game.world_root, slot);
    if !is_hot_rollback || disk_files_changed > 0 {
        let new_save = horizon_client::save::SaveDir::new(
            game.save.root.clone(),
            game.save.shape,
        );
        if let Some(tables) = game.save.tables.get() {
            let _ = new_save.tables.set(Arc::clone(tables));
        }
        game.save = Arc::new(new_save);
    }

    bench_begin(&format!("ROLLBACK RESTORE [{}]", slot), game.world.game_time);
    bench_add_players(snap.guests.len());
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

    let bench_report = bench_end_and_log();
    log::info!("{}", bench_report);
    append_to_rzero_log(&game.world_root, &bench_report);

    let client_snap = ClientBlockSnapshot {
        dim: game.dim,
        chunks: game.world.chunks.clone(),
    };
    set_client_slot_snapshot(slot, client_snap);
    bump_rollback_epoch();

    if let Ok(mut tr) = loop_tracker().write() {
        tr.on_rollback(snap.game_time);
    }

    true
}

// ============================================================================
// Submodule: persistence.rs
// ============================================================================


pub fn slot_dir(world_root: &Path, slot: &str) -> PathBuf {
    let clean = slot.trim();
    if clean.is_empty() || clean == "default" {
        world_root.join("rzero").join("checkpoint")
    } else {
        world_root.join("rzero").join("slots").join(clean)
    }
}

pub fn checkpoint_dir(world_root: &Path) -> PathBuf {
    slot_dir(world_root, "default")
}

pub fn has_disk_checkpoint(world_root: &Path) -> bool {
    has_disk_slot(world_root, "default")
}

pub fn has_disk_slot(world_root: &Path, slot: &str) -> bool {
    slot_dir(world_root, slot).join("meta.json").is_file()
}

pub fn copy_dir_recursive(src: &Path, dst: &Path) -> usize {
    if !src.is_dir() {
        return 0;
    }
    let mut copied = 0;
    let _ = fs::create_dir_all(dst);
    if let Ok(entries) = fs::read_dir(src) {
        for entry in entries.flatten() {
            let p = entry.path();
            let target = dst.join(entry.file_name());
            if p.is_dir() {
                copied += copy_dir_recursive(&p, &target);
            } else if p.is_file() {
                // Fast-path: skip copying if size and exact mtime match
                if let (Ok(m_src), Ok(m_dst)) = (p.metadata(), target.metadata()) {
                    if m_src.len() == m_dst.len() {
                        if let (Ok(t_src), Ok(t_dst)) = (m_src.modified(), m_dst.modified()) {
                            if t_src == t_dst {
                                continue;
                            }
                        }
                    }
                }
                if fs::copy(&p, &target).is_ok() {
                    copied += 1;
                    if let (Ok(m), Ok(f)) = (p.metadata(), fs::File::options().write(true).open(&target)) {
                        if let Ok(mtime) = m.modified() {
                            let _ = f.set_times(std::fs::FileTimes::new().set_modified(mtime));
                        }
                    }
                }
            }
        }
    }
    copied
}

pub fn collect_files_recursive(
    dir: &Path,
    map: &mut HashMap<PathBuf, Vec<u8>>,
    target_base: &Path,
    rel_prefix: &Path,
) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            let fname = entry.file_name();
            let next_rel = rel_prefix.join(fname);
            if p.is_dir() {
                collect_files_recursive(&p, map, target_base, &next_rel);
            } else if p.is_file() {
                if let Ok(bytes) = fs::read(&p) {
                    map.insert(target_base.join(&next_rel), bytes);
                }
            }
        }
    }
}

pub fn save_slot_to_disk(world_root: &Path, slot: &str, snap: &TimelineSnapshot) -> bool {
    let cp_dir = slot_dir(world_root, slot);
    if fs::create_dir_all(&cp_dir).is_err() {
        return false;
    }

    // 1. Копируем level.dat
    let level_dat = world_root.join("level.dat");
    if level_dat.is_file() {
        let _ = fs::copy(&level_dat, cp_dir.join("level.dat"));
    }

    // 2. Копируем папку players
    let players_dir = world_root.join("players");
    let cp_players = cp_dir.join("players");
    copy_dir_recursive(&players_dir, &cp_players);

    // 3. Копируем папку data
    let data_dir = world_root.join("data");
    let cp_data = cp_dir.join("data");
    copy_dir_recursive(&data_dir, &cp_data);

    // 3.5. Копируем файлы измерений и регионов
    let dims_dir = world_root.join("dimensions");
    let cp_dims = cp_dir.join("dimensions");
    copy_dir_recursive(&dims_dir, &cp_dims);

    // 4. Сохраняем метаданные гостей и чекпоинта
    let mut guests_json = Vec::new();
    for g in snap.guests.values() {
        guests_json.push(serde_json::json!({
            "uuid": format!("{:032x}", g.uuid),
            "dimension": g.dimension,
            "position": g.position,
            "velocity": [g.velocity.x, g.velocity.y, g.velocity.z],
            "yaw": g.yaw,
            "pitch": g.pitch,
            "head_yaw": g.head_yaw,
            "on_ground": g.on_ground,
            "game_mode": g.game_mode,
            "flying": g.flying,
            "sprinting": g.sprinting,
            "sneaking": g.sneaking,
            "fall_flying": g.fall_flying,
            "health": g.health,
            "absorption": g.absorption,
            "food": g.food,
            "saturation": g.saturation,
            "exhaustion": g.exhaustion,
            "fire_ticks": g.fire_ticks,
            "air": g.air,
            "ticks_frozen": g.ticks_frozen,
            "fall_distance": g.fall_distance,
            "xp_level": g.xp_level,
            "xp_progress": g.xp_progress,
            "xp_total": g.xp_total,
            "xp_seed": g.xp_seed,
            "selected": g.selected,
            "seen_credits": g.seen_credits,
        }));
    }

    let regions_list: Vec<String> = snap
        .region_files
        .iter()
        .filter_map(|p| {
            p.strip_prefix(world_root)
                .ok()
                .map(|rel| rel.to_string_lossy().to_string())
        })
        .collect();

    let anchors_list: Vec<String> = snap
        .anchor_ids
        .iter()
        .map(|u| format!("{:032x}", u))
        .collect();

    let meta = serde_json::json!({
        "slot": slot,
        "saved_at_unix": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
        "game_time": snap.game_time,
        "day_time": snap.day_time,
        "difficulty": snap.difficulty,
        "difficulty_locked": snap.difficulty_locked,
        "clock_rate": snap.clock_rate,
        "clock_frozen": snap.clock_frozen,
        "time_rate": snap.time_rate,
        "world_spawn": snap.world_spawn,
        "world_spawn_angle": snap.world_spawn_angle,
        "next_entity": snap.next_entity,
        "policy": snap.policy.to_json(),
        "anchor_ids": anchors_list,
        "region_files": regions_list,
        "guests": guests_json,
    });

    if let Ok(json_str) = serde_json::to_string_pretty(&meta) {
        let _ = fs::write(cp_dir.join("meta.json"), json_str);
    }

    true
}

pub fn save_checkpoint_to_disk(world_root: &Path, snap: &TimelineSnapshot) -> bool {
    save_slot_to_disk(world_root, "default", snap)
}

pub fn restore_disk_files_for_slot(world_root: &Path, slot: &str) -> usize {
    let cp_dir = slot_dir(world_root, slot);
    let mut changed = 0;
    let cp_players = cp_dir.join("players");
    let target_players = world_root.join("players");
    if cp_players.is_dir() {
        changed += copy_dir_recursive(&cp_players, &target_players);
    }
    let cp_data = cp_dir.join("data");
    let target_data = world_root.join("data");
    if cp_data.is_dir() {
        changed += copy_dir_recursive(&cp_data, &target_data);
    }
    let cp_dims = cp_dir.join("dimensions");
    let target_dims = world_root.join("dimensions");
    if cp_dims.is_dir() {
        changed += copy_dir_recursive(&cp_dims, &target_dims);
    }
    changed
}

pub fn restore_disk_files_on_rollback(world_root: &Path) {
    restore_disk_files_for_slot(world_root, "default");
}

pub fn load_slot_from_disk(world_root: &Path, slot: &str) -> Option<TimelineSnapshot> {
    let cp_dir = slot_dir(world_root, slot);
    let meta_file = cp_dir.join("meta.json");
    let meta_str = fs::read_to_string(meta_file).ok()?;
    let meta: serde_json::Value = serde_json::from_str(&meta_str).ok()?;

    let game_time = meta["game_time"].as_i64()?;
    let day_time = meta["day_time"].as_i64().unwrap_or(game_time);
    let difficulty = meta["difficulty"].as_u64().unwrap_or(2) as u8;
    let difficulty_locked = meta["difficulty_locked"].as_bool().unwrap_or(false);
    let next_entity = meta["next_entity"].as_u64().unwrap_or(100) as u32;

    let world_spawn = meta["world_spawn"].as_array().and_then(|arr| {
        if arr.len() == 3 {
            Some([
                arr[0].as_i64()? as i32,
                arr[1].as_i64()? as i32,
                arr[2].as_i64()? as i32,
            ])
        } else {
            None
        }
    });

    let world_spawn_angle = meta["world_spawn_angle"].as_array().map_or([0.0, 0.0], |arr| {
        if arr.len() == 2 {
            [
                arr[0].as_f64().unwrap_or(0.0) as f32,
                arr[1].as_f64().unwrap_or(0.0) as f32,
            ]
        } else {
            [0.0, 0.0]
        }
    });

    let mut anchor_ids = HashSet::new();
    if let Some(arr) = meta["anchor_ids"].as_array() {
        for v in arr {
            if let Some(s) = v.as_str() {
                if let Ok(u) = u128::from_str_radix(s, 16) {
                    anchor_ids.insert(u);
                }
            }
        }
    }

    let mut region_files = HashSet::new();
    if let Some(arr) = meta["region_files"].as_array() {
        for v in arr {
            if let Some(s) = v.as_str() {
                region_files.insert(world_root.join(s));
            }
        }
    }

    let mut offline_player_data = HashMap::new();
    let cp_players = cp_dir.join("players");
    collect_files_recursive(&cp_players, &mut offline_player_data, &world_root.join("players"), Path::new(""));

    let mut guests = HashMap::new();
    if let Some(g_arr) = meta["guests"].as_array() {
        for g_val in g_arr {
            let uuid_str = g_val["uuid"].as_str().unwrap_or("");
            let uuid = u128::from_str_radix(uuid_str, 16).unwrap_or(0);
            if uuid == 0 {
                continue;
            }

            let pos = g_val["position"].as_array().map_or([0.0, 65.0, 0.0], |arr| {
                [
                    arr.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0),
                    arr.get(1).and_then(|v| v.as_f64()).unwrap_or(65.0),
                    arr.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0),
                ]
            });

            let vel = g_val["velocity"].as_array().map_or(Vec3::ZERO, |arr| {
                Vec3::new(
                    arr.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0),
                    arr.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0),
                    arr.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0),
                )
            });

            let player_dat_hex = cp_players.join("data").join(format!("{:032x}.dat", uuid));
            let player_dat_std = cp_players.join("data").join(format!("{}.dat", format_uuid_std(uuid)));
            let player_dat = if player_dat_hex.is_file() {
                player_dat_hex
            } else {
                player_dat_std
            };
            let player_tag = read_dat(&player_dat);

            let mut guest_snap = GuestSnapshot {
                uuid,
                dimension: g_val["dimension"]
                    .as_str()
                    .unwrap_or("minecraft:overworld")
                    .to_owned(),
                position: pos,
                velocity: vel,
                yaw: g_val["yaw"].as_f64().unwrap_or(0.0) as f32,
                pitch: g_val["pitch"].as_f64().unwrap_or(0.0) as f32,
                head_yaw: g_val["head_yaw"].as_f64().unwrap_or(0.0) as f32,
                on_ground: g_val["on_ground"].as_bool().unwrap_or(true),
                game_mode: g_val["game_mode"].as_i64().unwrap_or(0) as i32,
                flying: g_val["flying"].as_bool().unwrap_or(false),
                sprinting: g_val["sprinting"].as_bool().unwrap_or(false),
                sneaking: g_val["sneaking"].as_bool().unwrap_or(false),
                fall_flying: g_val["fall_flying"].as_bool().unwrap_or(false),
                sleeping: false,
                bed: None,
                sleep_timer: 0,
                vehicle: None,
                vehicle_good: None,
                spawn: None,
                respawn_elsewhere: None,
                last_death: None,
                health: g_val["health"].as_f64().unwrap_or(20.0) as f32,
                absorption: g_val["absorption"].as_f64().unwrap_or(0.0) as f32,
                food: g_val["food"].as_i64().unwrap_or(20) as i32,
                saturation: g_val["saturation"].as_f64().unwrap_or(5.0) as f32,
                exhaustion: g_val["exhaustion"].as_f64().unwrap_or(0.0) as f32,
                fire_ticks: g_val["fire_ticks"].as_i64().unwrap_or(-20) as i32,
                air: g_val["air"].as_i64().unwrap_or(300) as i32,
                ticks_frozen: g_val["ticks_frozen"].as_i64().unwrap_or(0) as i32,
                fall_distance: g_val["fall_distance"].as_f64().unwrap_or(0.0),
                xp_level: g_val["xp_level"].as_i64().unwrap_or(0) as i32,
                xp_progress: g_val["xp_progress"].as_f64().unwrap_or(0.0) as f32,
                xp_total: g_val["xp_total"].as_i64().unwrap_or(0) as i32,
                xp_seed: g_val["xp_seed"].as_i64().unwrap_or(0) as i32,
                inventory: vec![None; 36],
                armor: [None, None, None, None],
                offhand: None,
                carried: None,
                ender_items: vec![None; 27],
                selected: g_val["selected"].as_u64().unwrap_or(0) as usize,
                craft_grid: [None, None, None, None],
                craft_result: None,
                effects: Vec::new(),
                shield_cooldown: 0,
                cooldowns: Cooldowns::default(),
                recipe_book: RecipeBookSettings::default(),
                advancements: horizon_client::advancements::Progress::default(),
                stats: BTreeMap::new(),
                seen_credits: g_val["seen_credits"].as_bool().unwrap_or(false),
                saved: player_tag.clone(),
                hook: None,
            };

            if let Some(tag) = &player_tag {
                if let Some(Tag::List(items)) = tag.get("Inventory") {
                    for it in items {
                        let slot = it
                            .get("Slot")
                            .and_then(|v| match v {
                                Tag::Byte(b) => Some(*b as i16),
                                Tag::Short(s) => Some(*s),
                                Tag::Int(i) => Some(*i as i16),
                                _ => None,
                            })
                            .unwrap_or(0);
                        if let Some(stack) = item_from_nbt(it) {
                            match slot {
                                0..=35 => guest_snap.inventory[slot as usize] = Some(stack),
                                100 => guest_snap.armor[3] = Some(stack),
                                101 => guest_snap.armor[2] = Some(stack),
                                102 => guest_snap.armor[1] = Some(stack),
                                103 => guest_snap.armor[0] = Some(stack),
                                -106 | 150 => guest_snap.offhand = Some(stack),
                                _ => {}
                            }
                        }
                    }
                }
                if let Some(Tag::List(ender)) = tag.get("EnderItems") {
                    for it in ender {
                        let slot = it
                            .get("Slot")
                            .and_then(|v| match v {
                                Tag::Byte(b) => Some(*b as usize),
                                Tag::Short(s) => Some(*s as usize),
                                Tag::Int(i) => Some(*i as usize),
                                _ => None,
                            })
                            .unwrap_or(0);
                        if slot < guest_snap.ender_items.len() {
                            if let Some(stack) = item_from_nbt(it) {
                                guest_snap.ender_items[slot] = Some(stack);
                            }
                        }
                    }
                }
            }

            guests.insert(uuid, guest_snap);
        }
    }

    Some(TimelineSnapshot {
        anchor_ids,
        policy: RollbackPolicy::default(),
        game_time,
        day_time,
        weather: Weather::default(),
        health_rules: HealthRules::default(),
        rules: horizon_client::gamerules::GameRules::default(),
        border: WorldBorder::default(),
        bossbars: Tag::Compound(Default::default()),
        sculk_tracker: horizon_client::sculk::WardenSpawnTracker::default(),
        sculk_darkness: (0.0, 0.0),
        dragon_fight_dat: None,
        trader_dat: None,
        rand_sequences: horizon_client::random_sequences::RandomSequences::default(),
        region_files,
        difficulty,
        difficulty_locked,
        clock_rate: meta["clock_rate"].as_f64().map(|v| v as f32).unwrap_or(20.0),
        clock_frozen: meta["clock_frozen"].as_bool().unwrap_or(false),
        time_rate: meta["time_rate"].as_f64().map(|v| v as f32).unwrap_or(1.0),
        world_spawn,
        world_spawn_angle,
        next_entity,
        scoreboard: horizon_client::scoreboard::Scoreboard::default(),
        guests,
        offline_player_data,
        levels: HashMap::new(),
        mob_players: horizon_client::fast_hash::FxHashMap::default(),
    })
}

pub fn load_checkpoint_from_disk(world_root: &Path) -> Option<TimelineSnapshot> {
    load_slot_from_disk(world_root, "default")
}

pub fn list_disk_slots(world_root: &Path) -> Vec<serde_json::Value> {
    let mut list = Vec::new();
    // 1. Дефолтный слот
    let def_meta = checkpoint_dir(world_root).join("meta.json");
    if let Ok(s) = fs::read_to_string(&def_meta) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&s) {
            list.push(serde_json::json!({
                "slot": "default",
                "game_time": val.get("game_time"),
                "saved_at_unix": val.get("saved_at_unix"),
                "guests_count": val.get("guests").and_then(|g| g.as_array()).map(|a| a.len()).unwrap_or(0)
            }));
        }
    }

    // 2. Именованные слоты
    let slots_base = world_root.join("rzero").join("slots");
    if let Ok(entries) = fs::read_dir(slots_base) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                let meta_f = p.join("meta.json");
                if let Ok(s) = fs::read_to_string(&meta_f) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&s) {
                        list.push(serde_json::json!({
                            "slot": name,
                            "game_time": val.get("game_time"),
                            "saved_at_unix": val.get("saved_at_unix"),
                            "guests_count": val.get("guests").and_then(|g| g.as_array()).map(|a| a.len()).unwrap_or(0)
                        }));
                    }
                }
            }
        }
    }

    list
}

pub fn delete_disk_slot(world_root: &Path, slot: &str) -> bool {
    let dir = slot_dir(world_root, slot);
    if dir.exists() {
        fs::remove_dir_all(dir).is_ok()
    } else {
        false
    }
}

fn format_uuid_std(u: u128) -> String {
    let hex = format!("{:032x}", u);
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

// ============================================================================
// Submodule: commands.rs
// ============================================================================


pub fn execute_rzero_command(
    game: &mut Game<'_>,
    requester: Option<u128>,
    command: &str,
) -> Vec<String> {
    let words: Vec<&str> = command.split_whitespace().collect();
    let sub = words.get(1).copied().unwrap_or("status");
    let reply = match sub {
        "json" => report_status_json(game),
        "status" => {
            if words.get(2).copied() == Some("--json") {
                report_status_json(game)
            } else {
                report_status(game)
            }
        }
        "set" | "save" => {
            let slot = words.get(2).copied().unwrap_or("default");
            create_slot_checkpoint_on_server(game, requester, slot);
            with_state_mut(|st| st.current_slot = slot.to_owned());
            format!("[RZero] Checkpoint saved to slot '{}' successfully.", slot)
        }
        "load" | "rollback" => {
            let slot = words.get(2).copied().unwrap_or("default");
            apply_client_level_rollback(game, slot);
            if rollback_slot_checkpoint_on_server(game, slot) {
                with_state_mut(|st| st.current_slot = slot.to_owned());
                format!(
                    "[RZero] Timeline rolled back to slot '{}' (tick={}).",
                    slot, game.world.game_time
                )
            } else {
                format!("[RZero] Rollback aborted: No timeline origin found for slot '{}'.", slot)
            }
        }
        "slot" | "slots" => handle_slot_subcommand(game, &words[2..], requester),
        "policy" => handle_policy_subcommand(game, &words[2..]),
        "metrics" => report_metrics_json(game),
        "rewind" => {
            let slot = words.get(2).copied().unwrap_or("default");
            apply_client_level_rollback(game, slot);
            if rollback_slot_checkpoint_on_server(game, slot) {
                format!("[RZero] Immediate rewind executed to slot '{}'.", slot)
            } else {
                format!("[RZero] Rewind failed: Slot '{}' not found.", slot)
            }
        }
        "trigger" => handle_trigger_subcommand(game, &words[2..], requester),
        "purge_memory" => {
            clear_snapshot();
            clear_client_snapshots();
            "[RZero] In-memory snapshots cleared.".to_owned()
        }
        "anchor" => handle_anchor_subcommand(game, &words[2..], requester),
        "autosave" => handle_autosave_subcommand(game, &words[2..]),
        "bench" | "benchmark" => handle_bench_subcommand(),
        "hitboxes" | "f3" => {
            let mode = words.get(2).copied();
            let val = match mode {
                Some("on" | "true" | "1") => true,
                Some("off" | "false" | "0") => false,
                _ => with_state(|st| !st.f3_loop_hitboxes),
            };
            with_state_mut(|st| st.f3_loop_hitboxes = val);
            save_current_settings(game);
            format!("[RZero] F3 loop hitboxes: {}", if val { "ON" } else { "OFF" })
        }
        _ => "[RZero] Usage: /rzero <set|load|json|status|slot|policy|metrics|anchor|autosave|bench|f3>".to_owned(),
    };
    vec![reply]
}

pub fn handle_bench_subcommand() -> String {
    let bench = with_state(|st| st.last_benchmark);
    match bench {
        Some(b) => {
            format!(
                "[RZero Benchmark] Last Rollback: {:.2} ms\n  ├── Blocks & Chunks:     {:.2} ms ({:.1}%)\n  ├── Entities & Mobs:     {:.2} ms ({:.1}%)\n  ├── Block Entities:      {:.2} ms ({:.1}%)\n  ├── Player & Network:    {:.2} ms ({:.1}%)\n  └── Globals & Weather:   {:.2} ms ({:.1}%)",
                b.total_ms,
                b.blocks_ms,
                if b.total_ms > 0.0 { (b.blocks_ms / b.total_ms) * 100.0 } else { 0.0 },
                b.entities_ms,
                if b.total_ms > 0.0 { (b.entities_ms / b.total_ms) * 100.0 } else { 0.0 },
                b.containers_ms,
                if b.total_ms > 0.0 { (b.containers_ms / b.total_ms) * 100.0 } else { 0.0 },
                b.player_ms,
                if b.total_ms > 0.0 { (b.player_ms / b.total_ms) * 100.0 } else { 0.0 },
                b.globals_ms,
                if b.total_ms > 0.0 { (b.globals_ms / b.total_ms) * 100.0 } else { 0.0 }
            )
        }
        None => "[RZero Benchmark] No rollback has been executed yet.".to_owned(),
    }
}

pub fn handle_slot_subcommand(
    game: &mut Game<'_>,
    args: &[&str],
    requester: Option<u128>,
) -> String {
    let sub = args.first().copied().unwrap_or("list");
    match sub {
        "list" => {
            let disk_slots = list_disk_slots(&game.world_root);
            serde_json::to_string_pretty(&serde_json::json!({
                "current_slot": with_state(|st| st.current_slot.clone()),
                "slots": disk_slots,
            }))
            .unwrap_or_else(|_| "{}".to_owned())
        }
        "save" => {
            let name = args.get(1).copied().unwrap_or("default");
            create_slot_checkpoint_on_server(game, requester, name);
            with_state_mut(|st| st.current_slot = name.to_owned());
            format!("[RZero] Saved timeline to slot '{}'.", name)
        }
        "load" => {
            let name = args.get(1).copied().unwrap_or("default");
            apply_client_level_rollback(game, name);
            if rollback_slot_checkpoint_on_server(game, name) {
                with_state_mut(|st| st.current_slot = name.to_owned());
                format!("[RZero] Rolled back to slot '{}'.", name)
            } else {
                format!("[RZero] Slot '{}' not found.", name)
            }
        }
        "select" => {
            let name = args.get(1).copied().unwrap_or("default");
            with_state_mut(|st| st.current_slot = name.to_owned());
            format!("[RZero] Active slot set to '{}'.", name)
        }
        "delete" => {
            let name = args.get(1).copied().unwrap_or("");
            if name.is_empty() || name == "default" {
                "[RZero] Cannot delete default slot.".to_owned()
            } else {
                delete_slot_snapshot(name);
                let deleted = delete_disk_slot(&game.world_root, name);
                if deleted {
                    format!("[RZero] Deleted slot '{}'.", name)
                } else {
                    format!("[RZero] Slot '{}' not found.", name)
                }
            }
        }
        _ => "[RZero] Usage: /rzero slot <list|save|load|select|delete> [name]".to_owned(),
    }
}

pub fn handle_policy_subcommand(game: &mut Game<'_>, args: &[&str]) -> String {
    let sub = args.first().copied().unwrap_or("get");
    match sub {
        "get" => {
            let pol = with_state(|st| st.policy.to_json());
            serde_json::to_string_pretty(&pol).unwrap_or_else(|_| "{}".to_owned())
        }
        "set" => {
            let key = args.get(1).copied().unwrap_or("");
            let val = match args.get(2).copied().unwrap_or("").to_ascii_lowercase().as_str() {
                "true" | "1" | "on" => Some(true),
                "false" | "0" | "off" => Some(false),
                _ => None,
            };
            if key.is_empty() || val.is_none() {
                return "[RZero] Usage: /rzero policy set <field> <true|false>".to_owned();
            }
            let success = with_state_mut(|st| st.policy.set_field(key, val.unwrap()));
            if success {
                save_current_settings(game);
                format!("[RZero] Policy '{}' set to {}.", key, val.unwrap())
            } else {
                format!("[RZero] Unknown policy field '{}'.", key)
            }
        }
        "reset" => {
            with_state_mut(|st| st.policy = RollbackPolicy::default());
            save_current_settings(game);
            "[RZero] Rollback policy reset to deterministic standard (100% rollback).".to_owned()
        }
        _ => "[RZero] Usage: /rzero policy <get|set|reset> [field] [value]".to_owned(),
    }
}

pub fn report_metrics_json(game: &Game<'_>) -> String {
    let (intensity, peak, relax, last_boss) = with_state(|st| {
        (
            st.current_intensity,
            st.peak_intensity,
            st.relax_ticks,
            st.last_boss_death_tick,
        )
    });

    let (player_health, player_food) = game
        .lan
        .as_ref()
        .and_then(|h| h.hub.guests.iter().find(|g| g.placed))
        .map(|g| (g.health, g.food))
        .unwrap_or((20.0, 20));

    let threat_level = (intensity / 10.0).clamp(0.0, 1.0);
    let is_safe = threat_level < 0.25 && player_health >= 15.0;

    let rec = if threat_level > 0.7 || player_health < 6.0 {
        "critical_danger"
    } else if threat_level > 0.25 {
        "combat_active"
    } else {
        "safe_to_save"
    };

    serde_json::to_string_pretty(&serde_json::json!({
        "threat_level": threat_level,
        "combat_intensity": intensity,
        "peak_intensity": peak,
        "relax_ticks": relax,
        "ticks_since_boss_defeat": game.world.game_time - last_boss,
        "player_health": player_health,
        "player_food": player_food,
        "safe_to_save": is_safe,
        "recommended_action": rec,
    }))
    .unwrap_or_else(|_| "{}".to_owned())
}

pub fn handle_trigger_subcommand(
    game: &mut Game<'_>,
    args: &[&str],
    requester: Option<u128>,
) -> String {
    match args.first().copied().unwrap_or("rewind") {
        "rewind" => {
            let slot = args.get(1).copied().unwrap_or("default");
            if rollback_slot_checkpoint_on_server(game, slot) {
                format!("[RZero] Triggered rewind to slot '{}'.", slot)
            } else {
                format!("[RZero] Trigger failed: Slot '{}' not found.", slot)
            }
        }
        "checkpoint" | "save" => {
            let slot = args.get(1).copied().unwrap_or("default");
            create_slot_checkpoint_on_server(game, requester, slot);
            format!("[RZero] Triggered checkpoint creation for slot '{}'.", slot)
        }
        _ => "[RZero] Usage: /rzero trigger <rewind|checkpoint> [slot]".to_owned(),
    }
}

pub fn report_status_json(game: &Game<'_>) -> String {
    let now = game.world.game_time;
    let online: Vec<u128> = game
        .lan
        .as_ref()
        .map(|h| {
            h.hub
                .guests
                .iter()
                .filter(|g| g.placed)
                .map(|g| g.uuid)
                .collect()
        })
        .unwrap_or_default();

    let (
        has_cp,
        last_save,
        anchor_mode,
        cooldown_sec,
        auto_save,
        adaptive,
        interval_sec,
        intensity,
        peak,
        current_slot,
    ) = with_state(|st| {
        (
            st.has_checkpoint,
            st.last_save_tick,
            st.anchor.mode.id(),
            st.anchor.rollback_cooldown_seconds,
            st.auto_save,
            st.adaptive_mode,
            st.fixed_interval_sec,
            st.current_intensity,
            st.peak_intensity,
            st.current_slot.clone(),
        )
    });

    let player_json = game
        .lan
        .as_ref()
        .and_then(|h| h.hub.guests.iter().find(|g| g.placed))
        .map(|g| {
            serde_json::json!({
                "uuid": format!("{:032x}", g.uuid),
                "name": g.name,
                "dimension": g.dimension,
                "position": g.position,
                "health": g.health,
                "food": g.food,
                "game_mode": match g.game_mode {
                    1 => "creative",
                    2 => "adventure",
                    3 => "spectator",
                    _ => "survival",
                },
                "on_ground": g.on_ground,
                "flying": g.flying,
            })
        });

    let anchors: Vec<String> = with_state(|st| {
        resolve_anchors(&st.anchor, &online, now, None)
            .into_iter()
            .map(|u| format!("{:032x}", u))
            .collect()
    });

    serde_json::to_string_pretty(&serde_json::json!({
        "rzero_version": "0.1.0",
        "has_checkpoint": has_cp,
        "current_slot": current_slot,
        "game_time": now,
        "day_time": game.world.day_time,
        "last_save_tick": last_save,
        "checkpoint_age_ticks": if has_cp { now - last_save } else { -1 },
        "anchor_mode": anchor_mode,
        "active_anchors": anchors,
        "rollback_cooldown_seconds": cooldown_sec,
        "auto_save": auto_save,
        "adaptive_mode": adaptive,
        "fixed_interval_seconds": interval_sec,
        "combat_intensity": intensity,
        "peak_intensity": peak,
        "player": player_json,
        "last_benchmark": with_state(|st| st.last_benchmark),
    }))
    .unwrap_or_else(|_| "{}".to_owned())
}

pub fn handle_anchor_subcommand(
    game: &mut Game<'_>,
    args: &[&str],
    requester: Option<u128>,
) -> String {
    match args.first().copied().unwrap_or("status") {
        "cooldown" => {
            let seconds = args
                .get(1)
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0)
                .max(0);
            with_state_mut(|st| {
                st.anchor.rollback_cooldown_seconds = seconds;
                st.last_rollback_tick = None;
            });
            save_current_settings(game);
            format!("[RZero] Rollback cooldown set to {} s", seconds)
        }
        "rotate" | "interval" => {
            let seconds = args
                .get(1)
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(300)
                .max(10);
            with_state_mut(|st| st.anchor.rotation_seconds = seconds);
            save_current_settings(game);
            format!("[RZero] Key player rotation interval set to {} s", seconds)
        }
        "pin" => {
            let target = match args.get(1) {
                Some(name) => {
                    let host = game.lan.as_ref();
                    host.and_then(|h| {
                        h.hub
                            .guests
                            .iter()
                            .find(|g| g.name.eq_ignore_ascii_case(name))
                            .map(|g| g.uuid)
                    })
                }
                None => requester,
            };
            if let Some(uuid) = target {
                let added = with_state_mut(|st| {
                    if !st.anchor.pinned.contains(&uuid) {
                        st.anchor.pinned.push(uuid);
                        true
                    } else {
                        false
                    }
                });
                save_current_settings(game);
                if added {
                    format!("[RZero] Pinned player {:032x} as key player.", uuid)
                } else {
                    format!("[RZero] Player {:032x} is already pinned.", uuid)
                }
            } else {
                "[RZero] Player not found.".to_owned()
            }
        }
        "unpin" => {
            let target = match args.get(1) {
                Some(name) => {
                    let host = game.lan.as_ref();
                    host.and_then(|h| {
                        h.hub
                            .guests
                            .iter()
                            .find(|g| g.name.eq_ignore_ascii_case(name))
                            .map(|g| g.uuid)
                    })
                }
                None => requester,
            };
            if let Some(uuid) = target {
                let removed = with_state_mut(|st| {
                    if let Some(idx) = st.anchor.pinned.iter().position(|u| *u == uuid) {
                        st.anchor.pinned.remove(idx);
                        true
                    } else {
                        false
                    }
                });
                save_current_settings(game);
                if removed {
                    format!("[RZero] Unpinned player {:032x}.", uuid)
                } else {
                    format!("[RZero] Player {:032x} was not pinned.", uuid)
                }
            } else {
                "[RZero] Player not found.".to_owned()
            }
        }
        "mode" => {
            let mode_str = args.get(1).copied().unwrap_or("fixed");
            let mode = AnchorMode::by_id(mode_str);
            with_state_mut(|st| {
                st.anchor.mode = mode;
                if mode == AnchorMode::Fixed
                    && st.anchor.pinned.is_empty()
                    && let Some(req) = requester
                {
                    st.anchor.pinned = vec![req];
                }
            });
            save_current_settings(game);
            format!("[RZero] Key player mode: {}", mode.id())
        }
        _ => report_status(game),
    }
}

pub fn handle_autosave_subcommand(game: &mut Game<'_>, args: &[&str]) -> String {
    match args.first().copied().unwrap_or("status") {
        "on" | "interval" => {
            let sec = args.get(1).and_then(|s| s.parse::<i32>().ok());
            with_state_mut(|st| {
                st.auto_save = true;
                st.adaptive_mode = false;
                if let Some(s) = sec {
                    st.fixed_interval_sec = s.max(5);
                }
                st.current_interval_ticks = i64::from(st.fixed_interval_sec) * 20;
            });
            save_current_settings(game);
            "[RZero] Interval auto-save enabled.".to_owned()
        }
        "adaptive" => {
            let sec = args.get(1).and_then(|s| s.parse::<i32>().ok());
            with_state_mut(|st| {
                st.auto_save = true;
                st.adaptive_mode = true;
                if let Some(s) = sec {
                    st.adaptive_interval_sec = s.max(60);
                }
            });
            save_current_settings(game);
            "[RZero] Adaptive auto-save enabled.".to_owned()
        }
        "off" => {
            with_state_mut(|st| st.auto_save = false);
            save_current_settings(game);
            "[RZero] Auto-save disabled.".to_owned()
        }
        _ => with_state(|st| {
            format!(
                "[RZero] Auto-save: {} | adaptive: {} | interval: {}s",
                st.auto_save, st.adaptive_mode, st.fixed_interval_sec
            )
        }),
    }
}

pub fn report_status(game: &Game<'_>) -> String {
    let online: Vec<u128> = game
        .lan
        .as_ref()
        .map(|h| {
            h.hub
                .guests
                .iter()
                .filter(|g| g.placed)
                .map(|g| g.uuid)
                .collect()
        })
        .unwrap_or_default();
    let now = game.world.game_time;
    with_state(|st| {
        let anchors = resolve_anchors(&st.anchor, &online, now, None);
        let cd_text = if st.anchor.rollback_cooldown_seconds <= 0 {
            "disabled".to_owned()
        } else {
            let rem = st
                .last_rollback_tick
                .map(|last| {
                    (i64::from(st.anchor.rollback_cooldown_seconds) * 20 - (now - last)).max(0) / 20
                })
                .unwrap_or(0);
            format!(
                "{} s (ready in {} s)",
                st.anchor.rollback_cooldown_seconds, rem
            )
        };
        format!(
            "[RZero] Slot: {} | Key player mode: {} | rollback cooldown: {} | active key players: {} | has_checkpoint: {}",
            st.current_slot,
            st.anchor.mode.id(),
            cd_text,
            anchors.len(),
            st.has_checkpoint
        )
    })
}

pub fn handle_client_command(game: &mut Game<'_>, command: &str, ran: &mut bool) {
    if !game.native_on(MOD_ID) {
        return;
    }
    let trimmed = command.trim().trim_start_matches('/');
    if !trimmed.starts_with("rzero") && trimmed != "rzero" {
        return;
    }
    if let Some(first) = trimmed.split_whitespace().next()
        && first != "rzero"
    {
        return;
    }
    *ran = true;
    let is_rollback = trimmed == "rzero load"
        || trimmed.starts_with("rzero load ")
        || trimmed == "rzero rollback"
        || trimmed.starts_with("rzero rollback ")
        || trimmed == "rzero rewind"
        || trimmed.starts_with("rzero rewind ")
        || trimmed.starts_with("rzero slot load ")
        || trimmed.starts_with("rzero trigger rewind");
    if is_rollback {
        game.close_container();
        game.screen = super::super::Screen::Playing;
    }
    let cmd_owned = trimmed.to_owned();
    if let Some(server) = &game.integrated {
        if let Some(lines) = server.call(move |srv| execute_rzero_command(srv, None, &cmd_owned)) {
            for line in lines {
                game.native_chat(&line);
            }
        }
    } else {
        for line in execute_rzero_command(game, None, &cmd_owned) {
            game.native_chat(&line);
        }
    }
}

pub fn intercept_owner_console_command(
    game: &mut Game<'_>,
    command: &str,
) -> ControlFlow<Vec<String>> {
    if !game.native_on(MOD_ID) {
        return ControlFlow::Continue(());
    }
    let trimmed = command.trim().trim_start_matches('/');
    if trimmed.split_whitespace().next() != Some("rzero") {
        return ControlFlow::Continue(());
    }
    let owner = game.serving.as_ref().map(|s| s.owner);
    let lines = execute_rzero_command(game, owner, trimmed);
    if let Some(host) = &game.lan
        && let Some(owner_uuid) = owner
        && let Some(g) = host.hub.guests.iter().find(|g| g.uuid == owner_uuid)
    {
        for line in &lines {
            g.send(&C::SystemChat(play::SystemChat {
                content: literal(line),
                overlay: false,
            }));
        }
    }
    ControlFlow::Break(lines)
}

pub fn intercept_guest_command(
    game: &mut Game<'_>,
    host: &mut LanHost,
    conn: u32,
    command: &str,
) -> ControlFlow<()> {
    if !game.native_on(MOD_ID) {
        return ControlFlow::Continue(());
    }
    let trimmed = command.trim().trim_start_matches('/');
    if trimmed.split_whitespace().next() != Some("rzero") {
        return ControlFlow::Continue(());
    }
    let requester = host.hub.guest(conn).map(|g| g.uuid);
    let lines = execute_rzero_command(game, requester, trimmed);
    if let Some(g) = host.hub.guest(conn) {
        for line in lines {
            g.send(&C::SystemChat(play::SystemChat {
                content: literal(&line),
                overlay: false,
            }));
        }
    }
    ControlFlow::Break(())
}

// ============================================================================
// Submodule: ui.rs
// ============================================================================


pub fn register_native_mod(mods: &mut Vec<NativeMod>) {
    mods.push(NativeMod {
        id: MOD_ID.to_owned(),
        title: "RZero 0.1.0".to_owned(),
        description:
            "Return by Death. Completely rewind the timeline and the world state to your last checkpoint upon death."
                .to_owned(),
        keys: vec![
            [
                "save".to_owned(),
                "Create Checkpoint".to_owned(),
                "key.keyboard.f6".to_owned(),
            ],
            [
                "load".to_owned(),
                "Return to Checkpoint".to_owned(),
                "key.keyboard.f7".to_owned(),
            ],
            [
                "anchor_menu".to_owned(),
                "Cycle Key Player Mode".to_owned(),
                "key.keyboard.f8".to_owned(),
            ],
        ],
    });
}

pub fn populate_settings(game: &mut Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    if mod_id != MOD_ID {
        return;
    }
    load_saved_settings(game);
    with_state(|st| {
        buttons.push(format!(
            "Auto-Save: {}",
            if st.auto_save { "ON" } else { "OFF" }
        ));
        buttons.push(format!(
            "Adaptive Save: {}",
            if st.adaptive_mode { "ON" } else { "OFF" }
        ));
        buttons.push(format!("Auto-Save Interval: {} s", st.fixed_interval_sec));
        buttons.push(format!("Key Player Mode: {}", st.anchor.mode.id()));
        let cd = st.anchor.rollback_cooldown_seconds;
        buttons.push(if cd <= 0 {
            "Rollback Cooldown: Off".to_owned()
        } else {
            format!("Rollback Cooldown: {cd} s")
        });
        buttons.push(format!(
            "Checkpoint on Boss Defeat: {}",
            if st.save_on_boss_defeat { "ON" } else { "OFF" }
        ));
        buttons.push(format!("Logs: {}", if st.logs { "ON" } else { "OFF" }));
        buttons.push(format!(
            "F3 Loop Hitboxes: {}",
            if st.f3_loop_hitboxes { "ON" } else { "OFF" }
        ));
    });
}

pub fn handle_settings_click(game: &mut Game<'_>, mod_id: &str, button: usize) {
    if mod_id != MOD_ID {
        return;
    }
    with_state_mut(|st| match button {
        0 => st.auto_save = !st.auto_save,
        1 => st.adaptive_mode = !st.adaptive_mode,
        2 => {
            st.fixed_interval_sec = match st.fixed_interval_sec {
                60 => 180,
                180 => 300,
                300 => 600,
                600 => 1200,
                _ => 60,
            };
            st.current_interval_ticks = i64::from(st.fixed_interval_sec) * 20;
        }
        3 => st.anchor.mode = st.anchor.mode.next(),
        4 => {
            st.anchor.rollback_cooldown_seconds = match st.anchor.rollback_cooldown_seconds {
                0 => 10,
                10 => 30,
                30 => 60,
                60 => 300,
                _ => 0,
            };
            st.last_rollback_tick = None;
        }
        5 => st.save_on_boss_defeat = !st.save_on_boss_defeat,
        6 => st.logs = !st.logs,
        7 => st.f3_loop_hitboxes = !st.f3_loop_hitboxes,
        _ => {}
    });
    save_current_settings(game);
}

pub fn load_saved_settings(game: &Game<'_>) {
    let auto_save = game
        .native_setting(MOD_ID, "auto_save")
        .map(|v| v != "false");
    let adaptive = game
        .native_setting(MOD_ID, "adaptive_mode")
        .map(|v| v == "true");
    let interval = game
        .native_setting(MOD_ID, "fixed_interval_sec")
        .and_then(|v| v.parse::<i32>().ok());
    let mode = game
        .native_setting(MOD_ID, "anchor_mode")
        .map(|v| AnchorMode::by_id(&v));
    let cooldown = game
        .native_setting(MOD_ID, "rollback_cooldown_sec")
        .and_then(|v| v.parse::<i32>().ok());
    let boss_save = game
        .native_setting(MOD_ID, "save_on_boss_defeat")
        .map(|v| v != "false");
    let logs = game.native_setting(MOD_ID, "logs").map(|v| v == "true");
    let f3_hitboxes = game
        .native_setting(MOD_ID, "f3_loop_hitboxes")
        .map(|v| v != "false");

    let json_cfg = fs::read_to_string(Path::new("config").join("rzero.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());

    with_state_mut(|st| {
        if let Some(v) = &json_cfg {
            let b = |val: &serde_json::Value, d: bool| val.as_bool().unwrap_or(d);
            let rb = &v["checkpointPolicy"]["rollback"];
            if rb.is_object() {
                st.policy.blocks = b(&rb["blocks"], true);
                st.policy.biomes = b(&rb["biomes"], true);
                st.policy.block_entities = b(&rb["block_entities"], true);
                st.policy.block_ticks = b(&rb["block_ticks"], true);
                st.policy.fluid_ticks = b(&rb["fluid_ticks"], true);
                st.policy.block_events = b(&rb["block_events"], true);
                let ent = &rb["entities"];
                if ent.is_object() {
                    st.policy.entities_presence = b(&ent["presence"], true);
                    st.policy.dropped_items = b(&ent["dropped_items"], true);
                    st.policy.experience_orbs = b(&ent["experience_orbs"], true);
                }
                let pl = &rb["players"];
                if pl.is_object() {
                    st.policy.player_position = b(&pl["position"], true);
                    st.policy.player_motion = b(&pl["motion"], true);
                    st.policy.player_health = b(&pl["health"], true);
                    st.policy.player_hunger = b(&pl["hunger"], true);
                    st.policy.player_experience = b(&pl["experience"], true);
                    st.policy.player_inventory = b(&pl["inventory"], true);
                    st.policy.player_ender_chest = b(&pl["ender_chest"], true);
                    st.policy.player_potion_effects = b(&pl["potion_effects"], true);
                    st.policy.player_cooldowns = b(&pl["cooldowns"], true);
                    st.policy.player_abilities = b(&pl["abilities"], true);
                    st.policy.player_recipe_book = b(&pl["recipe_book"], true);
                    st.policy.player_spawn_point = b(&pl["spawn_point"], true);
                    st.policy.player_stats = b(&pl["stats"], true);
                    st.policy.preserve_new_player_inventory =
                        b(&pl["preserve_new_player_inventory"], false);
                }
                let w = &rb["world"];
                if w.is_object() {
                    st.policy.world_time = b(&w["time"], true);
                    st.policy.world_weather = b(&w["weather"], true);
                    st.policy.world_raids = b(&w["raids"], true);
                    st.policy.world_dragon_fight = b(&w["dragon_fight"], true);
                    st.policy.world_scoreboard = b(&w["scoreboard"], true);
                    st.policy.world_level_rng = b(&w["level_rng"], true);
                    st.policy.world_difficulty = b(&w["difficulty"], true);
                    st.policy.world_spawn = b(&w["world_spawn"], true);
                    let sg = &w["server_globals"];
                    if sg.is_object() {
                        st.policy.server_bossbars = b(&sg["bossbars"], true);
                        st.policy.server_forced_chunks = b(&sg["forced_chunks"], true);
                        st.policy.server_world_border = b(&sg["world_border"], true);
                        st.policy.server_game_rules = b(&sg["game_rules"], true);
                    }
                }
            }
            let ad = &v["adaptive"];
            if ad.is_object() {
                if let Some(gap) = ad["min_save_gap_seconds"].as_i64() {
                    st.min_save_gap_sec = (gap as i32).max(5);
                }
                if let Some(rel) = ad["relax_time_seconds"].as_i64() {
                    st.relax_time_sec = (rel as i32).max(1);
                }
                if let Some(pct) = ad["post_combat_save_percent"].as_i64() {
                    st.post_combat_save_pct = (pct as i32).clamp(0, 100);
                }
                st.save_on_boss_defeat = b(&ad["save_on_boss_defeat"], st.save_on_boss_defeat);
            }
        }
        if let Some(v) = auto_save {
            st.auto_save = v;
        }
        if let Some(v) = adaptive {
            st.adaptive_mode = v;
        }
        if let Some(v) = interval {
            st.fixed_interval_sec = v.max(5);
            st.current_interval_ticks = i64::from(st.fixed_interval_sec) * 20;
        }
        if let Some(v) = mode {
            st.anchor.mode = v;
        }
        if let Some(v) = cooldown {
            st.anchor.rollback_cooldown_seconds = v.max(0);
        }
        if let Some(v) = boss_save {
            st.save_on_boss_defeat = v;
        }
        if let Some(v) = logs {
            st.logs = v;
        }
        if let Some(v) = f3_hitboxes {
            st.f3_loop_hitboxes = v;
        }
    });
}

pub fn save_current_settings(game: &Game<'_>) {
    with_state(|st| {
        game.native_set_setting(
            MOD_ID,
            "auto_save",
            if st.auto_save { "true" } else { "false" },
        );
        game.native_set_setting(
            MOD_ID,
            "adaptive_mode",
            if st.adaptive_mode { "true" } else { "false" },
        );
        game.native_set_setting(
            MOD_ID,
            "fixed_interval_sec",
            &st.fixed_interval_sec.to_string(),
        );
        game.native_set_setting(MOD_ID, "anchor_mode", st.anchor.mode.id());
        game.native_set_setting(
            MOD_ID,
            "rollback_cooldown_sec",
            &st.anchor.rollback_cooldown_seconds.to_string(),
        );
        game.native_set_setting(
            MOD_ID,
            "save_on_boss_defeat",
            if st.save_on_boss_defeat {
                "true"
            } else {
                "false"
            },
        );
        game.native_set_setting(MOD_ID, "logs", if st.logs { "true" } else { "false" });
        game.native_set_setting(
            MOD_ID,
            "f3_loop_hitboxes",
            if st.f3_loop_hitboxes { "true" } else { "false" },
        );
    });
}

// ============================================================================
// Submodule: tracker.rs
// ============================================================================


#[derive(Clone, Copy, Debug)]
pub struct MobTimelineState {
    pub pos: [f64; 3],
    pub head_yaw: f32,
    pub head_pitch: f32,
    pub health: f32,
}

pub struct LoopTracker {
    pub checkpoint_tick: i64,
    pub previous_tracks: FxHashMap<u32, Vec<(i64, MobTimelineState)>>,
    pub current_tracks: FxHashMap<u32, Vec<(i64, MobTimelineState)>>,
}

impl LoopTracker {
    pub fn new() -> Self {
        Self {
            checkpoint_tick: 0,
            previous_tracks: FxHashMap::default(),
            current_tracks: FxHashMap::default(),
        }
    }

    pub fn on_checkpoint(&mut self, save_tick: i64) {
        self.checkpoint_tick = save_tick;
        self.previous_tracks.clear();
        self.current_tracks.clear();
    }

    pub fn on_rollback(&mut self, restore_tick: i64) {
        self.checkpoint_tick = restore_tick;
        self.previous_tracks = std::mem::take(&mut self.current_tracks);
        self.current_tracks.clear();
    }

    pub fn record_tick(&mut self, now_tick: i64, mobs: &horizon_client::join_seq::JoinList<Mob>) {
        let delta_t = now_tick - self.checkpoint_tick;
        if delta_t < 0 {
            return;
        }
        for m in mobs.iter() {
            if m.removed {
                continue;
            }
            let track = self.current_tracks.entry(m.id).or_default();
            if track.len() > 3600 {
                track.remove(0);
            }
            track.push((
                delta_t,
                MobTimelineState {
                    pos: [m.position.x, m.position.y, m.position.z],
                    head_yaw: m.head_yaw,
                    head_pitch: m.head_pitch,
                    health: m.health,
                },
            ));
        }
    }

    pub fn get_mob_color(
        &self,
        mob_id: u32,
        now_tick: i64,
        current_pos: [f64; 3],
        current_yaw: f32,
        current_hp: f32,
    ) -> u32 {
        let Some(prev_track) = self.previous_tracks.get(&mob_id) else {
            // Моб не был в прошлой петле -> Белая обводка
            return 0xFFFF_FFFF;
        };

        if prev_track.is_empty() {
            return 0xFFFF_FFFF;
        }

        let delta_t = (now_tick - self.checkpoint_tick).max(0);
        let frame = match prev_track.binary_search_by_key(&delta_t, |&(t, _)| t) {
            Ok(idx) => Some(&prev_track[idx].1),
            Err(idx) => {
                if idx == 0 {
                    prev_track.first().map(|(_, f)| f)
                } else if idx >= prev_track.len() {
                    prev_track.last().map(|(_, f)| f)
                } else {
                    let (t_left, f_left) = &prev_track[idx - 1];
                    let (t_right, f_right) = &prev_track[idx];
                    if (delta_t - t_left).abs() <= (delta_t - t_right).abs() {
                        Some(f_left)
                    } else {
                        Some(f_right)
                    }
                }
            }
        };

        let Some(prev_state) = frame else {
            return 0xFFFF_FFFF;
        };

        let dx = current_pos[0] - prev_state.pos[0];
        let dy = current_pos[1] - prev_state.pos[1];
        let dz = current_pos[2] - prev_state.pos[2];
        let dist_sq = dx * dx + dy * dy + dz * dz;

        let mut dyaw = (current_yaw - prev_state.head_yaw).abs() % 360.0;
        if dyaw > 180.0 {
            dyaw = 360.0 - dyaw;
        }

        let dhp = (current_hp - prev_state.health).abs();

        // Порог совпадения действий: смещение до 0.35 блока, поворот головы до 40 градусов
        if dist_sq <= (0.35 * 0.35) && dyaw <= 40.0 && dhp < 0.5 {
            // Зеленая: моб делает те же действия из прошлой петли
            0xFF22_FF55
        } else {
            // Красная: моб делает другие действия
            0xFFFF_3333
        }
    }
}

static LOOP_TRACKER: OnceLock<RwLock<LoopTracker>> = OnceLock::new();

pub fn loop_tracker() -> &'static RwLock<LoopTracker> {
    LOOP_TRACKER.get_or_init(|| RwLock::new(LoopTracker::new()))
}

pub fn process_hitbox_lines(game: &Game<'_>, lines: &mut Vec<horizon_client::gizmos::Line>) {
    let enabled = with_state(|st| st.f3_loop_hitboxes);
    if !enabled {
        return;
    }

    let tracker_guard = match loop_tracker().read() {
        Ok(g) => g,
        Err(_) => return,
    };

    let now_tick = game.world.game_time;

    for m in game.mobs.iter() {
        if m.removed || m.invisible {
            continue;
        }
        let pos = [m.position.x, m.position.y, m.position.z];
        let color = tracker_guard.get_mob_color(m.id, now_tick, pos, m.head_yaw, m.health);

        if color == 0xFFFF_FFFF {
            // Белая по умолчанию
            continue;
        }

        let half = f64::from(m.width()) / 2.0;
        let min_x = pos[0] - half - 0.08;
        let max_x = pos[0] + half + 0.08;
        let min_y = pos[1] - 0.08;
        let max_y = pos[1] + f64::from(m.height()) + 0.08;
        let min_z = pos[2] - half - 0.08;
        let max_z = pos[2] + half + 0.08;

        for line in lines.iter_mut() {
            if line.color == 0xFFFF_FFFF {
                let inside_start = line.start[0] >= min_x && line.start[0] <= max_x
                    && line.start[1] >= min_y && line.start[1] <= max_y
                    && line.start[2] >= min_z && line.start[2] <= max_z;
                let inside_end = line.end[0] >= min_x && line.end[0] <= max_x
                    && line.end[1] >= min_y && line.end[1] <= max_y
                    && line.end[2] >= min_z && line.end[2] <= max_z;
                if inside_start && inside_end {
                    line.color = color;
                }
            }
        }
    }
}

// ============================================================================
// Submodule: hooks.rs
// ============================================================================


pub fn handle_key(game: &mut Game<'_>, key: &str, pressed: bool) {
    if !pressed || !game.native_on(MOD_ID) || !game.native_playing() {
        return;
    }
    let save_key = game.native_mod_key(MOD_ID, "save", "key.keyboard.f6");
    let load_key = game.native_mod_key(MOD_ID, "load", "key.keyboard.f7");
    let mode_key = game.native_mod_key(MOD_ID, "anchor_menu", "key.keyboard.f8");

    if key == save_key {
        if let Some(server) = &game.integrated {
            let msg = server.call(|srv| {
                create_checkpoint_on_server(srv, None);
                "[RZero] Checkpoint created successfully.".to_owned()
            });
            if let Some(text) = msg {
                game.native_chat(&text);
            }
        } else {
            with_state_mut(|st| st.pending_save_request = true);
        }
    } else if key == load_key {
        game.close_container();
        game.screen = super::super::Screen::Playing;
        apply_client_level_rollback(game, "default");
        if let Some(server) = &game.integrated {
            let msg = server.call(|srv| {
                if rollback_checkpoint_on_server(srv) {
                    format!(
                        "[RZero] Timeline rolled back to checkpoint (tick={}).",
                        srv.world.game_time
                    )
                } else {
                    "[RZero] Rollback aborted: No timeline origin found.".to_owned()
                }
            });
            if let Some(text) = msg {
                game.native_chat(&text);
            }
        } else {
            with_state_mut(|st| st.pending_load_request = true);
        }
    } else if key == mode_key {
        let next_mode = with_state_mut(|st| {
            st.anchor.mode = st.anchor.mode.next();
            st.anchor.mode
        });
        save_current_settings(game);
        game.native_chat(&format!("[RZero] Key player mode: {}", next_mode.id()));
    }
}

pub fn handle_lifecycle(game: &mut Game<'_>, phase: WorldPhase) {
    match phase {
        WorldPhase::WorldOpenReady => {
            load_saved_settings(game);
            let has_disk_cp = has_disk_checkpoint(&game.world_root);
            if has_disk_cp {
                if get_snapshot_clone().is_none() {
                    if let Some(snap) = load_checkpoint_from_disk(&game.world_root) {
                        set_snapshot(Some(Arc::new(snap)));
                    }
                }
                with_state_mut(|st| {
                    st.has_checkpoint = true;
                });
            } else {
                clear_snapshot();
                with_state_mut(|st| {
                    st.has_checkpoint = false;
                });
            }
            with_state_mut(|st| {
                st.anchor_ids.clear();
                st.pending_death_rollback = None;
                st.last_save_tick = game.world.game_time;
                st.current_interval_ticks = i64::from(st.fixed_interval_sec.max(5)) * 20;
                st.last_rollback_tick = None;
                st.last_boss_death_tick = -9999;
                st.current_intensity = 0.0;
                st.peak_intensity = 0.0;
                st.relax_ticks = 0;
            });
        }
        WorldPhase::WorldCloseFinal
        | WorldPhase::WorldCloseFailed
        | WorldPhase::WorldOpenFailed => {
            with_state_mut(|st| {
                st.pending_death_rollback = None;
            });
        }
    }
}

pub fn handle_tick(game: &mut Game<'_>) {
    if !game.native_on(MOD_ID) {
        return;
    }

    if game.screen == super::super::Screen::Dead {
        let has_cp = with_state(|st| st.has_checkpoint);
        if has_cp {
            game.screen = super::super::Screen::Playing;
            game.ui.death_time = 0;
            game.ui.died_at = None;
            game.ui.death_confirm = None;
            game.boss_overlay.reset();
            if let Some(p) = game.player.as_mut() {
                let _ = p.set_health(20.0, false);
                p.health.dead = false;
                p.invulnerable = false;
                p.may_fly = false;
                p.spectator = false;
                p.flying = false;
                p.fall_distance = 0.0;
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
            apply_client_level_rollback(game, "default");
            if let Some(server) = &game.integrated {
                server.call(|srv| {
                    rollback_checkpoint_on_server(srv);
                });
            } else {
                rollback_checkpoint_on_server(game);
            }
            return;
        }
    }

    if game.native_window() {
        return;
    }

    if game.remote.is_some() {
        let current_epoch = get_rollback_epoch();
        std::thread_local! {
            static CLIENT_LAST_EPOCH: RefCell<u64> = const { RefCell::new(0) };
        }
        let needs_client_rollback = CLIENT_LAST_EPOCH.with(|cell| {
            let mut last = cell.borrow_mut();
            if *last != current_epoch {
                *last = current_epoch;
                true
            } else {
                false
            }
        });

        if needs_client_rollback && current_epoch > 0 {
            let slot = with_state(|st| st.current_slot.clone());
            apply_client_level_rollback(game, &slot);
        }
    }

    let (do_save, do_load, dead_rollback) = with_state_mut(|st| {
        (
            std::mem::replace(&mut st.pending_save_request, false),
            std::mem::replace(&mut st.pending_load_request, false),
            st.pending_death_rollback.take(),
        )
    });

    if do_save {
        create_checkpoint_on_server(game, None);
    }
    if do_load || dead_rollback.is_some() {
        apply_client_level_rollback(game, "default");
        rollback_checkpoint_on_server(game);
        return;
    }

    if with_state(|st| st.has_checkpoint) {
        if let Ok(mut tr) = loop_tracker().write() {
            tr.record_tick(game.world.game_time, &game.mobs);
        }
    }

    tick_autosave_on_server(game);
}

pub fn intercept_guest_die(
    game: &mut Game<'_>,
    host: &mut LanHost,
    gi: usize,
    _conn: u32,
) -> ControlFlow<()> {
    if !game.native_on(MOD_ID) {
        return ControlFlow::Continue(());
    }
    if host.hub.guests[gi].health > 0.0 || host.hub.guests[gi].dead {
        return ControlFlow::Continue(());
    }
    let player_uuid = host.hub.guests[gi].uuid;
    let online: Vec<u128> = host
        .hub
        .guests
        .iter()
        .filter(|g| g.placed)
        .map(|g| g.uuid)
        .collect();
    let now = game.world.game_time;

    let should_intercept = with_state_mut(|st| {
        if !st.has_checkpoint {
            return false;
        }
        let active_anchors = resolve_anchors(&st.anchor, &online, now, None);
        let is_anchor =
            active_anchors.contains(&player_uuid) || st.anchor_ids.contains(&player_uuid);
        if !is_anchor {
            return false;
        }
        let cd = st.anchor.rollback_cooldown_seconds;
        if cd > 0
            && let Some(last) = st.last_rollback_tick
            && now >= last
            && now - last < i64::from(cd) * 20
        {
            return false;
        }
        st.last_rollback_tick = Some(now);
        st.pending_death_rollback = Some(player_uuid);
        true
    });

    if !should_intercept {
        return ControlFlow::Continue(());
    }

    let g = &mut host.hub.guests[gi];
    g.health = 20.0;
    g.dead = false;
    g.death_time = 0;
    g.sent_health = None;
    if let Some(m) = g.motion.as_mut() {
        m.health.dead = false;
        m.invulnerable = false;
        m.may_fly = false;
        m.spectator = false;
        m.flying = false;
        m.fall_distance = 0.0;
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
        let _ = m.set_health(20.0, false);
    }
    ControlFlow::Break(())
}
