use bevy_ecs::prelude::*;
use temper_core::pos::BlockPos;
use temper_world_format::BlockEntityKind;

/// Emitted when a block with an associated block entity is placed, so type-specific
/// systems can react (opening the sign editor, for example).
#[derive(Message, Clone)]
pub struct BlockEntityPlaced {
    pub player: Entity,
    pub position: BlockPos,
    pub kind: BlockEntityKind,
}
