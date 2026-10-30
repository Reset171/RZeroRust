use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RZeroAdaptiveSettings {
    #[serde(default = "default_min_save_gap")]
    pub min_save_gap_seconds: i32,
    #[serde(default = "default_relax_time")]
    pub relax_time_seconds: i32,
    #[serde(default = "default_post_combat_pct")]
    pub post_combat_save_percent: i32,
    #[serde(default = "default_true")]
    pub save_on_boss_defeat: bool,
}

fn default_min_save_gap() -> i32 {
    30
}

fn default_relax_time() -> i32 {
    5
}

fn default_post_combat_pct() -> i32 {
    70
}

fn default_true() -> bool {
    true
}

impl Default for RZeroAdaptiveSettings {
    fn default() -> Self {
        Self {
            min_save_gap_seconds: 30,
            relax_time_seconds: 5,
            post_combat_save_percent: 70,
            save_on_boss_defeat: true,
        }
    }
}

impl RZeroAdaptiveSettings {
    pub fn sanitize(self) -> Self {
        Self {
            min_save_gap_seconds: self.min_save_gap_seconds.max(5),
            relax_time_seconds: self.relax_time_seconds.max(1),
            post_combat_save_percent: self.post_combat_save_percent.clamp(0, 100),
            save_on_boss_defeat: self.save_on_boss_defeat,
        }
    }
}