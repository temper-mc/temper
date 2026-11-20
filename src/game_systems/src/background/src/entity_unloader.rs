use bevy_ecs::prelude::{MessageWriter, Query, Res};
use std::collections::HashSet;
use temper_components::player::chunk_receiver::ChunkReceiver;
use temper_core::pos::FullChunkPos;
use temper_state::GlobalStateResource;

pub fn handle(
    state: Res<GlobalStateResource>,
    query: Query<&ChunkReceiver>,
    mut save_entity_writer: MessageWriter<temper_messages::save_chunk_entities::SaveChunkEntities>,
) {
    let mut all_chunks: HashSet<FullChunkPos> = HashSet::new();
    let mut visible_chunks = HashSet::new();
    'chunk_iter: for pos in state.0.world.cached_chunk_keys() {
        all_chunks.insert(pos);
        for chunk_receiver in query.iter() {
            if chunk_receiver.chunks.is_loaded(pos) {
                visible_chunks.insert(pos);
                continue 'chunk_iter;
            }
        }
    }
    for chunk_pos in all_chunks.difference(&visible_chunks) {
        save_entity_writer.write(temper_messages::save_chunk_entities::SaveChunkEntities(
            *chunk_pos,
        ));
    }
}
