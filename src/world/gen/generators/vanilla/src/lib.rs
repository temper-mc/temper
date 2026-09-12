use gen_core::{
    ChunkGenerator, GenStage, GenerationError, GeneratorId, StageDependencies, StageInput,
    StageSpec,
};
use include_dir::{Dir, include_dir};
use std::collections::HashMap;
use temper_core::block_state_id::BlockStateId;
use temper_core::math::TemperMathExt;
use temper_core::pos::{ChunkBlockPos, ChunkPos};
use temper_core::random::{RandomSource, XoroshiroRandomSource};
use temper_density::BoxedDensityFunction;
use temper_density::compile::Compiler;
use temper_density::json::{DensityFunctionArgument, deserialize_function};
use temper_density::wrapped::WrappedDensityFunction;
use temper_macros::block;

pub struct VanillaGenerator {
    _rand: XoroshiroRandomSource,
    final_density: BoxedDensityFunction,
    default_block_state: BlockStateId,
    default_fluid_state: BlockStateId,
    water_level: i16,
}

impl VanillaGenerator {
    pub fn new(seed: u64) -> VanillaGenerator {
        const BASE: &str = include_str!("function.json");
        const EXTERNAL: Dir =
            include_dir!("assets/generated/generated/data/minecraft/worldgen/density_function");

        let mut rand = XoroshiroRandomSource::new(seed);
        let func = deserialize_function(BASE).unwrap();

        let mut external = HashMap::new();
        fn gather(external: &mut HashMap<String, DensityFunctionArgument>, root: &Dir) {
            for entry in root.entries() {
                if let Some(dir) = entry.as_dir() {
                    gather(external, dir);
                    continue;
                }

                if let Some(file) = entry.as_file() {
                    let path = file.path().display().to_string();
                    let name = format!(
                        "minecraft:{}",
                        path.strip_suffix(".json").unwrap_or(path.as_str())
                    );

                    let func = deserialize_function(file.contents_utf8().unwrap())
                        .unwrap_or_else(|e| panic!("{}: {}", name, e));

                    external.insert(name, func);
                }
            }
        }

        gather(&mut external, &EXTERNAL);
        let compiled = Compiler::compile(&mut rand.fork_positional(), &external, func);

        Self {
            _rand: rand,
            final_density: compiled,
            default_block_state: block!("stone"),
            default_fluid_state: block!("water", { level: 0 }),
            water_level: 63,
        }
    }
}

impl ChunkGenerator for VanillaGenerator {
    fn id(&self) -> GeneratorId {
        GeneratorId::new("vanilla")
    }

    fn final_stage(&self) -> GenStage {
        GenStage::SURFACE
    }

    fn stage_spec(&self, stage: GenStage) -> Option<StageSpec> {
        match stage {
            GenStage::EMPTY => Some(StageSpec::new(stage, "empty", StageDependencies::NONE)),
            GenStage::NOISE => Some(StageSpec::new(stage, "noise", StageDependencies::NONE)),
            GenStage::SURFACE => Some(StageSpec::new(
                stage,
                "surface",
                StageDependencies::only_own(GenStage::NOISE),
            )),
            _ => None,
        }
    }

    fn advance_stage(&self, input: StageInput<'_>) -> Result<(), GenerationError> {
        match input.stage {
            GenStage::EMPTY => Ok(()),
            GenStage::NOISE => self.fill_noise(input),
            GenStage::SURFACE => self.generate_surface(input),
            _ => Ok(()),
        }
    }
}

