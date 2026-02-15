use bevy_ecs::entity::Entity;
use temper_components::entity_identity::EntityIdentity;
use temper_components::player::player_identity::PlayerIdentity;
use uuid::Uuid;

pub(crate) fn resolve_uuid(
    uuid: Uuid,
    iter: impl Iterator<Item = (Entity, Option<&EntityIdentity>, Option<&PlayerIdentity>)>,
) -> Option<Entity> {
    for (entity, entity_id_opt, player_id_opt) in iter {
        match (player_id_opt, entity_id_opt) {
            (Some(player_id), _) if player_id.uuid == uuid => return Some(entity),
            (_, Some(entity_id)) if entity_id.uuid == uuid => return Some(entity),
            _ => {}
        }
    }
    None
}
