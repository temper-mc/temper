use crate::{ChunkStore, EvictedChunk, MutChunk, RefChunk, World};
use temper_core::dimension::Dimension;
use temper_core::pos::{ChunkPos, FullChunkPos};
use temper_world_format::errors::WorldError;
use temper_world_format::Chunk;
use world_db::chunks::{
    chunk_exists_internal, delete_chunk_internal, load_chunk_internal, save_chunk_internal,
    save_serialized_chunk_internal, sync_internal,
};

struct ChunkSaveSnapshot {
    pos: ChunkPos,
    dimension: Dimension,
    chunk: Chunk,
}

impl ChunkStore {
    fn load_cached_chunk(&self, key: FullChunkPos) -> Result<(), WorldError> {
        let _load = self.lifecycle.begin_cache_load(key);
        if self.cache.contains_key(&key) {
            return Ok(());
        }

        let chunk = load_chunk_internal(&self.storage_backend, *key, key.dimension(), self.verify)?;
        self.cache.entry(key).or_insert(chunk);
        Ok(())
    }

    pub fn generation_stage(
        &self,
        pos: ChunkPos,
        dimension: Dimension,
    ) -> Result<Option<u8>, WorldError> {
        match self.get_chunk(pos, dimension) {
            Ok(chunk) => Ok(Some(chunk.stage)),
            Err(WorldError::ChunkNotFound) => Ok(None),
            Err(err) => Err(err),
        }
    }

    pub fn ensure_chunk(&self, pos: ChunkPos, dimension: Dimension) -> Result<(), WorldError> {
        let key = FullChunkPos::new(pos, dimension);
        let _access = self.lifecycle.begin_access(key);
        if self.chunk_exists(pos, dimension)? {
            return Ok(());
        }

        let chunk = Chunk::new_empty_with_height(dimension.chunk_height());
        chunk.mark_dirty();
        self.cache.insert(FullChunkPos::new(pos, dimension), chunk);

        Ok(())
    }

    /// Save a chunk to the storage backend
    ///
    /// This function will save a chunk to the storage backend and update the cache with the new
    /// chunk data. If the chunk already exists in the cache, it will be updated with the new data.
    pub fn insert_chunk(
        &self,
        pos: ChunkPos,
        dimension: Dimension,
        chunk: Chunk,
    ) -> Result<(), WorldError> {
        let _write = self
            .lifecycle
            .begin_storage_write(FullChunkPos::new(pos, dimension))
            .ok_or_else(|| {
                WorldError::WorldGenerationError(format!(
                    "cannot write chunk {pos} while it is active"
                ))
            })?;
        let key = FullChunkPos::new(pos, dimension);
        if self.cache.get(&key).is_some_and(|cached| cached.is_dirty()) {
            return Err(WorldError::WorldGenerationError(format!(
                "cannot replace dirty cached chunk {pos}"
            )));
        }
        chunk.clear_dirty();
        save_chunk_internal(&self.storage_backend, pos, dimension, &chunk)?;
        self.cache.remove(&key);
        Ok(())
    }

