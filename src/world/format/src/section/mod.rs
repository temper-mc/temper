use crate::errors::WorldError;
use crate::light::{LightStorage, SectionLightData};
use crate::section::biome::{BiomeData, BiomeType};
use crate::section::direct::DirectSection;
use crate::section::paletted::{PalettedSection, PalettedSectionResult};
use crate::section::uniform::UniformSection;
use crate::vanilla_chunk_format::Section;
use deepsize::DeepSizeOf;
use serde_derive::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use temper_core::block_state_id::BlockStateId;
use temper_core::pos::SectionBlockPos;
use temper_macros::{block, match_block};
use type_hash::TypeHash;

mod biome;
mod direct;
pub mod network;
mod paletted;
mod uniform;

pub const CHUNK_SECTION_LENGTH: usize = 16 * 16 * 16;

pub(crate) const AIR: BlockStateId = block!("air");

#[derive(Clone, DeepSizeOf, Serialize, Deserialize, TypeHash)]
pub(crate) enum ChunkSectionType {
    Uniform(UniformSection),
    Paletted(PalettedSection),
    Direct(DirectSection),
}

impl ChunkSectionType {
    #[inline]
    pub fn get_block(&self, pos: SectionBlockPos) -> BlockStateId {
        let pos = pos.pack() as usize;

        self.get_block_index(pos)
    }

    #[inline]
    pub(crate) fn get_block_index(&self, pos: usize) -> BlockStateId {
        match self {
            Self::Uniform(data) => data.get_block(),
            Self::Paletted(data) => data.get_block(pos),
            Self::Direct(data) => data.get_block(pos),
        }
    }

    #[inline]
    pub fn set_block(&mut self, pos: SectionBlockPos, id: BlockStateId) {
        let pos = pos.pack() as usize;

        match self {
            Self::Uniform(data) => {
                // Check if the id doesn't match the block type that fills the section,
                // If not, then create a PalettedSection to hold more than one block type
                if id != data.get_block() {
                    let mut new_data = PalettedSection::from(data);
                    new_data.set_block(pos, id);
                    *self = Self::Paletted(new_data);
                }
            }
            Self::Paletted(data) => match data.set_block(pos, id) {
                // Shrink the PalettedSection into a UniformSection if one block fills the entire section
                PalettedSectionResult::Shrink(block) => {
                    *self = ChunkSectionType::Uniform(UniformSection::new_with(block))
                }
                // Expand the PalettedSection into a DirectSection if more than u8::MAX block types are in the section
                PalettedSectionResult::Expand => {
                    let mut new_data = DirectSection::from(data);
                    new_data.set_block(pos, id);
                    *self = Self::Direct(new_data);
                }
                PalettedSectionResult::Keep => {}
            },
            Self::Direct(data) => data.set_block(pos, id),
        }
    }

    pub fn fill(&mut self, id: BlockStateId) {
        match self {
            Self::Uniform(data) => data.fill(id),
            _ => *self = Self::Uniform(UniformSection::new_with(id)),
        }
    }

    pub fn block_count(&self) -> u16 {
        match self {
            Self::Uniform(data) => {
                if data.get_block() == block!("air") {
                    0
                } else {
                    4096
                }
            }
            Self::Paletted(data) => data.block_count(),
            Self::Direct(data) => data.block_count(),
        }
    }

    pub fn fluid_count(&self) -> u16 {
        match self {
            Self::Uniform(data) => {
                let state = data.get_block();
                if match_block!("water", state) || match_block!("lava", state) {
                    0
                } else {
                    4096
                }
            }
            Self::Paletted(data) => data.fluid_count(),
            Self::Direct(data) => data.fluid_count(),
        }
    }
}

#[derive(Clone, DeepSizeOf, Serialize, Deserialize, TypeHash)]
pub struct ChunkSection {
    pub(crate) y: i8,
    pub(crate) inner: ChunkSectionType,
    pub(crate) light: SectionLightData,
    pub(crate) biome: BiomeData,
    pub(crate) dirty: Arc<AtomicBool>,
}

impl ChunkSection {
    pub fn new_uniform(id: BlockStateId, y: i8) -> Self {
        Self {
            y,
            inner: ChunkSectionType::Uniform(UniformSection::new_with(id)),
            light: SectionLightData::default(),
            biome: BiomeData::Uniform(BiomeType(5)),
            dirty: Arc::new(AtomicBool::new(true)),
        }
    }

