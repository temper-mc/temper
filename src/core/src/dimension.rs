use crate::pos::ChunkHeight;
use serde::{Deserialize, Serialize};
use temper_codec::net_types::var_int::VarInt;
use type_hash::TypeHash;

/// Represents a dimension in the world. The first three variants are the standard Minecraft
/// dimensions, and the `Custom` variant can be used for modded dimensions or other custom implementations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, TypeHash)]
pub enum Dimension {
    #[default]
    Overworld,
    Nether,
    End,
    Custom(u16),
}

impl Dimension {
    pub fn as_var_int(&self) -> VarInt {
        match self {
            Self::Overworld => VarInt::from(0), // overworld caves is id 1, idk what that is
            Self::Nether => VarInt::from(3),
            Self::End => VarInt::from(2),
            Self::Custom(v) => VarInt::from(i32::from(*v) + 4),
        }
    }

    pub fn as_identifier(&self) -> &'static str {
        match self {
            Self::Overworld => "minecraft:overworld",
            Self::Nether => "minecraft:the_nether",
            Self::End => "minecraft:the_end",
            _ => "temper:custom_dimension",
        }
    }

    pub fn chunk_height(&self) -> ChunkHeight {
        match self {
            Dimension::Overworld => ChunkHeight::new(-64, 384),
            _ => ChunkHeight::new(0, 256),
        }
    }
}
