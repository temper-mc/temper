use bevy_ecs::prelude::{Entity, Message};
use temper_codec::net_types::var_int::VarInt;
use temper_core::block_state_id::BlockStateId;
use temper_core::pos::BlockPos;

/// Message sent when a player right-clicks an interactive block (door, lever, etc.)
/// and is NOT sneaking.
///
/// Emitted by the PlaceBlock packet handler; consumed by the interaction listener.
#[derive(Message, Clone, Debug)]
pub struct BlockInteractMessage {
    pub player: Entity,
    pub position: BlockPos,
    pub sequence: VarInt,
}

/// Emitted when a block has been toggled (door opened/closed, etc.).
///
/// Fired by the interaction listener after a successful toggle.
#[derive(Message, Clone, Debug)]
pub struct BlockToggledEvent {
    pub player: Entity,
    pub position: BlockPos,
    pub is_active: bool,
}

/// Emitted when a door block is toggled, so the door system can toggle the other half.
#[derive(Message, Clone, Debug)]
pub struct DoorToggledEvent {
    pub position: BlockPos,
    pub new_state: BlockStateId,
}
