use bevy_ecs::prelude::Bundle;
use temper_components::interaction::{Door, InteractableBlock, InteractionCooldown, Toggleable};
use temper_components::player::position::Position;
use temper_core::pos::BlockPos;

/// Bundle to spawn a door block entity in Bevy ECS.
#[derive(Bundle)]
pub struct DoorBlockBundle {
    pub position: Position,
    pub interactable: InteractableBlock,
    pub toggleable: Toggleable,
    pub cooldown: InteractionCooldown,
    pub door: Door,
}

impl DoorBlockBundle {
    pub fn new(pos: BlockPos) -> Self {
        Self {
            position: Position::new(pos.pos.x as f64, pos.pos.y as f64, pos.pos.z as f64),
            interactable: InteractableBlock,
            toggleable: Toggleable { is_active: false },
            cooldown: InteractionCooldown::default(),
            door: Door,
        }
    }

    pub fn new_open(pos: BlockPos) -> Self {
        Self {
            position: Position::new(pos.pos.x as f64, pos.pos.y as f64, pos.pos.z as f64),
            interactable: InteractableBlock,
            toggleable: Toggleable { is_active: true },
            cooldown: InteractionCooldown::default(),
            door: Door,
        }
    }
}
