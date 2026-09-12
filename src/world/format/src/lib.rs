pub mod errors;
pub mod heightmap;
pub mod light;
pub mod network;
mod palette;
pub mod section;
pub mod vanilla_chunk_format;

use crate::errors::WorldError;
use crate::heightmap::Heightmaps;
use crate::section::{AIR, ChunkSection};
use dashmap::DashMap;
use serde_derive::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use temper_core::block_state_id::BlockStateId;
use temper_core::pos::{ChunkBlockPos, ChunkHeight};
use temper_entities::entity_types::EntityTypeEnum;
use temper_macros::{NBTSerialize, block, match_block};
use temper_nbt::{NBTSerializable, NBTSerializeOptions};
use temper_text::TextComponent;
use type_hash::TypeHash;
use uuid::Uuid;
use vanilla_chunk_format::VanillaChunk;

#[derive(Clone, Serialize, Deserialize, TypeHash)]
pub struct Chunk {
    pub sections: Box<[ChunkSection]>,
    height: ChunkHeight,
    #[type_hash(foreign_type)]
    pub entities: DashMap<Uuid, (EntityTypeEnum, Vec<u8>)>,

    #[type_hash(foreign_type)]
    pub block_entities: DashMap<ChunkBlockPos, BlockEntityData>,

    pub heightmaps: Heightmaps,
    dirty: Arc<AtomicBool>,
    pub stage: u8,
    pub noise: ChunkNoises,
}

#[derive(Clone, Serialize, Deserialize, TypeHash)]
pub struct ChunkNoises {
    /// Continentalness noise. Indexed as noise\[z]\[x] from 0,0 to 15,15
    pub continentalness: [[f32; 16]; 16],
    /// Erosion noise. Indexed as noise\[z]\[x] from 0,0 to 15,15
    pub erosion: [[f32; 16]; 16],
    /// Weirdness noise. Indexed as noise\[z]\[x] from 0,0 to 15,15
    pub weirdness: [[f32; 16]; 16],
    /// Jagged noise. Indexed as noise\[z]\[x] from 0,0 to 15,15
    pub jaggedness: [[f32; 16]; 16],
    /// Temperature noise. Indexed as noise\[z]\[x] from 0,0 to 15,15
    pub temperature: [[f32; 16]; 16],
    /// Humidity noise. Indexed as noise\[z]\[x] from 0,0 to 15,15
    pub humidity: [[f32; 16]; 16],

    // Stored as vecs to avoid putting 16*16*384 f32s on the stack. Which is nearly 400kb each.
    /// Base 3d noise. Indexed as noise\[z]\[y]\[x] from 0,-64,0 to 15,383,15
    pub base3d: Vec<f32>,
    /// Cheese cave noise. Indexed as noise\[z]\[y]\[x] from 0,-64,0 to 15,383,15
    pub cheese_caves: Vec<f32>,
    /// Spaghetti cave noise. Indexed as noise\[z]\[y]\[x] from 0,-64,0 to 15,383,15
    pub spaghetti_caves: Vec<f32>,
    /// Noodle cave noise. Indexed as noise\[z]\[y]\[x] from 0,-64,0 to 15,383,15
    pub noddle_caves: Vec<f32>,
}

impl Default for ChunkNoises {
    fn default() -> Self {
        Self {
            continentalness: [[0.0; 16]; 16],
            erosion: [[0.0; 16]; 16],
            weirdness: [[0.0; 16]; 16],
            jaggedness: [[0.0; 16]; 16],
            temperature: [[0.0; 16]; 16],
            humidity: [[0.0; 16]; 16],
            base3d: Vec::new(),
            cheese_caves: Vec::new(),
            spaghetti_caves: Vec::new(),
            noddle_caves: Vec::new(),
        }
    }
}