    /// Load a chunk from the storage backend. If the chunk is in the cache, it will be returned
    /// from the cache instead of the storage backend. If the chunk is not in the cache, it will be
    /// loaded from the storage backend and inserted into the cache.
    pub fn get_chunk(
        &'_ self,
        pos: ChunkPos,
        dimension: Dimension,
    ) -> Result<RefChunk<'_>, WorldError> {
        let key = FullChunkPos::new(pos, dimension);
        let _access = self.lifecycle.begin_access(key);
        if let Some(chunk) = self.cache.get(&key) {
            return Ok(chunk);
        }
        self.load_cached_chunk(key)?;
        Ok(self
            .cache
            .get(&key)
            .expect("loaded chunk should be present in the cache"))
    }

    /// Load a mutable chunk from the storage backend. If the chunk is in the cache, it will be returned
    /// from the cache instead of the storage backend. If the chunk is not in the cache, it will be
    /// loaded from the storage backend and inserted into the cache.
    pub fn get_chunk_mut(
        &'_ self,
        pos: ChunkPos,
        dimension: Dimension,
    ) -> Result<MutChunk<'_>, WorldError> {
        let key = FullChunkPos::new(pos, dimension);
        let _access = self.lifecycle.begin_access(key);
        if let Some(chunk) = self.cache.get_mut(&key) {
            return Ok(chunk);
        }
        self.load_cached_chunk(key)?;
        Ok(self
            .cache
            .get_mut(&key)
            .expect("loaded chunk should be present in the cache"))
    }

    /// Check if a chunk exists in the storage backend.
    ///
    /// It will first check if the chunk is in the cache and if it is, it will return true. If the
    /// chunk is not in the cache, it will check the storage backend for the chunk, returning true
    /// if it exists and false if it does not.
    pub fn chunk_exists(&self, pos: ChunkPos, dimension: Dimension) -> Result<bool, WorldError> {
        let key = FullChunkPos::new(pos, dimension);
        let _access = self.lifecycle.begin_access(key);
        if self.cache.contains_key(&key) {
            return Ok(true);
        }
        chunk_exists_internal(&self.storage_backend, pos, dimension)
    }

    /// Delete a chunk from the storage backend.
    ///
    /// This function will remove the chunk from the cache and delete it from the storage backend.
    pub fn delete_chunk(&self, pos: ChunkPos, dimension: Dimension) -> Result<(), WorldError> {
        let deletion = self.begin_delete(pos, dimension).ok_or_else(|| {
            WorldError::WorldGenerationError(format!("cannot delete referenced chunk {pos}"))
        })?;
        let storage_backend = self.storage_backend.clone();
        deletion.delete_with(move |pos, dimension| {
            delete_chunk_internal(&storage_backend, pos, dimension)
        })
    }

    /// Sync the storage backend.
    ///
    /// This function will save fully generated dirty chunks in the cache to the storage backend and
    /// then sync the storage backend. This should be run after inserting or updating a large number
    /// of chunks to ensure that the data is properly saved to disk.
    pub fn sync(&self, minimum_stage: u8) -> Result<(), WorldError> {
        // Finish any asynchronous eviction writes first so failed writes that
        // restored their chunks to the cache are included in this snapshot.
        self.lifecycle.wait_for_pending_saves();
        let mut snapshots = Vec::new();
        let mut access_leases = Vec::new();

        for key in self.cached_chunk_keys() {
            let access = self.lifecycle.begin_access(key);
            let Some(v) = self.cache.get(&key) else {
                continue;
            };
            if !v.is_dirty() {
                continue;
            }
            if v.stage < minimum_stage {
                continue;
            }

            snapshots.push(ChunkSaveSnapshot {
                pos: *key,
                dimension: key.dimension(),
                chunk: v.clone_without_transient_noise(),
            });
            v.clear_dirty();
            drop(v);
            access_leases.push(access);
        }

        for snapshot in snapshots {
            let data = bitcode::serialize(&snapshot.chunk).expect("Unable to serialize chunk");
            if let Err(err) = save_serialized_chunk_internal(
                &self.storage_backend,
                snapshot.pos,
                snapshot.dimension,
                &data,
            ) {
                if let Ok(chunk) = self.get_chunk(snapshot.pos, snapshot.dimension) {
                    chunk.mark_dirty();
                }

                return Err(err);
            }
        }

        let result = sync_internal(&self.storage_backend);
        drop(access_leases);
        result
    }
}

impl World {
    pub fn insert_chunk(&self, pos: FullChunkPos, chunk: Chunk) -> Result<(), WorldError> {
        self.chunks.insert_chunk(*pos, pos.dimension(), chunk)
    }

