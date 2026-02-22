use bevy_ecs::prelude::Bundle;
use temper_components::interaction::{
    BlockPosition, Door, InteractableBlock, InteractionCooldown, Toggleable,
};
use temper_core::pos::BlockPos;

/// Bundle to spawn a door block entity in Bevy ECS.
#[derive(Bundle)]
pub struct DoorBlockBundle {
    pub block_pos: BlockPosition,
    pub interactable: InteractableBlock,
    pub toggleable: Toggleable,
    pub cooldown: InteractionCooldown,
    pub door: Door,
}

impl DoorBlockBundle {
    pub fn new(pos: BlockPos) -> Self {
        Self {
            block_pos: BlockPosition(pos),
            interactable: InteractableBlock,
            toggleable: Toggleable { is_active: false },
            cooldown: InteractionCooldown::default(),
            door: Door,
        }
    }

    pub fn new_open(pos: BlockPos) -> Self {
        Self {
            block_pos: BlockPosition(pos),
            interactable: InteractableBlock,
            toggleable: Toggleable { is_active: true },
            cooldown: InteractionCooldown::default(),
            door: Door,
        }
    }
}
