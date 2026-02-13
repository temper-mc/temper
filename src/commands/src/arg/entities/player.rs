use bevy_ecs::prelude::Entity;
use ionic_components::entity_identity::EntityIdentity;
use ionic_components::player::player_identity::PlayerIdentity;

pub(crate) fn resolve_player_name(
    name: String,
    iter: impl Iterator<Item=(Entity, Option<&EntityIdentity>, Option<&PlayerIdentity>)>,
) -> Option<Entity> {
    for (entity, _, player_id) in iter {
        if let Some(identity) = player_id
            && identity.username == name
        {
            return Some(entity);
        }
    }
    None
}
