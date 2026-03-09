use crate::BlockFace;
use temper_core::block_data::BlockData;
use temper_core::pos::BlockPos;
use temper_inventories::item::ItemID;
use temper_world::WorldError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BlockPlaceError {
    #[error("Invalid block face for placement")]
    InvalidBlockFace(BlockFace),
    #[error("Target block is not empty")]
    TargetBlockNotEmpty(BlockPos),
    #[error("Item cannot be placed as a block")]
    ItemNotPlaceable(ItemID),
    #[error("World Error: {0}")]
    WorldError(#[from] WorldError),
    #[error("Item can't be mapped to block")]
    ItemNotMappedToBlock(ItemID),
    #[error("Block can't be mapped to block state id")]
    BlockNotMappedToBlockStateId(BlockData),
}
