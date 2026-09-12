use crate::BlockBehavior;
use std::collections::HashMap;
use temper_block_data::{PlacedBlocks, PlacementContext};
use temper_block_properties::Direction;
use temper_blocks_generated::{SignBlock, SignBlockType, WallSignBlock, WallSignBlockType};
use temper_core::block_face::BlockFace;
use temper_core::block_state_id::BlockStateId;
use temper_macros::match_block;

use crate::decorative::yaw_to_sign_rotation;

impl BlockBehavior for SignBlock {
    fn get_placement_state(&mut self, context: PlacementContext) -> PlacedBlocks {
        place_sign(context, self.block_type.clone())
    }

    fn is_interactable(&self) -> bool {
        true
    }
}

fn place_sign(context: PlacementContext, ty: SignBlockType) -> PlacedBlocks {
    let existing = context
        .level
        .get_chunk(context.block_pos.chunk(), context.dimension)
        .map(|c| c.get_block(context.block_pos.chunk_block_pos()))
        .unwrap_or(BlockStateId::new(0));
    let waterlogged = match_block!("water", existing);

    let block = match context.face {
        BlockFace::Top => SignBlock {
            block_type: ty,
            rotation: yaw_to_sign_rotation(context.player_rotation.yaw),
            waterlogged,
        }
        .try_into()
        .expect("Should be able to convert SignBlock to id"),

        BlockFace::Bottom => {
            // Signs can't hang from the underside of a block.
            return PlacedBlocks {
                blocks: HashMap::with_capacity(0),
                take_item: false,
                place_original: false,
            };
        }

        face => {
            let facing = match face {
                BlockFace::East => Direction::East,
                BlockFace::West => Direction::West,
                BlockFace::North => Direction::North,
                BlockFace::South => Direction::South,
                _ => unreachable!(),
            };

            WallSignBlock {
                block_type: wall_sign_type(&ty),
                facing,
                waterlogged,
            }
            .try_into()
            .expect("Should be able to convert WallSignBlock to id")
        }
    };

    PlacedBlocks {
        blocks: [(context.block_pos, BlockStateId::new(block))]
            .into_iter()
            .collect::<HashMap<_, _>>(),
        take_item: true,
        place_original: false,
    }
}

fn wall_sign_type(ty: &SignBlockType) -> WallSignBlockType {
    match ty {
        SignBlockType::AcaciaSign => WallSignBlockType::AcaciaWallSign,
        SignBlockType::BambooSign => WallSignBlockType::BambooWallSign,
        SignBlockType::BirchSign => WallSignBlockType::BirchWallSign,
        SignBlockType::CherrySign => WallSignBlockType::CherryWallSign,
        SignBlockType::CrimsonSign => WallSignBlockType::CrimsonWallSign,
        SignBlockType::DarkOakSign => WallSignBlockType::DarkOakWallSign,
        SignBlockType::JungleSign => WallSignBlockType::JungleWallSign,
        SignBlockType::MangroveSign => WallSignBlockType::MangroveWallSign,
        SignBlockType::OakSign => WallSignBlockType::OakWallSign,
        SignBlockType::PaleOakSign => WallSignBlockType::PaleOakWallSign,
        SignBlockType::SpruceSign => WallSignBlockType::SpruceWallSign,
        SignBlockType::WarpedSign => WallSignBlockType::WarpedWallSign,
    }
}
