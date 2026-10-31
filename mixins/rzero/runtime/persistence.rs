use super::super::{Dim, Game};
use super::snapshots::*;
use super::types::*;
use horizon_client::{
    cooldowns::Cooldowns,
    items::ItemStack,
    recipe_book::Settings as RecipeBookSettings,
    save::{item_from_nbt, read_dat, Level},
    world::{HealthRules, Weather},
    worldborder::WorldBorder,
};
use horizon_core::geometry::Vec3;
use horizon_world::nbt::Tag;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

pub fn slot_dir(world_root: &Path, slot: &str) -> PathBuf {
    let clean = slot.trim();
    if clean.is_empty() || clean == "default" {
        world_root.join("rzero").join("checkpoint")
    } else {
        world_root.join("rzero").join("slots").join(clean)
    }
}

pub fn checkpoint_dir(world_root: &Path) -> PathBuf {
    slot_dir(world_root, "default")
}

pub fn has_disk_checkpoint(world_root: &Path) -> bool {
    has_disk_slot(world_root, "default")
}

pub fn has_disk_slot(world_root: &Path, slot: &str) -> bool {
    slot_dir(world_root, slot).join("meta.json").is_file()
}

pub fn copy_dir_recursive(src: &Path, dst: &Path) {
    if !src.is_dir() {
        return;
    }
    let _ = fs::create_dir_all(dst);
    if let Ok(entries) = fs::read_dir(src) {
        for entry in entries.flatten() {
            let p = entry.path();
            let target = dst.join(entry.file_name());
            if p.is_dir() {
                copy_dir_recursive(&p, &target);
            } else if p.is_file() {
                let _ = fs::copy(&p, &target);
            }
        }
    }
}

pub fn collect_files_recursive(
    dir: &Path,
    map: &mut HashMap<PathBuf, Vec<u8>>,
    target_base: &Path,
    rel_prefix: &Path,
) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            let fname = entry.file_name();
            let next_rel = rel_prefix.join(fname);
            if p.is_dir() {
                collect_files_recursive(&p, map, target_base, &next_rel);
            } else if p.is_file() {
                if let Ok(bytes) = fs::read(&p) {
                    map.insert(target_base.join(&next_rel), bytes);
                }
            }
        }
    }
}

pub fn save_slot_to_disk(world_root: &Path, slot: &str, snap: &TimelineSnapshot) -> bool {
    let cp_dir = slot_dir(world_root, slot);
    if fs::create_dir_all(&cp_dir).is_err() {
        return false;
    }

    // 1. Копируем level.dat
    let level_dat = world_root.join("level.dat");
    if level_dat.is_file() {
        let _ = fs::copy(&level_dat, cp_dir.join("level.dat"));
    }

    // 2. Копируем папку players
    let players_dir = world_root.join("players");
    let cp_players = cp_dir.join("players");
    copy_dir_recursive(&players_dir, &cp_players);

    // 3. Копируем папку data
    let data_dir = world_root.join("data");
    let cp_data = cp_dir.join("data");
    copy_dir_recursive(&data_dir, &cp_data);

    // 3.5. Копируем файлы измерений и регионов
    let dims_dir = world_root.join("dimensions");
    let cp_dims = cp_dir.join("dimensions");
    copy_dir_recursive(&dims_dir, &cp_dims);

    // 4. Сохраняем метаданные гостей и чекпоинта
    let mut guests_json = Vec::new();
    for g in snap.guests.values() {
        guests_json.push(serde_json::json!({
            "uuid": format!("{:032x}", g.uuid),
            "dimension": g.dimension,
            "position": g.position,
            "velocity": [g.velocity.x, g.velocity.y, g.velocity.z],
            "yaw": g.yaw,
            "pitch": g.pitch,
            "head_yaw": g.head_yaw,
            "on_ground": g.on_ground,
            "game_mode": g.game_mode,
            "flying": g.flying,
            "sprinting": g.sprinting,
            "sneaking": g.sneaking,
            "fall_flying": g.fall_flying,
            "health": g.health,
            "absorption": g.absorption,
            "food": g.food,
            "saturation": g.saturation,
            "exhaustion": g.exhaustion,
            "fire_ticks": g.fire_ticks,
            "air": g.air,
            "ticks_frozen": g.ticks_frozen,
            "fall_distance": g.fall_distance,
            "xp_level": g.xp_level,
            "xp_progress": g.xp_progress,
            "xp_total": g.xp_total,
            "xp_seed": g.xp_seed,
            "selected": g.selected,
            "seen_credits": g.seen_credits,
        }));
    }

    let regions_list: Vec<String> = snap
        .region_files
        .iter()
        .filter_map(|p| {
            p.strip_prefix(world_root)
                .ok()
                .map(|rel| rel.to_string_lossy().to_string())
        })
        .collect();

    let anchors_list: Vec<String> = snap
        .anchor_ids
        .iter()
        .map(|u| format!("{:032x}", u))
        .collect();

    let meta = serde_json::json!({
        "slot": slot,
        "saved_at_unix": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
        "game_time": snap.game_time,
        "day_time": snap.day_time,
        "difficulty": snap.difficulty,
        "difficulty_locked": snap.difficulty_locked,
        "world_spawn": snap.world_spawn,
        "world_spawn_angle": snap.world_spawn_angle,
        "next_entity": snap.next_entity,
        "policy": snap.policy.to_json(),
        "anchor_ids": anchors_list,
        "region_files": regions_list,
        "guests": guests_json,
    });

    if let Ok(json_str) = serde_json::to_string_pretty(&meta) {
        let _ = fs::write(cp_dir.join("meta.json"), json_str);
    }

    true
}

