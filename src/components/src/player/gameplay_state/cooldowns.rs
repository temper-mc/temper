use bevy_ecs::prelude::Component;
use std::collections::HashMap;
use std::time::Instant;
use temper_inventories::item::ItemID;

/// Tracks item cooldowns (e.g., Ender Pearl).
#[derive(Component, Debug, Clone, Default)]
pub struct Cooldowns {
    /// Maps an ItemID to the `Instant` when it will be usable again.
    pub map: HashMap<ItemID, Instant>,
}
