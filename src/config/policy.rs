use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RZeroCheckpointPolicy {
    #[serde(default)]
    pub rollback: Rollback,
    #[serde(default)]
    pub determinism: Determinism,
}

impl RZeroCheckpointPolicy {
    pub fn sanitize(self) -> Self {
        Self {
            rollback: self.rollback.sanitize(),
            determinism: self.determinism.sanitize(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rollback {
    #[serde(default = "default_true")]
    pub blocks: bool,
    #[serde(default = "default_true")]
    pub block_entities: bool,
    #[serde(default = "default_true")]
    pub block_ticks: bool,
    #[serde(default = "default_true")]
    pub fluid_ticks: bool,
    #[serde(default = "default_true")]
    pub block_events: bool,
    #[serde(default = "default_true")]
    pub pois: bool,
    #[serde(default = "default_true")]
    pub biomes: bool,
    #[serde(default)]
    pub players: Players,
    #[serde(default)]
    pub entities: Entities,
    #[serde(default)]
    pub world: World,
}

impl Default for Rollback {
    fn default() -> Self {
        Self {
            blocks: true,
            block_entities: true,
            block_ticks: true,
            fluid_ticks: true,
            block_events: true,
            pois: true,
            biomes: true,
            players: Players::default(),
            entities: Entities::default(),
            world: World::default(),
        }
    }
}

impl Rollback {
    pub fn sanitize(self) -> Self {
        Self {
            players: self.players.sanitize(),
            entities: self.entities.sanitize(),
            world: self.world.sanitize(),
            ..self
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Players {
    #[serde(default = "default_true")]
    pub position: bool,
    #[serde(default = "default_true")]
    pub motion: bool,
    #[serde(default = "default_true")]
    pub health: bool,
    #[serde(default = "default_true")]
    pub hunger: bool,
    #[serde(default = "default_true")]
    pub experience: bool,
    #[serde(default = "default_true")]
    pub inventory: bool,
    #[serde(default = "default_true")]
    pub ender_chest: bool,
    #[serde(default = "default_true")]
    pub potion_effects: bool,
    #[serde(default = "default_true")]
    pub cooldowns: bool,
    #[serde(default = "default_true")]
    pub open_menu: bool,
    #[serde(default = "default_true")]
    pub abilities: bool,
    #[serde(default = "default_true")]
    pub recipe_book: bool,
    #[serde(default = "default_true")]
    pub spawn_point: bool,
    #[serde(default = "default_true")]
    pub score: bool,
    #[serde(default = "default_true")]
    pub stats: bool,
    #[serde(default)]
    pub preserve_new_player_inventory: bool,
}

impl Default for Players {
    fn default() -> Self {
        Self {
            position: true,
            motion: true,
            health: true,
            hunger: true,
            experience: true,
            inventory: true,
            ender_chest: true,
            potion_effects: true,
            cooldowns: true,
            open_menu: true,
            abilities: true,
            recipe_book: true,
            spawn_point: true,
            score: true,
            stats: true,
            preserve_new_player_inventory: false,
        }
    }
}

impl Players {
    pub fn sanitize(self) -> Self {
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entities {
    #[serde(default = "default_true")]
    pub presence: bool,
    #[serde(default = "default_true")]
    pub entity_id: bool,
    #[serde(default = "default_true")]
    pub tick_count: bool,
    #[serde(default = "default_true")]
    pub rng_state: bool,
    #[serde(default = "default_true")]
    pub living_timers: bool,
    #[serde(default = "default_true")]
    pub target: bool,
    #[serde(default = "default_true")]
    pub navigation: bool,
    #[serde(default = "default_true")]
    pub brain_ram: bool,
    #[serde(default = "default_true")]
    pub passengers: bool,
    #[serde(default = "default_true")]
    pub mob_ram_cache: bool,
    #[serde(default = "default_true")]
    pub dropped_items: bool,
    #[serde(default = "default_true")]
    pub experience_orbs: bool,
}

impl Default for Entities {
    fn default() -> Self {
        Self {
            presence: true,
            entity_id: true,
            tick_count: true,
            rng_state: true,
            living_timers: true,
            target: true,
            navigation: true,
            brain_ram: true,
            passengers: true,
            mob_ram_cache: true,
            dropped_items: true,
            experience_orbs: true,
        }
    }
}

impl Entities {
    pub fn sanitize(self) -> Self {
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct World {
    #[serde(default = "default_true")]
    pub time: bool,
    #[serde(default = "default_true")]
    pub weather: bool,
    #[serde(default = "default_true")]
    pub raids: bool,
    #[serde(default = "default_true")]
    pub dragon_fight: bool,
    #[serde(default = "default_true")]
    pub scoreboard: bool,
    #[serde(default = "default_true")]
    pub level_rng: bool,
    #[serde(default = "default_true")]
    pub difficulty: bool,
    #[serde(default = "default_true")]
    pub world_spawn: bool,
    #[serde(default)]
    pub server_globals: ServerGlobals,
}

impl Default for World {
    fn default() -> Self {
        Self {
            time: true,
            weather: true,
            raids: true,
            dragon_fight: true,
            scoreboard: true,
            level_rng: true,
            difficulty: true,
            world_spawn: true,
            server_globals: ServerGlobals::default(),
        }
    }
}

impl World {
    pub fn sanitize(self) -> Self {
        Self {
            server_globals: self.server_globals.sanitize(),
            ..self
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerGlobals {
    #[serde(default = "default_true")]
    pub bossbars: bool,
    #[serde(default = "default_true")]
    pub forced_chunks: bool,
    #[serde(default = "default_true")]
    pub world_border: bool,
    #[serde(default = "default_true")]
    pub game_rules: bool,
    #[serde(default = "default_true")]
    pub random_sequences: bool,
    #[serde(default = "default_true")]
    pub saved_data: bool,
    #[serde(default = "default_true")]
    pub server_tick_count: bool,
    #[serde(default = "default_true")]
    pub shuffling_counter: bool,
    #[serde(default = "default_true")]
    pub scheduled_events: bool,
    #[serde(default = "default_true")]
    pub tick_rate: bool,
}

impl Default for ServerGlobals {
    fn default() -> Self {
        Self {
            bossbars: true,
            forced_chunks: true,
            world_border: true,
            game_rules: true,
            random_sequences: true,
            saved_data: true,
            server_tick_count: true,
            shuffling_counter: true,
            scheduled_events: true,
            tick_rate: true,
        }
    }
}

impl ServerGlobals {
    pub fn sanitize(self) -> Self {
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Determinism {
    #[serde(default)]
    pub natural_spawn: NaturalSpawn,
    #[serde(default)]
    pub mob_ai: MobAi,
    #[serde(default)]
    pub loot: Loot,
    #[serde(default)]
    pub spawns: Spawns,
    #[serde(default)]
    pub combat: Combat,
}

impl Determinism {
    pub fn sanitize(self) -> Self {
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NaturalSpawn {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub use_spawn_engine: bool,
    #[serde(default = "default_true")]
    pub independent_spawning: bool,
    #[serde(default)]
    pub catch_up: bool,
    #[serde(default)]
    pub monster_catch_up: bool,
    #[serde(default = "default_true")]
    pub stamp_spawn_origin: bool,
}

impl Default for NaturalSpawn {
    fn default() -> Self {
        Self {
            enabled: true,
            use_spawn_engine: true,
            independent_spawning: true,
            catch_up: false,
            monster_catch_up: false,
            stamp_spawn_origin: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MobAi {
    #[serde(default = "default_true")]
    pub brain_rng: bool,
    #[serde(default = "default_true")]
    pub behavior_duration_rng: bool,
    #[serde(default = "default_true")]
    pub shuffling_list_rng: bool,
    #[serde(default = "default_true")]
    pub memory_iteration_order: bool,
}

impl Default for MobAi {
    fn default() -> Self {
        Self {
            brain_rng: true,
            behavior_duration_rng: true,
            shuffling_list_rng: true,
            memory_iteration_order: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Loot {
    #[serde(default = "default_true")]
    pub container_loot: bool,
    #[serde(default = "default_true")]
    pub mob_death_drops: bool,
}

impl Default for Loot {
    fn default() -> Self {
        Self {
            container_loot: true,
            mob_death_drops: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spawns {
    #[serde(default = "default_true")]
    pub raid_seed: bool,
    #[serde(default = "default_true")]
    pub raid_spawn_position: bool,
    #[serde(default = "default_true")]
    pub wandering_trader: bool,
}

impl Default for Spawns {
    fn default() -> Self {
        Self {
            raid_seed: true,
            raid_spawn_position: true,
            wandering_trader: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Combat {
    #[serde(default = "default_true")]
    pub knockback_rng: bool,
}

impl Default for Combat {
    fn default() -> Self {
        Self {
            knockback_rng: true,
        }
    }
}