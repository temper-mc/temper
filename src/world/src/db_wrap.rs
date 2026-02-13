use crate::{MutChunk, RefChunk, World};
use temper_core::pos::ChunkPos;
use temper_world_format::errors::WorldError;
use temper_world_format::Chunk;
use tracing::trace;
use world_db::chunks::{
    chunk_exists_internal, delete_chunk_internal, load_chunk_batch_internal, load_chunk_internal,
    save_chunk_internal, sync_internal,
};

impl World {
    /// Save a chunk to the storage backend
    ///
    /// This function will save a chunk to the storage backend and update the cache with the new
    /// chunk data. If the chunk already exists in the cache, it will be updated with the new data.
    pub fn insert_chunk(
        &self,
        pos: ChunkPos,
        dimension: &str,
        chunk: Chunk,
    ) -> Result<(), WorldError> {
        let mut chunk = chunk;
        chunk.sections.iter_mut().for_each(|c| c.dirty = false);
        save_chunk_internal(&self.storage_backend, pos, dimension, &chunk)?;
        // self.cache.insert((pos, dimension.to_string()), chunk);
        Ok(())
    }

    /// Load a chunk from the storage backend. If the chunk is in the cache, it will be returned
    /// from the cache instead of the storage backend. If the chunk is not in the cache, it will be
    /// loaded from the storage backend and inserted into the cache.
    pub fn get_chunk(&'_ self, pos: ChunkPos, dimension: &str) -> Result<RefChunk<'_>, WorldError> {
        if let Some(chunk) = self.cache.get(&(pos, dimension.to_string())) {
            return Ok(chunk);
        }
        let chunk = load_chunk_internal(&self.storage_backend, pos, dimension);
        match chunk {
            Ok(c) => {
                self.cache.insert((pos, dimension.to_string()), c);
                Ok(self
                    .cache
                    .get(&(pos, dimension.to_string()))
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
        dimension: &'_ str,
    ) -> Result<MutChunk<'_>, WorldError> {
        if let Some(chunk) = self.cache.get_mut(&(pos, dimension.to_string())) {
            return Ok(chunk);
        }
        let chunk = load_chunk_internal(&self.storage_backend, pos, dimension);
        match chunk {
            Ok(c) => {
                self.cache.insert((pos, dimension.to_string()), c);
                Ok(self
                    .cache
                    .get_mut(&(pos, dimension.to_string()))
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
    pub fn chunk_exists(&self, pos: ChunkPos, dimension: &str) -> Result<bool, WorldError> {
        if self.cache.contains_key(&(pos, dimension.to_string())) {
            return Ok(true);
        }
        chunk_exists_internal(&self.storage_backend, pos, dimension)
    }

    /// Delete a chunk from the storage backend.
    ///
    /// This function will remove the chunk from the cache and delete it from the storage backend.
    pub fn delete_chunk(&self, pos: ChunkPos, dimension: &str) -> Result<(), WorldError> {
        self.cache.remove(&(pos, dimension.to_string()));
        delete_chunk_internal(&self.storage_backend, pos, dimension)
    }

    /// Sync the storage backend.
    ///
    /// This function will save all chunks in the cache to the storage backend and then sync the
    /// storage backend. This should be run after inserting or updating a large number of chunks
    /// to ensure that the data is properly saved to disk.
    pub fn sync(&self) -> Result<(), WorldError> {
        for pair in self.cache.iter() {
            let k = pair.key();
            let v = pair.value();
            if v.sections.iter().any(|c| c.dirty) {
                trace!("Chunk at {:?} is dirty, saving.", k.0);
            } else {
                continue;
            }
            trace!("Syncing chunk: {:?}", k.0);
            save_chunk_internal(&self.storage_backend, k.0, &k.1, v)?;
        }

        sync_internal(&self.storage_backend)
    }

    /// Load a batch of chunks from the storage backend.
    ///
    /// This function attempts to load as many chunks as it can find from the cache first, then fetches
    /// the missing chunks from the storage backend. The chunks are then inserted into the cache and
    /// returned as a vector.
    pub fn load_chunk_batch(
        &'_ self,
        coords: &'_ [(ChunkPos, &'_ str)],
    ) -> Result<Vec<RefChunk<'_>>, WorldError> {
        let mut found_chunks = Vec::new();
        let mut missing_chunks = Vec::new();
        for coord in coords {
            if let Some(chunk) = self.cache.get(&(coord.0, coord.1.to_string())) {
                found_chunks.push(chunk);
            } else {
                missing_chunks.push(*coord);
            }
        }
        let fetched = load_chunk_batch_internal(&self.storage_backend, &missing_chunks)?;
        for (chunk, (pos, dimension)) in fetched.into_iter().zip(missing_chunks) {
            self.cache.insert((pos, dimension.to_string()), chunk);
            let found_chunk = self
                .cache
                .get(&(pos, dimension.to_string()))
                .expect("Chunk was just inserted into the cache");
            found_chunks.push(found_chunk);
        }
        Ok(found_chunks)
    }

    pub fn load_chunk_batch_mut(
        &'_ self,
        coords: &'_ [(ChunkPos, &'_ str)],
    ) -> Result<Vec<MutChunk<'_>>, WorldError> {
        let mut found_chunks = Vec::new();
        let mut missing_chunks = Vec::new();
        for coord in coords {
            if let Some(chunk) = self.cache.get_mut(&(coord.0, coord.1.to_string())) {
                found_chunks.push(chunk);
            } else {
                missing_chunks.push(*coord);
            }
        }
        let fetched = load_chunk_batch_internal(&self.storage_backend, &missing_chunks)?;
        for (chunk, (pos, dimension)) in fetched.into_iter().zip(missing_chunks) {
            self.cache.insert((pos, dimension.to_string()), chunk);
            let found_chunk = self
                .cache
                .get_mut(&(pos, dimension.to_string()))
                .expect("Chunk was just inserted into the cache");
            found_chunks.push(found_chunk);
        }
        Ok(found_chunks)
    }
}
