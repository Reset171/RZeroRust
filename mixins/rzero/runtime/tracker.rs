use super::super::{
    Game, Mob,
};
use super::snapshots::*;
use super::types::*;
use horizon_client::fast_hash::FxHashMap;
use std::sync::{OnceLock, RwLock};

#[derive(Clone, Copy, Debug)]
pub struct MobTimelineState {
    pub pos: [f64; 3],
    pub head_yaw: f32,
    pub head_pitch: f32,
    pub health: f32,
}

pub struct LoopTracker {
    pub checkpoint_tick: i64,
    pub previous_tracks: FxHashMap<u32, Vec<(i64, MobTimelineState)>>,
    pub current_tracks: FxHashMap<u32, Vec<(i64, MobTimelineState)>>,
}

impl LoopTracker {
    pub fn new() -> Self {
        Self {
            checkpoint_tick: 0,
            previous_tracks: FxHashMap::default(),
            current_tracks: FxHashMap::default(),
        }
    }

    pub fn on_checkpoint(&mut self, save_tick: i64) {
        self.checkpoint_tick = save_tick;
        self.previous_tracks.clear();
        self.current_tracks.clear();
    }

    pub fn on_rollback(&mut self, restore_tick: i64) {
        self.checkpoint_tick = restore_tick;
        self.previous_tracks = std::mem::take(&mut self.current_tracks);
        self.current_tracks.clear();
    }

    pub fn record_tick(&mut self, now_tick: i64, mobs: &horizon_client::join_seq::JoinList<Mob>) {
        let delta_t = now_tick - self.checkpoint_tick;
        if delta_t < 0 {
            return;
        }
        for m in mobs.iter() {
            if m.removed {
                continue;
            }
            let track = self.current_tracks.entry(m.id).or_default();
            if track.len() > 3600 {
                track.remove(0);
            }
            track.push((
                delta_t,
                MobTimelineState {
                    pos: [m.position.x, m.position.y, m.position.z],
                    head_yaw: m.head_yaw,
                    head_pitch: m.head_pitch,
                    health: m.health,
                },
            ));
        }
    }

    pub fn get_mob_color(
        &self,
        mob_id: u32,
        now_tick: i64,
        current_pos: [f64; 3],
        current_yaw: f32,
        current_hp: f32,
    ) -> u32 {
        let Some(prev_track) = self.previous_tracks.get(&mob_id) else {
            // Моб не был в прошлой петле -> Белая обводка
            return 0xFFFF_FFFF;
        };

        if prev_track.is_empty() {
            return 0xFFFF_FFFF;
        }

        let delta_t = (now_tick - self.checkpoint_tick).max(0);
        let frame = match prev_track.binary_search_by_key(&delta_t, |&(t, _)| t) {
            Ok(idx) => Some(&prev_track[idx].1),
            Err(idx) => {
                if idx == 0 {
                    prev_track.first().map(|(_, f)| f)
                } else if idx >= prev_track.len() {
                    prev_track.last().map(|(_, f)| f)
                } else {
                    let (t_left, f_left) = &prev_track[idx - 1];
                    let (t_right, f_right) = &prev_track[idx];
                    if (delta_t - t_left).abs() <= (delta_t - t_right).abs() {
                        Some(f_left)
                    } else {
                        Some(f_right)
                    }
                }
            }
        };

        let Some(prev_state) = frame else {
            return 0xFFFF_FFFF;
        };

        let dx = current_pos[0] - prev_state.pos[0];
        let dy = current_pos[1] - prev_state.pos[1];
        let dz = current_pos[2] - prev_state.pos[2];
        let dist_sq = dx * dx + dy * dy + dz * dz;

        let mut dyaw = (current_yaw - prev_state.head_yaw).abs() % 360.0;
        if dyaw > 180.0 {
            dyaw = 360.0 - dyaw;
        }

        let dhp = (current_hp - prev_state.health).abs();

        // Порог совпадения действий: смещение до 0.35 блока, поворот головы до 40 градусов
        if dist_sq <= (0.35 * 0.35) && dyaw <= 40.0 && dhp < 0.5 {
            // Зеленая: моб делает те же действия из прошлой петли
            0xFF22_FF55
        } else {
            // Красная: моб делает другие действия
            0xFFFF_3333
        }
    }
}

static LOOP_TRACKER: OnceLock<RwLock<LoopTracker>> = OnceLock::new();

pub fn loop_tracker() -> &'static RwLock<LoopTracker> {
    LOOP_TRACKER.get_or_init(|| RwLock::new(LoopTracker::new()))
}

pub fn process_hitbox_lines(game: &Game<'_>, lines: &mut Vec<horizon_client::gizmos::Line>) {
    let enabled = with_state(|st| st.f3_loop_hitboxes);
    if !enabled {
        return;
    }

    let tracker_guard = match loop_tracker().read() {
        Ok(g) => g,
        Err(_) => return,
    };

    let now_tick = game.world.game_time;

    for m in game.mobs.iter() {
        if m.removed || m.invisible {
            continue;
        }
        let pos = [m.position.x, m.position.y, m.position.z];
        let color = tracker_guard.get_mob_color(m.id, now_tick, pos, m.head_yaw, m.health);

        if color == 0xFFFF_FFFF {
            // Белая по умолчанию
            continue;
        }

        let half = f64::from(m.width()) / 2.0;
        let min_x = pos[0] - half - 0.08;
        let max_x = pos[0] + half + 0.08;
        let min_y = pos[1] - 0.08;
        let max_y = pos[1] + f64::from(m.height()) + 0.08;
        let min_z = pos[2] - half - 0.08;
        let max_z = pos[2] + half + 0.08;

        for line in lines.iter_mut() {
            if line.color == 0xFFFF_FFFF {
                let inside_start = line.start[0] >= min_x && line.start[0] <= max_x
                    && line.start[1] >= min_y && line.start[1] <= max_y
                    && line.start[2] >= min_z && line.start[2] <= max_z;
                let inside_end = line.end[0] >= min_x && line.end[0] <= max_x
                    && line.end[1] >= min_y && line.end[1] <= max_y
                    && line.end[2] >= min_z && line.end[2] <= max_z;
                if inside_start && inside_end {
                    line.color = color;
                }
            }
        }
    }
}
