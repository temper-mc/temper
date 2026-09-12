use crate::maps::perlin::PerlinNoise;
use crate::params::NoiseParameter;
use bevy_math::DVec3;
use std::fmt::{Debug, Formatter};
use temper_core::random::RandomSource;

#[derive(Clone)]
pub struct NormalNoise {
    first: PerlinNoise,
    second: PerlinNoise,
    value_factor: f64,
    __param: &'static NoiseParameter,
}

impl NormalNoise {
    const CUSTOM: NoiseParameter = NoiseParameter {
        name: "custom_normal_noise",
        amplitudes: &[],
        first_octave: 0,
    };

    pub fn new<R: RandomSource>(rand: &mut R, param: &'static NoiseParameter) -> NormalNoise {
        let mut this = Self::new_custom(rand, param.first_octave, param.amplitudes);
        this.__param = param;
        this
    }

    pub fn new_custom<R: RandomSource>(
        rand: &mut R,
        first_octave: i32,
        amplitudes: &[f64],
    ) -> NormalNoise {
        let first = PerlinNoise::new(rand, first_octave, amplitudes);
        let second = PerlinNoise::new(rand, first_octave, amplitudes);

        let min_octave = amplitudes
            .iter()
            .position(|v| *v != 0.0)
            .map(|i| i as i32)
            .unwrap_or(i32::MAX);
        let max_octave = amplitudes
            .iter()
            .rposition(|v| *v != 0.0)
            .map(|i| i as i32)
            .unwrap_or(i32::MIN);

        let value_factor = 0.16666666666666666
            / (0.1 * (1.0 + 1.0 / (max_octave.wrapping_sub(min_octave) + 1) as f64));

        NormalNoise {
            first,
            second,
            value_factor,
            __param: &Self::CUSTOM,
        }
    }

    #[inline(always)]
    pub fn noise(&self, pos: DVec3) -> f64 {
        const DELTA: f64 = 1.0181268882175227;
        (self.first.noise(pos) + self.second.noise(pos * DELTA)) * self.value_factor
    }
}

impl Debug for NormalNoise {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "NormalNoise {{\"{}\"}}", self.__param.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maps::tests::data::NORMAL_TEST;
    use crate::maps::tests::run_test;

    #[test]
    fn test_normal_noise() {
        run_test(
            &NORMAL_TEST,
            |rand, (first_octave, amplitudes)| {
                NormalNoise::new_custom(rand, *first_octave, amplitudes)
            },
            NormalNoise::noise,
        )
    }
}
