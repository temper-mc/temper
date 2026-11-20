mod db_wrap;
mod generation;
mod helpers;
mod importing;
mod lifecycle;
pub mod player;

use dashmap::DashMap;
pub use generation::WorldChunkGenerator;
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::sync::Arc;
use temper_config::ServerConfig;
pub use temper_core::dimension::Dimension;
use temper_core::math::java_hash_code;
use temper_core::pos::{ChunkPos, FullChunkPos};
use temper_general_purpose::paths::get_root_path;
use temper_storage::lmdb::StorageBackend;
pub use temper_world_format::errors::WorldError;
use temper_world_format::errors::WorldError::InvalidWorldGenerator;
use temper_world_format::Chunk;
use tracing::{error, warn};
use world_db::chunks::save_chunk_internal;
pub use world_db::*;
pub use world_gen;
use wyhash::WyHasherBuilder;

pub use lifecycle::{ChunkKey, ChunkLifecycle};

#[derive(Clone)]
pub struct World {
    pub chunks: ChunkStore,
    pub chunk_generators: DashMap<Dimension, WorldChunkGenerator>,
}

#[derive(Clone)]
pub struct ChunkStore {
    pub storage_backend: StorageBackend,
    cache: Arc<ChunkCache>,
    lifecycle: Arc<ChunkLifecycle>,
    verify: bool,
}

impl World {
    /// Creates a new world instance.
    ///
    /// You'd probably want to call this at the start of your program. And then use the returned
    /// in a state struct or something.
    pub fn new(
        backend_path: impl Into<PathBuf>,
        config: &ServerConfig,
    ) -> Result<Self, WorldError> {
        if let Err(e) = check_config_validity(config) {
            error!("Fatal error in database config: {}", e);
            exit(1);
        }
        let mut backend_path = backend_path.into();
        // Clones are kinda ok here since this is only run once at startup.
        if backend_path.is_relative() {
            backend_path = get_root_path().join(backend_path);
        }
        // Convert the map size from GB to bytes and round it to the nearest page size.
        let map_size = config.database.map_size as usize * 1024 * 1024 * 1024;
        let storage_backend = StorageBackend::initialize(Some(backend_path), map_size)
            .expect("Failed to initialize database");

        let seed = match config.world_gen.seed.parse::<u64>() {
            Ok(seed) => seed,
            Err(_) => java_hash_code(config.world_gen.seed.as_str()).into(),
        };

        let chunks = ChunkStore::new(
            storage_backend,
            config.database.verify_chunk_data,
            WyHasherBuilder::new(seed),
        );
        let overworld_chunk_generator =
            WorldChunkGenerator::from_name(&config.world_gen.generator, seed, Dimension::Overworld)
                .ok_or(InvalidWorldGenerator(
                    match config.world_gen.generator.as_str() {
                        "" => "<empty string>".to_string(),
                        other => other.to_string(),
                    },
                ))?;
        let nether_chunk_generator =
            WorldChunkGenerator::from_name(&config.world_gen.generator, seed, Dimension::Nether)
                .ok_or(InvalidWorldGenerator(
                    match config.world_gen.generator.as_str() {
                        "" => "<empty string>".to_string(),
                        other => other.to_string(),
                    },
                ))?;
        let end_chunk_generator =
            WorldChunkGenerator::from_name(&config.world_gen.generator, seed, Dimension::End)
                .ok_or(InvalidWorldGenerator(
                    match config.world_gen.generator.as_str() {
                        "" => "<empty string>".to_string(),
                        other => other.to_string(),
                    },
                ))?;

        Ok(World {
            chunks,
            chunk_generators: [
                (Dimension::Overworld, overworld_chunk_generator),
                (Dimension::Nether, nether_chunk_generator),
                (Dimension::End, end_chunk_generator),
            ]
            .into_iter()
            .collect(),
        })
    }

    pub fn chunk_lifecycle(&self) -> &ChunkLifecycle {
        self.chunks.chunk_lifecycle()
    }

    pub fn cached_chunk_keys(&self) -> Vec<ChunkKey> {
        self.chunks.cached_chunk_keys()
    }

    pub fn is_chunk_cached(&self, key: ChunkKey) -> bool {
        self.chunks.is_chunk_cached(key)
    }

    pub fn cached_chunk_count(&self) -> usize {
        self.chunks.cached_chunk_count()
    }

    pub fn final_generation_stage(&self, dimension: Dimension) -> u8 {
        self.chunk_generators
            .get(&dimension)
            .map(|g| g.final_stage().raw())
            .unwrap_or_default()
    }

    pub fn is_fully_generated(&self, chunk: &Chunk, dimension: Dimension) -> bool {
        chunk.stage >= self.final_generation_stage(dimension)
    }

