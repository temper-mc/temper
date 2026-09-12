pub mod binary;
pub mod conditional;
pub mod mapped;
pub mod marker;
pub mod noise;
pub mod spline;
pub mod unary;

pub use marker::*;

use crate::BoxedDensityFunction;
use crate::mapped::Gradient;
use crate::wrapped::binary::BinaryDensityFunction;
use crate::wrapped::conditional::ConditionalDensityFunction;
use crate::wrapped::mapped::Lerp;
use crate::wrapped::noise::NoiseDensityFunction;
use crate::wrapped::spline::FlattenedSpline;
use crate::wrapped::unary::UnaryDensityFunction;
use temper_core::pos::BlockPos;

pub struct WrappedDensityFunction<'a> {
    functions: Box<[FlattenedDensityFunction<'a>]>,
    pos: BlockPos,
}

pub struct WrapContext<'a> {
    ops: Vec<FlattenedDensityFunction<'a>>,
    pub size_xz: i32,
    pub first_x: i32,
    pub first_z: i32,
}

pub enum FlattenedDensityFunction<'a> {
    Constant(f64),
    Unary {
        op: UnaryDensityFunction,
        arg: usize,
    },
    Binary {
        op: BinaryDensityFunction,
        lhs: usize,
        rhs: usize,
    },
    Spline(FlattenedSpline<'a>),
    Marker {
        op: MarkerDensityFunction,
        arg: usize,
    },
    Gradient(&'a Gradient),
    Lerp(Lerp),
    Conditional {
        op: ConditionalDensityFunction<'a>,
        input: usize,
    },
    Noise(NoiseDensityFunction<'a>),
}

impl WrappedDensityFunction<'_> {
    pub fn wrap(
        func: &BoxedDensityFunction,
        size_xz: i32,
        first_x: i32,
        first_z: i32,
    ) -> WrappedDensityFunction<'_> {
        let mut wrap_ctx = WrapContext {
            ops: Vec::new(),
            size_xz,
            first_x,
            first_z,
        };
        func.wrap(&mut wrap_ctx);

        WrappedDensityFunction {
            functions: wrap_ctx.ops.into_boxed_slice(),
            pos: BlockPos::of(i32::MAX, i32::MAX, i32::MAX),
        }
    }

    pub fn execute(&mut self, pos: BlockPos) -> f64 {
        self.pos = pos;
        self.execute_inner(0)
    }

    fn execute_inner(&self, idx: usize) -> f64 {
        let func = unsafe {
            ((&raw const self.functions[idx]) as *mut FlattenedDensityFunction).as_mut_unchecked()
        };
        func.execute(self)
    }

    fn execute_inner_at(&self, idx: usize, pos: BlockPos) -> f64 {
        let old_pos = self.pos;
        unsafe {
            *(&raw const self.pos as *mut BlockPos).as_mut_unchecked() = pos;
        }
        let func = unsafe {
            ((&raw const self.functions[idx]) as *mut FlattenedDensityFunction).as_mut_unchecked()
        };
        let out = func.execute(self);
        unsafe {
            *(&raw const self.pos as *mut BlockPos).as_mut_unchecked() = old_pos;
        }
        out
    }
}

impl FlattenedDensityFunction<'_> {
    fn execute(&mut self, func: &WrappedDensityFunction) -> f64 {
        match self {
            FlattenedDensityFunction::Constant(val) => *val,
            FlattenedDensityFunction::Unary { op, arg } => op.execute(func.execute_inner(*arg)),
            FlattenedDensityFunction::Binary { op, lhs, rhs } => {
                op.execute(func.execute_inner(*lhs), func.execute_inner(*rhs))
            }
            FlattenedDensityFunction::Spline(spline) => spline.sample(func),
            FlattenedDensityFunction::Marker { op, arg } => op.execute(*arg, func),
            FlattenedDensityFunction::Gradient(gradient) => gradient.sample(func.pos),
            FlattenedDensityFunction::Lerp(lerp) => lerp.compute(func),
            FlattenedDensityFunction::Conditional { op, input } => {
                op.compute(func.execute_inner(*input), func)
            }
            FlattenedDensityFunction::Noise(noise) => noise.sample(func),
        }
    }
}

impl<'a> WrapContext<'a> {
    pub fn push_op(&mut self, f: impl FnOnce(&mut Self) -> FlattenedDensityFunction<'a>) -> usize {
        let idx = self.ops.len();
        // SAFETY: we replace the data before returning
        self.ops.push(unsafe { std::mem::zeroed() });
        self.ops[idx] = f(self);
        idx
    }
}
