use crate::Chunk;
use crate::heightmap::{Heightmaps, NetworkHeightmap};
use crate::section::network::NetworkSection;
use std::io::Cursor;
use temper_codec::encode::errors::NetEncodeError;
use temper_codec::encode::{NetEncode, NetEncodeOpts};
use temper_codec::net_types::byte_array::ByteArray;
use temper_codec::net_types::length_prefixed_vec::LengthPrefixedVec;
use temper_macros::NetEncode;

#[derive(NetEncode)]
pub struct NetworkChunk {
    heightmaps: LengthPrefixedVec<NetworkHeightmap>,
    data: ByteArray,
}

impl TryFrom<&Chunk> for NetworkChunk {
    type Error = NetEncodeError;

    fn try_from(chunk: &Chunk) -> Result<Self, Self::Error> {
        let heightmaps = Heightmaps::get_network_repr(&chunk.heightmaps);
        let mut data = Cursor::new(vec![]);

        for section in chunk.sections.iter() {
            let section = NetworkSection::from(section);
            section.encode(&mut data, &NetEncodeOpts::None)?;
        }

        Ok(Self {
            heightmaps,
            data: ByteArray::new(data.into_inner()),
        })
    }
}

impl Default for NetworkChunk {
    fn default() -> Self {
        Self {
            heightmaps: LengthPrefixedVec::default(),
            data: ByteArray::new(Vec::new()),
        }
    }
}
