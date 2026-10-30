use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RZeroClientRestoreSettings {
    #[serde(default = "default_true")]
    pub camera_type: bool,
    #[serde(default = "default_true")]
    pub stop_future_sounds: bool,
    #[serde(default = "default_true")]
    pub clear_particles: bool,
    #[serde(default = "default_true")]
    pub restore_inventory_screen: bool,
    #[serde(default = "default_true")]
    pub restore_creative_tab: bool,
    #[serde(default = "default_true")]
    pub restore_recipe_book_tab: bool,
    #[serde(default = "default_true")]
    pub restore_search_text: bool,
    #[serde(default = "default_true")]
    pub reset_hurt_animation: bool,
    #[serde(default = "default_true")]
    pub snap_player_rotation: bool,
    #[serde(default = "default_true")]
    pub skip_equip_animation: bool,
    #[serde(default = "default_true")]
    pub restore_chat_state: bool,
    #[serde(default)]
    pub suppress_terrain_loading_screen: bool,
    #[serde(default = "default_true")]
    pub display_chat_notifications: bool,
    #[serde(default)]
    pub mesh_cache_enabled: bool,
    #[serde(default = "default_mesh_radius")]
    pub mesh_cache_radius: i32,
    #[serde(default = "default_mesh_budget")]
    pub mesh_cache_budget_mb: i32,
}

fn default_true() -> bool {
    true
}

fn default_mesh_radius() -> i32 {
    12
}

fn default_mesh_budget() -> i32 {
    512
}

impl Default for RZeroClientRestoreSettings {
    fn default() -> Self {
        Self {
            camera_type: true,
            stop_future_sounds: true,
            clear_particles: true,
            restore_inventory_screen: true,
            restore_creative_tab: true,
            restore_recipe_book_tab: true,
            restore_search_text: true,
            reset_hurt_animation: true,
            snap_player_rotation: true,
            skip_equip_animation: true,
            restore_chat_state: true,
            suppress_terrain_loading_screen: false,
            display_chat_notifications: true,
            mesh_cache_enabled: false,
            mesh_cache_radius: 12,
            mesh_cache_budget_mb: 512,
        }
    }
}

impl RZeroClientRestoreSettings {
    pub const MIN_MESH_RADIUS: i32 = 2;
    pub const MAX_MESH_RADIUS: i32 = 32;
    pub const MAX_MESH_BUDGET_MB: i32 = 4096;

    pub fn sanitize(mut self) -> Self {
        self.mesh_cache_radius = self
            .mesh_cache_radius
            .clamp(Self::MIN_MESH_RADIUS, Self::MAX_MESH_RADIUS);
        self.mesh_cache_budget_mb = self.mesh_cache_budget_mb.clamp(0, Self::MAX_MESH_BUDGET_MB);
        self
    }

    pub fn mesh_cache_budget_bytes(&self) -> u64 {
        if self.mesh_cache_enabled {
            (self.mesh_cache_budget_mb.max(0) as u64) * 1024 * 1024
        } else {
            0
        }
    }
}