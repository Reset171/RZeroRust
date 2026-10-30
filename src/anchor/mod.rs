use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnchorMode {
    Fixed,
    Multi,
    Everyone,
    Rotating,
}

impl Default for AnchorMode {
    fn default() -> Self {
        Self::Fixed
    }
}

impl AnchorMode {
    pub fn id(self) -> &'static str {
        match self {
            Self::Fixed => "fixed",
            Self::Multi => "multi",
            Self::Everyone => "everyone",
            Self::Rotating => "rotating",
        }
    }

    pub fn by_id(id: &str) -> Self {
        match id.trim().to_ascii_lowercase().as_str() {
            "multi" => Self::Multi,
            "everyone" => Self::Everyone,
            "rotating" => Self::Rotating,
            _ => Self::Fixed,
        }
    }
}

fn default_rotation_seconds() -> i32 {
    300
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RZeroAnchorSettings {
    #[serde(default)]
    pub mode: AnchorMode,
    #[serde(default = "default_rotation_seconds")]
    pub rotation_seconds: i32,
    #[serde(default)]
    pub rollback_cooldown_seconds: i32,
    #[serde(default)]
    pub pinned: Vec<u128>,
}

impl Default for RZeroAnchorSettings {
    fn default() -> Self {
        Self {
            mode: AnchorMode::Fixed,
            rotation_seconds: 300,
            rollback_cooldown_seconds: 0,
            pinned: Vec::new(),
        }
    }
}

impl RZeroAnchorSettings {
    pub const MIN_ROTATION_SECONDS: i32 = 5;

    pub fn sanitize(mut self) -> Self {
        self.rotation_seconds = self.rotation_seconds.max(Self::MIN_ROTATION_SECONDS);
        self.rollback_cooldown_seconds = self.rollback_cooldown_seconds.max(0);
        let mut unique = Vec::with_capacity(self.pinned.len());
        for id in self.pinned {
            if !unique.contains(&id) {
                unique.push(id);
            }
        }
        self.pinned = unique;
        self
    }

    pub fn with_mode(mut self, mode: AnchorMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_pinned(mut self, pinned: Vec<u128>) -> Self {
        self.pinned = pinned;
        self.sanitize()
    }

    pub fn with_rotation_seconds(mut self, seconds: i32) -> Self {
        self.rotation_seconds = seconds;
        self.sanitize()
    }

    pub fn with_rollback_cooldown_seconds(mut self, seconds: i32) -> Self {
        self.rollback_cooldown_seconds = seconds;
        self.sanitize()
    }
}

pub struct AnchorSelector;

impl AnchorSelector {
    pub fn resolve_anchors(
        settings: &RZeroAnchorSettings,
        online_players: &[u128],
        game_time: i64,
        requester: Option<u128>,
    ) -> Vec<u128> {
        let mut online = online_players.to_vec();
        online.sort_unstable();
        let mut anchors = Vec::new();

        match settings.mode {
            AnchorMode::Everyone => {
                anchors.extend_from_slice(&online);
            }
            AnchorMode::Multi => {
                for &pinned in &settings.pinned {
                    if online.contains(&pinned) && !anchors.contains(&pinned) {
                        anchors.push(pinned);
                    }
                }
            }
            AnchorMode::Rotating => {
                if !online.is_empty() {
                    let rotation_ticks = i64::from(settings.rotation_seconds.max(1)) * 20;
                    let idx = game_time
                        .div_euclid(rotation_ticks)
                        .rem_euclid(online.len() as i64) as usize;
                    anchors.push(online[idx]);
                }
            }
            AnchorMode::Fixed => {
                for &pinned in &settings.pinned {
                    if online.contains(&pinned) {
                        anchors.push(pinned);
                        break;
                    }
                }
            }
        }

        if anchors.is_empty() {
            if let Some(req) = requester {
                anchors.push(req);
            } else if let Some(&first) = online.first() {
                anchors.push(first);
            }
        }
        anchors
    }

    pub fn resolve_primary(
        settings: &RZeroAnchorSettings,
        online_players: &[u128],
        game_time: i64,
        requester: Option<u128>,
    ) -> Option<u128> {
        Self::resolve_anchors(settings, online_players, game_time, requester)
            .into_iter()
            .next()
            .or(requester)
    }
}

#[derive(Clone, Debug, Default)]
pub struct RollbackCooldown {
    last_rollback_tick: Option<i64>,
}

impl RollbackCooldown {
    pub const fn new() -> Self {
        Self {
            last_rollback_tick: None,
        }
    }

    pub fn try_consume(&mut self, cooldown_seconds: i32, server_tick_count: i64) -> bool {
        if cooldown_seconds <= 0 {
            self.last_rollback_tick = Some(server_tick_count);
            return true;
        }
        if let Some(last) = self.last_rollback_tick
            && server_tick_count >= last
        {
            let cooldown_ticks = i64::from(cooldown_seconds) * 20;
            if server_tick_count - last < cooldown_ticks {
                return false;
            }
        }
        self.last_rollback_tick = Some(server_tick_count);
        true
    }

    pub fn remaining_ticks(&self, cooldown_seconds: i32, server_tick_count: i64) -> i64 {
        if cooldown_seconds <= 0 {
            return 0;
        }
        let Some(last) = self.last_rollback_tick else {
            return 0;
        };
        if server_tick_count < last {
            return 0;
        }
        let remaining = i64::from(cooldown_seconds) * 20 - (server_tick_count - last);
        remaining.max(0)
    }

    pub fn reset(&mut self) {
        self.last_rollback_tick = None;
    }
}