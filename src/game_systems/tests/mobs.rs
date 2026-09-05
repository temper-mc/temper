#[path = "mobs/chunk_visibility_lifecycle.rs"]
mod chunk_visibility_lifecycle;
#[path = "mobs/cross_chunk_persistence.rs"]
mod cross_chunk_persistence;
#[path = "mobs/entity_persistence.rs"]
mod entity_persistence;
#[path = "mobs/player_distance_reload.rs"]
mod player_distance_reload;
#[path = "mobs/spawn_mob_bundle.rs"]
mod spawn_mob_bundle;

use bevy_ecs::{
    message::MessageWriter,
    system::{Query, Res},
};
use temper_components::player::chunk_receiver::ChunkReceiver;
use temper_core::pos::ChunkPos;
use temper_messages::load_chunk_entities::LoadChunkEntities;
use temper_state::GlobalStateResource;
use temper_world::{Dimension, chunks::load_chunk_internal};
use temper_world_format::Chunk;

/// `chunk_unloader` dispatches storage writes to the thread pool, so a chunk
/// evicted from the cache is not immediately readable from storage. Poll the
/// storage backend directly — going through `get_chunk` would hit the cache
/// and could pass without the write having landed.
pub fn wait_for_saved_chunk(state: &GlobalStateResource, pos: ChunkPos) -> Chunk {
    let mut cycles = 200;
    // CI can be slow so give it much more time
    if std::env::var("CI").is_ok_and(|v| v == "true") {
        cycles = 1000;
    }
    for _ in 0..cycles {
        if let Ok(chunk) = load_chunk_internal(
            &state.0.world.chunks.storage_backend,
            pos,
            Dimension::Overworld,
            false,
        ) {
            return chunk;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("chunk {pos:?} never reached storage after unload");
}

pub fn emit_load_messages_for_known_chunks(
    state: Res<temper_state::GlobalStateResource>,
    mut query: Query<&mut ChunkReceiver>,
    mut writer: MessageWriter<LoadChunkEntities>,
) {
    for mut receiver in query.iter_mut() {
        while let Some(chunk) = receiver.loading.pop_front() {
            receiver.loaded.insert(chunk);

            if state
                .0
                .world
                .chunk_exists(chunk, Dimension::Overworld)
                .expect("chunk existence check should succeed")
            {
                writer.write(LoadChunkEntities(chunk));
            }
        }
    }
}
