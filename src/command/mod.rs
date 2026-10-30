use crate::{
    RZeroConfig,
    anchor::{AnchorMode, AnchorSelector, RZeroAnchorSettings},
    checkpoint::CheckpointManager,
    runtime::{RZeroRuntime, SnapshotRegistry},
};

pub struct CommandRZeroBase;

impl CommandRZeroBase {
    pub fn execute(
        command: &str,
        requester: Option<u128>,
        current_dim: &str,
        current_tick: i64,
    ) -> Option<Vec<String>> {
        let trimmed = command.trim().trim_start_matches('/');
        let words: Vec<&str> = trimmed.split_whitespace().collect();
        if words.first().copied() != Some("rzero") {
            return None;
        }

        let sub = words.get(1).copied().unwrap_or("status");
        let reply = match sub {
            "set" | "save" => {
                CheckpointManager::set_checkpoint(current_dim, requester, current_tick);
                "[RZero] Checkpoint created successfully.".to_owned()
            }
            "load" | "rollback" => {
                if CheckpointManager::restore_checkpoint(requester, current_tick) {
                    format!("[RZero] Timeline rolled back to checkpoint (tick={current_tick}).")
                } else {
                    "[RZero] Rollback aborted: No timeline origin found.".to_owned()
                }
            }
            "status" => Self::report_status(current_tick),
            "anchor" => Self::handle_anchor_subcommand(&words[2..], requester, current_tick),
            "autosave" => Self::handle_autosave_subcommand(&words[2..]),
            _ => "[RZero] Usage: /rzero <set|load|status|anchor|autosave>".to_owned(),
        };

        Some(vec![reply])
    }

    fn handle_anchor_subcommand(
        args: &[&str],
        requester: Option<u128>,
        current_tick: i64,
    ) -> String {
        match args.first().copied().unwrap_or("status") {
            "status" => Self::report_status(current_tick),
            "cooldown" => {
                let seconds = args
                    .get(1)
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(0)
                    .max(0);
                RZeroRuntime::with_mut(|rt| {
                    let updated = rt
                        .anchor_settings
                        .clone()
                        .with_rollback_cooldown_seconds(seconds);
                    rt.set_anchor_settings(updated);
                    rt.cooldown.reset();
                });
                RZeroConfig::save();
                if seconds <= 0 {
                    "[RZero] Rollback cooldown disabled: every key player death rewinds the world."
                        .to_owned()
                } else {
                    format!("[RZero] Rollback cooldown: {seconds} (in seconds)")
                }
            }
            "mode" => {
                let mode_name = args.get(1).copied().unwrap_or("fixed");
                let mode = AnchorMode::by_id(mode_name);
                match mode {
                    AnchorMode::Everyone => {
                        Self::apply_anchor_update(|s| s.with_mode(AnchorMode::Everyone), current_tick);
                        "[RZero] Key player mode: All Players".to_owned()
                    }
                    AnchorMode::Rotating => {
                        let seconds = args
                            .get(2)
                            .and_then(|s| s.parse::<i32>().ok())
                            .unwrap_or(300)
                            .max(RZeroAnchorSettings::MIN_ROTATION_SECONDS);
                        Self::apply_anchor_update(
                            |s| s.with_mode(AnchorMode::Rotating).with_rotation_seconds(seconds),
                            current_tick,
                        );
                        format!("[RZero] Key player mode: Rotation ({seconds} in seconds)")
                    }
                    AnchorMode::Multi => {
                        let pinned: Vec<u128> = requester.into_iter().collect();
                        let count = pinned.len();
                        Self::apply_anchor_update(
                            |s| s.with_mode(AnchorMode::Multi).with_pinned(pinned),
                            current_tick,
                        );
                        format!("[RZero] Key player mode: Multiple Key Players ({count} pinned)")
                    }
                    AnchorMode::Fixed => {
                        let pinned: Vec<u128> = requester.into_iter().collect();
                        Self::apply_anchor_update(
                            |s| s.with_mode(AnchorMode::Fixed).with_pinned(pinned),
                            current_tick,
                        );
                        "[RZero] Key player mode: Single Player".to_owned()
                    }
                }
            }
            _ => Self::report_status(current_tick),
        }
    }

    fn handle_autosave_subcommand(args: &[&str]) -> String {
        match args.first().copied().unwrap_or("status") {
            "on" | "interval" => {
                let sec = args.get(1).and_then(|s| s.parse::<i32>().ok());
                RZeroRuntime::with_mut(|rt| {
                    rt.game_rules.auto_save = true;
                    rt.game_rules.use_adaptive_mode = false;
                    if let Some(s) = sec {
                        rt.game_rules.fixed_interval = s.max(5);
                    }
                });
                "[RZero] Interval auto-save enabled.".to_owned()
            }
            "adaptive" => {
                let sec = args.get(1).and_then(|s| s.parse::<i32>().ok());
                RZeroRuntime::with_mut(|rt| {
                    rt.game_rules.auto_save = true;
                    rt.game_rules.use_adaptive_mode = true;
                    if let Some(s) = sec {
                        rt.game_rules.adaptive_interval = s.max(60);
                    }
                });
                "[RZero] Adaptive auto-save enabled.".to_owned()
            }
            "off" => {
                RZeroRuntime::with_mut(|rt| {
                    rt.game_rules.auto_save = false;
                });
                "[RZero] Auto-save disabled.".to_owned()
            }
            _ => RZeroRuntime::with(|rt| {
                format!(
                    "[RZero] Auto-save: {} | adaptive: {} | interval: {}s",
                    rt.game_rules.auto_save,
                    rt.game_rules.use_adaptive_mode,
                    rt.game_rules.fixed_interval
                )
            }),
        }
    }

    fn apply_anchor_update(
        f: impl FnOnce(RZeroAnchorSettings) -> RZeroAnchorSettings,
        current_tick: i64,
    ) {
        RZeroRuntime::with_mut(|rt| {
            let updated = f(rt.anchor_settings.clone());
            rt.set_anchor_settings(updated);
        });
        RZeroConfig::save();
        SnapshotRegistry::sync_active_snapshot_anchors(current_tick);
    }

    fn report_status(current_tick: i64) -> String {
        let (settings, online, rem_ticks) = RZeroRuntime::with(|rt| {
            let rem = rt.cooldown.remaining_ticks(
                rt.anchor_settings.rollback_cooldown_seconds,
                current_tick,
            );
            (
                rt.anchor_settings.clone(),
                rt.online_players.clone(),
                rem,
            )
        });
        let anchors = AnchorSelector::resolve_anchors(&settings, &online, current_tick, None);
        let cd_text = if settings.rollback_cooldown_seconds <= 0 {
            "disabled".to_owned()
        } else {
            format!(
                "{} s (ready in {} s)",
                settings.rollback_cooldown_seconds,
                rem_ticks / 20
            )
        };
        format!(
            "[RZero] Key player mode: {} | rollback cooldown: {} | active key players: {} | has_checkpoint: {}",
            settings.mode.id(),
            cd_text,
            anchors.len(),
            SnapshotRegistry::has_checkpoint()
        )
    }
}