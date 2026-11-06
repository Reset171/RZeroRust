use super::data::{
    BlockEntityEntry, BlockEventEntry, CheckpointData, ChunkScope, EntityRamSnapshot,
    EntitySnapshot, OfflinePlayerFiles, PoiEntry, ScheduledTickCodec,
};
use crate::{
    adaptive::AdaptiveSaveEngine,
    anchor::AnchorSelector,
    api::DevHooks,
    block::{BIOME_SECTION_SIZE, SECTION_SIZE, SectionSnapshot},
    engine::EntityIdCounter,
    log_info,
    runtime::{MobRamCache, RZeroRuntime, RestoreQueues, SnapshotRegistry},
    serial::RZBlob,
    util::{Phase, RZBenchmark},
};
use std::{path::Path, time::Instant};

pub struct CheckpointWriter;

impl CheckpointWriter {
    pub fn set_checkpoint(
        anchor_dimension: &str,
        requester: Option<u128>,
        current_tick: i64,
        scope: Option<ChunkScope>,
    ) {
        RZBenchmark::begin("CHECKPOINT CAPTURE", current_tick as i32);
        log_info(&format!(
            "Creating checkpoint in dimension {anchor_dimension} (tick={current_tick})"
        ));

        if scope.is_none() {
            RZBlob::clear_intern_pool();
        }
        SnapshotRegistry::clear_allowed_entities();
        RestoreQueues::clear_capture_windows();
        MobRamCache::clear();

        let (policy, anchor_settings, online_players) = RZeroRuntime::with(|rt| {
            (
                rt.settings.checkpoint_policy.clone(),
                rt.anchor_settings.clone(),
                rt.online_players.clone(),
            )
        });
        RZeroRuntime::with_mut(|rt| rt.set_active_checkpoint_policy(policy.clone()));

        let anchors =
            AnchorSelector::resolve_anchors(&anchor_settings, &online_players, current_tick, requester);
        let primary_anchor = anchors.first().copied().or(requester);

        let mut data = CheckpointData {
            anchor_id: primary_anchor,
            anchor_ids: anchors.iter().copied().collect(),
            scope,
            policy,
            ..Default::default()
        };

        let mut adaptive = AdaptiveSaveEngine::capture_adaptive_state();
        adaptive.entity_id_counter = EntityIdCounter::get();
        data.adaptive_state = Some(adaptive);

        AdaptiveSaveEngine::on_checkpoint_created(&anchors, current_tick, adaptive.entity_id_counter);
        SnapshotRegistry::put_active_snapshot(anchor_dimension.to_owned(), data);

        RZBenchmark::end_and_log();
        DevHooks::fire_checkpoint_saved(current_tick);
    }

    pub fn capture_offline_players(world_root: &Path, data: &mut CheckpointData) {
        let t0 = Instant::now();
        OfflinePlayerFiles::backup_into(world_root, &mut data.raw_players_nbt);
        OfflinePlayerFiles::backup_stats_into(world_root, &mut data.raw_player_stats);
        RZBenchmark::accum(Phase::CapturePlayers, t0.elapsed().as_nanos() as u64);
    }
}

pub struct ChunkCapture;

impl ChunkCapture {
    pub fn register_chunk(
        data: &mut CheckpointData,
        chunk_key: i64,
        sections_count: usize,
        inhabited: i64,
    ) {
        if let Some(scope) = &data.scope
            && !scope.includes(&scope.dim, chunk_key)
        {
            return;
        }
        if data.section_snapshots.contains_key(&chunk_key) {
            return;
        }
        data.section_snapshots
            .insert(chunk_key, vec![None; sections_count]);
        data.chunk_inhabited_time.insert(chunk_key, inhabited);
    }

