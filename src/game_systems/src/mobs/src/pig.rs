use bevy_ecs::prelude::{Query, With};
use temper_components::player::player_identity::PlayerIdentity;
use temper_components::player::position::Position;
use temper_entities::markers::entity_types::Pig;

#[expect(unused_variables)]
pub fn tick_pig(
    query: Query<&Position, With<Pig>>,
    players: Query<&Position, With<PlayerIdentity>>,
) {
}
