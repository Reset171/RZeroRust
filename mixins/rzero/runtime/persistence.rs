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

pub fn copy_dir_recursive(src: &Path, dst: &Path) -> usize {
    copy_dir_recursive_opts(src, dst, false)
}

pub fn copy_dir_recursive_opts(src: &Path, dst: &Path, force: bool) -> usize {
    if !src.is_dir() {
        return 0;
    }
    let mut copied = 0;
    let _ = fs::create_dir_all(dst);
    if let Ok(entries) = fs::read_dir(src) {
        for entry in entries.flatten() {
            let p = entry.path();
            let target = dst.join(entry.file_name());
            if p.is_dir() {
                copied += copy_dir_recursive_opts(&p, &target, force);
            } else if p.is_file() {
                if !force {
                    if let (Ok(m_src), Ok(m_dst)) = (p.metadata(), target.metadata()) {
                        if m_src.len() == m_dst.len() {
                            if let (Ok(t_src), Ok(t_dst)) = (m_src.modified(), m_dst.modified()) {
                                if t_src == t_dst {
                                    continue;
                                }
                            }
                        }
                    }
                }
                if fs::copy(&p, &target).is_ok() {
                    copied += 1;
                    if let (Ok(m), Ok(f)) = (p.metadata(), fs::File::options().write(true).open(&target)) {
                        if let Ok(mtime) = m.modified() {
                            let _ = f.set_times(std::fs::FileTimes::new().set_modified(mtime));
                        }
                    }
                }
            }
        }
    }
    copied
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

    let mut loaded_chunks_json = serde_json::Map::new();
    for (&dim, lsnap) in &snap.levels {
        let mut coords: Vec<[i32; 2]> = lsnap.chunks.keys().map(|&(cx, cz)| [cx, cz]).collect();
        coords.sort_unstable();
        loaded_chunks_json.insert(dim.id().to_owned(), serde_json::json!(coords));
    }

    let meta = serde_json::json!({
        "slot": slot,
        "saved_at_unix": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
        "game_time": snap.game_time,
        "day_time": snap.day_time,
        "difficulty": snap.difficulty,
        "difficulty_locked": snap.difficulty_locked,
        "clock_rate": snap.clock_rate,
        "clock_frozen": snap.clock_frozen,
        "time_rate": snap.time_rate,
        "world_spawn": snap.world_spawn,
        "world_spawn_angle": snap.world_spawn_angle,
        "next_entity": snap.next_entity,
        "policy": snap.policy.to_json(),
        "anchor_ids": anchors_list,
        "region_files": regions_list,
        "loaded_chunks": loaded_chunks_json,
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

pub fn restore_disk_files_for_slot(world_root: &Path, slot: &str) -> usize {
    restore_disk_files_for_slot_opts(world_root, slot, false)
}

pub fn restore_disk_files_for_slot_opts(world_root: &Path, slot: &str, force: bool) -> usize {
    let cp_dir = slot_dir(world_root, slot);
    let mut changed = 0;
    let cp_level = cp_dir.join("level.dat");
    if cp_level.is_file() {
        if fs::copy(&cp_level, world_root.join("level.dat")).is_ok() {
            changed += 1;
        }
    }
    let cp_players = cp_dir.join("players");
    let target_players = world_root.join("players");
    if cp_players.is_dir() {
        changed += copy_dir_recursive_opts(&cp_players, &target_players, force);
    }
    let cp_data = cp_dir.join("data");
    let target_data = world_root.join("data");
    if cp_data.is_dir() {
        changed += copy_dir_recursive_opts(&cp_data, &target_data, force);
    }
    let cp_dims = cp_dir.join("dimensions");
    let target_dims = world_root.join("dimensions");
    if cp_dims.is_dir() {
        changed += copy_dir_recursive_opts(&cp_dims, &target_dims, force);
    }
    changed
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
                let mut p_level = Level::default();
                p_level.read_player(tag);
                guest_snap.spawn = p_level.spawn;
                guest_snap.respawn_elsewhere = p_level.respawn_elsewhere;
                guest_snap.last_death = p_level.last_death;
                guest_snap.recipe_book = p_level.recipe_book;

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
                if let Some(eq) = tag.get("equipment") {
                    if let Some(s) = eq.get("head").and_then(item_from_nbt) {
                        guest_snap.armor[0] = Some(s);
                    }
                    if let Some(s) = eq.get("chest").and_then(item_from_nbt) {
                        guest_snap.armor[1] = Some(s);
                    }
                    if let Some(s) = eq.get("legs").and_then(item_from_nbt) {
                        guest_snap.armor[2] = Some(s);
                    }
                    if let Some(s) = eq.get("feet").and_then(item_from_nbt) {
                        guest_snap.armor[3] = Some(s);
                    }
                    if let Some(s) = eq.get("offhand").and_then(item_from_nbt) {
                        guest_snap.offhand = Some(s);
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

    let mut policy = RollbackPolicy::default();
    if let Some(pol_obj) = meta["policy"].as_object() {
        for (k, v) in pol_obj {
            if let Some(b) = v.as_bool() {
                policy.set_field(k, b);
            }
        }
    }

    let disk_level = Level::read(&cp_dir);
    let weather = disk_level.as_ref().map(|l| l.weather).unwrap_or_default();
    let rules = horizon_client::gamerules::GameRules::load(&cp_dir);
    let dragon_fight_dat = fs::read(fight_dat_path(&cp_dir)).ok();
    let trader_dat = fs::read(trader_dat_path(&cp_dir)).ok();
    let rand_sequences = read_dat(&cp_dir.join("data/minecraft/random_sequences.dat"))
        .and_then(|t| t.get("data").cloned())
        .map(|d| horizon_client::random_sequences::RandomSequences::from_nbt(&d))
        .unwrap_or_default();

    Some(TimelineSnapshot {
        anchor_ids,
        policy,
        game_time,
        day_time,
        weather,
        health_rules: HealthRules::default(),
        rules,
        border: WorldBorder::default(),
        bossbars: Tag::Compound(Default::default()),
        sculk_tracker: horizon_client::sculk::WardenSpawnTracker::default(),
        sculk_darkness: (0.0, 0.0),
        dragon_fight_dat,
        trader_dat,
        rand_sequences,
        region_files,
        difficulty,
        difficulty_locked,
        clock_rate: meta["clock_rate"].as_f64().map(|v| v as f32).unwrap_or(20.0),
        clock_frozen: meta["clock_frozen"].as_bool().unwrap_or(false),
        time_rate: meta["time_rate"].as_f64().map(|v| v as f32).unwrap_or(1.0),
        world_spawn,
        world_spawn_angle,
        next_entity,
        scoreboard: horizon_client::scoreboard::Scoreboard::default(),
        guests,
        offline_player_data,
        levels: HashMap::new(),
        mob_players: horizon_client::fast_hash::FxHashMap::default(),
        disk_restored: false,
    })
}

pub fn hydrate_level_from_disk(
    game: &mut Game<'_>,
    slot: &str,
    dim: Dim,
    snap: &TimelineSnapshot,
) -> Option<LevelSnapshot> {
    let cp_dir = slot_dir(&game.world_root, slot);
    let cp_dim_dir = horizon_client::save::dimension_dir(&cp_dir, dim.id());
    if !cp_dim_dir.is_dir() {
        return None;
    }

    let shape = game.level_shapes.get(dim.id());
    let cp_save = horizon_client::save::SaveDir::new(cp_dim_dir, shape);
    if let Some(tables) = game.save.tables.get() {
        let _ = cp_save.tables.set(Arc::clone(tables));
    } else {
        let tables = Arc::new(horizon_client::save::SaveTables::new(&game.pack.assets));
        let _ = cp_save.tables.set(tables);
    }

    let mut candidate_set: HashSet<(i32, i32)> = HashSet::new();
    if let Ok(meta_str) = fs::read_to_string(cp_dir.join("meta.json")) {
        if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&meta_str) {
            if let Some(arr) = meta["loaded_chunks"][dim.id()].as_array() {
                for item in arr {
                    if let Some(pair) = item.as_array() {
                        if pair.len() == 2 {
                            if let (Some(cx), Some(cz)) = (pair[0].as_i64(), pair[1].as_i64()) {
                                candidate_set.insert((cx as i32, cz as i32));
                            }
                        }
                    }
                }
            }
        }
    }

    if game.dim == dim {
        candidate_set.extend(game.world.chunks.keys().copied());
    } else if let Some(parked) = game.levels.parked.get(&dim) {
        candidate_set.extend(parked.world.chunks.keys().copied());
    }

    for g in snap.guests.values() {
        if Dim::from_id(&g.dimension) == dim {
            let gx = (g.position[0].floor() as i32) >> 4;
            let gz = (g.position[2].floor() as i32) >> 4;
            for dz in -3..=3 {
                for dx in -3..=3 {
                    candidate_set.insert((gx + dx, gz + dz));
                }
            }
        }
    }

    let mut candidate_chunks: Vec<(i32, i32)> = candidate_set.into_iter().collect();
    candidate_chunks.sort_unstable();

    let mut chunks = FxHashMap::default();
    let mut block_entities = HashMap::new();
    let mut block_names = FxHashMap::default();
    let mut inhabited = FxHashMap::default();
    let mut chunk_ticks = HashMap::new();

    for &(cx, cz) in &candidate_chunks {
        let Some(mut disk_chunk) = cp_save.load(cx, cz, game.pack.assets.registry, game.res.biomes()) else {
            continue;
        };
        for &(pos, ref be) in &disk_chunk.block_entities {
            block_entities.insert(pos, be.clone());
        }
        for &(pos, ref name) in &disk_chunk.block_names {
            block_names.insert(pos, name.clone());
        }
        if let Some(saved) = disk_chunk.saved.as_deref() {
            if let Some(Tag::Long(t)) = saved.root.get(&"InhabitedTime".into()) {
                inhabited.insert((cx, cz), *t);
            }
            let b_ticks = ChunkTicks::from_saved(saved.block_ticks.clone());
            let f_ticks = ChunkTicks::from_saved(saved.fluid_ticks.clone());
            chunk_ticks.insert((cx, cz), (b_ticks, f_ticks));
        }

        let live_chunk_opt = if game.dim == dim {
            game.world.chunks.get(&(cx, cz))
        } else {
            game.levels.parked.get(&dim).and_then(|p| p.world.chunks.get(&(cx, cz)))
        };

        let chunk_arc = if let Some(live_chunk) = live_chunk_opt {
            let mut all_same = live_chunk.sections.len() == disk_chunk.sections.len();
            for (s_idx, disk_sec) in disk_chunk.sections.iter_mut().enumerate() {
                if let Some(live_sec) = live_chunk.sections.get(s_idx) {
                    if live_sec.blocks == disk_sec.blocks && live_sec.biomes == disk_sec.biomes {
                        *disk_sec = Arc::clone(live_sec);
                    } else {
                        all_same = false;
                    }
                } else {
                    all_same = false;
                }
            }
            if all_same {
                Arc::clone(live_chunk)
            } else {
                Arc::new(disk_chunk)
            }
        } else {
            Arc::new(disk_chunk)
        };
        chunks.insert((cx, cz), chunk_arc);
    }

    if chunks.is_empty() {
        return None;
    }

    let forced_chunks = cp_save.load_forced_chunks();
    let raids_dat = fs::read(raids_dat_path(&cp_dir, dim)).ok();

    let saved_game_time = game.world.game_time;
    let saved_random = game.world.random.clone();
    let saved_next_entity = game.next_entity;
    let saved_loaded_entities = std::mem::take(&mut game.world.loaded_entities);
    let saved_mobs = std::mem::take(&mut game.mobs);
    let saved_dropped = std::mem::take(&mut game.dropped);
    let saved_orbs = std::mem::take(&mut game.orbs);
    let saved_projectiles = std::mem::take(&mut game.creatures.projectiles);
    let saved_boats = std::mem::take(&mut game.boats);
    let saved_carts = std::mem::take(&mut game.carts);
    let saved_tnts = std::mem::take(&mut game.tnts);
    let saved_falling = std::mem::take(&mut game.falling);
    let saved_paintings = std::mem::take(&mut game.paintings);
    let saved_item_frames = std::mem::take(&mut game.item_frames);
    let saved_armor_stands = std::mem::take(&mut game.armor_stands);
    let saved_cushions = std::mem::take(&mut game.cushions);
    let saved_others = std::mem::take(&mut game.others);
    let saved_crystals = std::mem::take(&mut game.crystals);
    let saved_dragon = game.dragon.take();
    let saved_knot_ids = std::mem::take(&mut game.knot_ids);
    let saved_persist = std::mem::take(&mut game.persist);
    let saved_tridents = std::mem::take(&mut game.tridents);
    let saved_rockets = std::mem::take(&mut game.rockets);
    let saved_ominous = std::mem::take(&mut game.ominous_spawners);

    game.world.game_time = snap.game_time;
    game.next_entity = saved_next_entity.max(snap.next_entity);

    let mut chunk_keys: Vec<(i32, i32)> = chunks.keys().copied().collect();
    chunk_keys.sort_unstable();
    for (cx, cz) in chunk_keys {
        let tags = cp_save.load_entities(cx, cz);
        if !tags.is_empty() {
            game.world.loaded_entities.push(((cx, cz), tags));
        }
    }
    game.spawn_loaded_entities();

    let mobs = std::mem::replace(&mut game.mobs, saved_mobs);
    let dropped = std::mem::replace(&mut game.dropped, saved_dropped);
    let orbs = std::mem::replace(&mut game.orbs, saved_orbs);
    let projectiles = std::mem::replace(&mut game.creatures.projectiles, saved_projectiles);
    let boats = std::mem::replace(&mut game.boats, saved_boats);
    let carts = std::mem::replace(&mut game.carts, saved_carts);
    let tnts = std::mem::replace(&mut game.tnts, saved_tnts);
    let falling_live = std::mem::replace(&mut game.falling, saved_falling);
    let falling = falling_live.iter().map(FallingSnapshot::from_live).collect();
    let paintings = std::mem::replace(&mut game.paintings, saved_paintings);
    let item_frames = std::mem::replace(&mut game.item_frames, saved_item_frames);
    let armor_stands = std::mem::replace(&mut game.armor_stands, saved_armor_stands);
    let cushions = std::mem::replace(&mut game.cushions, saved_cushions);
    let others = std::mem::replace(&mut game.others, saved_others);
    let crystals = std::mem::replace(&mut game.crystals, saved_crystals);
    let dragon = game.dragon.take().as_ref().map(DragonSnapshot::from_live);
    game.dragon = saved_dragon;
    let knot_ids = std::mem::replace(&mut game.knot_ids, saved_knot_ids);
    let persist_mem = std::mem::replace(&mut game.persist, saved_persist);
    game.tridents = saved_tridents;
    game.rockets = saved_rockets;
    game.ominous_spawners = saved_ominous;
    game.world.loaded_entities = saved_loaded_entities;
    game.world.game_time = saved_game_time;
    game.world.random = saved_random.clone();

    Some(LevelSnapshot {
        chunks,
        block_entities,
        block_names,
        inhabited,
        random: saved_random,
        forced_chunks,
        chunk_ticks,
        random_tick_value: game.block_ticks.random_tick_value,
        comparator_outputs: HashMap::new(),
        torch_toggles: Vec::new(),
        moving_pistons: Vec::new(),
        piston_pushes: Vec::new(),
        block_events: Vec::new(),
        hoppers: HashMap::new(),
        mobs,
        dropped,
        orbs,
        projectiles,
        boats,
        carts,
        tnts,
        falling,
        paintings,
        item_frames,
        armor_stands,
        cushions,
        others,
        crystals,
        dragon,
        knot_ids,
        persist_uuids: persist_mem.uuids,
        persist_raw: persist_mem.raw,
        raids_dat,
        last_hurt_by_player: HashMap::new(),
        love_cause: HashMap::new(),
    })
}

pub fn hydrate_slot_from_disk_on_server(
    game: &mut Game<'_>,
    slot: &str,
) -> Option<Arc<TimelineSnapshot>> {
    let mut snap = match get_slot_snapshot_clone(slot) {
        Some(existing) => (*existing).clone(),
        None => load_slot_from_disk(&game.world_root, slot)?,
    };

    for dim in [Dim::Overworld, Dim::Nether, Dim::End] {
        if !snap.levels.contains_key(&dim) {
            if let Some(lsnap) = hydrate_level_from_disk(game, slot, dim, &snap) {
                if dim == game.dim {
                    set_client_slot_snapshot(
                        slot,
                        ClientBlockSnapshot {
                            dim,
                            chunks: lsnap.chunks.clone(),
                        },
                    );
                }
                snap.levels.insert(dim, lsnap);
            }
        }
    }

    let arc = Arc::new(snap);
    set_slot_snapshot(slot, Arc::clone(&arc));
    Some(arc)
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
