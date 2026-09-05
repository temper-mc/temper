pub mod active_effects;
pub mod bounds;
pub mod entity_identity;
pub mod health;
pub mod interaction;
pub mod player;

// Core entity components based on temper-data
pub mod bossbar;
pub mod combat;
pub mod game_id;
pub mod last_chunk_pos;
pub mod last_synced_position;
pub mod metadata;
pub mod mob_ai;
pub mod pathfinder;
pub mod physical;
pub mod spawn;

// Interaction components re-exports
pub use interaction::{Door, InteractableBlock, InteractionCooldown, Toggleable};