    /// Loads a chunk from the database or cache, generating or advancing it first if needed.
    pub fn get_or_generate_chunk(&'_ self, pos: FullChunkPos) -> Result<RefChunk<'_>, WorldError> {
        self.chunk_generators
            .get(&pos.dimension())
            .ok_or(WorldError::MissingWorldGenerator(pos.dimension()))?
            .generate(&self.chunks, pos.dimension(), *pos)?;
        self.get_chunk(pos)
    }

    /// Loads a chunk from the database or cache, generating or advancing it first if needed. Returns a mutable reference.
    pub fn get_or_generate_mut(&self, pos: FullChunkPos) -> Result<MutChunk<'_>, WorldError> {
        self.chunk_generators
            .get(&pos.dimension())
            .ok_or(WorldError::MissingWorldGenerator(pos.dimension()))?
            .generate(&self.chunks, pos.dimension(), *pos)?;
        self.get_chunk_mut(pos)
    }

    /// Get a mutable reference only if the chunk is already resident in the
    /// cache. Unlike `get_chunk_mut`, this never loads the chunk from storage.
    pub fn get_cached_chunk_mut(
        &'_ self,
        pos: ChunkPos,
        dimension: Dimension,
    ) -> Option<MutChunk<'_>> {
        self.chunks.get_cached_chunk_mut(pos, dimension)
    }
}

impl ChunkStore {
    pub fn new(storage_backend: StorageBackend, verify: bool, hasher: WyHasherBuilder) -> Self {
        Self {
            storage_backend,
            cache: Arc::new(ChunkCache::with_hasher(hasher)),
            lifecycle: Arc::new(ChunkLifecycle::default()),
            verify,
        }
    }

    pub fn cached_chunk_keys(&self) -> Vec<ChunkKey> {
        self.cache.iter().map(|entry| *entry.key()).collect()
    }

    pub fn chunk_lifecycle(&self) -> &ChunkLifecycle {
        &self.lifecycle
    }

    pub fn is_chunk_cached(&self, key: ChunkKey) -> bool {
        self.cache.contains_key(&key)
    }

    pub fn cached_chunk_count(&self) -> usize {
        self.cache.len()
    }

    /// Get a mutable reference only if the chunk is already resident in the
    /// cache. The cache entry is rechecked under a lifecycle access lease.
    pub fn get_cached_chunk_mut(
        &'_ self,
        pos: ChunkPos,
        dimension: Dimension,
    ) -> Option<MutChunk<'_>> {
        let key = FullChunkPos::new(pos, dimension);
        let _access = self.lifecycle.begin_access(key);
        self.cache.get_mut(&key)
    }

    /// Atomically detach an unreferenced chunk from the cache. Dirty chunks
    /// stay unavailable to readers until the returned eviction is completed.
    pub(crate) fn evict_chunk(&self, key: FullChunkPos) -> Option<EvictedChunk> {
        if !self.lifecycle.begin_eviction(key) {
            return None;
        }

        let Some((_, chunk)) = self.cache.remove(&key) else {
            self.lifecycle.eviction_cancelled(key);
            return None;
        };

        let needs_save = chunk.is_dirty();
        self.lifecycle.eviction_removed(key, needs_save);
        Some(self.eviction_transaction(key, Some(chunk), needs_save))
    }

    pub(crate) fn begin_delete(&self, pos: ChunkPos, dimension: Dimension) -> Option<EvictedChunk> {
        let key = FullChunkPos::new(pos, dimension);
        if !self.lifecycle.begin_eviction(key) {
            return None;
        }

        let chunk = self.cache.remove(&key).map(|(_, chunk)| chunk);
        self.lifecycle.eviction_removed(key, true);
        Some(self.eviction_transaction(key, chunk, true))
    }

    fn eviction_transaction(
        &self,
        key: ChunkKey,
        chunk: Option<Chunk>,
        needs_save: bool,
    ) -> EvictedChunk {
        EvictedChunk {
            key,
            chunk,
            needs_save,
            store: self.clone(),
            committed: false,
        }
    }
}

/// Owns a chunk from the instant it leaves the cache until eviction completes.
/// Dropping an unfinished eviction restores the chunk and releases its state
/// reservation, so a caller cannot accidentally strand it.
pub struct EvictedChunk {
    key: ChunkKey,
    chunk: Option<Chunk>,
    needs_save: bool,
    store: ChunkStore,
    committed: bool,
}

impl EvictedChunk {
    pub fn pos(&self) -> FullChunkPos {
        self.key
    }

    pub fn dimension(&self) -> Dimension {
        self.key.dimension()
    }

    pub fn needs_save(&self) -> bool {
        self.needs_save
    }

    pub fn finish_clean(mut self) -> Result<(), WorldError> {
        if self.needs_save {
            return Err(WorldError::WorldGenerationError(format!(
                "dirty chunk {} must be saved before eviction completes",
                self.key
            )));
        }
        self.chunk.take();
        self.store.lifecycle.eviction_finished(self.key);
        self.committed = true;
        Ok(())
    }

