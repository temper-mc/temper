use bevy_ecs::prelude::{Entity, Has, MessageReader, MessageWriter, Query};
use temper_codec::net_types::prefixed_optional::PrefixedOptional;
use temper_codec::net_types::var_int::VarInt;
use temper_components::entity_identity::Identity;
use temper_components::game_id::GameID;
use temper_components::health::Health;
use temper_components::player::hunger::Hunger;
use temper_components::player::player_marker::PlayerMarker;
use temper_components::player::position::Position;
use temper_messages::damage::DamageEvent;
use temper_messages::damage::DamageSource::DivineSmiting;
use temper_messages::kill_entity::KillEntity;
use temper_net_runtime::connection::StreamWriter;
use temper_protocol::outgoing::damage_player::DamagePlayer;
use temper_protocol::outgoing::hurt_animation::HurtAnimationPacket;
use temper_protocol::outgoing::set_health::SetHealth;
use tracing::error;

pub fn damage_entity(
    mut messages: MessageReader<DamageEvent>,
    mut player_query: Query<(
        Entity,
        &mut Health,
        &Hunger,
        Option<&StreamWriter>,
        Has<PlayerMarker>,
        &Identity,
        &GameID,
        &Position,
    )>,
    mut kill_writer: MessageWriter<KillEntity>,
) {
    for message in messages.read() {
        for (entity, mut health, hunger, stream_writer, is_player, identity, game_id, pos) in
            player_query.iter_mut()
        {
            if entity == message.target {
                if health.current > message.damage {
                    health.current -= message.damage;
                } else {
                    health.current = 0;
                    kill_writer.write(KillEntity {
                        entity,
                        message: Some(
                            format!(
                                "{} was killed by {:?}",
                                identity.name.as_ref().unwrap(),
                                message.source.to_vanilla_source()
                            )
                            .into(),
                        ),
                        source: DivineSmiting { silent: false },
                    });
                }

                if let Some(stream_writer) = stream_writer {
                    let health_packet = SetHealth {
                        health: f32::from(health.current),
                        food: hunger.level.into(),
                        saturation: hunger.saturation,
                    };
                    if let Err(err) = stream_writer.send_packet(health_packet) {
                        error!(
                            "Failed to send health packet to player {}: {}",
                            identity.name.as_ref().unwrap(),
                            err
                        );
                    }

                    let hurt_animation = HurtAnimationPacket {
                        entity_id: game_id.get(),
                        yaw: 0.0,
                    };

                    if let Err(err) = stream_writer.send_packet(hurt_animation) {
                        error!(
                            "Failed to send hurt animation packet to player {}: {}",
                            identity.name.as_ref().unwrap(),
                            err
                        );
                    }
                } else {
                    error!(
                        "Player {} does not have a stream writer",
                        identity.name.as_ref().unwrap()
                    );
                }
            }
            if let Some(stream_writer) = stream_writer {
                let damage_event = DamagePlayer {
                    entity_id: game_id.get(),
                    source_type_id: VarInt::new(i32::from(
                        message.source.to_vanilla_source().to_id(),
                    )),
                    source_cause_id: 0.into(),
                    source_direct_id: 0.into(),
                    source_position: PrefixedOptional::None,
                };
                if let Err(err) = stream_writer.send_packet(damage_event) {
                    error!(
                        "Failed to send damage event packet to player {}: {}",
                        identity.name.as_ref().unwrap(),
                        err
                    );
                }
            }
        }
    }
}
