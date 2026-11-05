use super::super::{
    native_hooks::NativeMod,
    Game,
};
use super::types::*;
use std::{
    fs,
    path::Path,
};

pub fn register_native_mod(mods: &mut Vec<NativeMod>) {
    mods.push(NativeMod {
        id: MOD_ID.to_owned(),
        title: "RZero 0.1.0".to_owned(),
        description: tr("mod.rzero.description"),
        keys: vec![
            [
                "save".to_owned(),
                tr("key.rzero.save"),
                "key.keyboard.f6".to_owned(),
            ],
            [
                "load".to_owned(),
                tr("key.rzero.load"),
                "key.keyboard.f7".to_owned(),
            ],
            [
                "anchor_menu".to_owned(),
                tr("key.rzero.anchor_menu"),
                "key.keyboard.f8".to_owned(),
            ],
        ],
    });
}

pub fn populate_settings(game: &mut Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    if mod_id != MOD_ID {
        return;
    }
    load_saved_settings(game);
    let on = tr("ui.rzero.on");
    let off = tr("ui.rzero.off");
    with_state(|st| {
        buttons.push(tr_args(
            "ui.rzero.auto_save",
            &[if st.auto_save { &on } else { &off }],
        ));
        buttons.push(tr_args(
            "ui.rzero.adaptive_save",
            &[if st.adaptive_mode { &on } else { &off }],
        ));
        buttons.push(tr_args(
            "ui.rzero.interval",
            &[&st.fixed_interval_sec.to_string()],
        ));
        buttons.push(tr_args(
            "ui.rzero.anchor_mode",
            &[&tr(st.anchor.mode.lang_key())],
        ));
        let cd = st.anchor.rollback_cooldown_seconds;
        if cd <= 0 {
            buttons.push(tr("ui.rzero.cooldown_off"));
        } else {
            buttons.push(tr_args("ui.rzero.cooldown", &[&cd.to_string()]));
        }
        buttons.push(tr_args(
            "ui.rzero.boss_save",
            &[if st.save_on_boss_defeat { &on } else { &off }],
        ));
        buttons.push(tr_args("ui.rzero.logs", &[if st.logs { &on } else { &off }]));
        buttons.push(tr_args(
            "ui.rzero.f3_hitboxes",
            &[if st.f3_loop_hitboxes { &on } else { &off }],
        ));
    });
}

pub fn handle_settings_click(game: &mut Game<'_>, mod_id: &str, button: usize) {
    if mod_id != MOD_ID {
        return;
    }
    with_state_mut(|st| match button {
        0 => st.auto_save = !st.auto_save,
        1 => st.adaptive_mode = !st.adaptive_mode,
        2 => {
            st.fixed_interval_sec = match st.fixed_interval_sec {
                60 => 180,
                180 => 300,
                300 => 600,
                600 => 1200,
                _ => 60,
            };
            st.current_interval_ticks = i64::from(st.fixed_interval_sec) * 20;
        }
        3 => st.anchor.mode = st.anchor.mode.next(),
        4 => {
            st.anchor.rollback_cooldown_seconds = match st.anchor.rollback_cooldown_seconds {
                0 => 10,
                10 => 30,
                30 => 60,
                60 => 300,
                _ => 0,
            };
            st.last_rollback_tick = None;
        }
        5 => st.save_on_boss_defeat = !st.save_on_boss_defeat,
        6 => st.logs = !st.logs,
        7 => st.f3_loop_hitboxes = !st.f3_loop_hitboxes,
        _ => {}
    });
    save_current_settings(game);
}

