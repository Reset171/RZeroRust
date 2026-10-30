use crate::api::{DevHooks, MetricsEvent};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

#[derive(Default)]
struct MetricsStore {
    damage_taken: HashMap<u128, f32>,
    damage_dealt: HashMap<u128, f32>,
    damage_behind: HashMap<u128, f32>,
    blocks_placed: HashMap<u128, i32>,
    items_burned: HashMap<u128, i32>,
    camera_jitter: HashMap<u128, f32>,
    last_yaw: HashMap<u128, f32>,
    last_pitch: HashMap<u128, f32>,
    combat_start_tick: HashMap<u128, i64>,
    last_respawn_or_checkpoint_tick: HashMap<u128, i64>,
    death_timestamps: HashMap<u128, Vec<i64>>,
}

static STORE: OnceLock<Mutex<MetricsStore>> = OnceLock::new();

fn store() -> &'static Mutex<MetricsStore> {
    STORE.get_or_init(|| Mutex::new(MetricsStore::default()))
}

pub struct PlayerMetrics;

impl PlayerMetrics {
    pub fn add_damage_taken(player: u128, amount: f32) {
        if let Ok(mut s) = store().lock() {
            *s.damage_taken.entry(player).or_insert(0.0) += amount;
        }
    }

    pub fn add_damage_dealt(player: u128, amount: f32) {
        if let Ok(mut s) = store().lock() {
            *s.damage_dealt.entry(player).or_insert(0.0) += amount;
        }
    }

    pub fn add_damage_behind(player: u128, amount: f32) {
        if let Ok(mut s) = store().lock() {
            *s.damage_behind.entry(player).or_insert(0.0) += amount;
        }
    }

    pub fn increment_blocks_placed(player: u128) {
        if let Ok(mut s) = store().lock() {
            *s.blocks_placed.entry(player).or_insert(0) += 1;
        }
    }

    pub fn increment_items_burned(player: u128) {
        if let Ok(mut s) = store().lock() {
            *s.items_burned.entry(player).or_insert(0) += 1;
        }
    }

    pub fn accumulate_camera_jitter(player: u128, yaw: f32, pitch: f32) {
        if let Ok(mut s) = store().lock() {
            let last_y = s.last_yaw.insert(player, yaw).unwrap_or(yaw);
            let last_p = s.last_pitch.insert(player, pitch).unwrap_or(pitch);
            let jitter = (yaw - last_y).abs() + (pitch - last_p).abs();
            *s.camera_jitter.entry(player).or_insert(0.0) += jitter;
        }
    }

    pub fn track_combat_window(player: u128, tick: i64, total_threat: f64) {
        if let Ok(mut s) = store().lock() {
            if total_threat > 20.0 {
                let entry = s.combat_start_tick.entry(player).or_insert(0);
                if *entry == 0 {
                    *entry = tick;
                }
            } else {
                s.combat_start_tick.insert(player, 0);
            }
        }
    }

    pub fn set_last_respawn_or_checkpoint_tick(player: u128, tick: i64) {
        if let Ok(mut s) = store().lock() {
            s.last_respawn_or_checkpoint_tick.insert(player, tick);
        }
    }

    pub fn record_death(player: u128, tick: i64) {
        if let Ok(mut s) = store().lock() {
            s.death_timestamps.entry(player).or_default().push(tick);
        }
    }
}

pub struct MetricsWriter;

impl MetricsWriter {
    pub fn write(
        player_uuid: u128,
        threat: f64,
        mobs_track: i32,
        mobs_attack: i32,
        free_space: i32,
        cornered_pct: f32,
        cliffs_near: i32,
        event_label: &str,
        proj_air: i32,
    ) {
        DevHooks::fire_metrics(&MetricsEvent {
            player_uuid,
            threat,
            mobs_track,
            mobs_attack,
            free_space,
            cornered_pct,
            cliffs_near,
            event_label: event_label.to_owned(),
            proj_air,
        });
    }
}