// temper/src/core/src/block_state_id.rs
use deepsize::DeepSizeOf;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use temper_codec::net_types::var_int::VarInt;
use type_hash::TypeHash;

use temper_registry::blocks::{self, BlockMaterial};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, DeepSizeOf, TypeHash)]
#[repr(transparent)]
pub struct BlockStateId(pub u32);

impl BlockStateId {
    #[inline(always)]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    #[inline(always)]
    pub fn from_varint(var_int: VarInt) -> Self {
        Self(var_int.0 as u32)
    }

    #[inline(always)]
    pub fn to_varint(&self) -> VarInt {
        VarInt(self.0 as i32)
    }

    #[inline(always)]
    pub const fn raw(&self) -> u32 {
        self.0
    }

    // --- Registry Lookups ---

    #[inline(always)]
    pub fn hardness(self) -> f32 {
        blocks::hardness(self.0)
    }

    #[inline(always)]
    pub fn material(self) -> BlockMaterial {
        blocks::material(self.0)
    }

    #[inline(always)]
    pub fn is_transparent(self) -> bool {
        blocks::is_transparent(self.0)
    }

    #[inline(always)]
    pub fn is_solid(self) -> bool {
        temper_registry::blocks::is_solid(self.0)
    }
}

impl Display for BlockStateId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BlockStateId({})", self.0)
    }
}

impl Default for BlockStateId {
    fn default() -> Self {
        Self(0)
    }
}

// -----------------------------------------------------------------------------
// ITEM TO BLOCK MAPPING
// Note: This still relies on runtime JSON parsing and a slow HashMap.
// You should migrate this to `temper-registry/src/items.rs` using a flat array
// in exactly the same way we just did for blocks.
// -----------------------------------------------------------------------------

use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::str::FromStr;

const ITEM_TO_BLOCK_MAPPING_FILE: &str =
    include_str!("../../../assets/data/item_to_block_mapping.json");
pub static ITEM_TO_BLOCK_MAPPING: OnceCell<HashMap<i32, BlockStateId>> = OnceCell::new();

pub fn create_item_to_block_mapping() -> HashMap<i32, BlockStateId> {
    let str_form: HashMap<String, String> = serde_json::from_str(ITEM_TO_BLOCK_MAPPING_FILE)
        .expect("Failed to parse item_to_block_mapping.json");
    str_form
        .into_iter()
        .map(|(k, v)| {
            (
                i32::from_str(&k).unwrap(),
                BlockStateId::new(u32::from_str(&v).unwrap()),
            )
        })
        .collect()
}
