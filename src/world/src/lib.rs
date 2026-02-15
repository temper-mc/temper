mod db_wrap;
mod importing;
pub mod player;

use dashmap::DashMap;
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};
use std::process::exit;
use temper_config::server_config::get_global_config;
use temper_core::pos::ChunkPos;
use temper_general_purpose::paths::get_root_path;
use temper_storage::lmdb::LmdbBackend;
use temper_world_format::errors::WorldError;
use temper_world_format::Chunk;
use tracing::{error, warn};
pub use world_db::*;
pub use world_gen;
use world_gen::WorldGenerator;
use wyhash::WyHasherBuilder;

#[derive(Clone)]
pub struct World {
    storage_backend: LmdbBackend,
    cache: ChunkCache,
    pub world_generator: WorldGenerator,
}

impl World {
    /// Creates a new world instance.
    ///
    /// You'd probably want to call this at the start of your program. And then use the returned
    /// in a state struct or something.
    pub fn new(backend_path: impl Into<PathBuf>, seed: u64) -> Self {
        if let Err(e) = check_config_validity() {
            error!("Fatal error in database config: {}", e);
            exit(1);
        }
        let mut backend_path = backend_path.into();
        // Clones are kinda ok here since this is only run once at startup.
        if backend_path.is_relative() {
            backend_path = get_root_path().join(backend_path);
        }
        // Convert the map size from GB to bytes and round it to the nearest page size.
        let map_size = get_global_config().database.map_size as usize * 1024 * 1024 * 1024;
        let storage_backend = LmdbBackend::initialize(Some(backend_path), map_size)
            .expect("Failed to initialize database");

        let rand_seed = rand::random();

        let cache = ChunkCache::with_hasher(WyHasherBuilder::new(rand_seed));
        let world_generator = WorldGenerator::new(seed);

        World {
            storage_backend,
            cache,
            world_generator,
        }
    }

    pub fn get_cache(&self) -> &ChunkCache {
        &self.cache
    }

    /// Loads a chunk from the database or cache, or generates it if it doesn't exist.
    pub fn get_or_generate_chunk<'a>(
        &'a self,
        chunk_pos: ChunkPos,
        dimension: &'a str,
    ) -> Result<RefChunk<'a>, WorldError> {
        if self.chunk_exists(chunk_pos, dimension)? {
            self.get_chunk(chunk_pos, dimension)
        } else {
            let chunk = self
                .world_generator
                .generate_chunk(chunk_pos)
                .map_err(|err| {
                    WorldError::WorldGenerationError(format!(
                        "Failed to generate chunk at {:?}: {}",
                        chunk_pos, err
                    ))
                })?;
            self.insert_chunk(chunk_pos, dimension, chunk)?;
            self.get_chunk(chunk_pos, dimension)
        }
    }

    /// Loads a chunk from the database or cache, or generates it if it doesn't exist. Returns a mutable reference.
    pub fn get_or_generate_mut<'a>(
        &'a self,
        chunk_pos: temper_core::pos::ChunkPos,
        dimension: &'a str,
    ) -> Result<MutChunk<'a>, WorldError> {
        if self.chunk_exists(chunk_pos, dimension)? {
            self.get_chunk_mut(chunk_pos, dimension)
        } else {
            let chunk = self
                .world_generator
                .generate_chunk(chunk_pos)
                .map_err(|err| {
                    temper_world_format::errors::WorldError::WorldGenerationError(format!(
                        "Failed to generate chunk at {:?}: {}",
                        chunk_pos, err
                    ))
                })?;
            self.insert_chunk(chunk_pos, dimension, chunk)?;
            self.get_chunk_mut(chunk_pos, dimension)
        }
    }
}

type ChunkCache = DashMap<(ChunkPos, String), Chunk, WyHasherBuilder>;
pub type MutChunk<'a> = dashmap::mapref::one::RefMut<'a, (ChunkPos, String), Chunk>;
pub type RefChunk<'a> = dashmap::mapref::one::Ref<'a, (ChunkPos, String), Chunk>;

fn check_config_validity() -> Result<(), WorldError> {
    // We don't actually check if the import path is valid here since that would brick a server
    // if the world is imported then deleted after the server starts. Those checks are handled in
    // the importing logic.

    let config = get_global_config();
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
    use temper_core::pos::ChunkPos;

    #[test]
    #[ignore]
    fn dump_chunk() {
        let world = World::new(std::env::current_dir().unwrap().join("../../../world"), 0);
        let chunk = world.get_chunk(ChunkPos::new(1, 1), "overworld").expect(
            "Failed to load chunk. If it's a bitcode error, chances are the chunk format \
             has changed since last generating a world so you'll need to regenerate",
        );
        let encoded = bitcode::encode(&*chunk);
        std::fs::write("../../../.etc/raw_chunk.dat", encoded).unwrap();
    }
}
