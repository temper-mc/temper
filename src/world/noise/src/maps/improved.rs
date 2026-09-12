use crate::maps::GRADIENT;
use bevy_math::{DVec3, ISizeVec3};
use temper_core::math::TemperMathExt;
use temper_core::random::RandomSource;

#[derive(Clone)]
pub struct ImprovedNoise {
    p: [u8; 256],
    pos: DVec3,
}

impl ImprovedNoise {
    pub fn new<T: RandomSource>(rand: &mut T) -> ImprovedNoise {
        let xo = rand.next_f64() * 256.0;
        let yo = rand.next_f64() * 256.0;
        let zo = rand.next_f64() * 256.0;

        let mut p: [u8; 256] = std::array::from_fn(|i| i as u8);
        for i in 0..256 {
            let offset = rand.next_u32_bounded(256 - i as u32) as usize;
            p.swap(i, i + offset);
        }

        Self {
            p,
            pos: DVec3::new(xo, yo, zo),
        }
    }

    fn grad_dot(hash: usize, pos: DVec3) -> f64 {
        GRADIENT[hash & 0xF].dot(pos)
    }

    #[inline(always)]
    pub fn noise(&self, pos: DVec3) -> f64 {
        let pos = self.pos + pos;
        let pos_f = pos.floor();
        let pos_r = pos - pos_f;

        self.sample_and_lerp(pos_f.as_isizevec3(), pos_r, pos_r.y)
    }

    pub fn noise_advanced(&self, pos: DVec3, y_scale: f64, y_fudge: f64) -> f64 {
        let pos = self.pos + pos;
        let pos_f = pos.floor();
        let pos_r = pos - pos_f;

        let yr_fudge = if y_scale != 0.0 {
            let limit = if y_fudge >= 0.0 {
                y_fudge.min(pos_r.y)
            } else {
                pos_r.y
            };

            (limit / y_scale + 1.0e-7f32 as f64).floor() * y_scale
        } else {
            0.0
        };

        self.sample_and_lerp(
            pos_f.as_isizevec3(),
            DVec3::new(pos_r.x, pos_r.y - yr_fudge, pos_r.z),
            pos_r.y,
        )
    }

    fn sample_and_lerp(&self, pos: ISizeVec3, pos_r: DVec3, yr_original: f64) -> f64 {
        let x: [usize; 2] =
            std::array::from_fn(|i| self.p[i.wrapping_add_signed(pos.x) & 0xFF] as usize);
        let xy: [usize; 4] = std::array::from_fn(|i| {
            self.p[(x[i & 1] + (i >> 1)).wrapping_add_signed(pos.y) & 0xFF] as usize
        });
        let [d000, d001, d010, d011, d100, d101, d110, d111] = std::array::from_fn(|i| {
            Self::grad_dot(
                self.p[(xy[i & 3] + (i >> 2)).wrapping_add_signed(pos.z) & 0xFF] as usize,
                pos_r - DVec3::new((i & 1) as f64, ((i >> 1) & 1) as f64, ((i >> 2) & 1) as f64),
            )
        });

        f64::lerp3(
            pos_r.x.smooth_step(),
            yr_original.smooth_step(),
            pos_r.z.smooth_step(),
            d000,
            d001,
            d010,
            d011,
            d100,
            d101,
            d110,
            d111,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maps::tests::data::IMPROVED_TEST;
    use crate::maps::tests::run_test;

    #[test]
    fn test_improved_noise() {
        run_test(
            &IMPROVED_TEST,
            |rand, _| ImprovedNoise::new(rand),
            ImprovedNoise::noise,
        )
    }
}