    #[expect(unused)]
    pub(crate) fn with_space_for(unique_blocks: u16, y: i8) -> Self {
        if unique_blocks <= 1 {
            Self {
                y,
                inner: ChunkSectionType::Uniform(UniformSection::air()),
                light: SectionLightData::default(),
                biome: BiomeData::Uniform(BiomeType(5)),
                dirty: Arc::new(AtomicBool::new(true)),
            }
        } else if unique_blocks < 256 {
            Self {
                y,
                inner: ChunkSectionType::Paletted(PalettedSection::new_with_block_count(
                    unique_blocks as _,
                )),
                light: SectionLightData::default(),
                biome: BiomeData::Uniform(BiomeType(5)),
                dirty: Arc::new(AtomicBool::new(true)),
            }
        } else {
            Self {
                y,
                inner: ChunkSectionType::Direct(DirectSection::default()),
                light: SectionLightData::default(),
                biome: BiomeData::Uniform(BiomeType(5)),
                dirty: Arc::new(AtomicBool::new(true)),
            }
        }
    }

    #[inline]
    pub(crate) fn get_block(&self, pos: SectionBlockPos) -> BlockStateId {
        self.inner.get_block(pos)
    }

    #[inline]
    pub(crate) fn get_block_index(&self, pos: usize) -> BlockStateId {
        debug_assert!(pos < CHUNK_SECTION_LENGTH);
        self.inner.get_block_index(pos)
    }

    #[inline]
    pub(crate) fn set_block(&mut self, pos: SectionBlockPos, id: BlockStateId) {
        self.dirty.store(true, std::sync::atomic::Ordering::Relaxed);
        self.inner.set_block(pos, id);
    }

    pub(crate) fn fill(&mut self, id: BlockStateId) {
        self.dirty.store(true, std::sync::atomic::Ordering::Relaxed);
        self.inner.fill(id);
    }

    #[expect(unused)]
    pub(crate) fn clear(&mut self) {
        self.fill(block!("air"))
    }

    #[inline]
    pub(crate) fn block_count(&self) -> u16 {
        self.inner.block_count()
    }

    #[inline]
    pub(crate) fn fluid_count(&self) -> u16 {
        self.inner.fluid_count()
    }
}

impl TryFrom<&Section> for ChunkSection {
    type Error = WorldError;

    fn try_from(value: &Section) -> Result<Self, Self::Error> {
        let sky_light = value
            .sky_light
            .clone()
            .map(LightStorage::from)
            .unwrap_or_default();
        let block_light = value
            .block_light
            .clone()
            .map(LightStorage::from)
            .unwrap_or_default();

        let light_data = SectionLightData::with_data(sky_light, block_light);

        if let Some(block_data) = value.block_states.as_ref() {
            let (block_count, block_states) = if let Some(blocks) = block_data.data.as_ref() {
                if let Some(palette) = block_data.palette.as_ref() {
                    let bits_per_block =
                        ((palette.len().saturating_sub(1) as u32).ilog2() + 1).max(4);

                    let mut values = Vec::with_capacity(4096);

                    for i in 0..4096 {
                        values.push(PalettedSection::unpack_value_unaligned(
                            bytemuck::cast_slice(blocks.as_slice()),
                            i,
                            bits_per_block as _,
                        ))
                    }

                    debug_assert_eq!(values.len(), 4096);

                    (
                        if bits_per_block >= 9 {
                            None
                        } else {
                            Some(palette.len())
                        },
                        values
                            .into_iter()
                            .map(|v| {
                                if bits_per_block >= 9 {
                                    BlockStateId::new(v.into())
                                } else {
                                    BlockStateId::from_block_data(&palette[v as usize])
                                }
                            })
                            .collect::<Vec<_>>(),
                    )
                } else {
                    return Err(WorldError::CorruptedChunkData(0, 0));
                }
            } else {
                return Ok(Self {
                    y: value.y,
                    light: light_data,
                    biome: BiomeData::Uniform(BiomeType(5)),
                    dirty: Arc::new(AtomicBool::new(false)),

                    inner: ChunkSectionType::Uniform(UniformSection::air()),
                });
            };

            let mut section_data = if let Some(block_count) = block_count {
                ChunkSectionType::Paletted(PalettedSection::new_with_block_count(block_count as _))
            } else {
                ChunkSectionType::Direct(DirectSection::default())
            };

            for (idx, block) in block_states.into_iter().enumerate() {
                section_data.set_block(
                    SectionBlockPos::unpack(idx as _).expect("should be in-bounds"),
                    block,
                )
            }

            Ok(Self {
                y: value.y,
                light: light_data,
                biome: BiomeData::Uniform(BiomeType(5)),
                dirty: Arc::new(AtomicBool::new(false)),
                inner: section_data,
            })
        } else {
            Ok(Self {
                y: value.y,
                light: light_data,
                biome: BiomeData::Uniform(BiomeType(5)),
                dirty: Arc::new(AtomicBool::new(false)),
                inner: ChunkSectionType::Uniform(UniformSection::air()),
            })
        }
    }
}