impl ChunkNoises {
    pub fn without_transient_3d(&self) -> Self {
        Self {
            continentalness: self.continentalness,
            erosion: self.erosion,
            weirdness: self.weirdness,
            jaggedness: self.jaggedness,
            temperature: self.temperature,
            humidity: self.humidity,
            base3d: Vec::new(),
            cheese_caves: Vec::new(),
            spaghetti_caves: Vec::new(),
            noddle_caves: Vec::new(),
        }
    }

    pub fn clear_transient_3d(&mut self) {
        self.base3d.clear();
        self.cheese_caves.clear();
        self.spaghetti_caves.clear();
        self.noddle_caves.clear();
    }
}

impl Chunk {
    /// Returns a chunk that is completely filled with air.
    ///
    /// This uses the overworld [`ChunkHeight`] (-64..320) as the chunk's height.
    ///
    /// # Returns
    ///
    /// * An empty chunk filled with air using the overworld [`ChunkHeight`].
    pub fn new_empty() -> Chunk {
        Self::new_empty_with_height(ChunkHeight::new(-64, 384))
    }

    /// Returns a chunk that is completely filled with air.
    ///
    /// # Arguments
    ///
    /// * `height` - The [`ChunkHeight`] that this chunk should be set to
    ///
    /// # Returns
    ///
    /// * An empty chunk filled with air using the given [`ChunkHeight`].
    pub fn new_empty_with_height(height: ChunkHeight) -> Chunk {
        let sections = (-4..20)
            .map(|y| ChunkSection::new_uniform(AIR, y))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Self {
            sections,
            height,
            entities: DashMap::new(),
            heightmaps: Heightmaps::default(),
            dirty: Arc::new(AtomicBool::new(false)),
            stage: 0,
            noise: ChunkNoises::default(),
            block_entities: DashMap::new(),
        }
    }

    /// Creates a chunk using the given sections and height.
    ///
    /// # Arguments
    ///
    /// * `sections` - The sections to fill the chunk with. These should be in order from the bottom of the world at index 0 and the top at the end of the slice.
    /// * `height` - The [`ChunkHeight`] to use.
    ///
    /// # Asserts
    ///
    /// * debug_assert_eq: `sections` contains enough [`ChunkSection`]s to fill the chunk based on the given [`ChunkHeight`].
    ///
    /// # Returns
    ///
    /// * A chunk using the given sections and [`ChunkHeight`]
    pub fn new_with_sections(sections: &[ChunkSection], height: ChunkHeight) -> Chunk {
        debug_assert_eq!(height.height as usize / 16, sections.len());

        Self {
            sections: sections.to_vec().into_boxed_slice(),
            height,
            heightmaps: Heightmaps::default(),
            entities: DashMap::new(),
            block_entities: DashMap::new(),
            dirty: Arc::new(AtomicBool::new(false)),
            stage: 0,
            noise: ChunkNoises::default(),
        }
    }

    pub fn clone_without_transient_noise(&self) -> Chunk {
        Chunk {
            sections: self.sections.clone(),
            height: self.height,
            entities: self.entities.clone(),
            block_entities: self.block_entities.clone(),
            heightmaps: self.heightmaps.clone(),
            dirty: Arc::clone(&self.dirty),
            stage: self.stage,
            noise: self.noise.without_transient_3d(),
        }
    }

    /// Fills an entire [`ChunkSection`] with the given block.
    ///
    /// # Arguments
    ///
    /// * `y` - The y of the section to fill.
    /// * `state` - The [`BlockStateId`] to fill the section with.
    ///
    /// # Asserts
    ///
    /// * `assert` - Checks if the given y value is in range of the height of the chunk.
    pub fn fill_section(&mut self, y: i8, state: BlockStateId) {
        assert!(i16::from(y) >= self.height.min_y / 16);
        assert!(i16::from(y) < (self.height.min_y + self.height.height as i16) / 16);

        let section = self
            .sections
            .iter_mut()
            .find(|s| s.y == y)
            .expect("Section not found");

        *section = ChunkSection::new_uniform(state, y);
    }

