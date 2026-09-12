use crate::BlockBehavior;
use temper_blocks_generated::WallSignBlock;

/// Wall signs have no item form,  they're only ever produced by `place_sign`
/// when a standing sign is placed against a block's side, which already sets
/// `facing` and `waterlogged`. Nothing places one from an item, so there's no
/// placement behaviour to implement.
impl BlockBehavior for WallSignBlock {
    fn is_interactable(&self) -> bool {
        true
    }
}
