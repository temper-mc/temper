use bitcode_derive::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Display;
use temper_macros::{NBTDeserialize, NBTSerialize};

/// Information about a block's name and properties.
///
/// This should be used sparingly, as it's much more efficient to use [BlockId] where possible.
///
/// If you want to use it as a literal and the convert to a BlockId, use the [temper_macros::block_data!] macro.
#[derive(
    NBTSerialize,
    NBTDeserialize,
    Debug,
    Clone,
    PartialEq,
    Encode,
    Serialize,
    Decode,
    Deserialize,
    Eq,
    deepsize::DeepSizeOf,
    Hash,
)]
pub struct BlockData {
    #[nbt(rename = "Name")]
    pub name: String,
    #[nbt(rename = "Properties")]
    pub properties: Option<BTreeMap<String, String>>,
}

impl Display for BlockData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Default for BlockData {
    fn default() -> Self {
        BlockData {
            name: String::from("minecraft:air"),
            properties: None,
        }
    }
}