pub fn load_saved_settings(game: &Game<'_>) {
    let auto_save = game
        .native_setting(MOD_ID, "auto_save")
        .map(|v| v != "false");
    let adaptive = game
        .native_setting(MOD_ID, "adaptive_mode")
        .map(|v| v != "false");
    let interval = game
        .native_setting(MOD_ID, "fixed_interval_sec")
        .and_then(|v| v.parse::<i32>().ok());
    let mode = game
        .native_setting(MOD_ID, "anchor_mode")
        .map(|v| AnchorMode::by_id(&v));
    let cooldown = game
        .native_setting(MOD_ID, "rollback_cooldown_sec")
        .and_then(|v| v.parse::<i32>().ok());
    let boss_save = game
        .native_setting(MOD_ID, "save_on_boss_defeat")
        .map(|v| v != "false");
    let logs = game.native_setting(MOD_ID, "logs").map(|v| v == "true");
    let f3_hitboxes = game
        .native_setting(MOD_ID, "f3_loop_hitboxes")
        .map(|v| v != "false");

    let json_cfg = fs::read_to_string(Path::new("config").join("rzero.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());

    with_state_mut(|st| {
        if let Some(v) = &json_cfg {
            let b = |val: &serde_json::Value, d: bool| val.as_bool().unwrap_or(d);
            let rb = &v["checkpointPolicy"]["rollback"];
            if rb.is_object() {
                st.policy.blocks = b(&rb["blocks"], true);
                st.policy.biomes = b(&rb["biomes"], true);
                st.policy.block_entities = b(&rb["block_entities"], true);
                st.policy.block_ticks = b(&rb["block_ticks"], true);
                st.policy.fluid_ticks = b(&rb["fluid_ticks"], true);
                st.policy.block_events = b(&rb["block_events"], true);
                let ent = &rb["entities"];
                if ent.is_object() {
                    st.policy.entities_presence = b(&ent["presence"], true);
                    st.policy.dropped_items = b(&ent["dropped_items"], true);
                    st.policy.experience_orbs = b(&ent["experience_orbs"], true);
                }
                let pl = &rb["players"];
                if pl.is_object() {
                    st.policy.player_position = b(&pl["position"], true);
                    st.policy.player_motion = b(&pl["motion"], true);
                    st.policy.player_health = b(&pl["health"], true);
                    st.policy.player_hunger = b(&pl["hunger"], true);
                    st.policy.player_experience = b(&pl["experience"], true);
                    st.policy.player_inventory = b(&pl["inventory"], true);
                    st.policy.player_ender_chest = b(&pl["ender_chest"], true);
                    st.policy.player_potion_effects = b(&pl["potion_effects"], true);
                    st.policy.player_cooldowns = b(&pl["cooldowns"], true);
                    st.policy.player_abilities = b(&pl["abilities"], true);
                    st.policy.player_recipe_book = b(&pl["recipe_book"], true);
                    st.policy.player_spawn_point = b(&pl["spawn_point"], true);
                    st.policy.player_stats = b(&pl["stats"], true);
                    st.policy.preserve_new_player_inventory =
                        b(&pl["preserve_new_player_inventory"], false);
                }
                let w = &rb["world"];
                if w.is_object() {
                    st.policy.world_time = b(&w["time"], true);
                    st.policy.world_weather = b(&w["weather"], true);
                    st.policy.world_raids = b(&w["raids"], true);
                    st.policy.world_dragon_fight = b(&w["dragon_fight"], true);
                    st.policy.world_scoreboard = b(&w["scoreboard"], true);
                    st.policy.world_level_rng = b(&w["level_rng"], true);
                    st.policy.world_difficulty = b(&w["difficulty"], true);
                    st.policy.world_spawn = b(&w["world_spawn"], true);
                    let sg = &w["server_globals"];
                    if sg.is_object() {
                        st.policy.server_bossbars = b(&sg["bossbars"], true);
                        st.policy.server_forced_chunks = b(&sg["forced_chunks"], true);
                        st.policy.server_world_border = b(&sg["world_border"], true);
                        st.policy.server_game_rules = b(&sg["game_rules"], true);
                    }
                }
            }
            let ad = &v["adaptive"];
            if ad.is_object() {
                if let Some(gap) = ad["min_save_gap_seconds"].as_i64() {
                    st.min_save_gap_sec = (gap as i32).max(5);
                }
                if let Some(rel) = ad["relax_time_seconds"].as_i64() {
                    st.relax_time_sec = (rel as i32).max(1);
                }
                if let Some(pct) = ad["post_combat_save_percent"].as_i64() {
                    st.post_combat_save_pct = (pct as i32).clamp(0, 100);
                }
                st.save_on_boss_defeat = b(&ad["save_on_boss_defeat"], st.save_on_boss_defeat);
            }
        }
        if let Some(v) = auto_save {
            st.auto_save = v;
        }
        if let Some(v) = adaptive {
            st.adaptive_mode = v;
        }
        if let Some(v) = interval {
            st.fixed_interval_sec = v.max(5);
            st.current_interval_ticks = i64::from(st.fixed_interval_sec) * 20;
        }
        if let Some(v) = mode {
            st.anchor.mode = v;
        }
        if let Some(v) = cooldown {
            st.anchor.rollback_cooldown_seconds = v.max(0);
        }
        if let Some(v) = boss_save {
            st.save_on_boss_defeat = v;
        }
        if let Some(v) = logs {
            st.logs = v;
        }
        if let Some(v) = f3_hitboxes {
            st.f3_loop_hitboxes = v;
        }
    });
}

pub fn save_current_settings(game: &Game<'_>) {
    with_state(|st| {
        game.native_set_setting(
            MOD_ID,
            "auto_save",
            if st.auto_save { "true" } else { "false" },
        );
        game.native_set_setting(
            MOD_ID,
            "adaptive_mode",
            if st.adaptive_mode { "true" } else { "false" },
        );
        game.native_set_setting(
            MOD_ID,
            "fixed_interval_sec",
            &st.fixed_interval_sec.to_string(),
        );
        game.native_set_setting(MOD_ID, "anchor_mode", st.anchor.mode.id());
        game.native_set_setting(
            MOD_ID,
            "rollback_cooldown_sec",
            &st.anchor.rollback_cooldown_seconds.to_string(),
        );
        game.native_set_setting(
            MOD_ID,
            "save_on_boss_defeat",
            if st.save_on_boss_defeat {
                "true"
            } else {
                "false"
            },
        );
        game.native_set_setting(MOD_ID, "logs", if st.logs { "true" } else { "false" });
        game.native_set_setting(
            MOD_ID,
            "f3_loop_hitboxes",
            if st.f3_loop_hitboxes { "true" } else { "false" },
        );
    });
}
