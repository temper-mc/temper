use bevy_ecs::prelude::Entity;
use temper_components::entity_identity::EntityIdentity;
use temper_components::player::player_identity::PlayerIdentity;

pub(crate) fn resolve_any_entity<'a>(
    iter: impl Iterator<
        Item = (
            Entity,
            Option<&'a EntityIdentity>,
            Option<&'a PlayerIdentity>,
        ),
    >,
) -> Vec<Entity> {
    iter.map(|(entity, _, _)| entity).collect()
}
