use bevy_ecs::prelude::{Entity, Query, Res};
use ionic_components::player::position::Position;
use ionic_core::block_state_id::BlockStateId;
use ionic_core::pos::BlockPos;
use ionic_macros::match_block;
use ionic_net_runtime::connection::StreamWriter;
use ionic_protocol::outgoing::synchronize_player_position::SynchronizePlayerPositionPacket;
use ionic_protocol::PlayerLoadedReceiver;
use ionic_state::GlobalStateResource;
use tracing::warn;

pub fn handle(
    ev: Res<PlayerLoadedReceiver>,
    state: Res<GlobalStateResource>,
    query: Query<(Entity, &Position, &StreamWriter)>,
) {
    for (_, player) in ev.0.try_iter() {
        let Ok((entity, player_pos, conn)) = query.get(player) else {
            warn!("Player position not found in query.");
            continue;
        };
        if !state.0.players.is_connected(entity) {
            warn!(
                "Player {} is not connected, skipping position synchronization.",
                player
            );
            continue;
        }
        let pos = BlockPos::of(
            player_pos.x as i32,
            player_pos.y as i32,
            player_pos.z as i32,
        );
        let chunk_pos = pos.chunk();
        let Ok(chunk) = state.0.world.get_or_generate_chunk(chunk_pos, "overworld") else {
            warn!(
                "Failed to get or generate chunk for player {} at position: ({}, {}, {})",
                player, player_pos.x, player_pos.y, player_pos.z
            );
            continue;
        };
        let head_block = chunk.get_block(pos.chunk_block_pos());
        if match_block!("air", head_block) || match_block!("cave_air", head_block) {
            tracing::info!(
                "Player {} loaded at position: ({}, {}, {})",
                player,
                player_pos.x,
                player_pos.y,
                player_pos.z
            );
        } else {
            tracing::info!(
                "Player {} loaded at position: ({}, {}, {}) with head block: {:?}",
                player,
                player_pos.x,
                player_pos.y,
                player_pos.z,
                head_block
            );
            // Teleport the player to the world center if their head block is not air
            let packet = SynchronizePlayerPositionPacket::default();
            if let Err(e) = conn.send_packet_ref(&packet) {
                tracing::error!(
                    "Failed to send synchronize player position packet for player {}: {:?}",
                    player,
                    e
                );
            } else {
                tracing::info!(
                    "Sent synchronize player position packet for player {}",
                    player
                );
            }
        }
    }
}
