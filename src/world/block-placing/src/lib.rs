mod blocks;
mod errors;

use crate::errors::BlockPlaceError;
use bevy_math::DVec3;
use std::collections::HashMap;
use temper_components::player::position::Position;
use temper_components::player::rotation::Rotation;
use temper_core::block_state_id::{BlockStateId, ITEM_TO_BLOCK_MAPPING};
use temper_core::dimension::Dimension;
use temper_core::pos::BlockPos;
use temper_inventories::item::ItemID;
use temper_macros::item;
use temper_state::GlobalState;

pub struct PlacedBlocks {
    pub blocks: HashMap<BlockPos, BlockStateId>,
    pub take_item: bool,
}

pub trait PlacableBlock {
    fn place(
        context: BlockPlaceContext,
        state: GlobalState,
    ) -> Result<PlacedBlocks, BlockPlaceError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockFace {
    Top,
    Bottom,
    North,
    South,
    West,
    East,
}

pub struct BlockPlaceContext {
    pub block_clicked: BlockStateId,
    pub block_position: BlockPos,
    pub face_clicked: BlockFace,
    pub click_position: DVec3,
    pub player_position: Position,
    pub player_rotation: Rotation,
    pub item_used: ItemID,
}

pub fn place_item(
    state: GlobalState,
    context: BlockPlaceContext,
) -> Result<PlacedBlocks, BlockPlaceError> {
    match context.item_used {
        item!("torch") => blocks::torch::PlaceableTorch::place(context, state),
        item!("oak_door")
        | item!("birch_door")
        | item!("spruce_door")
        | item!("jungle_door")
        | item!("acacia_door")
        | item!("dark_oak_door") => blocks::door::PlaceableDoor::place(context, state),

        unhandled => {
            let block_opt = ITEM_TO_BLOCK_MAPPING.get(&unhandled.0.0);
            if let Some(block) = block_opt {
                match state
                    .world
                    .get_or_generate_mut(context.block_position.chunk(), Dimension::Overworld)
                {
                    Ok(mut chunk) => {
                        chunk.set_block(context.block_position.chunk_block_pos(), *block);
                        Ok(PlacedBlocks {
                            blocks: HashMap::from([(context.block_position, *block)]),
                            take_item: true,
                        })
                    }
                    Err(e) => Err(e.into()),
                }
            } else {
                Err(BlockPlaceError::ItemNotPlaceable(context.item_used))
            }
        }
    }
}
