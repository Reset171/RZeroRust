use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=mixins/rzero/runtime");
    for file_name in [
        "types.rs",
        "snapshots.rs",
        "benchmark.rs",
        "adaptive.rs",
        "capture.rs",
        "restore.rs",
        "persistence.rs",
        "commands.rs",
        "ui.rs",
        "tracker.rs",
        "hooks.rs",
    ] {
        println!("cargo:rerun-if-changed=mixins/rzero/runtime/{file_name}");
    }
    assemble_runtime();
}

fn assemble_runtime() {
    let runtime_dir = Path::new("mixins/rzero/runtime");
    if !runtime_dir.is_dir() {
        return;
    }

    let files = [
        "types.rs",
        "snapshots.rs",
        "benchmark.rs",
        "adaptive.rs",
        "capture.rs",
        "restore.rs",
        "persistence.rs",
        "commands.rs",
        "ui.rs",
        "tracker.rs",
        "hooks.rs",
    ];

    let header = r#"// Auto-generated from mixins/rzero/runtime/*.rs. DO NOT EDIT DIRECTLY.
// To modify, edit files in mixins/rzero/runtime/ and run cargo build.

use super::super::{
    Dim, ExperienceOrb, FallingBlock, Game, ItemEntity, Mob,
    boats::Boat,
    commands::OtherEntity,
    dragon::{Crystal, Dragon},
    lan::LanHost,
    machines::HopperState,
    minecarts::Minecart,
    native_hooks::NativeMod,
    native_lifecycle::WorldPhase,
    tnt::PrimedTnt,
};
use horizon_client::{
    advancements::Progress,
    blocks::Fluid,
    bossbar::{self, BossBars},
    chunk::Chunk,
    containers::BlockEntity,
    cooldowns::Cooldowns,
    effects::EffectInstance,
    entities::{ArmorStand, Cushion, ItemFrame, Painting},
    falling_block::Damage as FallingDamage,
    fast_hash::FxHashMap,
    gamerules::GameRules,
    items::ItemStack,
    lan::guest::SavedBody,
    mobs::Species,
    projectiles::Projectile,
    recipe_book::Settings as RecipeBookSettings,
    save::{item_from_nbt, read_dat, write_dat, Level},
    scoreboard::Scoreboard,
    sculk::WardenSpawnTracker,
    text::Text,
    world::{HealthRules, Weather},
    worldborder::WorldBorder,
};
use horizon_core::{
    geometry::Vec3,
    position::{BlockPos, ChunkPos},
    random::{LegacyRandom, RandomSource},
    ticks::ChunkTicks,
};
use horizon_protocol::{
    packets::{
        play::{self, Clientbound as C},
        recipes,
    },
    text::literal,
};
use horizon_world::{
    level::{BlockEventData, MovingPiston, Ticks},
    nbt::Tag,
    registry::BlockStateId,
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    fs::{self, OpenOptions},
    io::{Read, Write},
    ops::ControlFlow,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, OnceLock, RwLock,
    },
    time::Instant,
};
"#;

    let mut out = String::from(header);

    for file_name in &files {
        let path = runtime_dir.join(file_name);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Could not read {}: {e}", path.display()));

        out.push_str(&format!(
            "\n// ============================================================================\n// Submodule: {}\n// ============================================================================\n\n",
            file_name
        ));

        let mut in_use_block = false;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("use super::")
                || trimmed.starts_with("use horizon_")
                || trimmed.starts_with("use std::")
            {
                if trimmed.ends_with(';') {
                    continue;
                } else {
                    in_use_block = true;
                    continue;
                }
            }
            if in_use_block {
                if trimmed.ends_with(';') {
                    in_use_block = false;
                }
                continue;
            }
            out.push_str(line);
            out.push('\n');
        }
    }

    let target = Path::new("mixins/rzero/runtime.rs");
    let current = fs::read_to_string(target).unwrap_or_default();
    if current != out {
        fs::write(target, out).expect("Could not write mixins/rzero/runtime.rs");
    }
}