    pub fn save(self) -> Result<(), WorldError> {
        let storage_backend = self.store.storage_backend.clone();
        self.save_with(move |pos, dimension, chunk| {
            save_chunk_internal(&storage_backend, pos, dimension, chunk)
        })
    }

    pub(crate) fn save_with(
        mut self,
        persist: impl FnOnce(ChunkPos, Dimension, &Chunk) -> Result<(), WorldError>,
    ) -> Result<(), WorldError> {
        if !self.needs_save {
            return self.finish_clean();
        }

        let result = persist(
            *self.key,
            self.key.dimension(),
            self.chunk
                .as_ref()
                .expect("unfinished eviction owns its chunk"),
        );
        match result {
            Ok(()) => {
                self.chunk
                    .take()
                    .expect("unfinished eviction owns its chunk")
                    .clear_dirty();
                self.store.lifecycle.save_finished(self.key);
                self.committed = true;
                Ok(())
            }
            Err(err) => {
                let chunk = self
                    .chunk
                    .take()
                    .expect("unfinished eviction owns its chunk");
                chunk.mark_dirty();
                self.store.cache.insert(self.key, chunk);
                self.store.lifecycle.save_finished(self.key);
                self.committed = true;
                Err(err)
            }
        }
    }

    pub(crate) fn delete_with(
        mut self,
        delete: impl FnOnce(ChunkPos, Dimension) -> Result<(), WorldError>,
    ) -> Result<(), WorldError> {
        delete(*self.key, self.key.dimension())?;
        self.chunk.take();
        self.store.lifecycle.save_finished(self.key);
        self.committed = true;
        Ok(())
    }
}

impl Drop for EvictedChunk {
    fn drop(&mut self) {
        if self.committed {
            return;
        }

        if let Some(chunk) = self.chunk.take() {
            self.store.cache.insert(self.key, chunk);
        }
        if self.needs_save {
            self.store.lifecycle.save_finished(self.key);
        } else {
            self.store.lifecycle.eviction_cancelled(self.key);
        }
    }
}

pub type ChunkCache = DashMap<FullChunkPos, Chunk, WyHasherBuilder>;
pub type MutChunk<'a> = dashmap::mapref::one::RefMut<'a, FullChunkPos, Chunk>;
pub type RefChunk<'a> = dashmap::mapref::one::Ref<'a, FullChunkPos, Chunk>;

fn check_config_validity(config: &ServerConfig) -> Result<(), WorldError> {
    // We don't actually check if the import path is valid here since that would brick a server
    // if the world is imported then deleted after the server starts. Those checks are handled in
    // the importing logic.

    let db_path = get_root_path().join(&config.database.db_path);

    if config.database.map_size == 0 {
        error!("Map size is set to 0. Please set the map size in the configuration file.");
        return Err(WorldError::InvalidMapSize(config.database.map_size));
    }
    if !Path::new(&db_path).exists() {
        warn!("World path does not exist. Attempting to create it.");
        if create_dir_all(&db_path).is_err() {
            error!("Could not create world path: {}", db_path.display());
            return Err(WorldError::InvalidWorldPath(
                db_path.to_string_lossy().to_string(),
            ));
        }
    }
    if Path::new(&db_path).is_file() {
        error!("World path is a file. Please set the world path to a directory.");
        return Err(WorldError::InvalidWorldPath(
            db_path.to_string_lossy().to_string(),
        ));
    }
    if let Err(e) = Path::new(&db_path).read_dir() {
        error!("Could not read world path: {}", e);
        return Err(WorldError::InvalidWorldPath(
            db_path.to_string_lossy().to_string(),
        ));
    }

    // Check if doing map_size * 1024^3 would overflow usize. You probably don't need a database
    // that's 18 exabytes anyway.
    if config.database.map_size as usize > ((usize::MAX / 1024) / 1024) / 1024 {
        error!(
            "Map size is too large, this would exceed the usize limit. You probably don't need a \
        database this big anyway. Are you sure you have set the map size in GB, not bytes?"
        );
        return Err(WorldError::InvalidMapSize(config.database.map_size));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::World;
    use temper_config::server_config::create_dummy_config;
    use temper_core::dimension::Dimension;
    use temper_core::pos::{ChunkPos, FullChunkPos};

    #[test]
    #[ignore]
    fn dump_chunk() {
        let world = World::new(
            std::env::current_dir().unwrap().join("../../../world"),
            &create_dummy_config(),
        )
        .unwrap();
        let chunk = world
            .get_chunk(FullChunkPos::new(ChunkPos::new(1, 1), Dimension::Overworld))
            .expect(
                "Failed to load chunk. If it's a bitcode error, chances are the chunk format \
             has changed since last generating a world so you'll need to regenerate",
            );
        let encoded = bitcode::serialize(&*chunk).unwrap();
        std::fs::write("../../../.etc/raw_chunk.dat", encoded).unwrap();
    }
}
