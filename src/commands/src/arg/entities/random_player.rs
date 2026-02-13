use bevy_ecs::entity::Entity;
use temper_components::entity_identity::EntityIdentity;
use temper_components::player::player_identity::PlayerIdentity;
use rand::prelude::IteratorRandom;

pub(crate) fn resolve_random_player(
    iter: impl Iterator<Item=(Entity, Option<&EntityIdentity>, Option<&PlayerIdentity>)>,
) -> Option<Entity> {
    let mut rng = rand::thread_rng();
    iter.filter_map(|(entity, _, player_id)| {
        if player_id.is_some() {
            Some(entity)
        } else {
            None
        }
    })
        .choose(&mut rng)
}
