use bevy_ecs::prelude::{Entity, Message};
use temper_text::TextComponent;
use crate::damage::DamageSource;

#[derive(Message)]
pub struct KillEntity {
    pub entity: Entity,
    pub message: Option<TextComponent>,
    pub source: DamageSource,
}