    pub fn get_chunk(&'_ self, pos: FullChunkPos) -> Result<RefChunk<'_>, WorldError> {
        self.chunks.get_chunk(*pos, pos.dimension())
    }

    pub fn evict_chunk(&self, pos: FullChunkPos) -> Option<EvictedChunk> {
        let evicted = self.chunks.evict_chunk(pos);
        if evicted.is_some() {
            if let Some(generator) = self.chunk_generators.get(&pos.dimension()) {
                generator.forget_chunk(pos.dimension(), *pos);
            }
        }
        evicted
    }

    pub fn get_chunk_mut(&'_ self, pos: FullChunkPos) -> Result<MutChunk<'_>, WorldError> {
        self.chunks.get_chunk_mut(*pos, pos.dimension())
    }

    pub fn chunk_exists(&self, pos: FullChunkPos) -> Result<bool, WorldError> {
        self.chunks.chunk_exists(*pos, pos.dimension())
    }

    pub fn delete_chunk(&self, pos: FullChunkPos) -> Result<(), WorldError> {
        self.chunks.delete_chunk(*pos, pos.dimension())?;
        if let Some(generator) = self.chunk_generators.get(&pos.dimension()) {
            generator.forget_chunk(pos.dimension(), *pos);
        }
        Ok(())
    }

    pub fn sync(&self) -> Result<(), WorldError> {
        self.chunks.sync(
            self.chunk_generators
                .iter()
                .map(|g| g.final_stage().raw())
                .max()
                .unwrap_or_default(),
        )
    }
}

#[cfg(test)]
mod tests {
    use gen_core::GenStage;
    use std::sync::{mpsc, Arc};
    use std::time::Duration;
    use temper_storage::lmdb::StorageBackend;
    use tempfile::TempDir;
    use wyhash::WyHasherBuilder;

    use super::*;

    fn test_store() -> (ChunkStore, TempDir) {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let storage =
            StorageBackend::initialize(Some(temp_dir.path().to_path_buf()), 1024 * 1024 * 1024)
                .expect("storage should initialize");

        (
            ChunkStore::new(storage, false, WyHasherBuilder::default()),
            temp_dir,
        )
    }

    #[test]
    fn sync_skips_dirty_chunks_below_the_save_stage() {
        let (store, _temp_dir) = test_store();
        let pos = ChunkPos::new(0, 0);
        let mut chunk = Chunk::new_empty();
        chunk.stage = GenStage::FEATURES.raw();
        chunk.mark_dirty();
        store
            .cache
            .insert(FullChunkPos::new(pos, Dimension::Overworld), chunk);

        store
            .sync(GenStage::FULL.raw())
            .expect("sync should succeed");

        let cached = store
            .cache
            .get(&FullChunkPos::new(pos, Dimension::Overworld))
            .expect("skipped chunk should stay cached");
        assert!(
            cached.is_dirty(),
            "skipped partial chunks should remain dirty for a later full save"
        );

        drop(cached);
        store.cache.clear();
        assert!(
            matches!(
                store.get_chunk(pos, Dimension::Overworld),
                Err(WorldError::ChunkNotFound)
            ),
            "partial chunks should not be written to storage"
        );
    }

    #[test]
    fn sync_saves_dirty_chunks_at_the_save_stage() {
        let (store, _temp_dir) = test_store();
        let pos = ChunkPos::new(1, 0);
        let mut chunk = Chunk::new_empty();
        chunk.stage = GenStage::FULL.raw();
        chunk.mark_dirty();
        store
            .cache
            .insert(FullChunkPos::new(pos, Dimension::Overworld), chunk);

        store
            .sync(GenStage::FULL.raw())
            .expect("sync should succeed");

        let cached = store
            .cache
            .get(&FullChunkPos::new(pos, Dimension::Overworld))
            .expect("saved chunk should stay cached");
        assert!(
            !cached.is_dirty(),
            "saved full chunks should be marked clean after snapshotting"
        );

        drop(cached);
        store.cache.clear();
        let loaded = store
            .get_chunk(pos, Dimension::Overworld)
            .expect("full chunk should reload from storage");
        assert_eq!(loaded.stage, GenStage::FULL.raw());
    }

    #[test]
    fn evicted_dirty_chunk_can_be_read_after_its_write_finishes() {
        let (store, _temp_dir) = test_store();
        let pos = ChunkPos::new(-7, 12);
        let mut chunk = Chunk::new_empty();
        chunk.stage = GenStage::FULL.raw();
        chunk.mark_dirty();
        store
            .cache
            .insert(FullChunkPos::new(pos, Dimension::Overworld), chunk);

        let eviction = store
            .evict_chunk(FullChunkPos::new(pos, Dimension::Overworld))
            .unwrap_or_else(|| panic!("unreferenced chunk should be evicted"));
        assert!(eviction.needs_save());
        assert!(!store
            .cache
            .contains_key(&FullChunkPos::new(pos, Dimension::Overworld)));

        eviction.save().expect("evicted chunk should be persisted");
        let loaded = store
            .get_chunk(pos, Dimension::Overworld)
            .expect("a completed write should be readable from storage");
        assert_eq!(loaded.stage, GenStage::FULL.raw());
    }

    #[test]
    fn reader_waits_for_the_eviction_write_before_loading_from_storage() {
        let (store, _temp_dir) = test_store();
        let store = Arc::new(store);
        let pos = ChunkPos::new(21, -13);
        let mut chunk = Chunk::new_empty();
        chunk.stage = GenStage::FULL.raw();
        chunk.mark_dirty();
        store
            .cache
            .insert(FullChunkPos::new(pos, Dimension::Overworld), chunk);

        let eviction = store
            .evict_chunk(FullChunkPos::new(pos, Dimension::Overworld))
            .unwrap_or_else(|| panic!("unreferenced chunk should be evicted"));
        let (started_tx, started_rx) = mpsc::channel();
        let (loaded_tx, loaded_rx) = mpsc::channel();
        let reader_store = Arc::clone(&store);
        let reader = std::thread::spawn(move || {
            started_tx
                .send(())
                .expect("test should receive reader start");
            let loaded = reader_store
                .get_chunk(pos, Dimension::Overworld)
                .expect("reader should see the completed persisted chunk");
            loaded_tx
                .send(loaded.stage)
                .expect("test should receive loaded stage");
        });

        started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("reader thread should start");
        assert!(
            loaded_rx.recv_timeout(Duration::from_millis(50)).is_err(),
            "a read must wait while an evicted dirty chunk is being saved"
        );

        eviction.save().expect("eviction write should succeed");
        assert_eq!(
            loaded_rx
                .recv_timeout(Duration::from_secs(1))
                .expect("reader should resume after the write"),
            GenStage::FULL.raw()
        );
        reader.join().expect("reader thread should finish cleanly");
    }

    #[test]
    fn failed_eviction_write_restores_the_dirty_chunk_to_cache() {
        let (store, _temp_dir) = test_store();
        let pos = ChunkPos::new(14, -9);
        let mut chunk = Chunk::new_empty();
        chunk.stage = GenStage::FULL.raw();
        chunk.mark_dirty();
        store
            .cache
            .insert(FullChunkPos::new(pos, Dimension::Overworld), chunk);

        let eviction = store
            .evict_chunk(FullChunkPos::new(pos, Dimension::Overworld))
            .unwrap_or_else(|| panic!("unreferenced chunk should be evicted"));
        let result = eviction.save_with(|_, _, _| {
            Err(WorldError::WorldGenerationError(
                "injected storage failure".to_string(),
            ))
        });

        assert!(result.is_err());
        let restored = store
            .cache
            .get(&FullChunkPos::new(pos, Dimension::Overworld))
            .expect("failed write should restore the chunk");
        assert!(restored.is_dirty());
        assert_eq!(restored.stage, GenStage::FULL.raw());
    }

    #[test]
    fn panicking_eviction_write_restores_the_dirty_chunk_to_cache() {
        let (store, _temp_dir) = test_store();
        let pos = ChunkPos::new(-19, 2);
        let mut chunk = Chunk::new_empty();
        chunk.stage = GenStage::FULL.raw();
        chunk.mark_dirty();
        store
            .cache
            .insert(FullChunkPos::new(pos, Dimension::Overworld), chunk);

        let eviction = store
            .evict_chunk(FullChunkPos::new(pos, Dimension::Overworld))
            .unwrap_or_else(|| panic!("unreferenced chunk should be evicted"));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = eviction.save_with(|_, _, _| panic!("injected persistence panic"));
        }));