    pub fn capture_section_on_write(
        data: &mut CheckpointData,
        dim: &str,
        chunk_key: i64,
        section_idx: usize,
        sections_count: usize,
        states: &[u32; SECTION_SIZE],
        biomes: &[u32; BIOME_SECTION_SIZE],
    ) -> bool {
        if RZeroRuntime::with(|rt| rt.is_restoring) {
            return false;
        }
        if let Some(scope) = &data.scope
            && !scope.includes(dim, chunk_key)
        {
            return false;
        }
        let arr = data
            .section_snapshots
            .entry(chunk_key)
            .or_insert_with(|| vec![None; sections_count]);
        if section_idx >= arr.len() || arr[section_idx].is_some() {
            return false;
        }
        arr[section_idx] = Some(SectionSnapshot::capture_from_states(states, biomes));
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub fn capture_full_chunk(
        data: &mut CheckpointData,
        chunk_key: i64,
        sections_count: usize,
        inhabited: i64,
        game_time: i64,
        block_entities: Vec<BlockEntityEntry>,
        block_ticks: &[([i32; 3], String, i64, i32, i64)],
        fluid_ticks: &[([i32; 3], String, i64, i32, i64)],
        pois: Vec<PoiEntry>,
        block_events: Vec<BlockEventEntry>,
        entities: Vec<(EntitySnapshot, EntityRamSnapshot)>,
    ) {
        if let Some(scope) = &data.scope
            && !scope.includes(&scope.dim, chunk_key)
        {
            return;
        }
        if data.section_snapshots.contains_key(&chunk_key) {
            return;
        }

        let t0 = Instant::now();
        data.section_snapshots
            .insert(chunk_key, vec![None; sections_count]);
        data.chunk_inhabited_time.insert(chunk_key, inhabited);
        if !block_entities.is_empty() {
            let mut sorted_be = block_entities;
            sorted_be.sort_by_key(|e| e.pos);
            data.chunk_block_entities.insert(chunk_key, sorted_be);
        }
        RZBenchmark::accum(Phase::CaptureBe, t0.elapsed().as_nanos() as u64);

        let t1 = Instant::now();
        if !block_ticks.is_empty() {
            data.chunk_block_ticks
                .insert(chunk_key, ScheduledTickCodec::encode(block_ticks, game_time));
        }
        if !fluid_ticks.is_empty() {
            data.chunk_fluid_ticks
                .insert(chunk_key, ScheduledTickCodec::encode(fluid_ticks, game_time));
        }
        if !pois.is_empty() {
            data.chunk_pois
                .insert(chunk_key, PoiCapture::capture(pois));
        }
        RZBenchmark::accum(Phase::CaptureTicksPois, t1.elapsed().as_nanos() as u64);

        let t2 = Instant::now();
        for ev in block_events {
            if ChunkScope::pack_chunk(ev.pos[0] >> 4, ev.pos[2] >> 4) == chunk_key {
                data.block_events.push(ev);
            }
        }
        let mut sorted_entities = entities;
        sorted_entities.sort_by_key(|(e, _)| e.uuid);
        for (entity, ram) in sorted_entities {
            EntityCapture::capture_with_ram(entity, ram, data);
        }
        RZBenchmark::accum(Phase::CaptureEntities, t2.elapsed().as_nanos() as u64);
    }
}

pub struct EntityCapture;

impl EntityCapture {
    #[allow(clippy::too_many_arguments)]
    pub fn build_snapshot(
        entity_id: i32,
        uuid: u128,
        kind: &str,
        blob: RZBlob,
        chunk_key: i64,
        pos: [f64; 3],
        vel: [f64; 3],
        yaw: f32,
        pitch: f32,
        head_yaw: f32,
        on_ground: bool,
        health: f32,
        tick_count: i32,
        rng_state: Vec<i64>,
        target_uuid: Option<u128>,
        path: Option<([i32; 3], f64)>,
    ) -> EntitySnapshot {
        let (has_path, path_target, path_speed) = match path {
            Some((target, speed)) => (true, target, speed),
            None => (false, [0; 3], 1.0),
        };
        EntitySnapshot {
            entity_id,
            uuid,
            kind: kind.to_owned(),
            blob,
            chunk_key,
            pos,
            vel,
            yaw,
            pitch,
            head_yaw,
            on_ground,
            health,
            target_uuid,
            rng_state,
            tick_count,
            has_path,
            path_target,
            path_speed,
            ..EntitySnapshot::default()
        }
    }

    pub fn capture_with_ram(
        entity: EntitySnapshot,
        ram: EntityRamSnapshot,
        data: &mut CheckpointData,
    ) {
        if SnapshotRegistry::is_entity_allowed(entity.uuid) {
            return;
        }
        let uuid = entity.uuid;
        let chunk_key = entity.chunk_key;
        SnapshotRegistry::allow_entity(uuid);
        data.entity_ram_snapshots.insert(uuid, ram);
        data.entities_by_chunk
            .entry(chunk_key)
            .or_default()
            .push(entity.clone());
        data.entities.push(entity);
    }

    pub fn apply_mob_ram(ram: &EntityRamSnapshot, snap: &mut EntitySnapshot) {
        snap.head_yaw = ram.y_head_rot;
    }
}

pub struct ParallelCapture;

impl ParallelCapture {
    pub fn capture_chunks(
        chunk_keys: &[i64],
        sections_count: usize,
        scope: Option<&ChunkScope>,
        data: &mut CheckpointData,
    ) {
        for &chunk_key in chunk_keys {
            if let Some(sc) = scope
                && !sc.includes(&sc.dim, chunk_key)
            {
                continue;
            }
            ChunkCapture::register_chunk(data, chunk_key, sections_count, 0);
        }
    }
}

pub struct PoiCapture;

impl PoiCapture {
    pub fn capture(mut records: Vec<PoiEntry>) -> Vec<PoiEntry> {
        records.sort_by_key(|r| r.pos);
        records
    }

    pub fn restore(
        saved_pois: &[PoiEntry],
        mut clear_chunk_pois: impl FnMut(),
        mut add_poi: impl FnMut([i32; 3], &str, i32),
    ) {
        clear_chunk_pois();
        for poi in saved_pois {
            add_poi(poi.pos, &poi.poi_type, poi.free_tickets);
        }
    }
}