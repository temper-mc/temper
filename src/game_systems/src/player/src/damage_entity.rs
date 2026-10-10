use bevy_ecs::prelude::{Entity, Has, MessageReader, MessageWriter, Query, With};
use temper_codec::net_types::prefixed_optional::PrefixedOptional;
use temper_codec::net_types::var_int::VarInt;
use temper_components::entity_identity::Identity;
use temper_components::game_id::GameID;
use temper_components::health::Health;
use temper_components::player::hunger::Hunger;
use temper_components::player::player_marker::PlayerMarker;
use temper_components::player::position::Position;
use temper_messages::damage::DamageEvent;
use temper_messages::kill_entity::KillEntity;
use temper_net_runtime::connection::StreamWriter;
use temper_protocol::outgoing::damage_player::DamagePlayer;
use temper_protocol::outgoing::hurt_animation::HurtAnimationPacket;
use temper_protocol::outgoing::set_health::SetHealth;
use tracing::error;

pub fn damage_entity(
    mut messages: MessageReader<DamageEvent>,
    mut entity_query: Query<(
        Entity,
        &mut Health,
        Option<&Identity>,
        Option<&GameID>,
        Option<&Position>,
        Option<(&Hunger, &StreamWriter)>,
        Has<PlayerMarker>,
    )>,
    player_stream_query: Query<&StreamWriter, With<PlayerMarker>>,
    mut kill_writer: MessageWriter<KillEntity>,
) {
    for message in messages.read() {
        let Ok((entity, mut health, identity, game_id, _pos, player_data, is_player)) =
            entity_query.get_mut(message.target)
        else {
            error!(
                "Could not find target entity {:?} for DamageEvent (Archetype mismatch or missing Health)",
                message.target
            );
            continue;
        };

        // 1. Deduct Health
        if health.current > message.damage {
            health.current -= message.damage;
        } else {
            health.current = 0.0;
            let entity_name = identity
                .and_then(|i| i.name.as_deref())
                .unwrap_or("An entity");

            let src = message.source.clone();

            kill_writer.write(KillEntity {
                entity,
                message: Some(
                    src.to_death_message(entity_name, None).parse().unwrap_or(
                        format!(
                            "{} was killed by {:?}",
                            entity_name,
                            message.source.to_vanilla_source()
                        )
                        .into(),
                    ),
                ),
                source: src,
            });
        }

        // 2. Handle Player-Specific HUD updates
        if is_player && let Some((hunger, stream_writer)) = player_data {
            let health_packet = SetHealth {
                health: health.current,
                food: hunger.level.into(),
                saturation: hunger.saturation,
            };
            let _ = stream_writer.send_packet(health_packet);
        }

        // 3. Broadcast Hurt Packets to Network Viewers
        if let Some(game_id) = game_id {
            let vanilla_source = message.source.to_vanilla_source();
            let hurt_animation = HurtAnimationPacket {
                entity_id: game_id.get(),
                yaw: 0.0,
            };
            let damage_event = DamagePlayer {
                entity_id: game_id.get(),
                source_type_id: VarInt::new(i32::from(vanilla_source.to_id())),
                source_cause_id: 0.into(),
                source_direct_id: 0.into(),
                source_position: PrefixedOptional::None,
            };

            for stream_writer in player_stream_query.iter() {
                let _ = stream_writer.send_packet(hurt_animation.clone());
                let _ = stream_writer.send_packet(damage_event.clone());
            }
        }
    }
}
