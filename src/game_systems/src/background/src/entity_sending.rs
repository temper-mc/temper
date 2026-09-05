use bevy_ecs::prelude::{Entity, Has, Query, Res};
use temper_components::entity_identity::Identity;
use temper_components::player::client_information::ClientInformationComponent;
use temper_components::player::entity_tracker::EntityTracker;
use temper_components::player::player_marker::PlayerMarker;
use temper_components::player::position::Position;
use temper_components::player::rotation::Rotation;
use temper_components::player::velocity::Velocity;
use temper_net_runtime::connection::StreamWriter;
use temper_protocol::outgoing::remove_entities::RemoveEntitiesPacket;
use temper_protocol::outgoing::spawn_entity::SpawnEntityPacket;
use temper_state::GlobalStateResource;
use tracing::debug;
use temper_components::game_id::GameID;

/// Protocol entity type ID for player entities in the current target version.
const PLAYER_TYPE_ID: u16 = temper_data::generated::entities::EntityType::PLAYER.id;

pub fn send_untracked_entities(
    mut player_query: Query<(&StreamWriter, &mut EntityTracker)>,
    identity_query: Query<&GameID>,
) {
    for (conn, entity_tracker) in player_query.iter_mut() {
        while let Some(entity) = entity_tracker.to_untrack.pop() {
            let Ok(game_id) = identity_query.get(entity) else {
                continue;
            };

            let packet = RemoveEntitiesPacket::from_entities(std::iter::once(*game_id));
            conn.send_packet(packet)
                .expect("Failed to send remove entities packet");
        }
    }
}

pub fn send_new_entities(
    mut player_query: Query<(
        &StreamWriter,
        &mut EntityTracker,
        &Position,
        &ClientInformationComponent,
    )>,
    entity_query: Query<(Entity, &Identity, &GameID, &Position, &Rotation, Has<PlayerMarker>)>,
    state: Res<GlobalStateResource>,
) {
    for (conn, mut entity_tracker, player_pos, client_info) in player_query.iter_mut() {
        let mut unresolved = Vec::new();

        while let Some((uuid, entity_type_id)) = entity_tracker.to_track.pop() {
            if let Some((entity, identity, game_id, entity_pos, rot, is_player)) = entity_query
                .iter()
                .find_map(|(entity, identity, game_id, pos, rot, is_player)| {
                    if identity.uuid == uuid {
                        Some((entity, identity, game_id, pos, rot, is_player))
                    } else {
                        None
                    }
                })
            {
                if entity_tracker.tracking.contains(&entity) {
                    continue;
                }

                let render_distance = client_info
                    .view_distance
                    .min(state.0.config.chunk_render_distance as u8);
                if player_pos.distance(**entity_pos) > (f64::from(render_distance) * 16.0) {
                    continue; // Skip entities outside of render distance
                }

                let entity_type_id = if is_player {
                    PLAYER_TYPE_ID
                } else {
                    entity_type_id
                };

                let packet = SpawnEntityPacket::new(
                    game_id.get(),
                    identity.uuid.as_u128(),
                    i32::from(entity_type_id),
                    entity_pos,
                    rot,
                    &Velocity::new(0.0, 0.0, 0.0),
                );
                conn.send_packet(packet)
                    .expect("Failed to send spawn entity packet");
                debug!(
                    "Sent spawn packet for entity {:#x} with UUID {} to player at position ({:.2} {:.2} {:.2})",
                    game_id.get().0,
                    identity.uuid,
                    player_pos.x,
                    player_pos.y,
                    player_pos.z,
                );
                entity_tracker.tracking.insert(entity);
            } else {
                // Retry unresolved entities on a later tick instead of reinserting
                // into the actively-drained queue and looping forever.
                unresolved.push((uuid, entity_type_id));
            }
        }

        for item in unresolved {
            entity_tracker.to_track.push(item);
        }
    }
}
