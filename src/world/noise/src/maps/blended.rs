use crate::PerlinNoise;
use bevy_math::DVec3;
use std::fmt::{Debug, Formatter};
use temper_core::math::TemperMathExt;
use temper_core::random::{RandomSource, XoroshiroRandomSource};

#[derive(Clone)]
pub struct BlendedNoise {
    min_limit_noise: PerlinNoise,
    max_limit_noise: PerlinNoise,
    main_noise: PerlinNoise,
    multiplier: DVec3,
    factor: DVec3,
    smear_scale_multiplier: f64,
}

impl BlendedNoise {
    pub fn new_unseeded(
        xz_scale: f64,
        y_scale: f64,
        xz_factor: f64,
        y_factor: f64,
        smear_scale_multiplier: f64,
    ) -> Self {
        let mut rand = XoroshiroRandomSource::new(0);

        Self::new_seeded(
            &mut rand,
            xz_scale,
            y_scale,
            xz_factor,
            y_factor,
            smear_scale_multiplier,
        )
    }

    pub fn new_seeded<R: RandomSource>(
        rand: &mut R,
        xz_scale: f64,
        y_scale: f64,
        xz_factor: f64,
        y_factor: f64,
        smear_scale_multiplier: f64,
    ) -> Self {
        Self {
            min_limit_noise: PerlinNoise::new_legacy_for_blended(
                rand,
                &(-15..=0).into_iter().collect::<Vec<_>>(),
            ),
            max_limit_noise: PerlinNoise::new_legacy_for_blended(
                rand,
                &(-15..=0).into_iter().collect::<Vec<_>>(),
            ),
            main_noise: PerlinNoise::new_legacy_for_blended(
                rand,
                &(-7..=0).into_iter().collect::<Vec<_>>(),
            ),
            multiplier: 684.412 * DVec3::new(xz_scale, y_scale, xz_scale),
            factor: DVec3::new(xz_factor, y_factor, xz_factor),
            smear_scale_multiplier,
        }
    }

    pub fn noise(&self, pos: DVec3) -> f64 {
        let limit = pos * self.multiplier;
        let main = limit / self.factor;

        let limit_smear = self.multiplier.y * self.smear_scale_multiplier;
        let main_smear = limit_smear / self.factor.y;
        let mut blend_min = 0.0;
        let mut blend_max = 0.0;
        let mut main_noise_value = 0.0;
        let mut pow = 1.0;

        for i in 0..8 {
            let (noise, _) = &self.main_noise.get_octave_noise(i);
            main_noise_value += noise.noise_advanced(
                main.map(|v| PerlinNoise::wrap(v * pow)),
                main_smear * pow,
                main.y * pow,
            ) / pow;

            pow /= 2.0;
        }

        let factor = (main_noise_value / 10.0 + 1.0) / 2.0;
        let is_max = factor >= 1.0;
        let is_min = factor <= 0.0;
        pow = 1.0;

        for i in 0..16 {
            let w = limit.map(|v| PerlinNoise::wrap(v * pow));
            let y_scale_pow = limit_smear * pow;

            if !is_max && let (noise, _) = self.min_limit_noise.get_octave_noise(i) {
                blend_min += noise.noise_advanced(w, y_scale_pow, limit.y * pow) / pow;
            }

            if !is_min && let (noise, _) = self.max_limit_noise.get_octave_noise(i) {
                blend_max += noise.noise_advanced(w, y_scale_pow, limit.y * pow) / pow;
            }

            pow /= 2.0;
        }

        factor.clamped_lerp(blend_min / 512.0, blend_max / 512.0) / 128.0
    }
}

impl Debug for BlendedNoise {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "BlendedNoise {{}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maps::tests::data::BLENDED_TEST;
    use crate::maps::tests::run_test;

    #[test]
    fn test_blended_noise() {
        run_test(
            &BLENDED_TEST,
            |_, (xz_scale, y_scale, xz_factor, y_factor, smear_scale_multiplier)| {
                BlendedNoise::new_unseeded(
                    *xz_scale,
                    *y_scale,
                    *xz_factor,
                    *y_factor,
                    *smear_scale_multiplier,
                )
            },
            BlendedNoise::noise,
        )
    }
}
