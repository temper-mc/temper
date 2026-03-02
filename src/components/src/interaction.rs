use bevy_ecs::prelude::*;

/// Marker: this block entity is interactive.
#[derive(Component, Debug, Clone, Copy)]
pub struct InteractableBlock;

/// Anti-spam cooldown for block interactions.
#[derive(Component, Debug, Clone)]
pub struct InteractionCooldown {
    pub cooldown_ms: u64,
    pub last_interaction: Option<std::time::Instant>,
}

impl Default for InteractionCooldown {
    fn default() -> Self {
        Self {
            cooldown_ms: 200,
            last_interaction: None,
        }
    }
}

/// Block that toggles between two states (open/closed).
#[derive(Component, Debug, Clone, Copy)]
pub struct Toggleable {
    pub is_active: bool,
}

/// Marker for door block entities.
#[derive(Component, Debug, Clone, Copy)]
pub struct Door;
