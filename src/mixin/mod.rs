use crate::{
    MODID, RZeroConfig,
    anchor::{AnchorMode, AnchorSelector},
    checkpoint::CheckpointManager,
    command::CommandRZeroBase,
    event::{ServerLifecycleEvents, ServerTickEvents},
    metrics::{MetricsWriter, PlayerMetrics},
    runtime::{RZeroRuntime, RestoreQueues, SnapshotRegistry},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeModEntry {
    pub id: String,
    pub title: String,
    pub description: String,
    pub keys: Vec<[String; 3]>,
}

pub fn on_native_mods(mods: &mut Vec<NativeModEntry>) {
    mods.push(NativeModEntry {
        id: MODID.to_owned(),
        title: "RZero 0.1.0".to_owned(),
        description:
            "Return by Death. Completely rewind the timeline and the world state to your last checkpoint upon death."
                .to_owned(),
        keys: vec![
            [
                "save".to_owned(),
                "Create Checkpoint".to_owned(),
                "key.keyboard.f6".to_owned(),
            ],
            [
                "load".to_owned(),
                "Return to Checkpoint".to_owned(),
                "key.keyboard.f7".to_owned(),
            ],
            [
                "anchor_menu".to_owned(),
                "Key Player Menu".to_owned(),
                "key.keyboard.f8".to_owned(),
            ],
        ],
    });
}

pub fn on_native_settings(mod_id: &str, buttons: &mut Vec<String>) {
    if mod_id != MODID {
        return;
    }
    RZeroRuntime::with(|rt| {
        buttons.push(format!(
            "Auto-Save: {}",
            if rt.game_rules.auto_save { "ON" } else { "OFF" }
        ));
        buttons.push(format!(
            "Adaptive Save: {}",
            if rt.game_rules.use_adaptive_mode {
                "ON"
            } else {
                "OFF"
            }
        ));
        buttons.push(format!("Key Player Mode: {}", rt.anchor_settings.mode.id()));
        let cd = rt.anchor_settings.rollback_cooldown_seconds;
        buttons.push(if cd <= 0 {
            "Rollback Cooldown: Off".to_owned()
        } else {
            format!("Rollback Cooldown: {cd} s")
        });
        buttons.push(format!(
            "Chunk Cache: {}",
            if rt.rzerochash_enabled { "ON" } else { "OFF" }
        ));
        buttons.push(format!(
            "Logs: {}",
            if rt.settings.logs { "ON" } else { "OFF" }
        ));
    });
}

pub fn on_native_settings_click(mod_id: &str, button: usize, current_tick: i64) {
    if mod_id != MODID {
        return;
    }
    RZeroRuntime::with_mut(|rt| match button {
        0 => {
            rt.game_rules.auto_save = !rt.game_rules.auto_save;
        }
        1 => {
            rt.game_rules.use_adaptive_mode = !rt.game_rules.use_adaptive_mode;
        }
        2 => {
            let next_mode = match rt.anchor_settings.mode {
                AnchorMode::Fixed => AnchorMode::Multi,
                AnchorMode::Multi => AnchorMode::Everyone,
                AnchorMode::Everyone => AnchorMode::Rotating,
                AnchorMode::Rotating => AnchorMode::Fixed,
            };
            let updated = rt.anchor_settings.clone().with_mode(next_mode);
            rt.set_anchor_settings(updated);
        }
        3 => {
            let next_cd = match rt.anchor_settings.rollback_cooldown_seconds {
                0 => 10,
                10 => 30,
                30 => 60,
                60 => 300,
                _ => 0,
            };
            let updated = rt
                .anchor_settings
                .clone()
                .with_rollback_cooldown_seconds(next_cd);
            rt.set_anchor_settings(updated);
            rt.cooldown.reset();
        }
        4 => {
            let next = !rt.rzerochash_enabled;
            let updated = rt.settings.clone().with_rzerochash_enabled(next);
            rt.set_settings(updated);
        }
        5 => {
            let next = !rt.settings.logs;
            let updated = rt.settings.clone().with_logs(next);
            rt.set_settings(updated);
        }
        _ => {}
    });
    RZeroConfig::save();
    SnapshotRegistry::sync_active_snapshot_anchors(current_tick);
}

pub fn on_native_key(
    key: &str,
    pressed: bool,
    save_binding: &str,
    load_binding: &str,
    player_uuid: Option<u128>,
    current_dim: &str,
    current_tick: i64,
) -> Option<String> {
    if !pressed {
        return None;
    }
    if key == save_binding {
        trigger_manual_save(player_uuid, current_dim, current_tick);
        return Some("[RZero] Checkpoint created successfully.".to_owned());
    }
    if key == load_binding {
        if trigger_manual_load(player_uuid, current_tick) {
            return Some(format!(
                "[RZero] Timeline rolled back to checkpoint (tick={current_tick})."
            ));
        }
        return Some("[RZero] Rollback aborted: No timeline origin found.".to_owned());
    }
    None
}

pub fn on_native_command(
    command: &str,
    ran: &mut bool,
    requester: Option<u128>,
    current_dim: &str,
    current_tick: i64,
) -> Option<Vec<String>> {
    let lines = CommandRZeroBase::execute(command, requester, current_dim, current_tick)?;
    *ran = true;
    Some(lines)
}

pub fn on_native_world_lifecycle(phase_name: &str, current_tick: i64) {
    match phase_name {
        "WorldOpenReady" => ServerLifecycleEvents::on_server_started(current_tick),
        "WorldCloseFinal" | "WorldCloseFailed" | "WorldOpenFailed" => {
            ServerLifecycleEvents::on_server_stopping()
        }
        _ => {}
    }
}

pub fn on_native_tick(is_window: bool, current_tick: i64) {
    if is_window {
        on_game_tick_head(current_tick);
    } else {
        on_server_tick_post(current_tick);
    }
}

pub fn on_game_tick_head(_current_tick: i64) {
    RZeroRuntime::with_mut(|rt| {
        rt.was_restored_this_tick = false;
    });
}

pub fn on_server_tick_post(current_tick: i64) {
    ServerTickEvents::on_server_tick_post(current_tick);
}

pub fn on_guest_die_head(player_uuid: u128, server_tick: i64, game_time: i64) -> bool {
    if !SnapshotRegistry::has_checkpoint() {
        return false;
    }
    let (settings, mut online) = RZeroRuntime::with(|rt| {
        (rt.anchor_settings.clone(), rt.online_players.clone())
    });
    if !online.contains(&player_uuid) {
        online.push(player_uuid);
    }
    let is_anchor = AnchorSelector::resolve_anchors(&settings, &online, game_time, None)
        .contains(&player_uuid)
        || SnapshotRegistry::is_anchor_in_snapshots(player_uuid);
    if !is_anchor {
        return false;
    }

    let allowed = RZeroRuntime::with_mut(|rt| {
        let cd = rt.anchor_settings.rollback_cooldown_seconds;
        rt.cooldown.try_consume(cd, server_tick)
    });
    if !allowed {
        return false;
    }

    PlayerMetrics::record_death(player_uuid, server_tick);
    MetricsWriter::write(
        player_uuid,
        100.0,
        0,
        0,
        0,
        0.0,
        0,
        "DEATH_ROLLBACK",
        0,
    );
    RestoreQueues::set_pending_death_rollback(player_uuid);
    PlayerMetrics::set_last_respawn_or_checkpoint_tick(player_uuid, server_tick);
    true
}

pub fn on_owner_console_command(
    command: &str,
    owner_uuid: Option<u128>,
    current_dim: &str,
    current_tick: i64,
) -> Option<Vec<String>> {
    CommandRZeroBase::execute(command, owner_uuid, current_dim, current_tick)
}

pub fn on_guest_command(
    command: &str,
    guest_uuid: u128,
    current_dim: &str,
    current_tick: i64,
) -> Option<Vec<String>> {
    CommandRZeroBase::execute(command, Some(guest_uuid), current_dim, current_tick)
}

pub fn on_hub_respawned(_entity_id: i32) {}

pub fn trigger_manual_save(player_uuid: Option<u128>, dim: &str, current_tick: i64) {
    CheckpointManager::set_checkpoint(dim, player_uuid, current_tick);
}

pub fn trigger_manual_load(player_uuid: Option<u128>, current_tick: i64) -> bool {
    CheckpointManager::restore_checkpoint(player_uuid, current_tick)
}