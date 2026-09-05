use bevy_ecs::prelude::Component;
use serde::{Deserialize, Serialize};

/// Identity component for entities in the game world, including players and non-player entities (mobs, items, etc.).
#[derive(Debug, Component, Clone, Serialize, Deserialize)]
pub struct Identity {
    /// Unique identifier for this entity.
    /// Generated randomly for each spawned entity.
    /// For players, this is the full UUID from Mojang's authentication system.
    pub uuid: uuid::Uuid,

    /// Optional name for the entity
    /// For players, this is the username. For other entities, it can be None or a custom name.
    pub name: Option<String>,
}

impl Identity {
    /// Creates a new entity identity with a unique ID and UUID.
    ///
    /// The entity_id is generated randomly to avoid collisions with player ids.
    /// The UUID is randomly generated.
    pub fn new(name: Option<String>) -> Self {
        Self {
            uuid: uuid::Uuid::new_v4(),
            name,
        }
    }

    /// Creates an entity identity with a specific UUID (for loading from disk).
    pub fn with_uuid(uuid: uuid::Uuid, name: Option<String>) -> Self {
        Self { uuid, name }
    }
}

impl Default for Identity {
    fn default() -> Self {
        Self::new(None)
    }
}