    /// Fills the entire chunk with the given block.
    ///
    /// # Arguments
    ///
    /// * `state` - The [`BlockStateId`] of the block to fill the chunk with.
    pub fn fill(&mut self, state: BlockStateId) {
        for section in &mut self.sections {
            *section = ChunkSection::new_uniform(state, section.y);
        }
    }

    /// Gets a block in the chunk.
    ///
    /// # Arguments
    ///
    /// * `pos` - The position of the block to get.
    ///
    /// # Returns
    ///
    /// * The [`BlockStateId`] of the block at the requested position. If the position is above the maximum y of the chunk, air is always returned.
    ///   If the position is below the minimum y of the chunk, void air is always returned.
    pub fn get_block(&self, pos: ChunkBlockPos) -> BlockStateId {
        let section = (pos.y() + -self.height.min_y) / 16;
        if section < 0 {
            return block!("void_air");
        }

        if section as usize >= self.sections.len() {
            return block!("air");
        }

        self.sections[section as usize].get_block(pos.section_block_pos())
    }

    /// Sets a block in the chunk.
    ///
    /// # Arguments
    ///
    /// * `pos` - The position of the block to set within the chunk.
    /// * `id` - The [`BlockStateId`] of the block to set.
    ///
    /// # Asserts
    ///
    /// * `assert` - Checks to ensure that the given position is in-bounds.
    pub fn set_block(&mut self, pos: ChunkBlockPos, id: BlockStateId) {
        let section = (pos.y() + -self.height.min_y) / 16;
        assert!(section >= 0);
        assert!((section as usize) < self.sections.len());

        self.sections[section as usize].set_block(pos.section_block_pos(), id);

        let motion_block_for_xz = self.heightmaps.motion_blocking.get_height(pos.x(), pos.z());
        let world_surface_for_xz = self.heightmaps.world_surface.get_height(pos.x(), pos.z());

        if pos.y() > motion_block_for_xz
            && !(match_block!("air", id) || match_block!("void_air", id))
        {
            self.heightmaps
                .motion_blocking
                .set_height(pos.x(), pos.z(), pos.y());
        } else if pos.y() == motion_block_for_xz
            && !(!(match_block!("air", id) || match_block!("void_air", id)))
        {
            self.recalculate_heightmap_column(pos.x(), pos.z());
        }

        if pos.y() > world_surface_for_xz
            && (!(match_block!("air", id) || match_block!("void_air", id))
                && !(match_block!("water", id) || match_block!("lava", id)))
        {
            self.heightmaps
                .world_surface
                .set_height(pos.x(), pos.z(), pos.y());
        } else if pos.y() == world_surface_for_xz
            && !(!(match_block!("air", id) || match_block!("void_air", id))
                && !(match_block!("water", id) || match_block!("lava", id)))
        {
            self.recalculate_heightmap_column(pos.x(), pos.z());
        }
    }

    /// Does what it says on the tin, sets blocks without updating the heightmaps. Remember to
    /// recalculate the heightmaps at the end.
    pub fn set_block_without_heightmap(&mut self, pos: ChunkBlockPos, id: BlockStateId) {
        let section = (pos.y() + -self.height.min_y) / 16;
        assert!(section >= 0);
        assert!((section as usize) < self.sections.len());

        self.sections[section as usize].set_block(pos.section_block_pos(), id);
    }

