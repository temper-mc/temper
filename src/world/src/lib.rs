mod db_wrap;
mod generation;
mod helpers;
mod importing;
pub mod player;

use dashmap::DashMap;
pub use generation::WorldChunkGenerator;
use std::fs::create_dir_all;
use std::hash::{BuildHasher, Hasher};
use std::path::{Path, PathBuf};
use std::process::exit;
use temper_config::ServerConfig;
pub use temper_core::dimension::Dimension;
use temper_core::pos::ChunkPos;
use temper_general_purpose::paths::get_root_path;
use temper_storage::lmdb::StorageBackend;
pub use temper_world_format::errors::WorldError;
use temper_world_format::errors::WorldError::InvalidWorldGenerator;
use temper_world_format::Chunk;
use tracing::{error, warn};
pub use world_db::*;
pub use world_gen;
use wyhash::WyHasherBuilder;

#[derive(Clone)]
pub struct World {
    pub chunks: ChunkStore,
    pub chunk_generator: WorldChunkGenerator,
}

#[derive(Clone)]
pub struct ChunkStore {
    pub storage_backend: StorageBackend,
    cache: ChunkCache,
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

        let seed = if let Ok(seed) = config.world_gen.seed.parse::<u64>() {
            seed
        } else {
            let mut hasher = wyhash::WyHasherBuilder::default().build_hasher();
            hasher.write(&config.world_gen.seed.clone().into_bytes());
            hasher.finish()
        };

        let chunks = ChunkStore::new(
            storage_backend,
            config.database.verify_chunk_data,
            WyHasherBuilder::new(seed),
        );
        let chunk_generator = WorldChunkGenerator::from_name(&config.world_gen.generator, seed);

        if let Some(chunk_generator) = chunk_generator {
            Ok(World {
                chunks,
                chunk_generator,
            })
        } else {
            Err(InvalidWorldGenerator(
                match config.world_gen.generator.as_str() {
                    "" => "<empty string>".to_string(),
                    other => other.to_string(),
                },
            ))
        }
    }

    pub fn get_cache(&self) -> &ChunkCache {
        self.chunks.get_cache()
    }

    pub fn final_generation_stage(&self) -> u8 {
        self.chunk_generator.final_stage().raw()
    }

    pub fn is_fully_generated(&self, chunk: &Chunk) -> bool {
        chunk.stage >= self.final_generation_stage()
    }

    /// Loads a chunk from the database or cache, generating or advancing it first if needed.
    pub fn get_or_generate_chunk(
        &'_ self,
        chunk_pos: ChunkPos,
        dimension: Dimension,
    ) -> Result<RefChunk<'_>, WorldError> {
        self.chunk_generator
            .generate(&self.chunks, dimension, chunk_pos)?;
        self.get_chunk(chunk_pos, dimension)
    }

    /// Loads a chunk from the database or cache, generating or advancing it first if needed. Returns a mutable reference.
    pub fn get_or_generate_mut(
        &self,
        chunk_pos: ChunkPos,
        dimension: Dimension,
    ) -> Result<MutChunk<'_>, WorldError> {
        self.chunk_generator
            .generate(&self.chunks, dimension, chunk_pos)?;
        self.get_chunk_mut(chunk_pos, dimension)
    }
}

impl ChunkStore {
    pub fn new(storage_backend: StorageBackend, verify: bool, hasher: WyHasherBuilder) -> Self {
        Self {
            storage_backend,
            cache: ChunkCache::with_hasher(hasher),
            verify,
        }
    }

    pub fn get_cache(&self) -> &ChunkCache {
        &self.cache
    }
}

pub type ChunkCache = DashMap<(ChunkPos, Dimension), Chunk, WyHasherBuilder>;
pub type MutChunk<'a> = dashmap::mapref::one::RefMut<'a, (ChunkPos, Dimension), Chunk>;
pub type RefChunk<'a> = dashmap::mapref::one::Ref<'a, (ChunkPos, Dimension), Chunk>;

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
    use temper_core::pos::ChunkPos;

    #[test]
    #[ignore]
    fn dump_chunk() {
        let world = World::new(
            std::env::current_dir().unwrap().join("../../../world"),
            &create_dummy_config(),
        )
        .unwrap();
        let chunk = world
            .get_chunk(ChunkPos::new(1, 1), Dimension::Overworld)
            .expect(
                "Failed to load chunk. If it's a bitcode error, chances are the chunk format \
             has changed since last generating a world so you'll need to regenerate",
            );
        let encoded = bitcode::serialize(&*chunk).unwrap();
        std::fs::write("../../../.etc/raw_chunk.dat", encoded).unwrap();
    }
}
