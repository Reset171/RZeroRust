use crate::log_info;
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
    time::Instant,
};

pub struct DetOrder;

impl DetOrder {
    pub fn sorted_entries<K: Clone + Ord, V: Clone>(map: &HashMap<K, V>) -> Vec<(K, V)> {
        let mut list: Vec<(K, V)> = map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        list.sort_by(|a, b| a.0.cmp(&b.0));
        list
    }
}

pub struct LootSeed;

impl LootSeed {
    pub const EQUIPMENT_SALT: i64 = -0x61C8864680B583EB;
    pub const CHEST_SALT: i64 = 0x5F37_59DF_7F4A_7C15;

    pub fn mix(uuid: u128, level_seed: i64) -> i64 {
        let msb = (uuid >> 64) as i64;
        let lsb = uuid as i64;
        let mut h = msb ^ lsb ^ level_seed;
        h ^= (h as u64 >> 30) as i64;
        h = h.wrapping_mul(-0x40A7B892E31B1A47);
        h ^= (h as u64 >> 27) as i64;
        h = h.wrapping_mul(-0x6B2FB644ECFEED15);
        h ^= (h as u64 >> 31) as i64;
        if h == 0 {
            -0x5A5A_5A5A_5A5A_5A5B
        } else {
            h
        }
    }

    pub fn mix_pos(pos_long: i64, level_seed: i64) -> i64 {
        let mut h = pos_long ^ level_seed ^ Self::CHEST_SALT;
        h ^= (h as u64 >> 30) as i64;
        h = h.wrapping_mul(-0x40A7B892E31B1A47);
        h ^= (h as u64 >> 27) as i64;
        h = h.wrapping_mul(-0x6B2FB644ECFEED15);
        h ^= (h as u64 >> 31) as i64;
        if h == 0 {
            -0x5A5A_5A5A_5A5A_5A5B
        } else {
            h
        }
    }
}

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

struct BenchState {
    open: bool,
    timed_out: bool,
    title: String,
    start: Option<Instant>,
    start_tick: i32,
    deferred_ticks: i32,
    nanos: [u64; PHASE_COUNT],
    blocks_changed: usize,
    chunks: usize,
    mobs: usize,
    players: usize,
}

impl Default for BenchState {
    fn default() -> Self {
        Self {
            open: false,
            timed_out: false,
            title: String::new(),
            start: None,
            start_tick: 0,
            deferred_ticks: 0,
            nanos: [0; PHASE_COUNT],
            blocks_changed: 0,
            chunks: 0,
            mobs: 0,
            players: 0,
        }
    }
}

static BENCH: OnceLock<Mutex<BenchState>> = OnceLock::new();

fn bench_lock() -> &'static Mutex<BenchState> {
    BENCH.get_or_init(|| Mutex::new(BenchState::default()))
}

pub struct RZBenchmark;

impl RZBenchmark {
    pub fn begin(title: &str, current_tick: i32) {
        if !crate::verbose() {
            return;
        }
        let mut b = bench_lock().lock().unwrap_or_else(|e| e.into_inner());
        b.open = true;
        b.timed_out = false;
        b.title = title.to_owned();
        b.start = Some(Instant::now());
        b.start_tick = current_tick;
        b.deferred_ticks = 0;
        b.nanos = [0; PHASE_COUNT];
        b.blocks_changed = 0;
        b.chunks = 0;
        b.mobs = 0;
        b.players = 0;
    }

    pub fn accum(phase: Phase, elapsed_nanos: u64) {
        let mut b = bench_lock().lock().unwrap_or_else(|e| e.into_inner());
        if b.open {
            b.nanos[phase as usize] += elapsed_nanos;
        }
    }

    pub fn tick(current_tick: i32) {
        let mut b = bench_lock().lock().unwrap_or_else(|e| e.into_inner());
        if !b.open {
            return;
        }
        b.deferred_ticks = (current_tick - b.start_tick).max(0);
        if b.deferred_ticks > 60 {
            b.timed_out = true;
        }
    }

    pub fn is_timed_out() -> bool {
        let b = bench_lock().lock().unwrap_or_else(|e| e.into_inner());
        b.timed_out
    }

    pub fn end_and_log() {
        let mut b = bench_lock().lock().unwrap_or_else(|e| e.into_inner());
        if !b.open || !crate::verbose() {
            b.open = false;
            return;
        }
        b.open = false;
        let total_ms = b
            .start
            .map_or(0.0, |s| s.elapsed().as_secs_f64() * 1000.0);
        let work_ns: u64 = b.nanos.iter().sum();
        let work_ms = work_ns as f64 / 1_000_000.0;
        log_info(&format!(
            "[Benchmark] === {}: {:.1} ms (wall) | work {:.1} ms ===",
            b.title, total_ms, work_ms
        ));
    }
}