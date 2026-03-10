//! Direct world block interactions.
//!
//! This module handles interactions with blocks directly in the world (chunks),
//! without creating ECS entities. This is more performant for simple toggleable
//! blocks like doors, levers, trapdoors, and buttons.
//!
//! ## How it works
//!
//! 1. Player right-clicks on a block
//! 2. System checks if the block is "interactive" based on its name
//! 3. If yes, toggle the relevant property (e.g., "open" for doors)
//! 4. Update the block in the chunk
//! 5. Broadcast BlockUpdate to nearby players
//!
//! ## Supported blocks
//!
//! - Doors (oak_door, iron_door, etc.) - toggles "open" property
//!
//! ## Future supported block
//!
//! - Trapdoors - toggles "open" property
//! - Fence gates - toggles "open" property
//! - Levers - toggles "powered" property
//! - Buttons - activates temporarily (TODO: timer system)

use temper_core::block_data::BlockData;
use temper_core::block_state_id::BlockStateId;
use tracing::{debug, warn};

/// Result of attempting to interact with a block.
#[derive(Debug, Clone)]
pub enum InteractionResult {
    /// Block was toggled, returns the new BlockStateId
    Toggled(BlockStateId),
    /// Block is not interactive
    NotInteractive,
    /// Block state not found (shouldn't happen)
    InvalidBlock,
}

/// Checks if a block is interactive and returns its interaction type.
pub fn get_interaction_type(block_data: &BlockData) -> Option<InteractionType> {
    let name = &block_data.name;

    // Doors
    if name.ends_with("_door") {
        return Some(InteractionType::Toggleable("open"));
    }

    None
}

/// Type of interaction for a block.
#[derive(Debug, Clone, Copy)]
pub enum InteractionType {
    /// Block toggles between two states (doors, levers)
    /// The string is the property name to toggle
    Toggleable(&'static str),
    // In the future maybe add Momentary or something like that
}

/// Attempts to interact with a block and returns the new state if successful.
///
/// This function:
/// 1. Gets the BlockData from the BlockStateId
/// 2. Checks if it's an interactive block
/// 3. Toggles the appropriate property
/// 4. Returns the new BlockStateId
pub fn try_interact(block_state_id: BlockStateId) -> InteractionResult {
    debug!(
        "try_interact called with block_state_id: {:?} (raw: {})",
        block_state_id,
        block_state_id.raw()
    );

    // Get the block data
    let Some(mut block_data) = block_state_id.to_block_data() else {
        warn!(
            "try_interact: InvalidBlock - could not convert {:?} to BlockData",
            block_state_id
        );
        return InteractionResult::InvalidBlock;
    };

    debug!(
        "try_interact: block_data name='{}', properties={:?}",
        block_data.name, block_data.properties
    );

    // Check if it's interactive
    let Some(interaction_type) = get_interaction_type(&block_data) else {
        debug!(
            "try_interact: block '{}' is not interactive",
            block_data.name
        );
        return InteractionResult::NotInteractive;
    };

    debug!("try_interact: interaction_type={:?}", interaction_type);

    let Some(properties) = block_data.properties.as_mut() else {
        warn!(
            "try_interact: interactive block '{}' has no properties",
            block_data.name
        );
        return InteractionResult::InvalidBlock;
    };

    match interaction_type {
        InteractionType::Toggleable(prop_name) => {
            let current_value = properties
                .get(prop_name)
                .map(|s| s.as_str())
                .unwrap_or("false");
            let new_value = if current_value == "true" {
                "false"
            } else {
                "true"
            };
            debug!(
                "try_interact: toggling '{}' from '{}' to '{}'",
                prop_name, current_value, new_value
            );
            properties.insert(prop_name.to_string(), new_value.to_string());
        }
    }

    debug!(
        "try_interact: modified block_data properties={:?}",
        block_data.properties
    );

    // Convert back to BlockStateId
    let new_state_id = BlockStateId::from_block_data(&block_data);
    debug!(
        "try_interact: new_state_id={:?} (raw: {})",
        new_state_id,
        new_state_id.raw()
    );

    if new_state_id.raw() == 0 {
        warn!(
            "try_interact: WARNING - new_state_id is 0 (air)! BlockData lookup failed for: name='{}', props={:?}",
            block_data.name, block_data.properties
        );
    }

    InteractionResult::Toggled(new_state_id)
}

/// Checks if a block is interactive without modifying it.
pub fn is_interactive(block_state_id: BlockStateId) -> bool {
    block_state_id
        .to_block_data()
        .as_ref()
        .and_then(get_interaction_type)
        .is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use temper_core::block_state_id::BlockStateId;
    use temper_macros::block;

    #[test]
    fn test_door_detection() {
        let door_data = BlockData {
            name: "minecraft:oak_door".to_string(),
            properties: Some(BTreeMap::from([
                ("facing".to_string(), "north".to_string()),
                ("open".to_string(), "false".to_string()),
                ("half".to_string(), "lower".to_string()),
                ("hinge".to_string(), "left".to_string()),
            ])),
        };

        assert!(matches!(
            get_interaction_type(&door_data),
            Some(InteractionType::Toggleable("open"))
        ));
    }

    #[test]
    fn test_try_interact_opens_door() {
        // A closed oak door (lower half, north-facing, left hinge, unpowered)
        let closed_door: BlockStateId = block!("oak_door", { facing: "north", half: "lower", hinge: "left", open: false, powered: false });

        let result = try_interact(closed_door);
        let InteractionResult::Toggled(new_id) = result else {
            panic!("Expected Toggled, got {:?}", result);
        };

        let new_data = new_id
            .to_block_data()
            .expect("new state ID should be valid");
        let props = new_data.properties.expect("door should have properties");
        assert_eq!(props["open"], "true", "door should be open after interact");
    }

    #[test]
    fn test_try_interact_closes_door() {
        // An already-open oak door
        let open_door: BlockStateId = block!("oak_door", { facing: "north", half: "lower", hinge: "left", open: true, powered: false });

        let result = try_interact(open_door);
        let InteractionResult::Toggled(new_id) = result else {
            panic!("Expected Toggled, got {:?}", result);
        };

        let new_data = new_id
            .to_block_data()
            .expect("new state ID should be valid");
        let props = new_data.properties.expect("door should have properties");
        assert_eq!(
            props["open"], "false",
            "door should be closed after interact"
        );
    }

    #[test]
    fn test_try_interact_not_interactive() {
        let stone: BlockStateId = block!("stone");
        assert!(matches!(
            try_interact(stone),
            InteractionResult::NotInteractive
        ));
    }

    #[test]
    fn test_is_interactive() {
        let door: BlockStateId = block!("oak_door", { facing: "north", half: "lower", hinge: "left", open: false, powered: false });
        let stone: BlockStateId = block!("stone");

        assert!(is_interactive(door), "door should be interactive");
        assert!(!is_interactive(stone), "stone should not be interactive");
    }
}
