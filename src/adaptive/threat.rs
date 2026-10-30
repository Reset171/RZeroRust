use super::EscapeSpaceAnalyzer;
use crate::metrics::{MetricsWriter, PlayerMetrics};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug)]
pub struct MobProbe {
    pub block_pos: [i32; 3],
    pub distance_sqr: f64,
    pub can_see_player: bool,
    pub can_reach_player: bool,
    pub actively_attacking: bool,
}

pub struct ThreatAssessor;

impl ThreatAssessor {
    pub fn calculate_instant_threat(
        player_uuid: u128,
        player_block_pos: [i32; 3],
        yaw: f32,
        pitch: f32,
        health: f32,
        max_health: f32,
        food_level: i32,
        air_supply: i32,
        max_air_supply: i32,
        has_poison_or_wither: bool,
        is_safe: bool,
        tick: i64,
        mobs: &[MobProbe],
        projectiles_in_air: i32,
        event_label: &str,
        mut blocks_motion: impl FnMut(i32, i32, i32) -> bool,
        mut fluid_flags: impl FnMut(i32, i32, i32) -> (bool, bool),
    ) -> f64 {
        if !is_safe {
            MetricsWriter::write(player_uuid, 100.0, 0, 0, 0, 0.0, 0, "FATAL_VETO", 0);
            return -1.0;
        }

        let mut env_threat = Self::environmental_threat(
            health,
            max_health,
            food_level,
            air_supply,
            max_air_supply,
            has_poison_or_wither,
        );

        PlayerMetrics::accumulate_camera_jitter(player_uuid, yaw, pitch);

        let mut mobs_tracking = 0;
        let mut mobs_attacking = 0;
        for mob in mobs {
            if !mob.can_see_player && !mob.can_reach_player && mob.distance_sqr >= 16.0 {
                continue;
            }
            mobs_tracking += 1;
            if mob.actively_attacking || mob.distance_sqr < 9.0 {
                mobs_attacking += 1;
            }
        }

        let mob_threat = (f64::from(mobs_tracking) * 5.0 + f64::from(mobs_attacking) * 25.0).min(60.0);
        if mob_threat == 0.0 {
            env_threat *= 0.5;
        }

        let total_threat = (env_threat + mob_threat).min(100.0);
        PlayerMetrics::track_combat_window(player_uuid, tick, total_threat);

        if tick % 20 == 0 {
            let mut mob_blocks = HashSet::new();
            for m in mobs {
                let [mx, my, mz] = m.block_pos;
                mob_blocks.insert(EscapeSpaceAnalyzer::pack_pos(mx, my, mz));
                mob_blocks.insert(EscapeSpaceAnalyzer::pack_pos(mx, my, mz - 1));
                mob_blocks.insert(EscapeSpaceAnalyzer::pack_pos(mx, my, mz + 1));
                mob_blocks.insert(EscapeSpaceAnalyzer::pack_pos(mx + 1, my, mz));
                mob_blocks.insert(EscapeSpaceAnalyzer::pack_pos(mx - 1, my, mz));
            }

            let total = EscapeSpaceAnalyzer::flood(
                player_block_pos,
                None,
                true,
                &mut blocks_motion,
                &mut fluid_flags,
            );
            let safe = EscapeSpaceAnalyzer::flood(
                player_block_pos,
                Some(&mob_blocks),
                false,
                &mut blocks_motion,
                &mut fluid_flags,
            );

            let free_space = total.reachable as i32;
            let cornered_pct = if free_space > 0 {
                1.0 - (safe.reachable as f32 / free_space as f32)
            } else {
                0.0
            };

            MetricsWriter::write(
                player_uuid,
                total_threat,
                mobs_tracking,
                mobs_attacking,
                free_space,
                cornered_pct,
                total.cliffs as i32,
                event_label,
                projectiles_in_air,
            );
        }

        total_threat
    }

    fn environmental_threat(
        health: f32,
        max_health: f32,
        food_level: i32,
        air_supply: i32,
        max_air_supply: i32,
        has_poison_or_wither: bool,
    ) -> f64 {
        let mut threat = 0.0;
        let hp_pct = (health / max_health.max(1.0)).clamp(0.0, 1.0);
        threat += f64::from(1.0 - hp_pct) * 60.0;

        if food_level < 6 {
            threat += 15.0;
        }
        if air_supply < max_air_supply / 2 {
            let air_pct = air_supply as f32 / max_air_supply.max(1) as f32;
            threat += f64::from(0.5 - air_pct) * 80.0;
        }
        if has_poison_or_wither {
            threat += 40.0;
        }
        threat
    }
}