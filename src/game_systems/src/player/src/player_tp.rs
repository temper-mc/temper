use bevy_ecs::prelude::{Entity, MessageReader, MessageWriter, Query};
use ionic_components::player::player_identity::PlayerIdentity;
use ionic_components::player::position::Position;
use ionic_components::player::teleport_tracker::TeleportTracker;
use ionic_messages::chunk_calc::ChunkCalc;
use ionic_messages::entity_update::SendEntityUpdate;
use ionic_messages::teleport_player::TeleportPlayer;
use ionic_net_runtime::connection::StreamWriter;
use ionic_protocol::outgoing::entity_position_sync::TeleportEntityPacket;
use ionic_protocol::outgoing::synchronize_player_position::SynchronizePlayerPositionPacket;
use tracing::error;

pub fn teleport_player(
    mut query: Query<(Entity, &StreamWriter, &mut Position, &mut TeleportTracker)>,
    id_query: Query<&PlayerIdentity>,
    mut message_reader: MessageReader<TeleportPlayer>,
    mut chunk_calc_msg: MessageWriter<ChunkCalc>,
    mut player_update_msg: MessageWriter<SendEntityUpdate>,
) {
    for message in message_reader.read() {
        let message_entity = message.entity;
        let id = match id_query.get(message_entity) {
            Ok(id) => id,
            Err(err) => {
                error!(
                    "Failed to get PlayerIdentity for entity {:?}: {}",
                    message_entity, err
                );
                continue;
            }
        };
        for (entity, conn, mut pos, mut tracker) in query.iter_mut() {
            if entity == message_entity {
                // Block movement tracking until the player has been teleported
                tracker.waiting_for_confirm = true;
                pos.x = message.x;
                pos.y = message.y;
                pos.z = message.z;
                // If it's the entity we are trying to teleport, send the sync player pos packet
                if let Err(err) = conn.send_packet(SynchronizePlayerPositionPacket {
                    teleport_id: rand::random::<i32>().into(),
                    x: message.x,
                    y: message.y,
                    z: message.z,
                    vel_x: message.vel_x,
                    vel_y: message.vel_y,
                    vel_z: message.vel_z,
                    yaw: message.yaw,
                    pitch: message.pitch,
                    flags: 0,
                }) {
                    error!("Failed to send teleport packet: {}", err);
                    continue;
                }
            } else {
                // Otherwise send teleport entity packet. This ideally should be handled by the send
                // entity updates system, but it seems to be a bit buggy
                if let Err(err) = conn.send_packet(TeleportEntityPacket {
                    entity_id: id.short_uuid.into(),
                    x: message.x,
                    y: message.y,
                    z: message.z,
                    vel_x: 0.0,
                    vel_y: 0.0,
                    vel_z: 0.0,
                    yaw: message.yaw,
                    pitch: message.pitch,
                    on_ground: false,
                }) {
                    error!("Failed to send teleport packet: {}", err);
                    continue;
                }
            }
        }

        // Notify the player update system to send the new position to the client
        player_update_msg.write(SendEntityUpdate(message_entity));

        // Notify the chunk calculation system to recalculate chunks for this player
        chunk_calc_msg.write(ChunkCalc(message_entity));
    }
}
