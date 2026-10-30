use std::collections::{HashSet, VecDeque};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EscapeSpaceResult {
    pub reachable: usize,
    pub cliffs: usize,
}

pub struct EscapeSpaceAnalyzer;

impl EscapeSpaceAnalyzer {
    const MAX_VISITED: usize = 200;
    const MAX_DISTANCE: i32 = 6;
    const DX: [i32; 4] = [0, 0, -1, 1];
    const DZ: [i32; 4] = [-1, 1, 0, 0];

    pub fn pack_pos(x: i32, y: i32, z: i32) -> i64 {
        ((x as i64 & 0x3FFFFFF) << 38)
            | ((z as i64 & 0x3FFFFFF) << 12)
            | (y as i64 & 0xFFF)
    }

    pub fn unpack_pos(packed: i64) -> (i32, i32, i32) {
        let x = (packed >> 38) as i32;
        let z = ((packed << 26) >> 38) as i32;
        let y = ((packed << 52) >> 52) as i32;
        (x, y, z)
    }

    pub fn flood(
        start: [i32; 3],
        blocked_cells: Option<&HashSet<i64>>,
        allow_drops: bool,
        mut blocks_motion: impl FnMut(i32, i32, i32) -> bool,
        mut fluid_flags: impl FnMut(i32, i32, i32) -> (bool, bool),
    ) -> EscapeSpaceResult {
        let min_dy = if allow_drops { -3 } else { -2 };
        let mut visited = HashSet::with_capacity(Self::MAX_VISITED);
        let mut queue = VecDeque::with_capacity(Self::MAX_VISITED);

        let start_long = Self::pack_pos(start[0], start[1], start[2]);
        queue.push_back(start_long);
        visited.insert(start_long);

        let mut cliffs = 0;

        while let Some(current) = queue.pop_front() {
            if visited.len() >= Self::MAX_VISITED {
                break;
            }
            let (cx, cy, cz) = Self::unpack_pos(current);

            for i in 0..4 {
                let nx = cx + Self::DX[i];
                let nz = cz + Self::DZ[i];
                let mut target = None;
                let mut is_cliff = false;

                for dy in (min_dy..=1).rev() {
                    let ny = cy + dy;
                    let state_blocks = blocks_motion(nx, ny, nz);
                    let floor_blocks = blocks_motion(nx, ny - 1, nz);
                    let (floor_has_fluid, floor_is_lava) = fluid_flags(nx, ny - 1, nz);

                    let standable = if allow_drops {
                        floor_blocks || floor_has_fluid
                    } else {
                        floor_blocks
                    };

                    if !state_blocks && standable {
                        target = Some((nx, ny, nz));
                        if allow_drops && (dy <= min_dy || floor_is_lava) {
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
                let target_long = Self::pack_pos(tx, ty, tz);
                if visited.contains(&target_long) {
                    continue;
                }
                if blocked_cells.is_some_and(|b| b.contains(&target_long)) {
                    continue;
                }
                let dist = (tx - start[0]).abs() + (ty - start[1]).abs() + (tz - start[2]).abs();
                if dist <= Self::MAX_DISTANCE {
                    visited.insert(target_long);
                    queue.push_back(target_long);
                }
            }
        }

        EscapeSpaceResult {
            reachable: visited.len(),
            cliffs,
        }
    }
}