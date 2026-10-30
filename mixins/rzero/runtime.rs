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
    bossbar::BossBars,
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
    fs,
    ops::ControlFlow,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
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
    pub id: u32,
    pub state: BlockStateId,
    pub start_pos: BlockPos,
    pub position: Vec3,
    pub previous: Vec3,
    pub velocity: Vec3,
    pub age: i32,
    pub no_gravity: bool,
    pub drop_item: bool,
    pub fall_distance: f64,
    pub stuck: Vec3,
    pub damage: FallingDamage,
    pub saved: Option<Box<([i32; 4], Tag)>>,
}

impl FallingSnapshot {
    pub fn from_live(f: &FallingBlock) -> Self {
        Self {
            id: f.id,
            state: f.state,
            start_pos: f.start_pos,
            position: f.position,
            previous: f.previous,
            velocity: f.velocity,
            age: f.age,
            no_gravity: f.no_gravity,
            drop_item: f.drop_item,
            fall_distance: f.fall_distance,
            stuck: f.stuck,
            damage: f.damage,
            saved: f.saved.clone(),
        }
    }

    pub fn to_live(&self) -> FallingBlock {
        FallingBlock {
            id: self.id,
            state: self.state,
            start_pos: self.start_pos,
            position: self.position,
            previous: self.previous,
            velocity: self.velocity,
            age: self.age,
            no_gravity: self.no_gravity,
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
    pub mobs: Vec<Mob>,
    pub dropped: Vec<ItemEntity>,
    pub orbs: Vec<ExperienceOrb>,
    pub projectiles: Vec<horizon_client::projectiles::Projectile>,
    pub boats: Vec<Boat>,
    pub carts: Vec<Minecart>,
    pub tnts: Vec<PrimedTnt>,
    pub falling: Vec<FallingSnapshot>,
    pub paintings: Vec<Painting>,
    pub item_frames: Vec<ItemFrame>,
    pub armor_stands: Vec<ArmorStand>,
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
    pub world_spawn: Option<[i32; 3]>,
    pub world_spawn_angle: [f32; 2],
    pub next_entity: u32,
    pub scoreboard: Scoreboard,
    pub guests: HashMap<u128, GuestSnapshot>,
    pub offline_player_data: HashMap<PathBuf, Vec<u8>>,
    pub levels: HashMap<Dim, LevelSnapshot>,
}

std::thread_local! {
    pub static SNAPSHOT: RefCell<Option<TimelineSnapshot>> = const { RefCell::new(None) };
}

pub fn get_snapshot_clone() -> Option<TimelineSnapshot> {
    SNAPSHOT.with(|cell| cell.borrow().clone())
}

pub fn set_snapshot(snap: Option<TimelineSnapshot>) {
    SNAPSHOT.with(|cell| *cell.borrow_mut() = snap);
}

pub fn clear_snapshot() {
    SNAPSHOT.with(|cell| *cell.borrow_mut() = None);
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
        let min_y = game.dim.min_y();
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
    game.save_raids();
    game.save_fight();
    game.save_wandering_trader();

    let mut guests = HashMap::new();
    let mut online = Vec::new();
    if let Some(host) = &game.lan {
        for g in host.hub.guests.iter().filter(|g| g.placed) {
            online.push(g.uuid);
            let motion = g.motion.as_deref();
            let (velocity, absorption, exhaustion, fire_ticks, air, fall_distance) = match motion {
                Some(m) => (
                    m.velocity,
                    m.health.absorption(),
                    m.health.food.exhaustion,
                    m.fire_ticks,
                    m.air,
                    m.fall_distance,
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
                    )
                }
            };
            let (shield_cooldown, cooldowns, stats) = match host.play.get(&g.conn) {
                Some(p) => (
                    p.shield_cooldown,
                    p.cooldowns.clone(),
                    p.stats.values.clone(),
                ),
                None => (0, Cooldowns::default(), BTreeMap::new()),
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

    let timeline = TimelineSnapshot {
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
        world_spawn: game.level.world_spawn,
        world_spawn_angle: game.level.world_spawn_angle,
        next_entity: game.next_entity,
        scoreboard: game.score.board.clone(),
        guests,
        offline_player_data,
        levels,
    };

    set_snapshot(Some(timeline));
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
    game.hook = None;
    game.remote_hooks.clear();
    game.lan_events.clear();
    game.lan_block_events.clear();
    game.lan_entity_events.clear();
    game.lan_critical_hits.clear();
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

    restore_offline_files(&snap.offline_player_data, policy.player_stats);

    game.streaming.wait(std::time::Duration::from_millis(200));
    let _ = game.save.flush();

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
                    if p.is_file() && !snap.region_files.contains(&p) {
                        let _ = fs::remove_file(&p);
                    }
                }
            }
        }
    }

