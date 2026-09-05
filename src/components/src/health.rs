use bevy_ecs::prelude::Component;
use bitcode_derive::{Decode, Encode};
use type_hash::TypeHash;

#[derive(Component, Debug, Clone, Copy, Decode, Encode, TypeHash)]
pub struct Health {
    pub current: u16,
    pub max: u16,
}

impl Default for Health {
    fn default() -> Self {
        Self {
            current: 20,
            max: 20,
        }
    }
}
