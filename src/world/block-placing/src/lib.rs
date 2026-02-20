mod blocks;

use bevy_math::DVec3;
use std::error::Error;
use temper_components::player::position::Position;
use temper_components::player::rotation::Rotation;
use temper_core::block_state_id::{BlockStateId, ITEM_TO_BLOCK_MAPPING};
use temper_core::dimension::Dimension;
use temper_core::pos::BlockPos;
use temper_inventories::item::ItemID;
use temper_macros::item;
use temper_state::GlobalState;
use tracing::error;

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
}

pub fn place_item(
    state: GlobalState,
    context: BlockPlaceContext,
    item: ItemID,
) -> (bool, Option<BlockStateId>) {
    let res = match item {
        item!("torch") => blocks::torch::place_torch(context, state),

        unhandled => {
            let block_opt = ITEM_TO_BLOCK_MAPPING.get(&unhandled.0.0);
            if let Some(block) = block_opt {
                match state
                    .world
                    .get_or_generate_mut(context.block_position.chunk(), Dimension::Overworld)
                {
                    Ok(mut chunk) => {
                        chunk.set_block(context.block_position.chunk_block_pos(), *block);
                        Ok((true, Some(*block)))
                    }
                    Err(e) => Err(e.into()),
                }
            } else {
                Ok((false, None))
            }
        }
    };
    res.unwrap_or_else(|e| {
        error!("Error placing block: {}", e);
        (false, None)
    })
}
