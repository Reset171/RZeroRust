use super::super::{Game, LanHost};
use super::capture::*;
use super::persistence::*;
use super::restore::*;
use super::snapshots::*;
use super::types::*;
use super::ui::save_current_settings;
use horizon_protocol::{
    packets::play::{self, Clientbound as C},
    text::literal,
};
use std::ops::ControlFlow;

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
            if rollback_slot_checkpoint_on_server(game, slot) {
                format!("[RZero] Immediate rewind executed to slot '{}'.", slot)
            } else {
                format!("[RZero] Rewind failed: Slot '{}' not found.", slot)
            }
        }
        "trigger" => handle_trigger_subcommand(game, &words[2..], requester),
        "purge_memory" => {
            clear_snapshot();
            "[RZero] In-memory snapshots cleared.".to_owned()
        }
        "anchor" => handle_anchor_subcommand(game, &words[2..], requester),
        "autosave" => handle_autosave_subcommand(game, &words[2..]),
        "bench" | "benchmark" => handle_bench_subcommand(),
        _ => "[RZero] Usage: /rzero <set|load|json|status|slot|policy|metrics|anchor|autosave|bench>".to_owned(),
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
