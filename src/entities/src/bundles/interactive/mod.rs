//! Bundles for interactive block entities.
//!
//! These bundles link ECS entities to block positions in the world grid,
//! allowing blocks like doors, levers, and chests to have complex behavior
//! managed through the ECS interaction system.
//!
//! ## Architecture
//!
//! Interactive blocks are represented as ECS entities with:
//! - `BlockPosition` - Links to world grid coordinates
//! - `InteractableBlock` - Marks as interactive
//! - Capability components (`Toggleable`, `Container`, `RedstoneEmitter`)
//! - Type markers (`Door`, `Lever`, `Chest`)
//!
//! When a player interacts with a block position:
//! 1. The packet handler looks up the entity by `BlockPosition`
//! 2. The interaction system checks for `InteractableBlock`
//! 3. Observers react based on capability components
//!
//! ## Adding a New Interactive Block
//!
//! 1. Create a new bundle file (e.g., `button.rs`)
//! 2. Add a marker component to `components/interaction.rs`
//! 3. Create a bundle with the appropriate capability components
//! 4. (Optional) Add an Observer for custom behavior

pub mod door;

// Re-export all bundles and their type markers
pub use door::DoorBlockBundle;
