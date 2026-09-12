use crate::wrapped::WrappedDensityFunction;
use bevy_math::DVec3;
use temper_noise::{BlendedNoise, NormalNoise};

pub enum NoiseDensityFunction<'a> {
    Noise {
        noise: &'a NormalNoise,
        xz_scale: f64,
        y_scale: f64,
        shift_x: Option<usize>,
        shift_y: Option<usize>,
        shift_z: Option<usize>,
    },
    BlendedNosie(&'a BlendedNoise),
    Shift(&'a NormalNoise),
    ShiftA(&'a NormalNoise),
    ShiftB(&'a NormalNoise),
}

impl NoiseDensityFunction<'_> {
    pub fn sample(&self, func: &WrappedDensityFunction) -> f64 {
        match self {
            Self::Noise {
                noise,
                xz_scale,
                y_scale,
                shift_x,
                shift_y,
                shift_z,
            } => {
                let shift_x = shift_x.map(|v| func.execute_inner(v)).unwrap_or_default();
                let shift_y = shift_y.map(|v| func.execute_inner(v)).unwrap_or_default();
                let shift_z = shift_z.map(|v| func.execute_inner(v)).unwrap_or_default();

                let pos = func.pos;
                noise.noise(DVec3::new(
                    (pos.pos.x as f64 * xz_scale) + shift_x,
                    (pos.pos.y as f64 * y_scale) + shift_y,
                    (pos.pos.z as f64 * xz_scale) + shift_z,
                ))
            }
            Self::BlendedNosie(noise) => noise.noise(func.pos.pos.as_dvec3()),
            Self::Shift(noise) => noise.noise(func.pos.pos.as_dvec3() * 0.25) * 4.0,
            Self::ShiftA(noise) => {
                noise.noise(DVec3::new(
                    func.pos.pos.x as f64 * 0.25,
                    0.0,
                    func.pos.pos.z as f64 * 0.25,
                )) * 4.0
            }
            Self::ShiftB(noise) => {
                noise.noise(DVec3::new(
                    func.pos.pos.z as f64 * 0.25,
                    func.pos.pos.x as f64 * 0.25,
                    0.0,
                )) * 4.0
            }
        }
    }
}
