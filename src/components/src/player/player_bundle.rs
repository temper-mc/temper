use crate::bounds::CollisionBounds;
use crate::entity_identity::Identity;
use crate::player::bossbar_sender::BossbarSender;
use crate::player::chunk_receiver::ChunkReceiver;
use crate::player::entity_tracker::EntityTracker;
use crate::player::grounded::OnGround;
use crate::player::player_marker::PlayerMarker;
use crate::player::player_properties::PlayerProperties;
use crate::player::position::Position;
use crate::player::rotation::Rotation;
use crate::{
    active_effects::ActiveEffects,
    health::Health,
    player::{
        abilities::PlayerAbilities, experience::Experience, gamemode::GameModeComponent,
        gameplay_state::ender_chest::EnderChest, hunger::Hunger, sneak::SneakState,
        swimming::SwimmingState,
    },
};
use bevy_ecs::prelude::Bundle;
use temper_inventories::{hotbar::Hotbar, inventory::Inventory};
use temper_permissions::player::PlayerPermission;
use crate::game_id::GameID;

/// A Bevy Bundle containing all components required for a player entity.
/// This groups all 17+ components into a single, spawnable unit.
#[derive(Bundle, Default)]
pub struct PlayerBundle {
    // Identity
    pub identity: Identity,
    pub game_id: GameID,

    // Core State
    pub abilities: PlayerAbilities,
    pub gamemode: GameModeComponent,
    pub player_properties: PlayerProperties,

    // Position/World
    pub position: Position,
    pub rotation: Rotation,
    pub on_ground: OnGround,
    pub chunk_receiver: ChunkReceiver,
    pub collision_bounds: CollisionBounds,
    pub entity_tracker: EntityTracker,

    // Inventory
    pub inventory: Inventory,
    pub hotbar: Hotbar,
    pub ender_chest: EnderChest,

    // Survival Stats
    pub health: Health,
    pub hunger: Hunger,
    pub experience: Experience,
    pub active_effects: ActiveEffects,

    // Movement State
    pub swimming: SwimmingState,
    pub sneak: SneakState,

    // Permissions
    pub permissions: PlayerPermission,

    // Player Marker
    pub player_marker: PlayerMarker,

    // Bossbar Sender
    pub bossbar_sender: BossbarSender,
}
