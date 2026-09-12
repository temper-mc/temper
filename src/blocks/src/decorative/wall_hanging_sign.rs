use crate::BlockBehavior;
use temper_blocks_generated::WallHangingSignBlock;

impl BlockBehavior for WallHangingSignBlock {
    fn is_interactable(&self) -> bool {
        true
    }
}
