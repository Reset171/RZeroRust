pub mod capture;
pub mod data;
pub mod restore;

pub use capture::{CheckpointWriter, ChunkCapture, EntityCapture, ParallelCapture, PoiCapture};
pub use data::{
    AdaptiveState, AdvancementSnapshot, BlockEntityEntry, BlockEventEntry, CheckpointData,
    ChunkScope, CooldownEntry, CustomSpawnersSnapshot, EntityRamSnapshot, EntitySnapshot,
    MobRamLive, OfflinePlayerFiles, OpenMenuSnapshot, PendingSpawnLedger, PlayerData, PoiEntry,
    ScheduledTickCodec, ScheduledTickEntry, ServerGlobalsSnapshot, VillageSiegeSnapshot,
    WorldSnapshot,
};
pub use restore::{
    BrainRestorer, ChunkRestoreOutput, ChunkRestorer, EntityRestorer, PlayerRestorer,
    RollbackCoordinator, WorldRestoreSummary, WorldStateRestorer,
};

pub struct CheckpointManager;

impl CheckpointManager {
    pub fn set_checkpoint(dimension: &str, requester: Option<u128>, current_tick: i64) {
        CheckpointWriter::set_checkpoint(dimension, requester, current_tick, None);
    }

    pub fn set_checkpoint_scoped(
        dimension: &str,
        requester: Option<u128>,
        current_tick: i64,
        scope: ChunkScope,
    ) {
        CheckpointWriter::set_checkpoint(dimension, requester, current_tick, Some(scope));
    }

    pub fn restore_checkpoint(requester: Option<u128>, current_tick: i64) -> bool {
        RollbackCoordinator::restore_checkpoint(requester, current_tick)
    }

    pub fn flush_sync_pending_restores() -> Vec<(String, i64)> {
        RollbackCoordinator::flush_sync_pending_restores()
    }

    pub fn get_snapshot(uuid: u128, data: &CheckpointData) -> Option<&EntitySnapshot> {
        data.entities.iter().find(|s| s.uuid == uuid)
    }
}