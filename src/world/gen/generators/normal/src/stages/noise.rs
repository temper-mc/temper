use crate::NormalGenerator;
use gen_core::{GenerationError, StageInput};
use quick_noise::simd::dispatch_simd;
use quick_noise::{Fbm, Perlin};

impl NormalGenerator {
    #[dispatch_simd(A)]
    pub(crate) fn generate_noises(&self, input: StageInput<'_>) -> Result<(), GenerationError> {
        let grid_2d = quick_noise::Grid::<2, A>::new(16, 16)
            .grid_position(input.pos.x().into(), input.pos.z().into())
            .seed(self.seed as i64);

        let mut end_grid_2d = [0f32; 256];

        // Continentalness
        grid_2d
            .builder::<Fbm, Perlin>()
            // Hardcoding seeds here is fine since it is combined with the grid/world seed and so each
            // noise type will be different from each other but the same when using the same world seed
            .seed(1)
            .frequency(1.0 / 1024.0)
            .octaves(3)
            .lacunarity(2.0)
            .persistence(0.45)
            .fill(end_grid_2d.as_mut_slice());

        end_grid_2d.chunks(16).enumerate().for_each(|(idx, chunk)| {
            assert!(idx < 16);
            assert_eq!(chunk.len(), 16);
            input.target.noise.continentalness[idx] =
                chunk.try_into().expect("Chunk length mismatch");
        });

        // Erosion
        grid_2d
            .builder::<Fbm, Perlin>()
            .seed(2)
            .frequency(1.0 / 360.0)
            .octaves(3)
            .lacunarity(2.0)
            .persistence(0.45)
            .fill(end_grid_2d.as_mut_slice());

        end_grid_2d.chunks(16).enumerate().for_each(|(idx, chunk)| {
            assert!(idx < 16);
            assert_eq!(chunk.len(), 16);
            input.target.noise.erosion[idx] = chunk.try_into().expect("Chunk length mismatch");
        });

        // Weirdness
        grid_2d
            .builder::<Fbm, Perlin>()
            .seed(3)
            .frequency(1.0 / 260.0)
            .octaves(2)
            .lacunarity(2.0)
            .persistence(0.5)
            .fill(end_grid_2d.as_mut_slice());

        end_grid_2d.chunks(16).enumerate().for_each(|(idx, chunk)| {
            assert!(idx < 16);
            assert_eq!(chunk.len(), 16);
            input.target.noise.weirdness[idx] = chunk.try_into().expect("Chunk length mismatch");
        });

        // Temperature
        grid_2d
            .builder::<Fbm, Perlin>()
            .seed(4)
            .frequency(1.0 / 1200.0)
            .octaves(2)
            .lacunarity(2.0)
            .persistence(0.4)
            .fill(end_grid_2d.as_mut_slice());

        end_grid_2d.chunks(16).enumerate().for_each(|(idx, chunk)| {
            assert!(idx < 16);
            assert_eq!(chunk.len(), 16);
            input.target.noise.temperature[idx] = chunk.try_into().expect("Chunk length mismatch");
        });

        // Humidity
        grid_2d
            .builder::<Fbm, Perlin>()
            .seed(5)
            .frequency(1.0 / 800.0)
            .octaves(2)
            .lacunarity(2.0)
            .persistence(0.45)
            .fill(end_grid_2d.as_mut_slice());

        end_grid_2d.chunks(16).enumerate().for_each(|(idx, chunk)| {
            assert!(idx < 16);
            assert_eq!(chunk.len(), 16);
            input.target.noise.humidity[idx] = chunk.try_into().expect("Chunk length mismatch");
        });

        // Jagged
        grid_2d
            .builder::<Fbm, Perlin>()
            .seed(6)
            .frequency(1.0 / 80.0)
            .octaves(2)
            .lacunarity(2.0)
            .persistence(0.5)
            .fill(end_grid_2d.as_mut_slice());

        end_grid_2d.chunks(16).enumerate().for_each(|(idx, chunk)| {
            assert!(idx < 16);
            assert_eq!(chunk.len(), 16);
            input.target.noise.jaggedness[idx] = chunk.try_into().expect("Chunk length mismatch");
        });

        input.target.noise.clear_transient_3d();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use gen_core::{GenStage, StageInput, StageNeighborhood};
    use temper_core::pos::ChunkPos;
    use temper_world_format::Chunk;

    use super::*;

    fn generate_noise(seed: u64, pos: ChunkPos) -> Chunk {
        let generator = NormalGenerator::new(seed);
        let mut chunk = Chunk::new_empty();

        generator
            .generate_noises(StageInput::new(
                pos,
                GenStage::NOISE,
                &mut chunk,
                StageNeighborhood::empty(),
            ))
            .expect("normal noise generation should succeed");

        chunk
    }

    #[test]
    fn transient_3d_noise_is_not_stored_by_default() {
        let chunk = generate_noise(10, ChunkPos::new(0, 0));

        assert!(chunk.noise.base3d.is_empty());
        assert!(chunk.noise.cheese_caves.is_empty());
        assert!(chunk.noise.spaghetti_caves.is_empty());
        assert!(chunk.noise.noddle_caves.is_empty());
    }
}
