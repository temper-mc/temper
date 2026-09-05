use bevy_ecs::prelude::*;
use temper_components::entity_identity::Identity;
use temper_components::game_id::GameID;

#[derive(Message, Clone)]
#[allow(unused)]
pub struct PlayerLeft {
    pub identity: Identity,
    pub entity: Entity,
    pub game_id: GameID
}
