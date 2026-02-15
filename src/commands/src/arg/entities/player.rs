use bevy_ecs::prelude::Entity;
use temper_components::entity_identity::EntityIdentity;
use temper_components::player::player_identity::PlayerIdentity;

pub(crate) fn resolve_player_name<'a>(
    name: String,
    iter: impl Iterator<
        Item = (
            Entity,
            Option<&'a EntityIdentity>,
            Option<&'a PlayerIdentity>,
        ),
    >,
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
