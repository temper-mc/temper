pub mod physical_registry;

// Re-exports
pub use temper_components::combat::CombatProperties;
pub use temper_components::last_synced_position::LastSyncedPosition;
pub use temper_components::metadata::EntityMetadata;
pub use temper_components::physical::PhysicalProperties;
pub use temper_components::spawn::SpawnProperties;

// Marker component for baby entities
use bevy_ecs::prelude::Component;

/// Marker component for baby entities.
/// When present, physics systems will use baby-scaled properties.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Baby;
