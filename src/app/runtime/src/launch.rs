//! Launch utilities for server initialization, chunk generation, and world import.

use crate::errors::BinaryError;
use std::time::Instant;
use temper_config::server_config::get_global_config;
use temper_core::pos::ChunkPos;
use temper_state::player_list::PlayerList;
use temper_state::{GlobalState, ServerState};
use temper_threadpool::ThreadPool;
use temper_world::World;
use tracing::{error, info};

/// Creates the initial server state with all required components.
pub fn create_state(start_time: Instant) -> Result<ServerState, BinaryError> {
    // Fixed seed for world generation. This seed ensures you spawn above land at the default spawn point.
    const SEED: u64 = 380;
    Ok(ServerState {
        world: World::new(&get_global_config().database.db_path, SEED),
        shut_down: false.into(),
        players: PlayerList::default(),
        thread_pool: ThreadPool::new(),
        start_time,
    })
}

/// Generates spawn chunks around the origin if they don't exist.
pub fn generate_spawn_chunks(state: GlobalState) -> Result<(), BinaryError> {
    info!("No overworld spawn chunk found, generating spawn chunks...");

    let start = Instant::now();
    let radius = get_global_config().chunk_render_distance as i32;

    // Collect all chunk coordinates to generate
    let chunks: Vec<(i32, i32)> = (-radius..=radius)
        .flat_map(|x| (-radius..=radius).map(move |z| (x, z)))
        .collect();

    let mut batch = state.thread_pool.batch();
    for (x, z) in chunks {
        let state_clone = state.clone();
        batch.execute(move || {
            let pos = ChunkPos::new(x, z);
            let chunk = state_clone.world.world_generator.generate_chunk(pos);

            match chunk {
                Ok(chunk) => {
                    if let Err(e) = state_clone.world.insert_chunk(pos, "overworld", chunk) {
                        error!("Error saving chunk ({}, {}): {:?}", x, z, e);
                    }
                }
                Err(e) => {
                    error!("Error generating chunk ({}, {}): {:?}", x, z, e);
                }
            }
        });
    }
    batch.wait();

    info!("Finished generating spawn chunks in {:?}", start.elapsed());
    Ok(())
}
