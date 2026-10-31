use super::super::Game;
use super::capture::*;
use super::types::*;
use horizon_client::{
    blocks::Fluid,
    mobs::Species,
    text::Text,
};
use horizon_core::{
    geometry::Vec3,
    random::{LegacyRandom, RandomSource},
};
use std::collections::{HashSet, VecDeque};

pub fn is_guest_safe(game: &Game<'_>, g: &horizon_client::lan::guest::Guest) -> bool {
    if !g.placed || g.dead || g.health <= 1.0 {
        return false;
    }
    if g.effects.list.iter().any(|e| e.id == "levitation") {
        return false;
    }
    let (fire_ticks, fall_distance) = match g.motion.as_deref() {
        Some(m) => (m.fire_ticks, m.fall_distance as f32),
        None => (0, 0.0),
    };
    if fire_ticks > 0 {
        return false;
    }
    let bx = g.position[0].floor() as i32;
    let by = g.position[1].floor() as i32;
    let bz = g.position[2].floor() as i32;
    let facts = game.res.facts();
    let here_fluid = facts.get(game.world.block(bx, by, bz)).fluid;
    if here_fluid == Fluid::Lava {
        return false;
    }
    let pos_vec = Vec3::new(g.position[0], g.position[1], g.position[2]);
    let imminent_tnt = game
        .tnts
        .iter()
        .any(|t| t.fuse <= 60 && t.position.distance_squared(pos_vec) <= 64.0);
    let imminent_creeper = game.mobs.iter().any(|m| {
        m.alive()
            && m.species() == Species::Creeper
            && (m.extra.counter > 0 || m.extra.flag)
            && m.position.distance_squared(pos_vec) <= 49.0
    });
    if imminent_tnt || imminent_creeper {
        return false;
    }
    if !g.on_ground && !g.flying && !g.creative() && !g.spectator() {
        let min_y = game.world.shape.min_y;
        let mut probe_y = by;
        let mut found_ground = false;
        let mut expected_damage = 0.0f32;
        for _ in 0..256 {
            probe_y -= 1;
            if probe_y < min_y {
                break;
            }
            let st = game.world.block(bx, probe_y, bz);
            let f = facts.get(st);
            if facts.is_air(st) && f.fluid == Fluid::None {
                continue;
            }
            if f.fluid == Fluid::Lava {
                return false;
            }
            found_ground = true;
            if f.fluid == Fluid::None {
                let total_fall = fall_distance + (g.position[1] as f32 - probe_y as f32);
                expected_damage = total_fall - 3.0;
            }
            break;
        }
        if !found_ground || expected_damage >= g.health {
            return false;
        }
    }
    true
}

pub fn pack_pos(x: i32, y: i32, z: i32) -> i64 {
    ((x as i64 & 0x3FF_FFFF) << 38) | ((z as i64 & 0x3FF_FFFF) << 12) | (y as i64 & 0xFFF)
}

pub fn unpack_pos(packed: i64) -> (i32, i32, i32) {
    let x = (packed >> 38) as i32;
    let z = ((packed << 26) >> 38) as i32;
    let y = ((packed << 52) >> 52) as i32;
    (x, y, z)
}

