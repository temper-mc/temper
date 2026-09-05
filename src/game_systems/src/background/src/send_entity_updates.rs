use bevy_ecs::prelude::{Entity, MessageReader, Query};
use temper_codec::net_types::angle::NetAngle;
use temper_components::player::entity_tracker::EntityTracker;
use temper_components::player::grounded::OnGround;
use temper_components::player::position::Position;
use temper_components::player::rotation::Rotation;
use temper_components::player::velocity::Velocity;
use temper_entities::LastSyncedPosition;
use temper_messages::entity_update::SendEntityUpdate;
use temper_net_runtime::connection::StreamWriter;
use temper_protocol::outgoing::entity_position_sync::TeleportEntityPacket;
use temper_protocol::outgoing::update_entity_position_and_rotation::UpdateEntityPositionAndRotationPacket;
use tracing::warn;
use temper_components::game_id::GameID;

pub fn handle(
    mut query: Query<(
        Entity,
        &Position,
        &Velocity,
        &Rotation,
        &mut LastSyncedPosition,
        &GameID,
        &OnGround,
    )>,
    mut player_query: Query<(Entity, &StreamWriter, &EntityTracker)>,
    mut reader: MessageReader<SendEntityUpdate>,
) {
    let mut entities_to_update = vec![];
    for msg in reader.read() {
        entities_to_update.push(msg.0);
    }
    for entity in entities_to_update {
        if let Ok((entity, pos, vel, rot, mut last_synced, game_id, grounded)) =
            query.get_mut(entity)
        {
            if last_synced.0.distance(pos.coords) >= 8.0 {
                let packet = TeleportEntityPacket {
                    entity_id: game_id.get(),
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                    vel_x: f64::from(vel.x),
                    vel_y: f64::from(vel.y),
                    vel_z: f64::from(vel.z),
                    yaw: rot.yaw,
                    pitch: rot.pitch,
                    on_ground: grounded.0,
                };
                for (recipient_entity, conn, tracker) in player_query.iter_mut() {
                    if recipient_entity == entity || !tracker.tracking.contains(&entity) {
                        continue;
                    }
                    if let Err(e) = conn.send_packet_ref(&packet) {
                        warn!(
                            "Failed to send teleport packet for entity {:?}: {:?}",
                            entity, e
                        );
                    }
                }
            } else {
                let (delta_x, delta_y, delta_z) = {
                    let delta = pos.coords - last_synced.0;
                    (
                        (delta.x * 4096.0).round() as i16,
                        (delta.y * 4096.0).round() as i16,
                        (delta.z * 4096.0).round() as i16,
                    )
                };
                let packet = UpdateEntityPositionAndRotationPacket {
                    entity_id: game_id.get(),
                    delta_x,
                    delta_y,
                    delta_z,
                    yaw: NetAngle::from_degrees(rot.yaw.into()),
                    pitch: NetAngle::from_degrees(rot.pitch.into()),
                    on_ground: grounded.0,
                };
                for (recipient_entity, conn, tracker) in player_query.iter_mut() {
                    if recipient_entity == entity || !tracker.tracking.contains(&entity) {
                        continue;
                    }
                    if let Err(e) = conn.send_packet_ref(&packet) {
                        warn!(
                            "Failed to send entity update packet for entity {:?}: {:?}",
                            entity, e
                        );
                    }
                }
            };
            *last_synced = LastSyncedPosition(pos.coords);
        } else {
            warn!(
                "Tried to send entity update for non-existent entity: {:?}",
                entity
            );
        }
    }
}
