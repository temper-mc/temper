use crate::Chunk;
use crate::errors::WorldError;
use crate::heightmap::{Heightmaps, NetworkHeightmap};
use crate::section::network::NetworkSection;
use std::io::Cursor;
use temper_codec::encode::errors::NetEncodeError;
use temper_codec::encode::{NetEncode, NetEncodeOpts};
use temper_codec::net_types::byte_array::ByteArray;
use temper_codec::net_types::length_prefixed_vec::LengthPrefixedVec;
use temper_codec::net_types::var_int::VarInt;
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

#[derive(NetEncode)]
pub struct BlockEntity {
    pub xz: u8,
    pub y: i16,
    pub entity_type: VarInt,
    pub nbt: Vec<u8>,
}

impl TryFrom<&Chunk> for Vec<BlockEntity> {
    type Error = WorldError;

    fn try_from(chunk: &Chunk) -> Result<Self, Self::Error> {
        chunk
            .block_entities
            .iter()
            .map(|entry| {
                let block_pos = entry.key();
                let data = entry.value();

                Ok(BlockEntity {
                    xz: (block_pos.x() << 4) | block_pos.z(),
                    y: block_pos.y(),
                    entity_type: VarInt::new(i32::from(data.protocol_id)),
                    nbt: data.kind.to_network_nbt(&data.blob)?,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BlockEntityData, BlockEntityKind, SignBlockEntity, SignText};
    use temper_core::pos::ChunkBlockPos;
    use temper_text::TextComponent;

    fn test_sign_blob() -> Vec<u8> {
        let sign = SignBlockEntity {
            is_waxed: false,
            front_text: SignText {
                messages: vec![TextComponent::default(); 4],
                color: "black".to_string(),
                has_glowing_text: false,
            },
            back_text: SignText {
                messages: vec![TextComponent::default(); 4],
                color: "black".to_string(),
                has_glowing_text: false,
            },
        };
        sign.to_blob().expect("sign should serialize")
    }

    #[test]
    fn block_entities_convert_with_packed_coordinates() {
        let chunk = Chunk::new_empty();
        chunk.block_entities.insert(
            ChunkBlockPos::new(3, 64, 11),
            BlockEntityData {
                kind: BlockEntityKind::Sign,
                protocol_id: 7,
                blob: test_sign_blob(),
            },
        );

        let entities = Vec::<BlockEntity>::try_from(&chunk).expect("should convert");

        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].xz, (3 << 4) | 11);
        assert_eq!(entities[0].y, 64);
        assert_eq!(entities[0].entity_type, VarInt::new(7));
    }

    #[test]
    fn block_entity_y_is_signed() {
        let chunk = Chunk::new_empty();
        chunk.block_entities.insert(
            ChunkBlockPos::new(0, -40, 0),
            BlockEntityData {
                kind: BlockEntityKind::Sign,
                protocol_id: 7,
                blob: test_sign_blob(),
            },
        );

        let entities = Vec::<BlockEntity>::try_from(&chunk).expect("should convert");
        assert_eq!(entities[0].y, -40);
    }
}
