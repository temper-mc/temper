use bevy_ecs::prelude::{Entity, Query, Res};
use temper_components::entity_identity::Identity;
use temper_components::player::position::Position;
use temper_protocol::PlayerLoadedReceiver;
use temper_state::GlobalStateResource;
use tracing::warn;

pub fn handle(
    ev: Res<PlayerLoadedReceiver>,
    state: Res<GlobalStateResource>,
    query: Query<(Entity, &Position, &Identity)>,
) {
    for (_, player) in ev.0.try_iter() {
        let Ok((entity, player_pos, identity)) = query.get(player) else {
            warn!("Player position not found in query.");
            continue;
        };
        if !state.0.players.is_connected(entity) {
            warn!(
                "Player {} is not connected, skipping position synchronization.",
                player
            );
            continue;
        }
        tracing::info!(
            "Player {} loaded at position: ({:.2}, {:.2}, {:.2})",
            identity.name.as_ref().expect("No Player Name"),
            player_pos.x,
            player_pos.y,
            player_pos.z
        );
    }
}
