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
use temper_core::pos::FullChunkPos;
use temper_messages::load_chunk_entities::LoadChunkEntities;
use temper_state::GlobalStateResource;
use temper_world::chunks::load_chunk_internal;
use temper_world_format::Chunk;

/// Clear test cache state through the same lifecycle path production uses.
pub fn clear_cached_chunks(state: &temper_state::GlobalStateResource) {
    state.0.world.chunk_lifecycle().wait_for_pending_saves();
    state
        .0
        .world
        .chunk_lifecycle()
        .replace_owners(std::collections::HashMap::new());

    for pos in state.0.world.cached_chunk_keys() {
        let eviction = state
            .0
            .world
            .evict_chunk(pos)
            .unwrap_or_else(|| panic!("test chunk {pos} could not be evicted"));
        if eviction.needs_save() {
            eviction
                .save()
                .expect("test chunk should save before cache is cleared");
        } else {
            eviction
                .finish_clean()
                .expect("clean test chunk should finish eviction");
        }
    }

    assert!(state.0.world.cached_chunk_keys().is_empty());
}

/// `chunk_unloader` dispatches storage writes to the thread pool, so a chunk
/// evicted from the cache is not immediately readable from storage. Poll the
/// storage backend directly — going through `get_chunk` would hit the cache
/// and could pass without the write having landed.
pub fn wait_for_saved_chunk(state: &GlobalStateResource, pos: FullChunkPos) -> Chunk {
    let mut cycles = 2000;
    // CI can be slow so give it much more time
    if std::env::var("CI").is_ok_and(|v| v == "true") {
        cycles = 10000;
    }
    for _ in 0..cycles {
        if let Ok(chunk) = load_chunk_internal(
            &state.0.world.chunks.storage_backend,
            *pos,
            pos.dimension(),
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
        while let Some(chunk) = receiver.chunks.pop_loading() {
            receiver.chunks.mark_loaded(chunk);

            if state
                .0
                .world
                .chunk_exists(chunk)
                .expect("chunk existence check should succeed")
            {
                writer.write(LoadChunkEntities(chunk));
            }
        }
    }
}
