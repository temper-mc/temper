use quick_noise::simd::Arch;
use quick_noise::simd::dispatch_simd;
use quick_noise::{BatchNoise, Fbm, Grid, Perlin, Simplex};

#[derive(Clone, Copy)]
pub(crate) struct NoiseGenerator {
    pub seed: u64,
}

impl NoiseGenerator {
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }

    #[dispatch_simd(A)]
    pub fn fill_height_noise(
        &self,
        origin_x: i32,
        origin_z: i32,
        step_xz: i32,
        size_x: usize,
        size_z: usize,
        output: &mut [f64],
    ) {
        assert_eq!(output.len(), size_x * size_z);

        let step = step_xz as f32;
        let grid = Grid::<2, A>::new(size_x, size_z)
            .sample_position(
                (origin_x as f32 / step).into(),
                (origin_z as f32 / step).into(),
            )
            .seed(self.seed as i64);

        let mut base = vec![0.0f32; output.len()];
        let mut mountain_mask = vec![0.0f32; output.len()];
        let mut peaks = vec![0.0f64; output.len()];

        grid.builder::<Fbm, Perlin>()
            .seed(0)
            .octaves(4)
            .frequency(0.002)
            .lacunarity(2.0)
            .persistence(0.5)
            .scaling(step, step)
            .fill(base.as_mut_slice());

        fill_ridged_perlin_2d(
            &grid,
            1,
            4,
            0.008,
            std::f32::consts::PI * 2.0 / 3.0,
            1.0,
            2.0,
            [step, step],
            &mut peaks,
        );

        grid.builder::<Fbm, Perlin>()
            .seed(2)
            .octaves(2)
            .frequency(0.0006)
            .lacunarity(2.0)
            .persistence(0.5)
            .scaling(step, step)
            .fill(mountain_mask.as_mut_slice());

        let to01 = |n: f64| (n * 0.5 + 0.5).clamp(0.0, 1.0);

        for (((dst, base), peaks), mask) in output
            .iter_mut()
            .zip(base.into_iter())
            .zip(peaks)
            .zip(mountain_mask.into_iter())
        {
            let height = shape_height(to01(f64::from(base)), to01(peaks), to01(f64::from(mask)));
            *dst = (height * 2.0) - 1.0;
        }
    }

    #[expect(clippy::too_many_arguments)]
    #[dispatch_simd(A)]
    pub fn fill_cave_noise(
        &self,
        origin_x: i32,
        origin_y: i32,
        origin_z: i32,
        step_xz: i32,
        step_y: i32,
        size_x: usize,
        size_y: usize,
        size_z: usize,
        output: &mut [f64],
    ) {
        assert_eq!(output.len(), size_x * size_y * size_z);

        let step_xz = step_xz as f32;
        let step_y = step_y as f32;
        let grid = Grid::<3, A>::new(size_x, size_y, size_z)
            .sample_position(
                (origin_x as f32 / step_xz).into(),
                (origin_y as f32 / step_y).into(),
                (origin_z as f32 / step_xz).into(),
            )
            .seed(self.seed as i64);

        fill_ridged_simplex_3d(
            &grid,
            100,
            5,
            0.01,
            2.5,
            0.8,
            0.3,
            [step_xz * 0.5, step_y * 0.5, step_xz * 0.5],
            output,
        );
    }
}

#[inline(always)]
fn calc_ridged_scale_factor(persistence: f64, attenuation: f64, octaves: usize) -> f64 {
    let mut denom = 0.0;
    let mut amplitude = 1.0;
    let mut signal = amplitude;

    denom += signal;

    if octaves >= 1 {
        denom += (1..=octaves).fold(0.0, |acc, octave| {
            amplitude *= persistence;
            let weight = (signal / attenuation.powi(octave as i32)).clamp(0.0, 1.0);
            signal = weight * amplitude;
            acc + signal
        });
    }

    2.0 / denom
}

#[expect(clippy::too_many_arguments)]
#[inline(always)]
fn fill_ridged_perlin_2d<A: Arch>(
    grid: &Grid<2, A>,
    seed: i64,
    octaves: usize,
    frequency: f32,
    lacunarity: f32,
    persistence: f64,
    attenuation: f64,
    scaling: [f32; 2],
    output: &mut [f64],
) {
    output.fill(0.0);

    let mut octave_noise = vec![0.0f32; output.len()];
    let mut weights = vec![1.0f64; output.len()];
    let mut current_frequency = frequency;
    let mut amplitude = 1.0f64;

    for octave in 0..octaves {
        grid.builder::<Fbm, Perlin>()
            .seed(seed + octave as i64)
            .octaves(1)
            .frequency(current_frequency)
            .scaling(scaling[0], scaling[1])
            .fill(octave_noise.as_mut_slice());

        for ((out, weight), sample) in output
            .iter_mut()
            .zip(weights.iter_mut())
            .zip(octave_noise.iter().copied())
        {
            let mut signal = f64::from(sample).abs();
            signal = 1.0 - signal;
            signal *= signal;
            signal *= *weight;
            *weight = (signal / attenuation).clamp(0.0, 1.0);
            *out += signal * amplitude;
        }

        amplitude *= persistence;
        current_frequency *= lacunarity;
    }

    let scale_factor = calc_ridged_scale_factor(persistence, attenuation, octaves);
    output
        .iter_mut()
        .for_each(|sample| *sample = (*sample * scale_factor) - 1.0);
}

