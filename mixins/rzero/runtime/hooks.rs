use super::super::{
    native_lifecycle::WorldPhase,
    Game, LanHost,
};
use super::adaptive::tick_autosave_on_server;
use super::capture::create_checkpoint_on_server;
use super::restore::rollback_checkpoint_on_server;
use super::snapshots::*;
use super::types::*;
use super::ui::{load_saved_settings, save_current_settings};
use std::ops::ControlFlow;

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