    if let Ok((fresh_streaming, fresh_save, _)) = super::super::world_loading::make_streaming(
        &game.res,
        &game.pack,
        &game.options,
        &game.settings,
        &game.world_root,
        game.dim,
        game.level.seed,
    ) {
        game.streaming = fresh_streaming;
        game.save = fresh_save;
    } else {
        game.save = Arc::new(horizon_client::save::SaveDir::new(
            game.save.root.clone(),
            game.save.shape,
        ));
    }

    let target_dim = snap
        .guests
        .values()
        .next()
        .map(|g| Dim::from_id(&g.dimension))
        .unwrap_or_else(|| {
            snap.levels.keys().copied().next().unwrap_or(Dim::Overworld)
        });

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

    for dim in other_dims {
        if let Some(level_snap) = snap.levels.get(&dim) {
            if game.enter_level(None, dim).is_ok() {
                apply_level_snapshot_in_place(game, dim, level_snap, policy);
            }
        }
    }
    if game.dim != target_dim {
        let _ = game.enter_level(None, target_dim);
    }

    if let Some(level_snap) = snap.levels.get(&target_dim) {
        apply_level_snapshot_in_place(game, target_dim, level_snap, policy);
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
        "set" | "save" => {
            create_checkpoint_on_server(game, requester);
            "[RZero] Checkpoint created successfully.".to_owned()
        }
        "load" | "rollback" => {
            if rollback_checkpoint_on_server(game) {
                format!(
                    "[RZero] Timeline rolled back to checkpoint (tick={}).",
                    game.world.game_time
                )
            } else {
                "[RZero] Rollback aborted: No timeline origin found.".to_owned()
            }
        }
        "status" => report_status(game),
        "anchor" => handle_anchor_subcommand(game, &words[2..], requester),
        "autosave" => handle_autosave_subcommand(game, &words[2..]),
        _ => "[RZero] Usage: /rzero <set|load|status|anchor|autosave>".to_owned(),
    };
    vec![reply]
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
            if seconds <= 0 {
                "[RZero] Rollback cooldown disabled: every key player death rewinds the world."
                    .to_owned()
            } else {
                format!("[RZero] Rollback cooldown: {seconds} (in seconds)")
            }
        }
        "mode" => {
            let mode = AnchorMode::by_id(args.get(1).copied().unwrap_or("fixed"));
            let rot = args
                .get(2)
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(300)
                .max(5);
            with_state_mut(|st| {
                st.anchor.mode = mode;
                if mode == AnchorMode::Rotating {
                    st.anchor.rotation_seconds = rot;
                }
                if matches!(mode, AnchorMode::Fixed | AnchorMode::Multi)
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
            "[RZero] Key player mode: {} | rollback cooldown: {} | active key players: {} | has_checkpoint: {}",
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
    game.lan = Some(std::mem::replace(host, empty_lan_placeholder()));
    let lines = execute_rzero_command(game, requester, trimmed);
    if let Some(restored_host) = game.lan.take() {
        *host = restored_host;
    }
    for line in lines {
        host.hub.send_to(
            conn,
            &C::SystemChat(play::SystemChat {
                content: literal(&line),
                overlay: false,
            }),
        );
    }
    ControlFlow::Break(())
}

pub fn empty_lan_placeholder() -> LanHost {
    unreachable!("game.lan is always restored")
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
    });
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
            clear_snapshot();
            with_state_mut(|st| {
                st.has_checkpoint = false;
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
            clear_snapshot();
            with_state_mut(|st| {
                st.has_checkpoint = false;
                st.anchor_ids.clear();
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
        rollback_checkpoint_on_server(game);
        return;
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
        let _ = m.set_health(20.0, false);
    }
    ControlFlow::Break(())
}
