use std::sync::{Mutex, OnceLock};

pub type ServerCallback = Box<dyn Fn(i64) + Send + Sync>;
pub type ProfileStartCallback = Box<dyn Fn(i64, &str) + Send + Sync>;
pub type ProfileEndCallback = Box<dyn Fn(i64, &str, usize, usize) + Send + Sync>;

#[derive(Clone, Debug)]
pub struct MetricsEvent {
    pub player_uuid: u128,
    pub threat: f64,
    pub mobs_track: i32,
    pub mobs_attack: i32,
    pub free_space: i32,
    pub cornered_pct: f32,
    pub cliffs_near: i32,
    pub event_label: String,
    pub proj_air: i32,
}

pub type MetricsCallback = Box<dyn Fn(&MetricsEvent) + Send + Sync>;

#[derive(Default)]
struct HookRegistry {
    on_server_started: Vec<ServerCallback>,
    on_server_tick_post: Vec<ServerCallback>,
    on_checkpoint_saved: Vec<ServerCallback>,
    on_checkpoint_loaded: Vec<ServerCallback>,
    on_profile_start: Vec<ProfileStartCallback>,
    on_profile_end: Vec<ProfileEndCallback>,
    on_metrics: Vec<MetricsCallback>,
}

static HOOKS: OnceLock<Mutex<HookRegistry>> = OnceLock::new();

fn registry() -> &'static Mutex<HookRegistry> {
    HOOKS.get_or_init(|| Mutex::new(HookRegistry::default()))
}

pub struct DevHooks;

impl DevHooks {
    pub fn on_server_started(cb: ServerCallback) {
        if let Ok(mut reg) = registry().lock() {
            reg.on_server_started.push(cb);
        }
    }

    pub fn on_server_tick_post(cb: ServerCallback) {
        if let Ok(mut reg) = registry().lock() {
            reg.on_server_tick_post.push(cb);
        }
    }

    pub fn on_checkpoint_saved(cb: ServerCallback) {
        if let Ok(mut reg) = registry().lock() {
            reg.on_checkpoint_saved.push(cb);
        }
    }

    pub fn on_checkpoint_loaded(cb: ServerCallback) {
        if let Ok(mut reg) = registry().lock() {
            reg.on_checkpoint_loaded.push(cb);
        }
    }

    pub fn on_metrics(cb: MetricsCallback) {
        if let Ok(mut reg) = registry().lock() {
            reg.on_metrics.push(cb);
        }
    }

    pub fn fire_server_started(tick: i64) {
        if let Ok(reg) = registry().lock() {
            for cb in &reg.on_server_started {
                cb(tick);
            }
        }
    }

    pub fn fire_server_tick_post(tick: i64) {
        if let Ok(reg) = registry().lock() {
            for cb in &reg.on_server_tick_post {
                cb(tick);
            }
        }
    }

    pub fn fire_checkpoint_saved(tick: i64) {
        if let Ok(reg) = registry().lock() {
            for cb in &reg.on_checkpoint_saved {
                cb(tick);
            }
        }
    }

    pub fn fire_checkpoint_loaded(tick: i64) {
        if let Ok(reg) = registry().lock() {
            for cb in &reg.on_checkpoint_loaded {
                cb(tick);
            }
        }
    }

    pub fn fire_profile_start(tick: i64, tag: &str) {
        if let Ok(reg) = registry().lock() {
            for cb in &reg.on_profile_start {
                cb(tick, tag);
            }
        }
    }

    pub fn fire_profile_end(tick: i64, tag: &str, chunks_processed: usize, chunks_queued: usize) {
        if let Ok(reg) = registry().lock() {
            for cb in &reg.on_profile_end {
                cb(tick, tag, chunks_processed, chunks_queued);
            }
        }
    }

    pub fn fire_metrics(event: &MetricsEvent) {
        if let Ok(reg) = registry().lock() {
            for cb in &reg.on_metrics {
                cb(event);
            }
        }
    }
}