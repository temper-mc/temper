use bevy_ecs::prelude::{Entity, MessageReader, MessageWriter, Query};
use temper_components::game_id::GameID;
use temper_components::player::position::Position;
use temper_components::player::rotation::Rotation;
use temper_components::player::teleport_tracker::TeleportTracker;
use temper_components::player::velocity::Velocity;
use temper_messages::chunk_calc::ChunkCalc;
use temper_messages::entity_update::SendEntityUpdate;
use temper_messages::teleport_entity::TeleportEntity;
use temper_net_runtime::connection::StreamWriter;
use temper_protocol::outgoing::entity_position_sync::TeleportEntityPacket;
use temper_protocol::outgoing::synchronize_player_position::SynchronizePlayerPositionPacket;
use tracing::error;

pub fn teleport_entities(
    mut target_query: Query<(
        Option<&StreamWriter>,
        &mut Position,
        Option<&mut Rotation>,
        Option<&mut Velocity>,
        Option<&mut TeleportTracker>,
    )>,
    player_query: Query<(Entity, &StreamWriter)>,
    id_query: Query<&GameID>,
    mut message_reader: MessageReader<TeleportEntity>,
    mut chunk_calc_msg: MessageWriter<ChunkCalc>,
    mut player_update_msg: MessageWriter<SendEntityUpdate>,
) {
    for message in message_reader.read() {
        let message_entity = message.entity;
        let id = match id_query.get(message_entity) {
            Ok(id) => id,
            Err(err) => {
                error!(
                    "Failed to get Identity for entity {:?}: {}",
                    message_entity, err
                );
                continue;
            }
        };

        let is_player_target = {
            let Ok((conn, mut pos, rotation, velocity, tracker)) =
                target_query.get_mut(message_entity)
            else {
                error!(
                    "Failed to get teleport target components for entity {:?}",
                    message_entity
                );
                continue;
            };

            if let Some(mut tracker) = tracker {
                // Block movement tracking until the player has been teleported.
                tracker.waiting_for_confirm = true;
            }

            *pos = message.position;

            if let Some(mut rotation) = rotation {
                *rotation = message.rotation;
            }

            if let Some(mut velocity) = velocity {
                *velocity = message.velocity;
            }

            if let Some(conn) = conn {
                if let Err(err) = conn.send_packet(SynchronizePlayerPositionPacket {
                    teleport_id: rand::random::<i32>().into(),
                    x: message.position.x,
                    y: message.position.y,
                    z: message.position.z,
                    vel_x: f64::from(message.velocity.x),
                    vel_y: f64::from(message.velocity.y),
                    vel_z: f64::from(message.velocity.z),
                    yaw: message.rotation.yaw,
                    pitch: message.rotation.pitch,
                    flags: 0,
                }) {
                    error!("Failed to send teleport packet: {}", err);
                    continue;
                }

                true
            } else {
                false
            }
        };

        for (entity, conn) in player_query.iter() {
            if entity == message_entity {
                continue;
            }

            // This ideally should be handled by the send entity updates system, but it seems to be
            // a bit buggy.
            if let Err(err) = conn.send_packet(TeleportEntityPacket {
                entity_id: id.get(),
                x: message.position.x,
                y: message.position.y,
                z: message.position.z,
                vel_x: f64::from(message.velocity.x),
                vel_y: f64::from(message.velocity.y),
                vel_z: f64::from(message.velocity.z),
                yaw: message.rotation.yaw,
                pitch: message.rotation.pitch,
                on_ground: false,
            }) {
                error!("Failed to send teleport packet: {}", err);
                continue;
            }
        }

        // Notify the player update system to send the new position to the client
        player_update_msg.write(SendEntityUpdate(message_entity));

        // Notify the chunk calculation system to recalculate chunks for this player
        if is_player_target {
            chunk_calc_msg.write(ChunkCalc(message_entity));
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::message::MessageRegistry;
    use bevy_ecs::prelude::{IntoScheduleConfigs, MessageWriter, Res, Resource, Schedule, World};
    use temper_components::entity_identity::Identity;

    use super::*;

    #[derive(Resource)]
    struct TeleportTarget(Entity);

    fn emit_mob_teleport(target: Res<TeleportTarget>, mut writer: MessageWriter<TeleportEntity>) {
        writer.write(TeleportEntity::new(
            target.0,
            Position::new(10.0, 65.0, -3.0),
            Rotation::new(90.0, 15.0),
        ));
    }

    #[test]
    fn teleport_updates_non_player_position() {
        let mut world = World::new();
        MessageRegistry::register_message::<TeleportEntity>(&mut world);
        MessageRegistry::register_message::<ChunkCalc>(&mut world);
        MessageRegistry::register_message::<SendEntityUpdate>(&mut world);

        let entity = world
            .spawn((
                Identity::default(),
                GameID::new(),
                Position::new(0.0, 64.0, 0.0),
                Rotation::default(),
                Velocity::new(1.0, 0.0, 0.0),
            ))
            .id();
        world.insert_resource(TeleportTarget(entity));

        let mut schedule = Schedule::default();
        schedule.add_systems((emit_mob_teleport, teleport_entities).chain());
        schedule.run(&mut world);

        let position = world.get::<Position>(entity).unwrap();
        let rotation = world.get::<Rotation>(entity).unwrap();
        let velocity = world.get::<Velocity>(entity).unwrap();

        assert_eq!(position.x, 10.0);
        assert_eq!(position.y, 65.0);
        assert_eq!(position.z, -3.0);
        assert_eq!(rotation.yaw, 90.0);
        assert_eq!(rotation.pitch, 15.0);
        assert_eq!(velocity.x, 0.0);
        assert_eq!(velocity.y, 0.0);
        assert_eq!(velocity.z, 0.0);
    }
}
