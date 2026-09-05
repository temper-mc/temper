use bevy_ecs::prelude::{Entity, Query, Res};
use temper_components::entity_identity::Identity;
use temper_components::game_id::GameID;
use temper_components::player::entity_tracker::EntityTracker;
use temper_net_runtime::connection::StreamWriter;
use temper_protocol::PlayerCommandPacketReceiver;
use temper_protocol::incoming::player_command::PlayerCommandAction;
use temper_protocol::outgoing::entity_metadata::{EntityMetadata, EntityMetadataPacket};
use tracing::trace;

/// Handles PlayerCommand packets (sprinting, leave bed, etc.)
/// Note: Sneaking is handled via PlayerInput packet, NOT here
pub fn handle(
    receiver: Res<PlayerCommandPacketReceiver>,
    conn_query: Query<(Entity, &StreamWriter, &EntityTracker)>,
    identity_query: Query<(&Identity, &GameID)>,
) {
    for (event, eid) in receiver.0.try_iter() {
        // Get the sender's identity to use the correct entity ID
        let Ok((identity, game_id)) = identity_query.get(eid) else {
            continue;
        };

        let entity_id = game_id.get();

        trace!(
            "PlayerCommand: {:?} from {} (entity_id={})",
            event.action,
            identity.name.as_ref().expect("No Player Name"),
            game_id.get()
        );

        match event.action {
            PlayerCommandAction::StartSprinting => {
                let packet =
                    EntityMetadataPacket::new(entity_id, [EntityMetadata::entity_sprinting()]);
                for (recipient, writer, tracker) in conn_query.iter() {
                    if recipient == eid || !writer.is_running() || !tracker.tracking.contains(&eid)
                    {
                        continue;
                    }
                    if let Err(err) = writer.send_packet_ref(&packet) {
                        tracing::error!("Failed to send sprint-start metadata packet: {:?}", err);
                    }
                }
            }
            PlayerCommandAction::StopSprinting => {
                let packet =
                    EntityMetadataPacket::new(entity_id, [EntityMetadata::entity_clear_state()]);
                for (recipient, writer, tracker) in conn_query.iter() {
                    if recipient == eid || !writer.is_running() || !tracker.tracking.contains(&eid)
                    {
                        continue;
                    }
                    if let Err(err) = writer.send_packet_ref(&packet) {
                        tracing::error!("Failed to send sprint-stop metadata packet: {:?}", err);
                    }
                }
            }
            _ => {}
        }
    }
}
