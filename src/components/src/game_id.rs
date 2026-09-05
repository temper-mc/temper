use bevy_ecs::prelude::Component;
use std::sync::atomic::AtomicI32;
use temper_codec::net_types::var_int::VarInt;

static ID_COUNTER: AtomicI32 = AtomicI32::new(i32::MIN);

/// Unique id for each entity for a session/instance. This is how the client tracks entities in game
#[derive(Component, Copy, Clone)]
pub struct GameID(i32);

impl Default for GameID {
    fn default() -> Self {
        Self::new()
    }
}

impl GameID {
    pub fn get(&self) -> VarInt {
        self.0.into()
    }
    
    pub fn new() -> Self {
        GameID(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst))
    }
}