    /// Marks the chunk as dirty.
    ///
    /// This indicates that the chunk has been modified and may need to be saved or updated.
    pub fn mark_dirty(&self) {
        self.dirty.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// Checks if the chunk is dirty.
    ///
    /// A chunk is considered dirty if it has been marked as dirty or if any of its sections are dirty.
    /// A dirty chunk may need to be saved or updated.
    pub fn is_dirty(&self) -> bool {
        self.dirty.load(std::sync::atomic::Ordering::Relaxed)
            || self
                .sections
                .iter()
                .any(|s| s.dirty.load(std::sync::atomic::Ordering::Relaxed))
    }

    /// Clears the dirty state of the chunk and all of its sections.
    ///
    /// This should be called after saving or updating a chunk to indicate that it is no longer dirty.
    pub fn clear_dirty(&self) {
        self.dirty
            .store(false, std::sync::atomic::Ordering::Relaxed);
        for section in &self.sections {
            section
                .dirty
                .store(false, std::sync::atomic::Ordering::Relaxed);
        }
    }

    pub fn recalculate_heightmap(&mut self) {
        const COLUMN_COUNT: usize = 16 * 16;

        let empty_height = self.height.min_y - 1;
        let mut world_surface_set = [false; COLUMN_COUNT];
        let mut motion_blocking_set = [false; COLUMN_COUNT];
        let mut remaining_world_surface = COLUMN_COUNT;
        let mut remaining_motion_blocking = COLUMN_COUNT;

        self.mark_dirty();

        for z in 0..16 {
            for x in 0..16 {
                self.heightmaps.world_surface.set_height(x, z, empty_height);
                self.heightmaps
                    .motion_blocking
                    .set_height(x, z, empty_height);
            }
        }

        for section in self.sections.iter().rev() {
            for y in (0u8..16).rev() {
                let height = i16::from(y) + i16::from(section.y) * 16;

                for z in 0u8..16 {
                    for x in 0u8..16 {
                        let idx = (usize::from(z) << 4) | usize::from(x);

                        if world_surface_set[idx] && motion_blocking_set[idx] {
                            continue;
                        }

                        let block_idx =
                            (usize::from(y) << 8) | (usize::from(z) << 4) | usize::from(x);
                        let block = section.get_block_index(block_idx);

                        if !world_surface_set[idx] && is_world_surface_block(block) {
                            world_surface_set[idx] = true;
                            remaining_world_surface -= 1;
                            self.heightmaps.world_surface.set_height(x, z, height);
                        }

                        if !motion_blocking_set[idx] && is_motion_blocking_block(block) {
                            motion_blocking_set[idx] = true;
                            remaining_motion_blocking -= 1;
                            self.heightmaps.motion_blocking.set_height(x, z, height);
                        }
                    }
                }

                if remaining_world_surface == 0 && remaining_motion_blocking == 0 {
                    return;
                }
            }
        }
    }

    pub fn recalculate_heightmap_column(&mut self, x: u8, z: u8) {
        let mut world_surface_set = false;
        let mut motion_blocking_set = false;

        for section in self.sections.iter().rev() {
            for y in (0u8..16).rev() {
                let block_idx = (usize::from(y) << 8) | (usize::from(z) << 4) | usize::from(x);
                let block = section.get_block_index(block_idx);
                let height = i16::from(y) + i16::from(section.y) * 16;

                if !world_surface_set && is_world_surface_block(block) {
                    world_surface_set = true;
                    self.heightmaps.world_surface.set_height(x, z, height);
                }

                if !motion_blocking_set && is_motion_blocking_block(block) {
                    motion_blocking_set = true;
                    self.heightmaps.motion_blocking.set_height(x, z, height);
                }

                if motion_blocking_set && world_surface_set {
                    return;
                }
            }
        }

        // No blocks in this column
        self.heightmaps
            .world_surface
            .set_height(x, z, self.height.min_y - 1);
        self.heightmaps
            .motion_blocking
            .set_height(x, z, self.height.min_y - 1);
        self.mark_dirty();
    }
}

fn is_air(block: BlockStateId) -> bool {
    match_block!("air", block) || match_block!("void_air", block)
}

fn is_fluid(block: BlockStateId) -> bool {
    match_block!("water", block) || match_block!("lava", block)
}

fn is_world_surface_block(block: BlockStateId) -> bool {
    !is_air(block) && !is_fluid(block)
}

fn is_motion_blocking_block(block: BlockStateId) -> bool {
    !is_air(block)
}

impl TryFrom<&VanillaChunk> for Chunk {
    type Error = WorldError;

