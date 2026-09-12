use crate::wrapped::{FlattenedDensityFunction, WrapContext};
use crate::{BoxedDensityFunction, DensityFunction};
use std::ops::{Div, Rem};
use temper_core::math::TemperMathExt;
use temper_core::pos::BlockPos;

#[derive(Debug)]
pub enum Axis {
    X,
    Y,
    Z,
}

#[derive(Debug)]
pub enum Tiling {
    ClampToEdge,
    Repeat,
    MirroredRepeat,
    Legacy,
}

#[derive(Debug)]
pub struct Gradient {
    pub axis: Axis,
    pub tiling: Tiling,
    pub from_coord: i32,
    pub to_coord: i32,
    pub from_value: f64,
    pub to_value: f64,
}

#[derive(Debug)]
#[expect(dead_code)]
pub struct Lerp {
    pub alpha: BoxedDensityFunction,
    pub first: BoxedDensityFunction,
    pub second: BoxedDensityFunction,
}

impl Axis {
    fn get_coord(&self, pos: &BlockPos) -> f64 {
        match self {
            Axis::X => pos.pos.x as f64,
            Axis::Y => pos.pos.y as f64,
            Axis::Z => pos.pos.z as f64,
        }
    }
}

impl DensityFunction for Gradient {
    fn wrap<'a>(&'a self, ctx: &mut WrapContext<'a>) -> usize {
        ctx.push_op(|_| FlattenedDensityFunction::Gradient(self))
    }
}

impl Gradient {
    pub fn sample(&self, pos: BlockPos) -> f64 {
        let coord = self.axis.get_coord(&pos);
        let coord_range = self.to_coord as f64 - self.from_coord as f64;
        let coord_factor = (self.to_value - self.from_value) / coord_range;

        match self.tiling {
            Tiling::ClampToEdge => {
                let rel = coord.clamp(self.from_coord as f64, self.to_coord as f64)
                    - self.from_coord as f64;
                self.from_value + rel * coord_factor
            }
            Tiling::MirroredRepeat => {
                let rel = coord - self.from_coord as f64;
                let tile_idx = rel.div(coord_range).floor();
                let local_coord = rel - tile_idx * coord_range;

                if (tile_idx as i32 & 1) == 0 {
                    self.from_value + local_coord * coord_factor
                } else {
                    self.from_value + (coord_range - local_coord) * coord_factor
                }
            }
            Tiling::Repeat => {
                let rel = coord - self.from_coord as f64;
                self.from_value + rel.rem(coord_range).floor() * coord_factor
            }
            Tiling::Legacy => coord
                .clamp(self.from_coord as f64, self.to_coord as f64)
                .clamped_map(
                    self.from_coord as f64,
                    self.to_coord as f64,
                    self.from_value,
                    self.to_value,
                ),
        }
    }
}

impl DensityFunction for Lerp {
    fn wrap<'a>(&'a self, ctx: &mut WrapContext<'a>) -> usize {
        ctx.push_op(|ctx| {
            FlattenedDensityFunction::Lerp(crate::wrapped::mapped::Lerp {
                alpha: self.alpha.wrap(ctx),
                first: self.first.wrap(ctx),
                second: self.second.wrap(ctx),
            })
        })
    }
}
