use super::super::{
    Dim, ExperienceOrb, FallingBlock, ItemEntity, Mob,
    boats::Boat,
    commands::OtherEntity,
    dragon::{Crystal, Dragon},
    machines::HopperState,
    minecarts::Minecart,
    tnt::PrimedTnt,
};
use super::types::*;
use horizon_client::{
    advancements::Progress,
    chunk::Chunk,
    containers::BlockEntity,
    cooldowns::Cooldowns,
    effects::EffectInstance,
    entities::{ArmorStand, Cushion, ItemFrame, Painting},
    falling_block::Damage as FallingDamage,
    fast_hash::FxHashMap,
    gamerules::GameRules,
    items::ItemStack,
    recipe_book::Settings as RecipeBookSettings,
    scoreboard::Scoreboard,
    sculk::WardenSpawnTracker,
    world::{HealthRules, Weather},
    worldborder::WorldBorder,
};
use horizon_core::{
    geometry::Vec3,
    position::BlockPos,
    random::LegacyRandom,
    ticks::ChunkTicks,
};
use horizon_world::{
    level::{BlockEventData, MovingPiston},
    nbt::Tag,
    registry::BlockStateId,
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

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
