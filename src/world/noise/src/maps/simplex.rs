use crate::maps::GRADIENT;
use bevy_math::DVec3;
use std::fmt::{Debug, Formatter};
use temper_core::random::RandomSource;

pub struct SimplexNoise {
    #[expect(dead_code)]
    orig: DVec3,
    permutation: Box<[i32]>,
}

impl Debug for SimplexNoise {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "SimplexNoise {{}}")
    }
}

impl SimplexNoise {
    pub fn new<R: RandomSource>(rand: &mut R) -> Self {
        let orig = DVec3::new(
            rand.next_f64() * 256.0,
            rand.next_f64() * 256.0,
            rand.next_f64() * 256.0,
        );

        let mut p = vec![0i32; 512];
        for (i, p) in p.iter_mut().enumerate().take(256) {
            *p = i as i32;
        }

        for ix in 0..256 {
            let offset = rand.next_u32_bounded(256 - ix as u32) as usize;
            p.swap(ix, ix + offset);
        }

        Self {
            orig,
            permutation: p.into_boxed_slice(),
        }
    }

    fn p(&self, x: i32) -> i32 {
        self.permutation[x as usize]
    }

    fn dot(g: &DVec3, x: f64, y: f64, z: f64) -> f64 {
        g.x * x + g.y * y + g.z * z
    }

    fn get_corner_noise_3d(index: i32, x: f64, y: f64, z: f64, base: f64) -> f64 {
        let t0 = base - x * x - y * y - z * z;

        if t0 < 0.0 {
            0.0
        } else {
            let t0 = t0 * t0;
            t0 * t0 * Self::dot(&GRADIENT[index as usize], x, y, z)
        }
    }

    pub fn get_value(&self, x_in: f64, y_in: f64) -> f64 {
        let f2 = 0.5 * (3f64.sqrt() - 1.0);
        let g2 = (3.0 - 3.0f64.sqrt()) / 6.0;

        let s = (x_in + y_in) * f2;
        let i = (x_in + s).floor();
        let j = (y_in + s).floor();

        let t = (i + j) * g2;
        let x0 = i - t;
        let y0 = j - t;
        let x_0 = x_in - x0;
        let y_0 = y_in - y0;

        let (i1, j1) = if x_0 > y_0 { (1.0, 0.0) } else { (0.0, 1.0) };

        let x1 = x_0 - i1 + g2;
        let y1 = y_0 - j1 + g2;
        let x2 = x_0 - 1.0 + 2.0 * g2;
        let y2 = y_0 - 1.0 + 2.0 * g2;
        let ii = i as i32 & 0xFF;
        let jj = j as i32 & 0xFF;
        let gi0 = self.p(ii + self.p(jj)) % 12;
        let gi1 = self.p(ii + i1 as i32 + self.p(jj + j1 as i32)) % 12;
        let gi2 = self.p(ii + 1 + self.p(jj + 1)) % 12;
        let n0 = Self::get_corner_noise_3d(gi0, x_0, y_0, 0.0, 0.5);
        let n1 = Self::get_corner_noise_3d(gi1, x1, y1, 0.0, 0.5);
        let n2 = Self::get_corner_noise_3d(gi2, x2, y2, 0.0, 0.5);
        70.0 * (n0 + n1 + n2)
    }
}
