use bevy_ecs::prelude::{MessageReader, Query, Res};
use std::collections::HashSet;
use temper_components::player::chunk_receiver::ChunkReceiver;
use temper_components::player::client_information::ClientInformationComponent;
use temper_components::player::position::Position;
use temper_core::pos::FullChunkPos;
use temper_messages::chunk_calc::ChunkCalc;
use temper_state::GlobalStateResource;
use tracing::warn;

pub fn handle(
    mut messages: MessageReader<ChunkCalc>,
    mut query: Query<(&Position, &mut ChunkReceiver, &ClientInformationComponent)>,
    state: Res<GlobalStateResource>,
) {
    for message in messages.read() {
        let (position, mut chunk_receiver, client_info) = match query.get_mut(message.0) {
            Ok(data) => data,
            Err(_) => {
                warn!("Player does not exist, skipping chunk calculation");
                continue;
            }
        };

        let server_render_distance = state.0.config.chunk_render_distance as i32;
        let client_view_distance = i32::from(client_info.view_distance);
        let radius = server_render_distance.min(client_view_distance);
        let player_chunk = position.chunk();

        // 1. Build the absolute set of chunks the player currently needs
        let mut needed_set = HashSet::new();
        for x in player_chunk.x() - radius..=player_chunk.x() + radius {
            for z in player_chunk.z() - radius..=player_chunk.z() + radius {
                needed_set.insert(FullChunkPos::of(x, z, position.dimension));
            }
        }

        state
            .0
            .world
            .chunk_lifecycle()
            .set_owner_chunks(message.0.to_bits(), needed_set.iter().copied());

        chunk_receiver.chunks.set_needed(&needed_set);

        // Prioritize the closest queued chunks first.
        chunk_receiver.chunks.sort_loading_by_distance(player_chunk);
    }
}
