use crossbeam_channel::RecvTimeoutError;
use std::sync::Arc;
use std::time::Duration;

use gen_core::{
    ChunkGenerator, GenStage, GenerationError, StageInput, StageNeighbor, StageNeighborhood,
    StageSpec,
};
use gen_scheduler::{JobKey, SchedulerError, SchedulerState};
use temper_core::dimension::Dimension;
use temper_core::pos::ChunkPos;
use temper_world_format::errors::WorldError;
use temper_world_format::Chunk;

use crate::ChunkStore;

#[derive(Clone)]
pub struct WorldChunkGenerator {
    scheduler: Arc<SchedulerState>,
    generator: Arc<dyn ChunkGenerator>,
}

impl WorldChunkGenerator {
    pub fn new(generator: Arc<dyn ChunkGenerator>) -> Self {
        Self {
            scheduler: Arc::new(SchedulerState::new()),
            generator,
        }
    }

    pub fn from_name(name: &str, seed: u64, dimension: Dimension) -> Option<Self> {
        Some(Self::new(world_gen::generator_from_name(
            name, seed, dimension,
        )?))
    }

    pub fn final_stage(&self) -> GenStage {
        self.generator.final_stage()
    }

    pub fn generate(
        &self,
        chunks: &ChunkStore,
        dimension: Dimension,
        pos: ChunkPos,
    ) -> Result<(), WorldError> {
        self.generate_to(chunks, dimension, pos, self.generator.final_stage())
    }

    pub fn generate_to(
        &self,
        chunks: &ChunkStore,
        dimension: Dimension,
        pos: ChunkPos,
        stage: GenStage,
    ) -> Result<(), WorldError> {
        if chunk_has_stage(chunks, dimension, pos, stage)? {
            return Ok(());
        }

        let target = JobKey::new(dimension, pos, stage);
        let request = self
            .scheduler
            .register_request(&*self.generator, target)
            .map_err(scheduler_error)?;
        let wake_receiver = request.wake_receiver();

        let result = loop {
            if chunk_has_stage(chunks, dimension, pos, stage)? {
                break Ok(());
            }

            if let Some(claimed) = self.scheduler.claim_next_for_request(&request) {
                let already = chunk_has_stage(
                    chunks,
                    claimed.key.dimension,
                    claimed.key.pos,
                    claimed.key.stage,
                )?;
                if !already {
                    if let Err(err) = self.run_stage(chunks, claimed.key) {
                        // never leave a claimed job stuck in Running since
                        // everything waiting on it would block forever.
                        self.scheduler.fail_job(claimed.key);
                        break Err(err);
                    }
                }

                self.scheduler
                    .mark_complete(claimed.key)
                    .map_err(scheduler_error)?;
                continue;
            }

            // poll rather than block indefinitely: a dependency may have
            // failed or been forgotten, in which case no wake is coming.
            match wake_receiver.recv_timeout(Duration::from_millis(50)) {
                Ok(()) => continue,
                Err(RecvTimeoutError::Timeout) => {
                    if self.scheduler.get_job(target).is_none() {
                        break Err(WorldError::WorldGenerationError(format!(
                            "generation jobs for {pos:?} were dropped while waiting"
                        )));
                    }
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => {
                    break Err(WorldError::WorldGenerationError(
                        "chunk generation request closed before the target completed".to_string(),
                    ));
                }
            }
        };

        self.scheduler.unregister_request(request.id);
        result
    }

    pub fn has_pending_jobs(&self, dimension: Dimension, pos: ChunkPos) -> bool {
        self.scheduler.has_pending_jobs(dimension, pos)
    }

    pub fn forget_chunk(&self, dimension: Dimension, pos: ChunkPos) {
        self.scheduler.forget_chunk(dimension, pos);
    }

    pub fn forget_all(&self) {
        self.scheduler.forget_all();
    }

    fn run_stage(&self, chunks: &ChunkStore, key: JobKey) -> Result<(), WorldError> {
        chunks.ensure_chunk(key.pos, key.dimension)?;

        let stage_spec = self
            .generator
            .stage_spec(key.stage)
            .ok_or_else(|| scheduler_error(SchedulerError::UnknownStage(key)))?;
        let neighbor_snapshots = neighbor_snapshots(chunks, key, stage_spec)?;
        let neighbors = neighbor_snapshots
            .iter()
            .map(|neighbor| StageNeighbor::new(neighbor.pos, neighbor.stage, &neighbor.chunk))
            .collect::<Vec<_>>();
        let neighborhood = StageNeighborhood::new(&neighbors);
        let mut target = chunks.get_chunk_mut(key.pos, key.dimension)?;

        self.generator
            .advance_stage(StageInput::new(
                key.pos,
                key.stage,
                &mut target,
                neighborhood,
            ))
            .map_err(generation_error)?;
        target.stage = key.stage.raw();
        target.mark_dirty();

        Ok(())
    }
}

struct NeighborSnapshot {
    pos: ChunkPos,
    stage: GenStage,
    chunk: Chunk,
}

fn neighbor_snapshots(
    chunks: &ChunkStore,
    key: JobKey,
    stage_spec: StageSpec,
) -> Result<Vec<NeighborSnapshot>, WorldError> {
    let Some(stage) = stage_spec.dependencies.neighbor_stage else {
        return Ok(Vec::new());
    };

    let radius = i32::from(stage_spec.dependencies.neighbor_radius);
    let mut snapshots = Vec::new();

    for x in -radius..=radius {
        for z in -radius..=radius {
            if x == 0 && z == 0 {
                continue;
            }

            let pos = offset_chunk_pos(key.pos, x, z);
            let chunk = chunks.get_chunk(pos, key.dimension)?;

            if chunk.stage < stage.raw() {
                return Err(WorldError::WorldGenerationError(format!(
                    "chunk {} is at stage {}, but stage {} requires neighbor stage {}",
                    pos, chunk.stage, key.stage, stage
                )));
            }

            snapshots.push(NeighborSnapshot {
                pos,
                stage: GenStage::new(chunk.stage),
                chunk: chunk.clone_without_transient_noise(),
            });
        }
    }

    Ok(snapshots)
}

fn chunk_has_stage(
    chunks: &ChunkStore,
    dimension: Dimension,
    pos: ChunkPos,
    stage: GenStage,
) -> Result<bool, WorldError> {
    Ok(chunks
        .generation_stage(pos, dimension)?
        .is_some_and(|chunk_stage| chunk_stage >= stage.raw()))
}

fn offset_chunk_pos(pos: ChunkPos, x: i32, z: i32) -> ChunkPos {
    let (chunk_x, chunk_z) = chunk_coords(pos);
    ChunkPos::new(chunk_x + x, chunk_z + z)
}

fn chunk_coords(pos: ChunkPos) -> (i32, i32) {
    (pos.pos.x.div_euclid(16), pos.pos.y.div_euclid(16))
}

fn scheduler_error(err: SchedulerError) -> WorldError {
    WorldError::WorldGenerationError(err.to_string())
}

fn generation_error(err: GenerationError) -> WorldError {
    WorldError::WorldGenerationError(err.to_string())
}
