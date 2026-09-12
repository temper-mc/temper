use crate::ImprovedNoise;
use bevy_math::DVec3;
use temper_core::random::{PositionalRandom, RandomSource};

#[derive(Clone)]
struct PerlinNoiseLevel {
    input_factor: f64,
    value_factor: f64,
    amplitude: f64,
    noise: ImprovedNoise,
}

#[derive(Clone)]
pub struct PerlinNoise {
    noise_levels: Box<[PerlinNoiseLevel]>,
}

impl PerlinNoise {
    pub fn new<R: RandomSource>(
        rand: &mut R,
        first_octave: i32,
        amplitudes: &[f64],
    ) -> PerlinNoise {
        let octaves = amplitudes.len();

        let mut noise_levels = Vec::with_capacity(octaves);
        let positional = rand.fork_positional();

        let lowest_freq_input_factor = 2f64.powi(first_octave);
        let lowest_freq_value_factor =
            2f64.powi(octaves as i32 - 1) / (2f64.powi(octaves as i32) - 1.0);

        for (i, amp) in amplitudes.iter().enumerate() {
            if *amp != 0.0 {
                let octave = first_octave + i as i32;
                let mut rand = positional.spawn_from_hash(format!("octave_{}", octave));
                noise_levels.push(PerlinNoiseLevel {
                    input_factor: lowest_freq_input_factor * 2f64.powi(i as i32),
                    value_factor: lowest_freq_value_factor * 0.5f64.powi(i as i32),
                    amplitude: *amp,
                    noise: ImprovedNoise::new(&mut rand),
                });
            }
        }

        Self {
            noise_levels: noise_levels.into_boxed_slice(),
        }
    }

    pub fn new_legacy<R: RandomSource>(rand: &mut R, octaves: &[i32]) -> PerlinNoise {
        debug_assert!(!octaves.is_empty());

        let low_freq_octaves = -octaves[0];
        let high_freq_octaves = octaves[octaves.len() - 1];
        let octave_range = low_freq_octaves + high_freq_octaves + 1;
        debug_assert!(octave_range >= 1);

        let mut amplitudes = vec![0.0; octave_range as usize];
        for octave in octaves {
            amplitudes[(octave + low_freq_octaves) as usize] = 1.0;
        }

        let first_octave = octaves[0];
        let octaves = amplitudes.len() as i32;
        let zero_octave_index = -first_octave;
        let mut noise_levels = vec![None; amplitudes.len()];

        let zero_octave = ImprovedNoise::new(rand);
        if zero_octave_index >= 0 && zero_octave_index < octaves {
            let zero_octave_amplitude = amplitudes[zero_octave_index as usize];
            if zero_octave_amplitude != 0.0 {
                noise_levels[zero_octave_index as usize] =
                    Some((zero_octave, zero_octave_amplitude));
            }
        }

        for i in (0..zero_octave_index).rev() {
            if i < octaves {
                let amplitude = amplitudes[i as usize];
                if amplitude != 0.0 {
                    noise_levels[i as usize] = Some((ImprovedNoise::new(rand), amplitude));
                } else {
                    rand.consume_count(262)
                }
            } else {
                rand.consume_count(262)
            }
        }

        debug_assert_eq!(
            noise_levels.iter().filter(|v| v.is_some()).count(),
            amplitudes.iter().filter(|&&v| v != 0.0).count(),
        );

        let lowest_freq_input_factor = 2f64.powi(-zero_octave_index);
        let lowest_freq_value_factor = 2f64.powi(octaves - 1) / (2f64.powi(octaves) - 1.0);

        Self {
            noise_levels: noise_levels
                .into_iter()
                .enumerate()
                .filter_map(|(i, level)| {
                    level.map(|(noise, amp)| PerlinNoiseLevel {
                        input_factor: lowest_freq_input_factor * 2f64.powi(i as i32),
                        value_factor: lowest_freq_value_factor * 0.5f64.powi(i as i32),
                        amplitude: amp,
                        noise,
                    })
                })
                .collect(),
        }
    }

    pub fn get_octave_noise(&self, octave: usize) -> (&ImprovedNoise, &f64) {
        let level = &self.noise_levels[self.noise_levels.len() - 1 - octave];
        (&level.noise, &level.amplitude)
    }

    pub(crate) fn wrap(x: f64) -> f64 {
        const FACTOR: f64 = 3.3554432E7;
        x - (x * FACTOR.recip() + 0.5).floor() * FACTOR
    }

    #[inline(always)]
    pub fn noise(&self, pos: DVec3) -> f64 {
        self.noise_levels
            .iter()
            .map(|level| {
                level
                    .noise
                    .noise(pos.map(|v| Self::wrap(v * level.input_factor)))
                    * level.amplitude
                    * level.value_factor
            })
            .sum()
    }

    pub fn noise_advanced(&self, pos: DVec3, y_scale: f64, y_fudge: f64) -> f64 {
        self.noise_levels
            .iter()
            .map(|level| {
                level.noise.noise_advanced(
                    pos.map(|v| Self::wrap(v * level.input_factor)),
                    y_scale * level.input_factor,
                    y_fudge * level.input_factor,
                ) * level.amplitude
                    * level.value_factor
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maps::tests::data::{PERLIN_LEGACY_TEST, PERLIN_TEST};
    use crate::maps::tests::run_test;

    #[test]
    fn test_perlin_noise() {
        run_test(
            &PERLIN_TEST,
            |rand, (first_octave, amplitudes)| PerlinNoise::new(rand, *first_octave, amplitudes),
            PerlinNoise::noise,
        )
    }

    #[test]
    fn test_legacy_perlin_noise() {
        run_test(
            &PERLIN_LEGACY_TEST,
            |rand, range| {
                PerlinNoise::new_legacy(
                    rand,
                    range.clone().into_iter().collect::<Vec<_>>().as_slice(),
                )
            },
            PerlinNoise::noise,
        )
    }
}