pub fn save_checkpoint_to_disk(world_root: &Path, snap: &TimelineSnapshot) -> bool {
    save_slot_to_disk(world_root, "default", snap)
}

pub fn restore_disk_files_for_slot(world_root: &Path, slot: &str) {
    let cp_dir = slot_dir(world_root, slot);
    let cp_players = cp_dir.join("players");
    let target_players = world_root.join("players");
    if cp_players.is_dir() {
        copy_dir_recursive(&cp_players, &target_players);
    }
    let cp_data = cp_dir.join("data");
    let target_data = world_root.join("data");
    if cp_data.is_dir() {
        copy_dir_recursive(&cp_data, &target_data);
    }
    let cp_dims = cp_dir.join("dimensions");
    let target_dims = world_root.join("dimensions");
    if cp_dims.is_dir() {
        copy_dir_recursive(&cp_dims, &target_dims);
    }
}

pub fn restore_disk_files_on_rollback(world_root: &Path) {
    restore_disk_files_for_slot(world_root, "default");
}

pub fn load_slot_from_disk(world_root: &Path, slot: &str) -> Option<TimelineSnapshot> {
    let cp_dir = slot_dir(world_root, slot);
    let meta_file = cp_dir.join("meta.json");
    let meta_str = fs::read_to_string(meta_file).ok()?;
    let meta: serde_json::Value = serde_json::from_str(&meta_str).ok()?;

    let game_time = meta["game_time"].as_i64()?;
    let day_time = meta["day_time"].as_i64().unwrap_or(game_time);
    let difficulty = meta["difficulty"].as_u64().unwrap_or(2) as u8;
    let difficulty_locked = meta["difficulty_locked"].as_bool().unwrap_or(false);
    let next_entity = meta["next_entity"].as_u64().unwrap_or(100) as u32;

    let world_spawn = meta["world_spawn"].as_array().and_then(|arr| {
        if arr.len() == 3 {
            Some([
                arr[0].as_i64()? as i32,
                arr[1].as_i64()? as i32,
                arr[2].as_i64()? as i32,
            ])
        } else {
            None
        }
    });

    let world_spawn_angle = meta["world_spawn_angle"].as_array().map_or([0.0, 0.0], |arr| {
        if arr.len() == 2 {
            [
                arr[0].as_f64().unwrap_or(0.0) as f32,
                arr[1].as_f64().unwrap_or(0.0) as f32,
            ]
        } else {
            [0.0, 0.0]
        }
    });

    let mut anchor_ids = HashSet::new();
    if let Some(arr) = meta["anchor_ids"].as_array() {
        for v in arr {
            if let Some(s) = v.as_str() {
                if let Ok(u) = u128::from_str_radix(s, 16) {
                    anchor_ids.insert(u);
                }
            }
        }
    }

    let mut region_files = HashSet::new();
    if let Some(arr) = meta["region_files"].as_array() {
        for v in arr {
            if let Some(s) = v.as_str() {
                region_files.insert(world_root.join(s));
            }
        }
    }

    let mut offline_player_data = HashMap::new();
    let cp_players = cp_dir.join("players");
    collect_files_recursive(&cp_players, &mut offline_player_data, &world_root.join("players"), Path::new(""));

    let mut guests = HashMap::new();
    if let Some(g_arr) = meta["guests"].as_array() {
        for g_val in g_arr {
            let uuid_str = g_val["uuid"].as_str().unwrap_or("");
            let uuid = u128::from_str_radix(uuid_str, 16).unwrap_or(0);
            if uuid == 0 {
                continue;
            }

            let pos = g_val["position"].as_array().map_or([0.0, 65.0, 0.0], |arr| {
                [
                    arr.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0),
                    arr.get(1).and_then(|v| v.as_f64()).unwrap_or(65.0),
                    arr.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0),
                ]
            });

            let vel = g_val["velocity"].as_array().map_or(Vec3::ZERO, |arr| {
                Vec3::new(
                    arr.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0),
                    arr.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0),
                    arr.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0),
                )
            });

            let player_dat_hex = cp_players.join("data").join(format!("{:032x}.dat", uuid));
            let player_dat_std = cp_players.join("data").join(format!("{}.dat", format_uuid_std(uuid)));
            let player_dat = if player_dat_hex.is_file() {
                player_dat_hex
            } else {
                player_dat_std
            };
            let player_tag = read_dat(&player_dat);

            let mut guest_snap = GuestSnapshot {
                uuid,
                dimension: g_val["dimension"]
                    .as_str()
                    .unwrap_or("minecraft:overworld")
                    .to_owned(),
                position: pos,
                velocity: vel,
                yaw: g_val["yaw"].as_f64().unwrap_or(0.0) as f32,
                pitch: g_val["pitch"].as_f64().unwrap_or(0.0) as f32,
                head_yaw: g_val["head_yaw"].as_f64().unwrap_or(0.0) as f32,
                on_ground: g_val["on_ground"].as_bool().unwrap_or(true),
                game_mode: g_val["game_mode"].as_i64().unwrap_or(0) as i32,
                flying: g_val["flying"].as_bool().unwrap_or(false),
                sprinting: g_val["sprinting"].as_bool().unwrap_or(false),
                sneaking: g_val["sneaking"].as_bool().unwrap_or(false),
                fall_flying: g_val["fall_flying"].as_bool().unwrap_or(false),
                sleeping: false,
                bed: None,
                sleep_timer: 0,
                vehicle: None,
                vehicle_good: None,
                spawn: None,
                respawn_elsewhere: None,
                last_death: None,
                health: g_val["health"].as_f64().unwrap_or(20.0) as f32,
                absorption: g_val["absorption"].as_f64().unwrap_or(0.0) as f32,
                food: g_val["food"].as_i64().unwrap_or(20) as i32,
                saturation: g_val["saturation"].as_f64().unwrap_or(5.0) as f32,
                exhaustion: g_val["exhaustion"].as_f64().unwrap_or(0.0) as f32,
                fire_ticks: g_val["fire_ticks"].as_i64().unwrap_or(-20) as i32,
                air: g_val["air"].as_i64().unwrap_or(300) as i32,
                ticks_frozen: g_val["ticks_frozen"].as_i64().unwrap_or(0) as i32,
                fall_distance: g_val["fall_distance"].as_f64().unwrap_or(0.0),
                xp_level: g_val["xp_level"].as_i64().unwrap_or(0) as i32,
                xp_progress: g_val["xp_progress"].as_f64().unwrap_or(0.0) as f32,
                xp_total: g_val["xp_total"].as_i64().unwrap_or(0) as i32,
                xp_seed: g_val["xp_seed"].as_i64().unwrap_or(0) as i32,
                inventory: vec![None; 36],
                armor: [None, None, None, None],
                offhand: None,
                carried: None,
                ender_items: vec![None; 27],
                selected: g_val["selected"].as_u64().unwrap_or(0) as usize,
                craft_grid: [None, None, None, None],
                craft_result: None,
                effects: Vec::new(),
                shield_cooldown: 0,
                cooldowns: Cooldowns::default(),
                recipe_book: RecipeBookSettings::default(),
                advancements: horizon_client::advancements::Progress::default(),
                stats: BTreeMap::new(),
                seen_credits: g_val["seen_credits"].as_bool().unwrap_or(false),
                saved: player_tag.clone(),
                hook: None,
            };

            if let Some(tag) = &player_tag {
                if let Some(Tag::List(items)) = tag.get("Inventory") {
                    for it in items {
                        let slot = it
                            .get("Slot")
                            .and_then(|v| match v {
                                Tag::Byte(b) => Some(*b as i16),
                                Tag::Short(s) => Some(*s),
                                Tag::Int(i) => Some(*i as i16),
                                _ => None,
                            })
                            .unwrap_or(0);
                        if let Some(stack) = item_from_nbt(it) {
                            match slot {
                                0..=35 => guest_snap.inventory[slot as usize] = Some(stack),
                                100 => guest_snap.armor[3] = Some(stack),
                                101 => guest_snap.armor[2] = Some(stack),
                                102 => guest_snap.armor[1] = Some(stack),
                                103 => guest_snap.armor[0] = Some(stack),
                                -106 | 150 => guest_snap.offhand = Some(stack),
                                _ => {}
                            }
                        }
                    }
                }
                if let Some(Tag::List(ender)) = tag.get("EnderItems") {
                    for it in ender {
                        let slot = it
                            .get("Slot")
                            .and_then(|v| match v {
                                Tag::Byte(b) => Some(*b as usize),
                                Tag::Short(s) => Some(*s as usize),
                                Tag::Int(i) => Some(*i as usize),
                                _ => None,
                            })
                            .unwrap_or(0);
                        if slot < guest_snap.ender_items.len() {
                            if let Some(stack) = item_from_nbt(it) {
                                guest_snap.ender_items[slot] = Some(stack);
                            }
                        }
                    }
                }
            }

            guests.insert(uuid, guest_snap);
        }
    }

    Some(TimelineSnapshot {
        anchor_ids,
        policy: RollbackPolicy::default(),
        game_time,
        day_time,
        weather: Weather::default(),
        health_rules: HealthRules::default(),
        rules: horizon_client::gamerules::GameRules::default(),
        border: WorldBorder::default(),
        bossbars: Tag::Compound(Default::default()),
        sculk_tracker: horizon_client::sculk::WardenSpawnTracker::default(),
        sculk_darkness: (0.0, 0.0),
        dragon_fight_dat: None,
        trader_dat: None,
        rand_sequences: horizon_client::random_sequences::RandomSequences::default(),
        region_files,
        difficulty,
        difficulty_locked,
        world_spawn,
        world_spawn_angle,
        next_entity,
        scoreboard: horizon_client::scoreboard::Scoreboard::default(),
        guests,
        offline_player_data,
        levels: HashMap::new(),
        mob_players: HashMap::new(),
    })
}

