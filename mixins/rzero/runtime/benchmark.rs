// 1:1 Rust port of RZBenchmark.java for nanosecond phase profiling of Rollback and Capture.

use std::cell::RefCell;
use std::time::Instant;

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
    use std::fs::OpenOptions;
    use std::io::Write;

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