impl VanillaGenerator {
    fn fill_noise(&self, input: StageInput) -> Result<(), GenerationError> {
        for y in -4..(self.water_level >> 4) {
            input.target.fill_section(y as i8, self.default_fluid_state)
        }

        for y in ((self.water_level >> 4) << 4)..self.water_level {
            for x in 0..16 {
                for z in 0..16 {
                    input.target.set_block_without_heightmap(
                        ChunkBlockPos::new(x, y, z),
                        self.default_fluid_state,
                    )
                }
            }
        }

        for y in -64..-54 {
            for x in 0..16 {
                for z in 0..16 {
                    input.target.set_block_without_heightmap(
                        ChunkBlockPos::new(x, y, z),
                        block!("lava", { level: 0 }),
                    )
                }
            }
        }

        let cell_size_xz = 1;
        let cell_size_y = 2;

        let cell_height = cell_size_y + 1;
        let cell_width = cell_size_xz + 1;

        let cell_count_xz = 16usize >> cell_width;
        let cell_count_y = (input.target.height().height as usize) >> cell_height;

        let cell_width_blocks = 1 << cell_width;
        let cell_height_blocks = 1 << cell_height;

        let chunk_min = input.pos.block_offset(0, 0, 0);
        let mut wrapped = WrappedDensityFunction::wrap(
            &self.final_density,
            cell_width_blocks,
            chunk_min.pos.x >> 2,
            chunk_min.pos.z >> 2,
        );

        let size_z = cell_count_xz + 1;
        let size_y = cell_count_y + 1;
        let mut slice0 = vec![0.0; size_z * size_y].into_boxed_slice();
        let mut slice1 = vec![0.0; size_z * size_y].into_boxed_slice();

        let min_y = input.target.height().min_y;

        fill_slice(
            &mut slice0,
            &input.pos,
            0,
            cell_count_xz,
            cell_count_y,
            cell_width,
            cell_height,
            min_y,
            &mut wrapped,
        );

        for x_cell in 0..cell_count_xz {
            let x_pos = x_cell << cell_width;
            fill_slice(
                &mut slice1,
                &input.pos,
                x_cell + 1,
                cell_count_xz,
                cell_count_y,
                cell_width,
                cell_height,
                min_y,
                &mut wrapped,
            );

            for z_cell in 0..cell_count_xz {
                let z_pos = z_cell << cell_width;

                for y_cell in (0..cell_count_y).rev() {
                    let y_pos = y_cell << cell_height;

                    let p000 = slice0[z_cell + y_cell * size_z];
                    let p001 = slice0[z_cell + (y_cell + 1) * size_z];
                    let p010 = slice1[z_cell + y_cell * size_z];
                    let p011 = slice1[z_cell + (y_cell + 1) * size_z];
                    let p100 = slice0[z_cell + 1 + y_cell * size_z];
                    let p101 = slice0[z_cell + 1 + (y_cell + 1) * size_z];
                    let p110 = slice1[z_cell + 1 + y_cell * size_z];
                    let p111 = slice1[z_cell + 1 + (y_cell + 1) * size_z];

                    for y in (0..cell_height_blocks).rev() {
                        let t0 = y as f64 / cell_height_blocks as f64;
                        let y00 = t0.lerp(p000, p001);
                        let y01 = t0.lerp(p010, p011);
                        let y10 = t0.lerp(p100, p101);
                        let y11 = t0.lerp(p110, p111);

                        for x in 0..cell_width_blocks {
                            let t1 = x as f64 / cell_width_blocks as f64;
                            let x0 = t1.lerp(y00, y01);
                            let x1 = t1.lerp(y10, y11);

                            for z in 0..cell_width_blocks {
                                let t2 = z as f64 / cell_width_blocks as f64;
                                let val = t2.lerp(x0, x1);

                                if val > 0.0 {
                                    input.target.set_block_without_heightmap(
                                        ChunkBlockPos::new(
                                            (x_pos as i32 + x) as u8,
                                            (y_pos + y) as i16 + min_y,
                                            (z_pos as i32 + z) as u8,
                                        ),
                                        self.default_block_state,
                                    );
                                }
                            }
                        }
                    }
                }
            }

            std::mem::swap(&mut slice0, &mut slice1);
        }

        input.target.recalculate_heightmap();

        Ok(())
    }

    fn generate_surface(&self, input: StageInput) -> Result<(), GenerationError> {
        let grass = block!("grass_block", { snowy: false });
        let dirt = block!("dirt");
        let sand = block!("sand");

        let min_y = input.target.height().min_y;
        let max_y = min_y + input.target.height().height as i16;

        for x in 0..16 {
            'outer: for z in 0..16 {
                for y in (min_y..max_y).rev() {
                    let above = ChunkBlockPos::new(x, (y + 1).min(max_y - 1), z);
                    let pos = ChunkBlockPos::new(x, y, z);

                    if input.target.get_block(pos) == self.default_block_state {
                        let (top_block, bottom_block) =
                            if input.target.get_block(above) == self.default_fluid_state {
                                (sand, sand)
                            } else {
                                (grass, dirt)
                            };

                        input.target.set_block_without_heightmap(pos, top_block);

                        let below = ChunkBlockPos::new(x, y - 1, z);
                        if input.target.get_block(below) == self.default_block_state {
                            input
                                .target
                                .set_block_without_heightmap(below, bottom_block);

                            let below = ChunkBlockPos::new(x, y - 2, z);
                            if input.target.get_block(below) == self.default_block_state {
                                input
                                    .target
                                    .set_block_without_heightmap(below, bottom_block);
                            }
                        }

                        continue 'outer;
                    }
                }
            }
        }

        Ok(())
    }
}

#[expect(clippy::too_many_arguments)]
fn fill_slice(
    slice: &mut [f64],
    chunk_pos: &ChunkPos,
    cell_x: usize,
    cell_count_xz: usize,
    cell_count_y: usize,
    cell_width: i32,
    cell_height: i32,
    min_y: i16,
    function: &mut WrappedDensityFunction,
) {
    let x_pos = (cell_x << cell_width) as i32;

    for cell_z in 0..=cell_count_xz {
        let z_pos = (cell_z << cell_width) as i32;

        for cell_y in 0..=cell_count_y {
            let y_pos = (cell_y << cell_height) as i16 + min_y;

            slice[cell_z + cell_y * (cell_count_xz + 1)] =
                function.execute(chunk_pos.block_offset(x_pos, y_pos as i32, z_pos));
        }
    }
}
