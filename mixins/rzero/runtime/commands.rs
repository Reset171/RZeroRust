use super::super::{Game, LanHost};
use super::capture::*;
use super::restore::*;
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
