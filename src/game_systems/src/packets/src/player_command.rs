use bevy_ecs::prelude::{Entity, Query, Res};
use temper_codec::net_types::var_int::VarInt;
use temper_components::player::player_identity::PlayerIdentity;
use temper_net_runtime::broadcast::broadcast_packet_except;
use temper_net_runtime::connection::StreamWriter;
use temper_protocol::PlayerCommandPacketReceiver;
use temper_protocol::incoming::player_command::PlayerCommandAction;
use temper_protocol::outgoing::entity_metadata::{EntityMetadata, EntityMetadataPacket};
use tracing::trace;

/// Handles PlayerCommand packets (sprinting, leave bed, etc.)
/// Note: Sneaking is handled via PlayerInput packet, NOT here
pub fn handle(
    receiver: Res<PlayerCommandPacketReceiver>,
    conn_query: Query<(Entity, &StreamWriter)>,
    identity_query: Query<&PlayerIdentity>,
) {
    for (event, eid) in receiver.0.try_iter() {
        // Get the sender's identity to use the correct entity ID
        let Ok(identity) = identity_query.get(eid) else {
            continue;
        };

        let entity_id = VarInt::new(identity.short_uuid);

        trace!(
            "PlayerCommand: {:?} from {} (entity_id={})",
            event.action, identity.username, identity.short_uuid
        );

        match event.action {
            PlayerCommandAction::StartSprinting => {
                let packet =
                    EntityMetadataPacket::new(entity_id, [EntityMetadata::entity_sprinting()]);
                broadcast_packet_except(eid, &packet, conn_query.iter());
            }
            PlayerCommandAction::StopSprinting => {
                let packet =
                    EntityMetadataPacket::new(entity_id, [EntityMetadata::entity_clear_state()]);
                broadcast_packet_except(eid, &packet, conn_query.iter());
            }
            _ => {}
        }
    }
}