pub fn load_checkpoint_from_disk(world_root: &Path) -> Option<TimelineSnapshot> {
    load_slot_from_disk(world_root, "default")
}

pub fn list_disk_slots(world_root: &Path) -> Vec<serde_json::Value> {
    let mut list = Vec::new();
    // 1. Дефолтный слот
    let def_meta = checkpoint_dir(world_root).join("meta.json");
    if let Ok(s) = fs::read_to_string(&def_meta) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&s) {
            list.push(serde_json::json!({
                "slot": "default",
                "game_time": val.get("game_time"),
                "saved_at_unix": val.get("saved_at_unix"),
                "guests_count": val.get("guests").and_then(|g| g.as_array()).map(|a| a.len()).unwrap_or(0)
            }));
        }
    }

    // 2. Именованные слоты
    let slots_base = world_root.join("rzero").join("slots");
    if let Ok(entries) = fs::read_dir(slots_base) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                let meta_f = p.join("meta.json");
                if let Ok(s) = fs::read_to_string(&meta_f) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&s) {
                        list.push(serde_json::json!({
                            "slot": name,
                            "game_time": val.get("game_time"),
                            "saved_at_unix": val.get("saved_at_unix"),
                            "guests_count": val.get("guests").and_then(|g| g.as_array()).map(|a| a.len()).unwrap_or(0)
                        }));
                    }
                }
            }
        }
    }

    list
}

pub fn delete_disk_slot(world_root: &Path, slot: &str) -> bool {
    let dir = slot_dir(world_root, slot);
    if dir.exists() {
        fs::remove_dir_all(dir).is_ok()
    } else {
        false
    }
}

fn format_uuid_std(u: u128) -> String {
    let hex = format!("{:032x}", u);
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}
