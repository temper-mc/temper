use crate::BlockBehavior;
use std::collections::HashMap;
use temper_block_data::{PlacedBlocks, PlacementContext};
use temper_block_properties::Direction;
use temper_blocks_generated::{
    HangingSignBlock, HangingSignBlockType, WallHangingSignBlock, WallHangingSignBlockType,
};
use temper_core::block_face::BlockFace;
use temper_core::block_state_id::BlockStateId;
use temper_macros::match_block;

use crate::decorative::yaw_to_sign_rotation;

impl BlockBehavior for HangingSignBlock {
    fn get_placement_state(&mut self, context: PlacementContext) -> PlacedBlocks {
        place_hanging_sign(context, self.block_type.clone())
    }

    fn is_interactable(&self) -> bool {
        true
    }
}

fn place_hanging_sign(context: PlacementContext, ty: HangingSignBlockType) -> PlacedBlocks {
    let existing = context
        .level
        .get_chunk(context.block_pos.chunk(), context.dimension)
        .map(|c| c.get_block(context.block_pos.chunk_block_pos()))
        .unwrap_or(BlockStateId::new(0));
    let waterlogged = match_block!("water", existing);

    let block = match context.face {
        // Hanging from the underside of a block.
        BlockFace::Bottom => HangingSignBlock {
            block_type: ty,
            // TODO: `attached` should be true when hanging directly from a
            // fence or another hanging sign rather than by chains.
            attached: false,
            rotation: yaw_to_sign_rotation(context.player_rotation.yaw),
            waterlogged,
        }
        .try_into()
        .expect("Should be able to convert HangingSignBlock to id"),

        BlockFace::Top => {
            // Hanging signs can't stand on the ground.
            return PlacedBlocks {
                blocks: HashMap::with_capacity(0),
                take_item: false,
                place_original: false,
            };
        }

        face => {
            // A wall hanging sign's board runs perpendicular to the wall it's
            // mounted on, so the axis comes from the clicked face.
            let wall = match face {
                BlockFace::East => Direction::East,
                BlockFace::West => Direction::West,
                BlockFace::North => Direction::North,
                BlockFace::South => Direction::South,
                _ => unreachable!(),
            };

            let yaw = context.player_rotation.yaw.rem_euclid(360.0);
            let facing = match wall {
                // Player is looking north; split on yaw sign (0..180 vs 180..360).
                Direction::South => {
                    if yaw > 180.0 {
                        Direction::West
                    } else {
                        Direction::East
                    }
                }
                // Player is looking south; split on yaw sign (0..180 vs 180..360).
                Direction::North => {
                    if yaw < 180.0 {
                        Direction::East
                    } else {
                        Direction::West
                    }
                }
                // Player is looking west; split on 90/270.
                Direction::East => {
                    if (90.0..270.0).contains(&yaw) {
                        Direction::South
                    } else {
                        Direction::North
                    }
                }
                // Player is looking east; split on 90/270.
                Direction::West => {
                    if (90.0..270.0).contains(&yaw) {
                        Direction::South
                    } else {
                        Direction::North
                    }
                }
                _ => unreachable!(),
            };

            WallHangingSignBlock {
                block_type: wall_hanging_sign_type(&ty),
                facing,
                waterlogged,
            }
            .try_into()
            .expect("Should be able to convert WallHangingSignBlock to id")
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

fn wall_hanging_sign_type(ty: &HangingSignBlockType) -> WallHangingSignBlockType {
    match ty {
        HangingSignBlockType::AcaciaHangingSign => WallHangingSignBlockType::AcaciaWallHangingSign,
        HangingSignBlockType::BambooHangingSign => WallHangingSignBlockType::BambooWallHangingSign,
        HangingSignBlockType::BirchHangingSign => WallHangingSignBlockType::BirchWallHangingSign,
        HangingSignBlockType::CherryHangingSign => WallHangingSignBlockType::CherryWallHangingSign,
        HangingSignBlockType::CrimsonHangingSign => {
            WallHangingSignBlockType::CrimsonWallHangingSign
        }
        HangingSignBlockType::DarkOakHangingSign => {
            WallHangingSignBlockType::DarkOakWallHangingSign
        }
        HangingSignBlockType::JungleHangingSign => WallHangingSignBlockType::JungleWallHangingSign,
        HangingSignBlockType::MangroveHangingSign => {
            WallHangingSignBlockType::MangroveWallHangingSign
        }
        HangingSignBlockType::OakHangingSign => WallHangingSignBlockType::OakWallHangingSign,
        HangingSignBlockType::PaleOakHangingSign => {
            WallHangingSignBlockType::PaleOakWallHangingSign
        }
        HangingSignBlockType::SpruceHangingSign => WallHangingSignBlockType::SpruceWallHangingSign,
        HangingSignBlockType::WarpedHangingSign => WallHangingSignBlockType::WarpedWallHangingSign,
    }
}