#[expect(clippy::too_many_arguments)]
#[inline(always)]
fn fill_ridged_simplex_3d<A: Arch>(
    grid: &Grid<3, A>,
    seed: i64,
    octaves: usize,
    frequency: f32,
    lacunarity: f32,
    persistence: f64,
    attenuation: f64,
    scaling: [f32; 3],
    output: &mut [f64],
) {
    output.fill(0.0);

    let mut octave_noise = vec![0.0f32; output.len()];
    let mut weights = vec![1.0f64; output.len()];
    let mut current_frequency = frequency;
    let mut amplitude = 1.0f64;

    for octave in 0..octaves {
        BatchNoise::<3, Fbm, Simplex>::builder(grid.x_iter(), grid.y_iter(), grid.z_iter())
            .seed(seed + octave as i64)
            .octaves(1)
            .frequency(current_frequency)
            .scaling(scaling[0], scaling[1], scaling[2])
            .fill(octave_noise.as_mut_slice());

        for ((out, weight), sample) in output
            .iter_mut()
            .zip(weights.iter_mut())
            .zip(octave_noise.iter().copied())
        {
            let mut signal = f64::from(sample).abs();
            signal = 1.0 - signal;
            signal *= signal;
            signal *= *weight;
            *weight = (signal / attenuation).clamp(0.0, 1.0);
            *out += signal * amplitude;
        }

        amplitude *= persistence;
        current_frequency *= lacunarity;
    }

    let scale_factor = calc_ridged_scale_factor(persistence, attenuation, octaves);
    output
        .iter_mut()
        .for_each(|sample| *sample = (*sample * scale_factor) - 1.0);
}

fn shape_height(base01: f64, peaks01: f64, mask01: f64) -> f64 {
    let land_lift = smoothstep(((base01 - 0.38) / 0.34).clamp(0.0, 1.0));
    let mountain_mask = smoothstep(((mask01 - 0.5) / 0.3).clamp(0.0, 1.0));
    let peak_gate = smoothstep(((base01 - 0.58) / 0.24).clamp(0.0, 1.0));

    let plains = base01.powf(1.15) * 0.92 + land_lift * 0.06;
    let peaks = peaks01.powf(3.0) * 0.16 * mountain_mask * peak_gate;

    (plains + peaks).clamp(0.0, 1.0)
}

#[inline(always)]
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

