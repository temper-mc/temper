use bevy_ecs::prelude::{Query, With};
use ionic_components::player::player_identity::PlayerIdentity;
use ionic_components::player::position::Position;
use ionic_entities::markers::entity_types::Pig;

#[expect(dead_code, unused_variables)]
pub fn tick_pig(
    query: Query<&Position, With<Pig>>,
    players: Query<&Position, With<PlayerIdentity>>,
) {}
