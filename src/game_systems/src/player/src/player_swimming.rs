use bevy_ecs::prelude::*;
use bevy_math::DVec3;
use ionic_codec::net_types::var_int::VarInt;
use ionic_components::player::player_identity::PlayerIdentity;
use ionic_components::player::position::Position;
use ionic_components::player::swimming::SwimmingState;
use ionic_core::block_state_id::BlockStateId;
use ionic_core::pos::BlockPos;
use ionic_macros::match_block;
use ionic_net_runtime::connection::StreamWriter;
use ionic_protocol::outgoing::entity_metadata::{EntityMetadata, EntityMetadataPacket};
use ionic_state::GlobalStateResource;
use tracing::error;

/// Height of player's eyes from feet (blocks)
const PLAYER_EYE_HEIGHT: f64 = 1.62;

/// Check if a player is in water by testing at eye level
fn is_player_in_water(state: &ionic_state::GlobalState, pos: &Position) -> bool {
    let eye_pos = DVec3::new(pos.x, pos.y + PLAYER_EYE_HEIGHT, pos.z)
        .floor()
        .as_ivec3();

    let pos = BlockPos::of(eye_pos.x, eye_pos.y, eye_pos.z);

    let Ok(chunk) = state.world.get_or_generate_chunk(pos.chunk(), "overworld") else {
        error!(
            "Failed to get or generate chunk at position: {:?}",
            pos.chunk()
        );
        return false;
    };

    match_block!("water", chunk.get_block(pos.chunk_block_pos()))
}

/// System that detects when players enter/exit water and updates their swimming state
/// Also broadcasts the swimming pose to all connected clients
pub fn detect_player_swimming(
    mut swimmers: Query<(&PlayerIdentity, &Position, &mut SwimmingState)>,
    all_connections: Query<(Entity, &StreamWriter)>,
    state: Res<GlobalStateResource>,
) {
    for (identity, pos, mut swimming_state) in swimmers.iter_mut() {
        let in_water = is_player_in_water(&state.0, pos);

        if in_water && !swimming_state.is_swimming {
            swimming_state.is_swimming = true;

            let entity_id = VarInt::new(identity.short_uuid);
            let packet = EntityMetadataPacket::new(
                entity_id,
                [
                    EntityMetadata::entity_swimming_state(),
                    EntityMetadata::entity_swimming_pose(),
                ],
            );

            broadcast_metadata(&packet, &all_connections, &state);
        } else if !in_water && swimming_state.is_swimming {
            swimming_state.is_swimming = false;

            let entity_id = VarInt::new(identity.short_uuid);
            let packet = EntityMetadataPacket::new(
                entity_id,
                [
                    EntityMetadata::entity_clear_state(),
                    EntityMetadata::entity_standing(),
                ],
            );

            broadcast_metadata(&packet, &all_connections, &state);
        }
    }
}

/// Helper function to broadcast entity metadata to all connected players
fn broadcast_metadata(
    packet: &EntityMetadataPacket,
    connections: &Query<(Entity, &StreamWriter)>,
    state: &GlobalStateResource,
) {
    for (entity, conn) in connections {
        if !state.0.players.is_connected(entity) {
            continue;
        }
        if let Err(err) = conn.send_packet_ref(packet) {
            error!("Failed to send entity metadata packet: {:?}", err);
        }
    }
}
