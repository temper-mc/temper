use bevy_ecs::entity::Entity;
use temper_components::entity_identity::EntityIdentity;
use temper_components::player::player_identity::PlayerIdentity;

pub(crate) fn resolve_any_player<'a>(
    iter: impl Iterator<
        Item = (
            Entity,
            Option<&'a EntityIdentity>,
            Option<&'a PlayerIdentity>,
        ),
    >,
) -> Vec<Entity> {
    let mut players = Vec::new();
    for (entity, _, player_id) in iter {
        if player_id.is_some() {
            players.push(entity);
        }
    }
    players
}