pub fn flood_escape_space(
    game: &Game<'_>,
    start: [i32; 3],
    blocked: Option<&HashSet<i64>>,
    allow_drops: bool,
) -> (usize, usize) {
    const MAX_VISITED: usize = 200;
    const MAX_DIST: i32 = 6;
    const DX: [i32; 4] = [0, 0, -1, 1];
    const DZ: [i32; 4] = [-1, 1, 0, 0];

    let min_dy = if allow_drops { -3 } else { -2 };
    let facts = game.res.facts();
    let mut visited = HashSet::with_capacity(MAX_VISITED);
    let mut queue = VecDeque::with_capacity(MAX_VISITED);
    let start_long = pack_pos(start[0], start[1], start[2]);
    visited.insert(start_long);
    queue.push_back(start_long);
    let mut cliffs = 0usize;

    while let Some(curr) = queue.pop_front() {
        if visited.len() >= MAX_VISITED {
            break;
        }
        let (cx, cy, cz) = unpack_pos(curr);
        for dir in 0..4 {
            let nx = cx + DX[dir];
            let nz = cz + DZ[dir];
            let mut target = None;
            let mut is_cliff = false;
            for dy in (min_dy..=1).rev() {
                let ny = cy + dy;
                let st_f = facts.get(game.world.block(nx, ny, nz));
                let fl_f = facts.get(game.world.block(nx, ny - 1, nz));
                let state_blocks = st_f.full_collision || st_f.legacy_solid;
                let floor_blocks = fl_f.full_collision || fl_f.legacy_solid;
                let floor_fluid = fl_f.fluid != Fluid::None;
                let floor_lava = fl_f.fluid == Fluid::Lava;
                let standable = if allow_drops {
                    floor_blocks || floor_fluid
                } else {
                    floor_blocks
                };
                if !state_blocks && standable {
                    target = Some((nx, ny, nz));
                    if allow_drops && (dy <= min_dy || floor_lava) {
                        is_cliff = true;
                    }
                    break;
                }
            }
            if is_cliff {
                cliffs += 1;
            }
            let Some((tx, ty, tz)) = target else {
                continue;
            };
            let t_long = pack_pos(tx, ty, tz);
            if visited.contains(&t_long) {
                continue;
            }
            if blocked.is_some_and(|b| b.contains(&t_long)) {
                continue;
            }
            let dist = (tx - start[0]).abs() + (ty - start[1]).abs() + (tz - start[2]).abs();
            if dist <= MAX_DIST {
                visited.insert(t_long);
                queue.push_back(t_long);
            }
        }
    }
    (visited.len(), cliffs)
}

pub fn calculate_guest_threat(game: &Game<'_>, g: &horizon_client::lan::guest::Guest) -> f64 {
    if !is_guest_safe(game, g) {
        return -1.0;
    }
    let hp_pct = (g.health / 20.0).clamp(0.0, 1.0);
    let mut env_threat = f64::from(1.0 - hp_pct) * 60.0;
    if g.food < 6 {
        env_threat += 15.0;
    }
    let air = g.motion.as_deref().map_or(300, |m| m.air);
    if air < 150 {
        let air_pct = air as f32 / 300.0;
        env_threat += f64::from(0.5 - air_pct) * 80.0;
    }
    if g.effects
        .list
        .iter()
        .any(|e| matches!(e.id, "poison" | "wither"))
    {
        env_threat += 40.0;
    }

    let p_vec = Vec3::new(g.position[0], g.position[1], g.position[2]);
    let mut mobs_tracking = 0;
    let mut mobs_attacking = 0;
    let mut mob_cells = HashSet::new();
    for m in &game.mobs {
        if !m.alive() || !m.hostile() {
            continue;
        }
        let d2 = m.position.distance_squared(p_vec);
        if d2 > 256.0 {
            continue;
        }
        if m.ai.target.is_some() || d2 < 16.0 {
            mobs_tracking += 1;
            if d2 < 9.0 || m.ai.aggressive {
                mobs_attacking += 1;
            }
            let mx = m.position.x.floor() as i32;
            let my = m.position.y.floor() as i32;
            let mz = m.position.z.floor() as i32;
            mob_cells.insert(pack_pos(mx, my, mz));
        }
    }

    let mob_threat = (f64::from(mobs_tracking) * 5.0 + f64::from(mobs_attacking) * 25.0).min(60.0);
    if mob_threat == 0.0 {
        env_threat *= 0.5;
    }
    let mut total_threat = (env_threat + mob_threat).min(100.0);
    if mobs_tracking > 0 {
        let start = [
            g.position[0].floor() as i32,
            g.position[1].floor() as i32,
            g.position[2].floor() as i32,
        ];
        let (free_space, cliffs) = flood_escape_space(game, start, None, true);
        let (safe_space, _) = flood_escape_space(game, start, Some(&mob_cells), false);
        if free_space > 0 {
            let cornered = 1.0 - (safe_space as f64 / free_space as f64);
            total_threat = (total_threat + cornered * 15.0 + (cliffs.min(4) as f64) * 2.5).min(100.0);
        }
    }
    total_threat
}

