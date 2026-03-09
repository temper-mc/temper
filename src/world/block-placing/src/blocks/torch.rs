use crate::errors::BlockPlaceError;
use crate::{BlockFace, BlockPlaceContext};
use crate::{BlockStateId, PlacableBlock, PlacedBlocks};
use std::collections::HashMap;
use temper_core::dimension::Dimension;
use temper_macros::block;
use temper_state::GlobalState;

pub(crate) struct PlaceableTorch;

impl PlacableBlock for PlaceableTorch {
    fn place(
        context: BlockPlaceContext,
        state: GlobalState,
    ) -> Result<PlacedBlocks, BlockPlaceError> {
        let block = match context.face_clicked {
            BlockFace::Top => block!("torch"),
            BlockFace::East => block!("wall_torch", {facing: "east"}),
            BlockFace::West => block!("wall_torch", {facing: "west"}),
            BlockFace::North => block!("wall_torch", {facing: "north"}),
            BlockFace::South => block!("wall_torch", {facing: "south"}),
            BlockFace::Bottom => {
                return Ok(PlacedBlocks {
                    blocks: HashMap::new(),
                    take_item: false,
                });
            } // can't place on bottom face
        };
        state
            .world
            .get_or_generate_mut(context.block_position.chunk(), Dimension::Overworld)?
            .set_block(context.block_position.chunk_block_pos(), block);
        Ok(PlacedBlocks {
            blocks: HashMap::from([(context.block_position, block)]),
            take_item: true,
        })
    }
}
