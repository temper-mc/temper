use crate::wrapped::noise::NoiseDensityFunction;
use crate::wrapped::{FlattenedDensityFunction, WrapContext};
use crate::{BoxedDensityFunction, DensityFunction};
use temper_noise::{BlendedNoise, NormalNoise};

#[derive(Debug)]
pub struct Noise {
    pub noise: NormalNoise,
    pub xz_scale: f64,
    pub y_scale: f64,
    pub shift_x: Option<BoxedDensityFunction>,
    pub shift_y: Option<BoxedDensityFunction>,
    pub shift_z: Option<BoxedDensityFunction>,
}

#[derive(Debug)]
pub struct OldBlendedNoise(pub BlendedNoise);

#[derive(Debug)]
pub struct Shift(pub NormalNoise);

#[derive(Debug)]
pub struct ShiftA(pub NormalNoise);

#[derive(Debug)]
pub struct ShiftB(pub NormalNoise);

impl DensityFunction for Noise {
    fn wrap<'a>(&'a self, ctx: &mut WrapContext<'a>) -> usize {
        ctx.push_op(|ctx| {
            FlattenedDensityFunction::Noise(NoiseDensityFunction::Noise {
                noise: &self.noise,
                xz_scale: self.xz_scale,
                y_scale: self.y_scale,
                shift_x: self.shift_x.as_ref().map(|v| v.wrap(ctx)),
                shift_y: self.shift_y.as_ref().map(|v| v.wrap(ctx)),
                shift_z: self.shift_z.as_ref().map(|v| v.wrap(ctx)),
            })
        })
    }
}

impl DensityFunction for OldBlendedNoise {
    fn wrap<'a>(&'a self, ctx: &mut WrapContext<'a>) -> usize {
        ctx.push_op(|_| {
            FlattenedDensityFunction::Noise(NoiseDensityFunction::BlendedNosie(&self.0))
        })
    }
}

impl DensityFunction for Shift {
    fn wrap<'a>(&'a self, ctx: &mut WrapContext<'a>) -> usize {
        ctx.push_op(|_| FlattenedDensityFunction::Noise(NoiseDensityFunction::Shift(&self.0)))
    }
}

impl DensityFunction for ShiftA {
    fn wrap<'a>(&'a self, ctx: &mut WrapContext<'a>) -> usize {
        ctx.push_op(|_| FlattenedDensityFunction::Noise(NoiseDensityFunction::ShiftA(&self.0)))
    }
}

impl DensityFunction for ShiftB {
    fn wrap<'a>(&'a self, ctx: &mut WrapContext<'a>) -> usize {
        ctx.push_op(|_| FlattenedDensityFunction::Noise(NoiseDensityFunction::ShiftB(&self.0)))
    }
}
