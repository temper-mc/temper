use bevy_ecs::prelude::*;
use temper_components::player::player_identity::PlayerIdentity;

#[derive(Message, Clone)]
#[allow(unused)]
pub struct PlayerLeft(pub PlayerIdentity);
