use crate::BlockStateId;
use crate::{BlockFace, BlockPlaceContext};
use temper_core::dimension::Dimension;
use temper_macros::block;
use temper_state::GlobalState;

pub fn place_torch(
    context: BlockPlaceContext,
    state: GlobalState,
) -> Result<(bool, Option<BlockStateId>), Box<dyn std::error::Error>> {
    let block = match context.face_clicked {
        BlockFace::Top => block!("torch"),
        BlockFace::East => block!("wall_torch", {facing: "east"}),
        BlockFace::West => block!("wall_torch", {facing: "west"}),
        BlockFace::North => block!("wall_torch", {facing: "north"}),
        BlockFace::South => block!("wall_torch", {facing: "south"}),
        BlockFace::Bottom => return Ok((false, None)), // can't place on bottom face
    };
    state
        .world
        .get_or_generate_mut(context.block_position.chunk(), Dimension::Overworld)?
        .set_block(context.block_position.chunk_block_pos(), block);
    Ok((true, Some(block)))
}