        assert!(result.is_err());
        let restored = store
            .get_chunk(pos, Dimension::Overworld)
            .expect("panicking write should restore the chunk and release its reservation");
        assert!(restored.is_dirty());
        assert_eq!(restored.stage, GenStage::FULL.raw());
    }

    #[test]
    fn abandoned_eviction_restores_the_chunk_and_releases_its_reservation() {
        let (store, _temp_dir) = test_store();
        let pos = ChunkPos::new(-11, 6);
        let mut chunk = Chunk::new_empty();
        chunk.stage = GenStage::FULL.raw();
        chunk.mark_dirty();
        store
            .cache
            .insert(FullChunkPos::new(pos, Dimension::Overworld), chunk);

        let eviction = store
            .evict_chunk(FullChunkPos::new(pos, Dimension::Overworld))
            .unwrap_or_else(|| panic!("unreferenced chunk should be evicted"));
        drop(eviction);

        let restored = store
            .get_chunk(pos, Dimension::Overworld)
            .expect("dropping an unfinished eviction should restore readability");
        assert!(restored.is_dirty());
        assert_eq!(restored.stage, GenStage::FULL.raw());
    }

    #[test]
    fn failed_delete_restores_the_chunk_and_releases_its_reservation() {
        let (store, _temp_dir) = test_store();
        let pos = ChunkPos::new(7, 19);
        let mut chunk = Chunk::new_empty();
        chunk.stage = GenStage::FULL.raw();
        chunk.mark_dirty();
        store
            .cache
            .insert(FullChunkPos::new(pos, Dimension::Overworld), chunk);

        let deletion = store
            .begin_delete(pos, Dimension::Overworld)
            .unwrap_or_else(|| panic!("unreferenced chunk should be deletable"));
        let result = deletion.delete_with(|_, _| {
            Err(WorldError::WorldGenerationError(
                "injected delete failure".to_string(),
            ))
        });

        assert!(result.is_err());
        let restored = store
            .get_chunk(pos, Dimension::Overworld)
            .expect("failed delete should restore the chunk and release its reservation");
        assert!(restored.is_dirty());
        assert_eq!(restored.stage, GenStage::FULL.raw());
    }

    #[test]
    fn successful_delete_removes_cache_and_storage_state() {
        let (store, _temp_dir) = test_store();
        let pos = ChunkPos::new(-16, 23);
        let mut chunk = Chunk::new_empty();
        chunk.stage = GenStage::FULL.raw();
        store
            .insert_chunk(pos, Dimension::Overworld, chunk)
            .expect("test chunk should be persisted");
        store
            .get_chunk_mut(pos, Dimension::Overworld)
            .expect("persisted chunk should load")
            .mark_dirty();

        store
            .delete_chunk(pos, Dimension::Overworld)
            .expect("chunk deletion should complete");

        assert!(!store.is_chunk_cached(FullChunkPos::new(pos, Dimension::Overworld)));
        assert!(!store
            .chunk_exists(pos, Dimension::Overworld)
            .expect("storage existence should be readable"));
        assert!(matches!(
            store.get_chunk(pos, Dimension::Overworld),
            Err(WorldError::ChunkNotFound)
        ));
    }
}
