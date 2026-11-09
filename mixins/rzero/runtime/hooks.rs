use super::super::{
    native_lifecycle::WorldPhase,
    Game, LanHost,
};
use super::adaptive::tick_autosave_on_server;
use super::capture::create_checkpoint_on_server;
use super::persistence::{
    has_disk_checkpoint, hydrate_slot_from_disk_on_server, load_checkpoint_from_disk,
};
use super::restore::{apply_client_level_rollback, rollback_checkpoint_on_server};
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
                if !has_cheat_permission(srv, None) {
                    return tr("command.rzero.no_permission");
                }
                if create_checkpoint_on_server(srv, None) {
                    tr("command.rzero.set.success")
                } else {
                    tr("command.rzero.set.failed")
                }
            });
            if let Some(text) = msg {
                game.native_chat(&text);
            }
        } else if !has_cheat_permission(game, None) {
            game.native_chat(&tr("command.rzero.no_permission"));
        } else {
            with_state_mut(|st| st.pending_save_request = true);
        }
    } else if key == load_key {
        if let Some(server) = &game.integrated {
            let res = server.call(|srv| {
                if !has_level_4_permission(srv, None) {
                    return (false, tr("command.rzero.no_permission"));
                }
                if rollback_checkpoint_on_server(srv) {
                    (
                        true,
                        tr_args(
                            "command.rzero.load.success",
                            &[&srv.world.game_time.to_string()],
                        ),
                    )
                } else {
                    (false, tr("command.rzero.load.failed"))
                }
            });
            if let Some((did_rollback, text)) = res {
                if did_rollback {
                    game.close_container();
                    game.screen = super::super::Screen::Playing;
                    let slot = with_state(|st| st.current_slot.clone());
                    apply_client_level_rollback(game, &slot);
                }
                game.native_chat(&text);
            }
        } else if !has_level_4_permission(game, None) {
            game.native_chat(&tr("command.rzero.no_permission"));
        } else {
            game.close_container();
            game.screen = super::super::Screen::Playing;
            with_state_mut(|st| st.pending_load_request = true);
        }
    } else if key == mode_key {
        let allowed = if let Some(server) = &game.integrated {
            server
                .call(|srv| has_cheat_permission(srv, None))
                .unwrap_or(false)
        } else {
            has_cheat_permission(game, None)
        };
        if !allowed {
            game.native_chat(&tr("gui.rzero.anchor.no_permission"));
            return;
        }
        let next_mode = with_state_mut(|st| {
            st.anchor.mode = st.anchor.mode.next();
            st.anchor.mode
        });
        save_current_settings(game);
        let mode_label = tr(next_mode.lang_key());
        game.native_chat(&tr_args("command.rzero.anchor.mode", &[&mode_label]));
    }
}

pub fn handle_lifecycle(game: &mut Game<'_>, phase: WorldPhase) {
    match phase {
        WorldPhase::WorldOpenReady => {
            load_saved_settings(game);
            clear_all_slots();
            clear_client_snapshots();
            let has_disk_cp = has_disk_checkpoint(&game.world_root);
            let mut restored_anchors = HashSet::new();
            let mut restored_save_tick = game.world.game_time;
            if has_disk_cp {
                if let Some(snap) = load_checkpoint_from_disk(&game.world_root) {
                    restored_anchors = snap.anchor_ids.clone();
                    restored_save_tick = snap.game_time;
                    set_snapshot(Some(Arc::new(snap)));
                }
                with_state_mut(|st| {
                    st.has_checkpoint = true;
                });
            } else {
                with_state_mut(|st| {
                    st.has_checkpoint = false;
                });
            }
            with_state_mut(|st| {
                st.current_slot = "default".to_owned();
                st.anchor_ids = restored_anchors;
                st.pending_death_rollback = None;
                st.last_save_tick = restored_save_tick;
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
            clear_all_slots();
            clear_client_snapshots();
            with_state_mut(|st| {
                st.pending_death_rollback = None;
            });
        }
    }
}

pub fn handle_tick(game: &mut Game<'_>) {
    if !game.native_on(MOD_ID) {
        return;
    }

    if game.screen == super::super::Screen::Dead {
        let has_cp = with_state(|st| st.has_checkpoint);
        if has_cp {
            game.screen = super::super::Screen::Playing;
            game.ui.death_time = 0;
            game.ui.died_at = None;
            game.ui.death_confirm = None;
            game.boss_overlay.reset();
            if let Some(p) = game.player.as_mut() {
                let _ = p.set_health(20.0, false);
                p.health.dead = false;
                p.invulnerable = false;
                p.may_fly = false;
                p.spectator = false;
                p.flying = false;
                p.fall_distance = 0.0;
                p.health.hurt.no_action_time = 0;
                p.health.hurt.cooldown = 0;
                p.health.hurt.last_hurt = 0.0;
                p.health.hurt.hurt_duration = 0;
                p.health.hurt.hurt_time = 0;
                p.health.hurt.last_source = None;
                p.health.hurt.last_stamp = 0;
                p.health.hurt_animation_time = 0;
                p.health.hurt_dir = 0.0;
                p.health.indicated_dir = None;
            }
            if let Some(server) = &game.integrated {
                server.call(|srv| {
                    rollback_checkpoint_on_server(srv);
                });
            } else {
                rollback_checkpoint_on_server(game);
            }
            let slot = with_state(|st| st.current_slot.clone());
            apply_client_level_rollback(game, &slot);
            return;
        }
    }

    if game.remote.is_some() {
        if client_epoch_needs_sync() && get_rollback_epoch() > 0 {
            let slot = with_state(|st| st.current_slot.clone());
            apply_client_level_rollback(game, &slot);
        }
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
        let slot = with_state(|st| st.current_slot.clone());
        apply_client_level_rollback(game, &slot);
        return;
    }

    if with_state(|st| st.has_checkpoint) {
        if !game.world.chunks.is_empty() {
            let active_slot = with_state(|st| st.current_slot.clone());
            let needs_hydration = get_slot_snapshot_clone(&active_slot)
                .is_none_or(|s| !s.levels.contains_key(&game.dim));
            if needs_hydration {
                let _ = hydrate_slot_from_disk_on_server(game, &active_slot);
            }
        }
        if let Ok(mut tr) = loop_tracker().write() {
            tr.record_tick(game.world.game_time, &game.mobs);
        }
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
        m.health.dead = false;
        m.invulnerable = false;
        m.may_fly = false;
        m.spectator = false;
        m.flying = false;
        m.fall_distance = 0.0;
        m.health.hurt.no_action_time = 0;
        m.health.hurt.cooldown = 0;
        m.health.hurt.last_hurt = 0.0;
        m.health.hurt.hurt_duration = 0;
        m.health.hurt.hurt_time = 0;
        m.health.hurt.last_source = None;
        m.health.hurt.last_stamp = 0;
        m.health.hurt_animation_time = 0;
        m.health.hurt_dir = 0.0;
        m.health.indicated_dir = None;
        let _ = m.set_health(20.0, false);
    }
    ControlFlow::Break(())
}
