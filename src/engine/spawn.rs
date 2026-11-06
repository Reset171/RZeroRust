use crate::checkpoint::data::PendingSpawnLedger;
use std::cell::RefCell;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MobCategory {
    Monster = 0,
    Creature = 1,
    Ambient = 2,
    Axolotls = 3,
    UndergroundWaterCreature = 4,
    WaterCreature = 5,
    WaterAmbient = 6,
    Misc = 7,
}

#[derive(Clone, Copy, Debug)]
pub struct SpawnContext {
    pub epoch: i64,
    pub game_time: i64,
    pub chunk_key: i64,
    pub seed: i64,
}

thread_local! {
    static SPAWN_CONTEXT: RefCell<Option<SpawnContext>> = const { RefCell::new(None) };
}

pub struct SpawnEngine;

impl SpawnEngine {
    pub const CREATURE_EPOCH_TICKS: i64 = 400;
    pub const MONSTER_EPOCH_TICKS: i64 = 24000;
    pub const MIN_CHUNK_DISTANCE: i32 = 2;
    pub const MAX_CHUNK_DISTANCE: i32 = 8;
    pub const ATTEMPTS_PER_CATEGORY: i32 = 2;

    const DIM_MIX: i64 = -0x61C8864680B583EB;
    const CHUNK_MIX: i64 = -0x3D4D51C2D82B14B1;
    const EPOCH_MIX: i64 = 0x165667B19E3779F9;

    pub const SALT_NATURAL_SPAWN: i64 = 0x5350_4157_4E45_5255;
    pub const SALT_CATCHUP: i64 = 0x0043_4154_4348_5550;
    pub const SALT_CATCHUP_MONSTER: i64 = 0x4D4F_4E43_4154_4348;

    pub fn epoch_of(game_time: i64) -> i64 {
        game_time.div_euclid(Self::CREATURE_EPOCH_TICKS)
    }

    pub fn monster_epoch_of(game_time: i64) -> i64 {
        game_time.div_euclid(Self::MONSTER_EPOCH_TICKS)
    }

    pub fn mix_stafford13(mut z: i64) -> i64 {
        z = (z ^ ((z as u64 >> 30) as i64)).wrapping_mul(-0x40A7B892E31B1A47);
        z = (z ^ ((z as u64 >> 27) as i64)).wrapping_mul(-0x6B2FB644ECFEED15);
        z ^ ((z as u64 >> 31) as i64)
    }

    pub fn derive(world_seed: i64, dim_hash: i64, chunk_key: i64, tick: i64, salt: i64) -> i64 {
        let raw = world_seed
            ^ dim_hash.wrapping_mul(Self::DIM_MIX)
            ^ chunk_key.wrapping_mul(Self::CHUNK_MIX)
            ^ tick.wrapping_mul(Self::EPOCH_MIX)
            ^ salt;
        Self::mix_stafford13(raw)
    }

    pub fn open(epoch: i64, game_time: i64, chunk_key: i64, seed: i64) {
        SPAWN_CONTEXT.with(|c| {
            *c.borrow_mut() = Some(SpawnContext {
                epoch,
                game_time,
                chunk_key,
                seed,
            })
        });
    }

    pub fn close() {
        SPAWN_CONTEXT.with(|c| *c.borrow_mut() = None);
    }

    pub fn current() -> Option<SpawnContext> {
        SPAWN_CONTEXT.with(|c| *c.borrow())
    }

    pub fn regional_category_cap(category: MobCategory) -> i32 {
        match category {
            MobCategory::Monster => 70,
            MobCategory::Creature => 10,
            MobCategory::Ambient => 15,
            MobCategory::UndergroundWaterCreature | MobCategory::WaterCreature => 5,
            MobCategory::WaterAmbient => 20,
            MobCategory::Axolotls => 5,
            MobCategory::Misc => 0,
        }
    }

    pub fn intra_chunk_cap(category: MobCategory) -> i32 {
        match category {
            MobCategory::Monster => 3,
            MobCategory::Creature => 2,
            MobCategory::Ambient => 2,
            MobCategory::UndergroundWaterCreature | MobCategory::WaterCreature => 2,
            MobCategory::WaterAmbient => 4,
            MobCategory::Axolotls => 2,
            MobCategory::Misc => 0,
        }
    }

    pub fn can_chunk_attempt_category(
        world_seed: i64,
        dim_hash: i64,
        chunk_key: i64,
        category: MobCategory,
        epoch: i64,
    ) -> bool {
        let ticket = Self::derive(
            world_seed,
            dim_hash,
            chunk_key,
            epoch ^ (category as i64),
            0x0074_6963_6B65_744C,
        );
        let stride = if category == MobCategory::Creature { 16 } else { 4 };
        ticket.rem_euclid(stride) == 0
    }
}

pub struct SpawnCatchUp;

impl SpawnCatchUp {
    pub fn on_chunk_start_ticking(
        ledger: &mut PendingSpawnLedger,
        chunk_key: i64,
        game_time: i64,
        catch_up_creatures: bool,
        catch_up_monsters: bool,
        mut replay: impl FnMut(MobCategory, i64),
    ) {
        if catch_up_creatures {
            let now_epoch = SpawnEngine::epoch_of(game_time);
            let from = ledger.catch_up_from(chunk_key, now_epoch);
            for epoch in from..=now_epoch {
                replay(
                    MobCategory::Creature,
                    epoch * SpawnEngine::CREATURE_EPOCH_TICKS,
                );
            }
        }
        if catch_up_monsters {
            let day = SpawnEngine::monster_epoch_of(game_time);
            if ledger.monster_owes_replay(chunk_key, day) {
                replay(MobCategory::Monster, day * SpawnEngine::MONSTER_EPOCH_TICKS);
            }
        }
    }
}