    fn try_from(value: &VanillaChunk) -> Result<Self, Self::Error> {
        let mut sections =
            Vec::with_capacity(value.sections.as_ref().map(|s| s.len()).unwrap_or(0));

        if value.status != "minecraft:full" {
            return Err(WorldError::CorruptedChunkData(0, 0));
        }

        for section in value
            .sections
            .as_ref()
            .ok_or(WorldError::CorruptedChunkData(
                value.x_pos as _,
                value.z_pos as _,
            ))?
            .iter()
        {
            sections.push(ChunkSection::try_from(section)?);
        }

        sections.sort_by_key(|a| a.y);

        Ok(Chunk {
            sections: sections.into_boxed_slice(),
            height: ChunkHeight::new(-64, 384),
            heightmaps: value
                .heightmaps
                .as_ref()
                .and_then(|h| Heightmaps::try_from(h).ok())
                .unwrap_or_default(),
            entities: DashMap::new(),
            block_entities: DashMap::new(),
            dirty: Arc::new(AtomicBool::new(false)),
            stage: 6,
            noise: ChunkNoises::default(),
        })
    }
}

/// A block entity stored in a chunk. `protocol_id` comes from the blockstate
/// via `temper_data`'s generated `block_entity_type_for_state` at placement
/// time, so it stays correct across version bumps without this crate
/// depending on the block data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BlockEntityData {
    pub kind: BlockEntityKind,
    pub protocol_id: u16,
    pub blob: Vec<u8>,
}

/// A block entity type stored in a chunk. The variant determines how the
/// accompanying blob deserializes; the protocol ID for the wire comes from
/// the blockstate via `temper_data`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockEntityKind {
    Sign,
}

impl BlockEntityKind {
    /// Deserializes a stored blob and re-serializes it as network NBT for the wire.
    ///
    /// Blobs are JSON rather than bitcode like the rest of the chunk: `TextComponent`
    /// uses `#[serde(flatten)]`, which serializes as a map with no known length, and
    /// bitcode requires one. The blob is opaque to `Chunk` either way.
    pub fn to_network_nbt(self, blob: &[u8]) -> Result<Vec<u8>, WorldError> {
        let mut buf = Vec::new();
        match self {
            Self::Sign => {
                let sign: SignBlockEntity = serde_json::from_slice(blob)
                    .map_err(|e| WorldError::BlockEntityDeserializeError(e.to_string()))?;
                NBTSerializable::serialize(&sign, &mut buf, &NBTSerializeOptions::Network);
            }
        }
        Ok(buf)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, NBTSerialize)]
pub struct SignText {
    pub messages: Vec<TextComponent>,
    pub color: String,
    pub has_glowing_text: bool,
}

