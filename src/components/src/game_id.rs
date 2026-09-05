use bevy_ecs::prelude::Component;
use std::sync::atomic::AtomicI32;
use std::sync::LazyLock;
use temper_codec::net_types::var_int::VarInt;

static ID_COUNTER: LazyLock<AtomicI32> = LazyLock::new(|| AtomicI32::new(rand::random::<i32>()));

/// Unique id for each entity for a session/instance. This is how the client tracks entities in game
#[derive(Component, Copy, Clone, Ord, PartialOrd, PartialEq, Eq, Debug)]
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
        // Wraps on overflow so no worries there
        GameID(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst))
    }
}
