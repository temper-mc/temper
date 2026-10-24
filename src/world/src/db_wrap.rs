use crate::{ChunkStore, MutChunk, RefChunk, World};
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
        chunk.clear_dirty();
        save_chunk_internal(&self.storage_backend, pos, dimension, &chunk)?;
        // self.cache.insert((pos, dimension.to_string()), chunk);
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
        let pos = FullChunkPos::new(pos, dimension);
        if let Some(chunk) = self.cache.get(&pos) {
            return Ok(chunk);
        }
        let chunk = load_chunk_internal(&self.storage_backend, *pos, dimension, self.verify);
        match chunk {
            Ok(c) => {
                self.cache.insert(pos, c);
                Ok(self
                    .cache
                    .get(&pos)
                    .expect("Chunk was just inserted into the cache"))
            }
            Err(e) => Err(e),
        }
    }

    /// Load a mutable chunk from the storage backend. If the chunk is in the cache, it will be returned
    /// from the cache instead of the storage backend. If the chunk is not in the cache, it will be
    /// loaded from the storage backend and inserted into the cache.
    pub fn get_chunk_mut(
        &'_ self,
        pos: ChunkPos,
        dimension: Dimension,
    ) -> Result<MutChunk<'_>, WorldError> {
        let pos = FullChunkPos::new(pos, dimension);
        if let Some(chunk) = self.cache.get_mut(&pos) {
            return Ok(chunk);
        }
        let chunk = load_chunk_internal(&self.storage_backend, *pos, dimension, self.verify);
        match chunk {
            Ok(c) => {
                self.cache.insert(pos, c);
                Ok(self
                    .cache
                    .get_mut(&pos)
                    .expect("Chunk was just inserted into the cache"))
            }
            Err(e) => Err(e),
        }
    }

    /// Check if a chunk exists in the storage backend.
    ///
    /// It will first check if the chunk is in the cache and if it is, it will return true. If the
    /// chunk is not in the cache, it will check the storage backend for the chunk, returning true
    /// if it exists and false if it does not.
    pub fn chunk_exists(&self, pos: ChunkPos, dimension: Dimension) -> Result<bool, WorldError> {
        if self.cache.contains_key(&FullChunkPos::new(pos, dimension)) {
            return Ok(true);
        }
        chunk_exists_internal(&self.storage_backend, pos, dimension)
    }

    /// Delete a chunk from the storage backend.
    ///
    /// This function will remove the chunk from the cache and delete it from the storage backend.
    pub fn delete_chunk(&self, pos: ChunkPos, dimension: Dimension) -> Result<(), WorldError> {
        self.cache.remove(&FullChunkPos::new(pos, dimension));
        delete_chunk_internal(&self.storage_backend, pos, dimension)
    }

    /// Sync the storage backend.
    ///
    /// This function will save fully generated dirty chunks in the cache to the storage backend and
    /// then sync the storage backend. This should be run after inserting or updating a large number
    /// of chunks to ensure that the data is properly saved to disk.
    pub fn sync(&self, minimum_stage: u8) -> Result<(), WorldError> {
        let mut snapshots = Vec::new();

        for pair in self.cache.iter() {
            let k = pair.key();
            let v = pair.value();
            if !v.is_dirty() {
                continue;
            }
            if v.stage < minimum_stage {
                continue;
            }

            snapshots.push(ChunkSaveSnapshot {
                pos: **k,
                dimension: k.dimension(),
                chunk: v.clone_without_transient_noise(),
            });
            v.clear_dirty();
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

        sync_internal(&self.storage_backend)
    }
}

impl World {
    pub fn insert_chunk(&self, pos: FullChunkPos, chunk: Chunk) -> Result<(), WorldError> {
        self.chunks.insert_chunk(*pos, pos.dimension(), chunk)
    }

    pub fn get_chunk(&'_ self, pos: FullChunkPos) -> Result<RefChunk<'_>, WorldError> {
        self.chunks.get_chunk(*pos, pos.dimension())
    }

    pub fn get_chunk_mut(&'_ self, pos: FullChunkPos) -> Result<MutChunk<'_>, WorldError> {
        self.chunks.get_chunk_mut(*pos, pos.dimension())
    }

    pub fn chunk_exists(&self, pos: FullChunkPos) -> Result<bool, WorldError> {
        self.chunks.chunk_exists(*pos, pos.dimension())
    }

    pub fn delete_chunk(&self, pos: FullChunkPos) -> Result<(), WorldError> {
        self.chunks.delete_chunk(*pos, pos.dimension())
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
            .get_cache()
            .get(&FullChunkPos::new(pos, Dimension::Overworld))
            .expect("skipped chunk should stay cached");
        assert!(
            cached.is_dirty(),
            "skipped partial chunks should remain dirty for a later full save"
        );

        drop(cached);
        store.get_cache().clear();
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
            .get_cache()
            .get(&FullChunkPos::new(pos, Dimension::Overworld))
            .expect("saved chunk should stay cached");
        assert!(
            !cached.is_dirty(),
            "saved full chunks should be marked clean after snapshotting"
        );

        drop(cached);
        store.get_cache().clear();
        let loaded = store
            .get_chunk(pos, Dimension::Overworld)
            .expect("full chunk should reload from storage");
        assert_eq!(loaded.stage, GenStage::FULL.raw());
    }
}
