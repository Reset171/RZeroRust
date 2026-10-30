pub mod escape;
pub mod safety;
pub mod threat;

pub use escape::{EscapeSpaceAnalyzer, EscapeSpaceResult};
pub use safety::PlayerSafetyCheck;
pub use threat::{MobProbe, ThreatAssessor};

use crate::{
    checkpoint::data::AdaptiveState,
    engine::DeterministicRandomSource,
    log_info,
    metrics::{MetricsWriter, PlayerMetrics},
    runtime::{RZeroRuntime, RestoreQueues},
};

pub struct AdaptiveSaveEngine;

impl AdaptiveSaveEngine {
    const DRAMATIC_PEAK: f64 = 35.0;

    pub fn capture_adaptive_state() -> AdaptiveState {
        RZeroRuntime::with(|rt| rt.adaptive_state)
    }

    pub fn restore_adaptive_state(state: Option<AdaptiveState>) {
        let Some(as_state) = state else {
            return;
        };
        RZeroRuntime::with_mut(|rt| {
            rt.adaptive_state = as_state;
        });
        RestoreQueues::clear_pending_death_rollback();
    }

    pub fn reset_auto_save_timer(current_game_time: i64, random_seed: i64) {
        let rules = RZeroRuntime::with(|rt| rt.game_rules.clone());
        let interval = Self::compute_interval(&rules, random_seed);
        RZeroRuntime::with_mut(|rt| {
            rt.adaptive_state.last_save_tick = current_game_time;
            rt.adaptive_state.current_interval = interval;
        });
        log_info("Auto-save timer reset after rollback.");
    }

    pub fn compute_interval(rules: &crate::ModGameRules, seed: i64) -> i64 {
        if rules.use_random_interval {
            let min = rules.random_min.max(5);
            let max = rules.random_max.max(min + 1);
            let mut rng = DeterministicRandomSource::new(seed);
            let extra = rng.next_int_bound((max - min) + 1);
            i64::from(min + extra) * 20
        } else {
            let base = rules.fixed_interval.max(5);
            i64::from(base) * 20
        }
    }

    pub fn on_checkpoint_created(anchors: &[u128], current_tick: i64, entity_id_counter: i32) {
        RZeroRuntime::with_mut(|rt| {
            rt.adaptive_state.last_save_tick = current_tick;
            rt.adaptive_state.current_intensity = 0.0;
            rt.adaptive_state.peak_intensity = 0.0;
            rt.adaptive_state.relax_ticks = 0;
            rt.adaptive_state.entity_id_counter = entity_id_counter;
        });
        for &anchor in anchors {
            PlayerMetrics::set_last_respawn_or_checkpoint_tick(anchor, current_tick);
        }
    }

    pub fn evaluate_adaptive_tick(
        most_threatened_anchor: Option<u128>,
        max_instant_threat: f64,
        current_tick: i64,
        random_float: f32,
    ) -> bool {
        if max_instant_threat < 0.0 {
            RZeroRuntime::with_mut(|rt| {
                rt.adaptive_state.relax_ticks = 0;
            });
            return false;
        }

        let (adaptive_cfg, tranquil_sec, mut state) = RZeroRuntime::with(|rt| {
            (
                rt.settings.adaptive,
                rt.game_rules.adaptive_interval,
                rt.adaptive_state,
            )
        });

        let time_since_last_save = current_tick - state.last_save_tick;
        if max_instant_threat > state.current_intensity {
            state.current_intensity += (max_instant_threat - state.current_intensity) * 0.2;
            state.relax_ticks = 0;
        } else {
            state.current_intensity -= (state.current_intensity - max_instant_threat) * 0.02;
        }
        if state.current_intensity > state.peak_intensity {
            state.peak_intensity = state.current_intensity;
        }

        let relax_ticks_required = (adaptive_cfg.relax_time_seconds * 20).max(20);
        let min_save_gap_ticks = (i64::from(adaptive_cfg.min_save_gap_seconds) * 20).max(100);
        let save_chance = adaptive_cfg.post_combat_save_percent as f32 / 100.0;

        let mut should_save = false;
        if state.peak_intensity > Self::DRAMATIC_PEAK {
            if state.current_intensity < state.peak_intensity * 0.5 && max_instant_threat <= 20.0 {
                state.relax_ticks += 1;
                if state.relax_ticks > relax_ticks_required {
                    if most_threatened_anchor.is_some() && random_float < save_chance {
                        should_save = true;
                    } else {
                        state.peak_intensity = 0.0;
                        state.current_intensity = 0.0;
                        state.relax_ticks = 0;
                    }
                }
            } else {
                state.relax_ticks = 0;
            }
        } else {
            let tranquil_interval_ticks = i64::from(tranquil_sec.max(60)) * 20;
            if time_since_last_save >= tranquil_interval_ticks && max_instant_threat <= 10.0 {
                should_save = true;
            }
        }

        RZeroRuntime::with_mut(|rt| {
            rt.adaptive_state = state;
        });

        if should_save
            && time_since_last_save > min_save_gap_ticks
            && let Some(anchor) = most_threatened_anchor
        {
            MetricsWriter::write(
                anchor,
                max_instant_threat,
                0,
                0,
                0,
                0.0,
                0,
                "CHECKPOINT_CREATED",
                0,
            );
            return true;
        }
        false
    }
}