impl Default for SignText {
    fn default() -> Self {
        Self {
            messages: vec![TextComponent::default(); 4],
            color: "black".to_string(),
            has_glowing_text: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, NBTSerialize)]
pub struct SignBlockEntity {
    pub is_waxed: bool,
    pub front_text: SignText,
    pub back_text: SignText,
}

impl SignBlockEntity {
    pub fn to_blob(&self) -> Result<Vec<u8>, WorldError> {
        serde_json::to_vec(self).map_err(|e| WorldError::BlockEntitySerializeError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use crate::BlockEntityKind;
    use crate::BlockStateId;
    use crate::Chunk;
    use crate::SignBlockEntity;
    use crate::SignText;
    use temper_core::pos::ChunkBlockPos;
    use temper_macros::block;
    use temper_text::TextComponent;
    use temper_text::TextContent;

    #[test]
    fn test_read_write() {
        let mut chunk = Chunk::new_empty();

        chunk.set_block(ChunkBlockPos::new(0, 0, 0), block!("stone"));
        chunk.set_block(ChunkBlockPos::new(0, 16, 1), block!("dirt"));

        assert_eq!(
            chunk.get_block(ChunkBlockPos::new(0, 0, 0)),
            block!("stone")
        );
        assert_eq!(
            chunk.get_block(ChunkBlockPos::new(0, 16, 1)),
            block!("dirt")
        );
    }

    #[test]
    fn motion_blocking_counts_water() {
        let mut chunk = Chunk::new_empty();

        chunk.set_block(ChunkBlockPos::new(0, 10, 0), block!("stone"));
        chunk.set_block(ChunkBlockPos::new(0, 63, 0), block!("water", {level: 0}));

        assert_eq!(chunk.heightmaps.world_surface.get_height(0, 0), 10);
        assert_eq!(chunk.heightmaps.motion_blocking.get_height(0, 0), 63);
    }

    #[test]
    fn transient_noise_can_be_dropped_from_chunk_snapshots() {
        let mut chunk = Chunk::new_empty();
        chunk.noise.continentalness[3][4] = 0.75;
        chunk.noise.base3d = vec![0.0; 43];
        chunk.noise.cheese_caves = vec![0.0; 43];
        chunk.noise.spaghetti_caves = vec![0.0; 43];
        chunk.noise.noddle_caves = vec![0.0; 43];
        chunk.noise.base3d[42] = 0.5;
        chunk.noise.cheese_caves[42] = 0.25;
        chunk.noise.spaghetti_caves[42] = 0.125;
        chunk.noise.noddle_caves[42] = 0.0625;

        let snapshot = chunk.clone_without_transient_noise();

        assert_eq!(snapshot.noise.continentalness[3][4], 0.75);
        assert!(snapshot.noise.base3d.is_empty());
        assert!(snapshot.noise.cheese_caves.is_empty());
        assert!(snapshot.noise.spaghetti_caves.is_empty());
        assert!(snapshot.noise.noddle_caves.is_empty());
    }

    #[test]
    fn recalculated_motion_blocking_counts_water() {
        let mut chunk = Chunk::new_empty();

        chunk.set_block(ChunkBlockPos::new(0, 10, 0), block!("stone"));
        chunk.set_block(ChunkBlockPos::new(0, 63, 0), block!("water", {level: 0}));
        chunk.recalculate_heightmap_column(0, 0);

        assert_eq!(chunk.heightmaps.world_surface.get_height(0, 0), 10);
        assert_eq!(chunk.heightmaps.motion_blocking.get_height(0, 0), 63);
    }

    #[test]
    fn fluids_do_not_replace_higher_world_surface() {
        let mut chunk = Chunk::new_empty();

        chunk.set_block(ChunkBlockPos::new(0, 80, 0), block!("stone"));
        chunk.set_block(ChunkBlockPos::new(0, 96, 0), block!("lava", {level: 0}));

        assert_eq!(chunk.heightmaps.world_surface.get_height(0, 0), 80);
        assert_eq!(chunk.heightmaps.motion_blocking.get_height(0, 0), 96);
    }

    #[test]
    fn removing_top_fluid_falls_back_to_solid_block() {
        let mut chunk = Chunk::new_empty();

        chunk.set_block(ChunkBlockPos::new(0, 10, 0), block!("stone"));
        chunk.set_block(ChunkBlockPos::new(0, 63, 0), block!("water", {level: 0}));
        chunk.set_block(ChunkBlockPos::new(0, 63, 0), block!("air"));

        assert_eq!(chunk.heightmaps.world_surface.get_height(0, 0), 10);
        assert_eq!(chunk.heightmaps.motion_blocking.get_height(0, 0), 10);
    }

    #[test]
    fn removing_top_surface_falls_back_to_fluid_for_motion_blocking() {
        let mut chunk = Chunk::new_empty();

        chunk.set_block(ChunkBlockPos::new(0, 10, 0), block!("stone"));
        chunk.set_block(ChunkBlockPos::new(0, 63, 0), block!("water", {level: 0}));
        chunk.set_block(ChunkBlockPos::new(0, 80, 0), block!("stone"));
        chunk.set_block(ChunkBlockPos::new(0, 80, 0), block!("air"));

        assert_eq!(chunk.heightmaps.world_surface.get_height(0, 0), 10);
        assert_eq!(chunk.heightmaps.motion_blocking.get_height(0, 0), 63);
    }

    #[test]
    fn full_recalculation_updates_every_column() {
        let mut chunk = Chunk::new_empty();

        chunk.set_block(ChunkBlockPos::new(0, 10, 0), block!("stone"));
        chunk.set_block(ChunkBlockPos::new(0, 63, 0), block!("water", {level: 0}));
        chunk.set_block(ChunkBlockPos::new(15, 80, 15), block!("stone"));
        chunk.heightmaps.world_surface.set_height(1, 1, 200);
        chunk.heightmaps.motion_blocking.set_height(1, 1, 200);

        chunk.recalculate_heightmap();

        assert_eq!(chunk.heightmaps.world_surface.get_height(0, 0), 10);
        assert_eq!(chunk.heightmaps.motion_blocking.get_height(0, 0), 63);
        assert_eq!(chunk.heightmaps.world_surface.get_height(15, 15), 80);
        assert_eq!(chunk.heightmaps.motion_blocking.get_height(15, 15), 80);
        assert_eq!(chunk.heightmaps.world_surface.get_height(1, 1), -65);
        assert_eq!(chunk.heightmaps.motion_blocking.get_height(1, 1), -65);
    }

    #[test]
    fn full_recalculation_matches_column_recalculation() {
        let mut full_chunk = Chunk::new_empty();
        let mut column_chunk = Chunk::new_empty();
        let placements = [
            (0, 10, 0, block!("stone")),
            (0, 63, 0, block!("water", {level: 0})),
            (3, 42, 7, block!("stone")),
            (3, 75, 7, block!("lava", {level: 0})),
            (15, -20, 15, block!("stone")),
        ];

        for (x, y, z, block) in placements {
            full_chunk.set_block(ChunkBlockPos::new(x, y, z), block);
            column_chunk.set_block(ChunkBlockPos::new(x, y, z), block);
        }

        full_chunk.recalculate_heightmap();
        for (x, z) in [(0, 0), (3, 7), (15, 15), (8, 8)] {
            column_chunk.recalculate_heightmap_column(x, z);

            assert_eq!(
                full_chunk.heightmaps.world_surface.get_height(x, z),
                column_chunk.heightmaps.world_surface.get_height(x, z)
            );
            assert_eq!(
                full_chunk.heightmaps.motion_blocking.get_height(x, z),
                column_chunk.heightmaps.motion_blocking.get_height(x, z)
            );
        }
    }

    #[test]
    fn recalculated_height_uses_absolute_y() {
        let mut chunk = Chunk::new_empty();

        chunk.set_block(ChunkBlockPos::new(0, 80, 0), block!("stone"));
        chunk.recalculate_heightmap_column(0, 0);

        assert_eq!(chunk.heightmaps.world_surface.get_height(0, 0), 80);
        assert_eq!(chunk.heightmaps.motion_blocking.get_height(0, 0), 80);
    }

    #[test]
    fn sign_serializes_to_network_nbt() {
        let sign = SignBlockEntity {
            is_waxed: false,
            front_text: SignText {
                messages: vec![
                    TextComponent {
                        content: TextContent::Text {
                            text: "hello".into(),
                        },
                        ..Default::default()
                    },
                    TextComponent::default(),
                    TextComponent::default(),
                    TextComponent::default(),
                ],
                color: "black".to_string(),
                has_glowing_text: false,
            },
            back_text: SignText {
                messages: vec![
                    TextComponent {
                        content: TextContent::Text {
                            text: "hello".into(),
                        },
                        ..Default::default()
                    },
                    TextComponent::default(),
                    TextComponent::default(),
                    TextComponent::default(),
                ],
                color: "black".to_string(),
                has_glowing_text: false,
            },
        };

        let blob = sign.to_blob().expect("sign should serialize");
        let nbt = BlockEntityKind::Sign
            .to_network_nbt(&blob)
            .expect("sign blob should convert to nbt");

        assert!(
            nbt.len() > 20,
            "sign nbt should contain the text components"
        );
        let restored: SignBlockEntity =
            serde_json::from_slice(&blob).expect("blob should deserialize");
        assert_eq!(restored, sign);
    }
}
