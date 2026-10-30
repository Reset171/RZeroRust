pub mod adaptive;
pub mod anchor;
pub mod api;
pub mod block;
pub mod checkpoint;
pub mod client;
pub mod command;
pub mod config;
pub mod engine;
pub mod event;
pub mod metrics;
pub mod mixin;
pub mod network;
pub mod runtime;
pub mod serial;
pub mod util;

pub use client::RZeroClient;
pub use config::RZeroConfig;

pub const MODID: &str = "rzero";
pub const RBD_SOUND_ID: &str = "rzero:rbd_sound";

#[derive(Clone, Debug)]
pub struct ModGameRules {
    pub auto_save: bool,
    pub fixed_interval: i32,
    pub use_random_interval: bool,
    pub random_min: i32,
    pub random_max: i32,
    pub play_rollback_sound: bool,
    pub use_adaptive_mode: bool,
    pub adaptive_interval: i32,
}

impl Default for ModGameRules {
    fn default() -> Self {
        Self {
            auto_save: true,
            fixed_interval: 300,
            use_random_interval: false,
            random_min: 300,
            random_max: 600,
            play_rollback_sound: true,
            use_adaptive_mode: false,
            adaptive_interval: 300,
        }
    }
}

pub fn verbose() -> bool {
    runtime::RZeroRuntime::with(|rt| rt.settings.logs)
}

pub fn log_info(msg: &str) {
    if verbose() {
        eprintln!("[RZero] {msg}");
    }
}

pub fn log_warn(msg: &str) {
    eprintln!("[RZero][WARN] {msg}");
}

pub fn log_error(msg: &str) {
    eprintln!("[RZero][ERROR] {msg}");
}


#[cfg(test)]
mod tests {
    use crate::{
        adaptive::EscapeSpaceAnalyzer,
        block::{BIOME_SECTION_SIZE, SECTION_SIZE, SectionSnapshot},
        checkpoint::CheckpointManager,
        command::CommandRZeroBase,
        engine::SurvivalClassifier,
        mixin,
        serial::{RZBlob, RZNbt, RZTag},
    };
    use std::collections::HashMap;

    #[test]
    fn rznbt_and_blob_roundtrip_and_intern() {
        let mut map = HashMap::new();
        map.insert("id".to_owned(), RZTag::String("minecraft:villager".to_owned()));
        map.insert("Health".to_owned(), RZTag::Float(20.0));
        map.insert("Age".to_owned(), RZTag::Int(-24000));

        let b1 = RZBlob::of(&map);
        let b2 = RZBlob::of(&map);
        assert_eq!(b1, b2);
        assert_eq!(b1.to_compound(), map);

        let encoded = RZNbt::encode(&map);
        let decoded = RZNbt::decode(&encoded);
        assert_eq!(decoded, map);
    }

    #[test]
    fn section_snapshot_respects_separate_block_and_biome_flags() {
        let mut orig_blocks = [0u32; SECTION_SIZE];
        orig_blocks[0] = 1;
        orig_blocks[42] = 7;
        let mut orig_biomes = [2u32; BIOME_SECTION_SIZE];
        orig_biomes[3] = 5;

        let snap = SectionSnapshot::capture_from_states(&orig_blocks, &orig_biomes);
        let mut live_blocks = [9u32; SECTION_SIZE];
        let mut live_biomes = [0u32; BIOME_SECTION_SIZE];
        let mut changed = Vec::new();
        let mut biome_changed = false;

        let count = snap.apply_diff_to(
            &mut live_blocks,
            &mut live_biomes,
            0,
            64,
            0,
            false,
            true,
            &mut changed,
            &mut biome_changed,
        );
        assert_eq!(count, 0);
        assert!(changed.is_empty());
        assert_eq!(live_blocks[0], 9);
        assert!(biome_changed);
        assert_eq!(live_biomes[3], 5);

        let count = snap.apply_diff_to(
            &mut live_blocks,
            &mut live_biomes,
            0,
            64,
            0,
            true,
            false,
            &mut changed,
            &mut biome_changed,
        );
        assert_eq!(count, SECTION_SIZE);
        assert_eq!(live_blocks[0], 1);
        assert_eq!(live_blocks[42], 7);
    }

    #[test]
    fn escape_space_analyzer_floods_open_ground() {
        let res = EscapeSpaceAnalyzer::flood(
            [0, 64, 0],
            None,
            true,
            |_x, y, _z| y < 64,
            |_x, _y, _z| (false, false),
        );
        assert!(res.reachable > 1);
        assert_eq!(res.cliffs, 0);
    }

    #[test]
    fn survival_classifier_scores_safe_and_lethal_states() {
        let mut safe_in = [0.0f64; SurvivalClassifier::N];
        safe_in[7] = 20.0;
        safe_in[3] = 100.0;
        safe_in[1] = 64.0;
        let score_safe = SurvivalClassifier::score(&safe_in);
        assert_eq!(score_safe, [0.0, 1.0]);

        let mut lethal_in = safe_in;
        lethal_in[7] = 0.5;
        let score_lethal = SurvivalClassifier::score(&lethal_in);
        assert_eq!(score_lethal, [1.0, 0.0]);
    }

    #[test]
    fn checkpoint_and_death_interception_flow() {
        let player = 0x1234_5678u128;
        let out = CommandRZeroBase::execute("/rzero set", Some(player), "minecraft:overworld", 100);
        assert!(out.is_some());
        assert!(mixin::on_guest_die_head(player, 120, 120));
        assert!(CheckpointManager::restore_checkpoint(Some(player), 120));
    }
}