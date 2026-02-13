use bevy_ecs::prelude::Entity;
use ionic_components::entity_identity::EntityIdentity;
use ionic_components::player::player_identity::PlayerIdentity;

pub(crate) fn resolve_any_entity(
    iter: impl Iterator<Item=(Entity, Option<&EntityIdentity>, Option<&PlayerIdentity>)>,
) -> Vec<Entity> {
    iter.map(|(entity, _, _)| entity).collect()
}