pub fn tick_autosave_on_server(game: &mut Game<'_>) {
    let now = game.world.game_time;

    let boss_just_died = game.dragon.as_ref().is_some_and(|d| d.death_time == 1)
        || game.mobs.iter().any(|m| {
            m.species() == Species::Wither && !m.alive() && m.death_time == 1
        });
    if boss_just_died {
        let should_boss_save = with_state_mut(|st| {
            if st.save_on_boss_defeat && now - st.last_boss_death_tick > 100 {
                st.last_boss_death_tick = now;
                true
            } else {
                false
            }
        });
        if should_boss_save {
            let safe = game
                .lan
                .as_ref()
                .is_some_and(|h| h.hub.guests.iter().any(|g| is_guest_safe(game, g)));
            if safe {
                create_checkpoint_on_server(game, None);
                if with_state(|st| st.logs) {
                    game.system_message(Text::literal(
                        "[RZero] Boss defeat checkpoint created.",
                    ));
                }
                return;
            }
        }
    }

    let (auto_save, adaptive_mode) = with_state(|st| (st.auto_save, st.adaptive_mode));
    if !auto_save {
        return;
    }

    if !adaptive_mode {
        let due = with_state(|st| now - st.last_save_tick >= st.current_interval_ticks.max(100));
        if !due {
            return;
        }
        let any_safe = game
            .lan
            .as_ref()
            .is_some_and(|h| h.hub.guests.iter().any(|g| is_guest_safe(game, g)));
        if any_safe {
            create_checkpoint_on_server(game, None);
            with_state_mut(|st| {
                st.current_interval_ticks = if st.use_random_interval {
                    let min = st.random_min_sec.max(5);
                    let max = st.random_max_sec.max(min + 1);
                    let mut rng = LegacyRandom::new(now);
                    let extra = rng.next_int_bound((max - min) + 1).unwrap_or(0);
                    i64::from(min + extra) * 20
                } else {
                    i64::from(st.fixed_interval_sec.max(5)) * 20
                };
            });
            if with_state(|st| st.logs) {
                game.system_message(Text::literal("[RZero] Auto-saved checkpoint."));
            }
        }
        return;
    }

    let Some(host) = &game.lan else {
        return;
    };
    let online: Vec<u128> = host
        .hub
        .guests
        .iter()
        .filter(|g| g.placed)
        .map(|g| g.uuid)
        .collect();
    let anchors = with_state(|st| resolve_anchors(&st.anchor, &online, now, None));
    let mut max_threat = 0.0f64;
    let mut any_anchor_here = false;
    for g in host
        .hub
        .guests
        .iter()
        .filter(|g| g.placed && anchors.contains(&g.uuid) && g.dimension == game.dim.id())
    {
        any_anchor_here = true;
        let threat = calculate_guest_threat(game, g);
        if threat < 0.0 {
            with_state_mut(|st| st.relax_ticks = 0);
            return;
        }
        if threat > max_threat {
            max_threat = threat;
        }
    }
    if !any_anchor_here {
        return;
    }

    let mut rng = LegacyRandom::new(now ^ 0x5DEECE66D);
    let rand_f = rng.next_float();
    let should_save = with_state_mut(|st| {
        let since_last = now - st.last_save_tick;
        if max_threat > st.current_intensity {
            st.current_intensity += (max_threat - st.current_intensity) * 0.2;
            st.relax_ticks = 0;
        } else {
            st.current_intensity -= (st.current_intensity - max_threat) * 0.02;
        }
        if st.current_intensity > st.peak_intensity {
            st.peak_intensity = st.current_intensity;
        }

        let relax_req = (st.relax_time_sec * 20).max(20);
        let min_gap = (i64::from(st.min_save_gap_sec) * 20).max(100);
        let chance = st.post_combat_save_pct as f32 / 100.0;

        let mut trigger = false;
        if st.peak_intensity > 35.0 {
            if st.current_intensity < st.peak_intensity * 0.5 && max_threat <= 20.0 {
                st.relax_ticks += 1;
                if st.relax_ticks > relax_req {
                    if rand_f < chance {
                        trigger = true;
                    } else {
                        st.peak_intensity = 0.0;
                        st.current_intensity = 0.0;
                        st.relax_ticks = 0;
                    }
                }
            } else {
                st.relax_ticks = 0;
            }
        } else {
            let tranquil_ticks = i64::from(st.adaptive_interval_sec.max(60)) * 20;
            if since_last >= tranquil_ticks && max_threat <= 10.0 {
                trigger = true;
            }
        }
        trigger && since_last > min_gap
    });

    if should_save {
        create_checkpoint_on_server(game, None);
        if with_state(|st| st.logs) {
            game.system_message(Text::literal("[RZero] Adaptive checkpoint created."));
        }
    }
}
