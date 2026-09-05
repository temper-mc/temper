use crate::damage::DamageSource;
use bevy_ecs::prelude::{Entity, Message};
use temper_text::TextComponent;

#[derive(Message)]
pub struct KillEntity {
    pub entity: Entity,
    pub message: Option<TextComponent>,
    pub source: DamageSource,
}
