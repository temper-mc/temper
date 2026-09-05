use bevy_ecs::prelude::{Entity, Has, Query};
use temper_components::entity_identity::Identity;
use temper_components::game_id::GameID;
use temper_components::player::chunk_receiver::ChunkReceiver;
use temper_components::player::entity_tracker::EntityTracker;
use temper_components::player::player_marker::PlayerMarker;
use temper_components::player::position::Position;
use temper_net_runtime::connection::StreamWriter;
use tracing::trace;

pub fn refresh_visible_entities(
    mut player_query: Query<(Entity, &StreamWriter, &ChunkReceiver, &mut EntityTracker)>,
    entity_query: Query<(
        Entity,
        &Identity,
        &GameID,
        &Position,
        Has<PlayerMarker>,
        Option<&StreamWriter>,
    )>,
) {
    for (player_entity, conn, chunk_receiver, mut tracker) in player_query.iter_mut() {
        if !conn.is_running() {
            continue;
        }

        let tracked_entities = tracker.tracking.iter().copied().collect::<Vec<_>>();
        for tracked_entity in tracked_entities {
            let should_keep = entity_query
                .get(tracked_entity)
                .map(
                    |(entity, _identity, _game_id, pos, is_player, maybe_writer)| {
                        if entity == player_entity {
                            return false;
                        }

                        if is_player && maybe_writer.is_none_or(|writer| !writer.is_running()) {
                            return false;
                        }

                        chunk_receiver.loaded.contains(&pos.chunk())
                    },
                )
                .unwrap_or(false);

            if !should_keep {
                tracker.tracking.remove(&tracked_entity);
                tracker.to_untrack.push(tracked_entity);
            }
        }

        for (entity, identity, game_id, pos, is_player, maybe_writer) in entity_query.iter() {
            if entity == player_entity {
                continue;
            }

            if is_player && maybe_writer.is_none_or(|writer| !writer.is_running()) {
                continue;
            }

            if !chunk_receiver.loaded.contains(&pos.chunk()) {
                continue;
            }

            if tracker.tracking.contains(&entity) {
                continue;
            }

            trace!(
                "Queueing entity {:#x} ({:?}) for player {:?}",
                game_id.get().0,
                identity.uuid,
                player_entity
            );
            tracker.to_track.push((identity.uuid, 0));
        }
    }
}
