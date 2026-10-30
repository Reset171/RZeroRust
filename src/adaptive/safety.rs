pub struct PlayerSafetyCheck;

impl PlayerSafetyCheck {
    const GROUND_PROBE_DEPTH: i32 = 256;
    const FALL_DAMAGE_GRACE: f32 = 3.0;

    pub fn is_player_safe(
        position: [f64; 3],
        on_ground: bool,
        fall_distance: f32,
        health: f32,
        dead_or_dying: bool,
        in_lava: bool,
        on_fire: bool,
        has_levitation: bool,
        has_imminent_explosion: bool,
        min_build_height: i32,
        mut block_probe: impl FnMut(i32, i32, i32) -> (bool, bool, bool),
    ) -> bool {
        if dead_or_dying || in_lava || on_fire || has_levitation {
            return false;
        }
        if !on_ground
            && !Self::will_survive_landing(
                position,
                fall_distance,
                health,
                min_build_height,
                &mut block_probe,
            )
        {
            return false;
        }
        !has_imminent_explosion
    }

    fn will_survive_landing(
        position: [f64; 3],
        fall_distance: f32,
        health: f32,
        min_build_height: i32,
        block_probe: &mut impl FnMut(i32, i32, i32) -> (bool, bool, bool),
    ) -> bool {
        let bx = position[0].floor() as i32;
        let mut by = position[1].floor() as i32;
        let bz = position[2].floor() as i32;
        let mut found_ground = false;
        let mut expected_damage = 0.0f32;

        for _ in 0..Self::GROUND_PROBE_DEPTH {
            by -= 1;
            if by < min_build_height {
                break;
            }
            let (is_air, has_fluid, is_lava) = block_probe(bx, by, bz);
            if is_air {
                continue;
            }
            if is_lava {
                return false;
            }
            found_ground = true;
            if !has_fluid {
                let total_fall = fall_distance + (position[1] as f32 - by as f32);
                expected_damage = total_fall - Self::FALL_DAMAGE_GRACE;
            }
            break;
        }
        found_ground && expected_damage < health
    }
}