#[inline(always)]
pub fn smoothstep(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

#[inline(always)]
pub fn bilerp(c00: f64, c10: f64, c01: f64, c11: f64, tx: f64, tz: f64) -> f64 {
    let x0 = lerp(c00, c10, tx);
    let x1 = lerp(c01, c11, tx);
    lerp(x0, x1, tz)
}

#[expect(clippy::too_many_arguments)]
#[inline(always)]
pub fn trilerp(
    c000: f64,
    c100: f64,
    c010: f64,
    c110: f64,
    c001: f64,
    c101: f64,
    c011: f64,
    c111: f64,
    tx: f64,
    ty: f64,
    tz: f64,
) -> f64 {
    let x00 = lerp(c000, c100, tx);
    let x10 = lerp(c010, c110, tx);
    let x01 = lerp(c001, c101, tx);
    let x11 = lerp(c011, c111, tx);

    let y0 = lerp(x00, x10, ty);
    let y1 = lerp(x01, x11, ty);

    lerp(y0, y1, tz)
}

#[inline(always)]
fn quick_hash(seed: u64, x: i32, z: i32) -> f64 {
    let mut value = seed
        ^ (x as u64).wrapping_mul(0x9E3779B185EBCA87)
        ^ (z as u64).wrapping_mul(0xC2B2AE3D27D4EB4F);
    value ^= value >> 33;
    value = value.wrapping_mul(0xFF51AFD7ED558CCD);
    value ^= value >> 33;
    value as f64 / u64::MAX as f64
}

#[inline(always)]
pub fn dither_field(seed: u64, x: i32, z: i32, cell_size: i32) -> f64 {
    let cx0 = x.div_euclid(cell_size);
    let cz0 = z.div_euclid(cell_size);
    let cx1 = cx0 + 1;
    let cz1 = cz0 + 1;

    let fx = f64::from(x.rem_euclid(cell_size)) / f64::from(cell_size);
    let fz = f64::from(z.rem_euclid(cell_size)) / f64::from(cell_size);

    let tx = smoothstep(fx);
    let tz = smoothstep(fz);

    let v00 = quick_hash(seed, cx0, cz0);
    let v10 = quick_hash(seed, cx1, cz0);
    let v01 = quick_hash(seed, cx0, cz1);
    let v11 = quick_hash(seed, cx1, cz1);

    let a = lerp(v00, v10, tx);
    let b = lerp(v01, v11, tx);
    lerp(a, b, tz)
}

#[cfg(test)]
mod tests {
    use super::*;
    use noise::{
        Fbm as LegacyFbm, MultiFractal, NoiseFn, OpenSimplex as LegacyOpenSimplex,
        Perlin as LegacyPerlin, RidgedMulti,
    };

    fn legacy_height_noise(seed: u64, x: f64, z: f64) -> f64 {
        let to01 = |n: f64| (n * 0.5 + 0.5).clamp(0.0, 1.0);

        let base = LegacyFbm::<LegacyPerlin>::new(seed as u32)
            .set_octaves(4)
            .set_frequency(0.002);
        let peaks = RidgedMulti::<LegacyPerlin>::new((seed as u32).wrapping_add(1))
            .set_octaves(4)
            .set_frequency(0.008);
        let mountain_mask = LegacyFbm::<LegacyPerlin>::new((seed as u32).wrapping_add(2))
            .set_octaves(2)
            .set_frequency(0.0006);

        let height = shape_height(
            to01(base.get([x, z])),
            to01(peaks.get([x, z])),
            to01(mountain_mask.get([x, z])),
        );

        (height * 2.0) - 1.0
    }

    fn legacy_cave_noise(seed: u64, x: f64, y: f64, z: f64) -> f64 {
        RidgedMulti::<LegacyOpenSimplex>::new((seed + 100) as u32)
            .set_frequency(0.01)
            .set_lacunarity(2.5)
            .set_octaves(5)
            .set_persistence(0.8)
            .set_attenuation(0.3)
            .get([x, y, z])
    }

    #[test]
    fn peak_noise_is_gated_out_of_lower_terrain() {
        let low_peak = shape_height(0.45, 0.1, 1.0);
        let high_peak = shape_height(0.45, 1.0, 1.0);

        assert!((high_peak - low_peak).abs() < 0.01);
    }

    #[test]
    fn peak_noise_still_lifts_high_terrain() {
        let low_peak = shape_height(0.75, 0.1, 1.0);
        let high_peak = shape_height(0.75, 1.0, 1.0);

        assert!(high_peak > low_peak + 0.1);
    }

    #[test]
    fn generated_height_noise_varies_across_the_grid() {
        let noise = NoiseGenerator::new(0);
        let mut samples = [0.0; 25];

        noise.fill_height_noise(0, 0, 4, 5, 5, &mut samples);

        let first = samples[0];
        assert!(samples.iter().any(|&sample| (sample - first).abs() > 0.01));
    }

    #[test]
    fn generated_cave_noise_has_mixed_density() {
        let noise = NoiseGenerator::new(0);
        let mut samples = [0.0; 5 * 21 * 5];

        noise.fill_cave_noise(0, -60, 0, 4, 8, 5, 21, 5, &mut samples);

        assert!(samples.iter().any(|&sample| sample <= 0.6));
        assert!(samples.iter().any(|&sample| sample > 0.6));
    }

    #[test]
    fn height_noise_stays_close_to_legacy_distribution() {
        let noise = NoiseGenerator::new(0);
        let mut samples = [0.0; 25];
        noise.fill_height_noise(0, 0, 4, 5, 5, &mut samples);

        let legacy: Vec<_> = (0..5)
            .flat_map(|z| {
                (0..5).map(move |x| legacy_height_noise(0, f64::from(x * 4), f64::from(z * 4)))
            })
            .collect();

        let avg_delta = samples
            .iter()
            .zip(legacy.iter())
            .map(|(new, old)| (new - old).abs())
            .sum::<f64>()
            / samples.len() as f64;

        assert!(avg_delta < 0.35, "avg delta was {avg_delta}");
    }

    #[test]
    fn cave_carve_ratio_stays_close_to_legacy_distribution() {
        let noise = NoiseGenerator::new(0);
        let mut samples = [0.0; 5 * 21 * 5];
        noise.fill_cave_noise(0, -60, 0, 4, 8, 5, 21, 5, &mut samples);

        let legacy: Vec<_> = (0..5)
            .flat_map(|z| {
                (0..21).flat_map(move |y| {
                    (0..5).map(move |x| {
                        legacy_cave_noise(
                            0,
                            f64::from(x * 4) / 2.0,
                            f64::from(-60 + y * 8) / 2.0,
                            f64::from(z * 4) / 2.0,
                        )
                    })
                })
            })
            .collect();

        let carve_ratio = |values: &[f64]| -> f64 {
            values.iter().filter(|&&value| value > 0.6).count() as f64 / values.len() as f64
        };

        let ratio_delta = (carve_ratio(&samples) - carve_ratio(&legacy)).abs();
        assert!(ratio_delta < 0.15, "carve ratio delta was {ratio_delta}");
    }
}
