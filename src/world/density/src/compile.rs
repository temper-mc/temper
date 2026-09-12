use crate::BoxedDensityFunction;
use crate::conditional::{IntervalSelect, RangeChoice};
use crate::json::{DensityFunction, DensityFunctionArgument, DensitySpline, ValueOrSpline};
use crate::mapped::{Axis, Gradient, Tiling};
use crate::marker::{Cache2d, CacheAllInCell, CacheOnce, FlatCache, Interpolated};
use crate::math::{
    Abs, Add, Clamp, Cube, Div, HalfNegative, Max, Min, Mul, Negate, QuarterNegative, Reciprocal,
    Square, Squeeze, Sub,
};
use crate::noise::{Noise, OldBlendedNoise, Shift, ShiftA, ShiftB};
use crate::spline::Spline;
use std::collections::HashMap;
use temper_core::random::{PositionalRandom, RandomSource};
use temper_noise::params::NoiseParameter;
use temper_noise::{BlendedNoise, NormalNoise};

pub struct Compiler<'a> {
    externals: &'a HashMap<String, DensityFunctionArgument>,
}

impl Compiler<'_> {
    pub fn compile<R: RandomSource, P: PositionalRandom<R>>(
        rand: &mut P,
        externals: &HashMap<String, DensityFunctionArgument>,
        func: DensityFunctionArgument,
    ) -> BoxedDensityFunction {
        let mut this = Compiler { externals };

        compile_arg(&mut this, rand, &func)
    }
}

fn compile_arg<R: RandomSource, P: PositionalRandom<R>>(
    compiler: &mut Compiler,
    rand: &mut P,
    arg: &DensityFunctionArgument,
) -> BoxedDensityFunction {
    match arg {
        DensityFunctionArgument::Constant(val) => Box::new(*val),
        DensityFunctionArgument::Function(val) => compile(compiler, rand, val.as_ref()),
        DensityFunctionArgument::External(val) => compile_arg(
            compiler,
            rand,
            compiler.externals.get(val).expect("missing"),
        ),
    }
}

fn compile<R: RandomSource, P: PositionalRandom<R>>(
    compiler: &mut Compiler,
    rand: &mut P,
    func: &DensityFunction,
) -> BoxedDensityFunction {
    match func {
        DensityFunction::Cache2d { input } => Box::new(Cache2d(compile_arg(compiler, rand, input))),
        DensityFunction::CacheAllInCell { input } => {
            Box::new(CacheAllInCell(compile_arg(compiler, rand, input)))
        }
        DensityFunction::CacheOnce { input } => {
            Box::new(CacheOnce(compile_arg(compiler, rand, input)))
        }
        DensityFunction::FlatCache { input } => {
            Box::new(FlatCache(compile_arg(compiler, rand, input)))
        }
        DensityFunction::Interpolated { input } => {
            Box::new(Interpolated(compile_arg(compiler, rand, input)))
        }
        DensityFunction::Noise {
            noise,
            xz_scale,
            y_scale,
        } => Box::new(Noise {
            noise: NormalNoise::new(
                &mut rand.spawn_from_hash(noise.as_str()),
                NoiseParameter::get_by_name(noise).expect("unknown noise"),
            ),
            xz_scale: *xz_scale,
            y_scale: *y_scale,
            shift_x: None,
            shift_y: None,
            shift_z: None,
        }),
        DensityFunction::OldBlendedNoise {
            xz_scale,
            y_scale,
            xz_factor,
            y_factor,
            smear_scale_multiplier,
        } => Box::new(OldBlendedNoise(BlendedNoise::new_seeded(
            &mut rand.spawn_from_hash("minecraft:terrain"),
            *xz_scale,
            *y_scale,
            *xz_factor,
            *y_factor,
            *smear_scale_multiplier,
        ))),
        DensityFunction::Shift { noise } => Box::new(Shift(NormalNoise::new(
            &mut rand.spawn_from_hash(noise.as_str()),
            NoiseParameter::get_by_name(noise).expect("unknown noise"),
        ))),
        DensityFunction::ShiftA { noise } => Box::new(ShiftA(NormalNoise::new(
            &mut rand.spawn_from_hash(noise.as_str()),
            NoiseParameter::get_by_name(noise).expect("unknown noise"),
        ))),
        DensityFunction::ShiftB { noise } => Box::new(ShiftB(NormalNoise::new(
            &mut rand.spawn_from_hash(noise.as_str()),
            NoiseParameter::get_by_name(noise).expect("unknown noise"),
        ))),
        DensityFunction::ShiftedNoise {
            noise,
            xz_scale,
            y_scale,
            shift_x,
            shift_y,
            shift_z,
        } => Box::new(Noise {
            noise: NormalNoise::new(
                &mut rand.spawn_from_hash(noise.as_str()),
                NoiseParameter::get_by_name(noise).expect("unknown noise"),
            ),
            xz_scale: *xz_scale,
            y_scale: *y_scale,
            shift_x: Some(compile_arg(compiler, rand, shift_x)),
            shift_y: Some(compile_arg(compiler, rand, shift_y)),
            shift_z: Some(compile_arg(compiler, rand, shift_z)),
        }),
        DensityFunction::Abs { input } => Box::new(Abs(compile_arg(compiler, rand, input))),
        DensityFunction::Add { left, right } => Box::new(Add {
            left: compile_arg(compiler, rand, left),
            right: compile_arg(compiler, rand, right),
        }),
        DensityFunction::Ceil { .. } => todo!(),
        DensityFunction::Clamp { input, min, max } => Box::new(Clamp {
            inner: compile_arg(compiler, rand, input),
            min: *min,
            max: *max,
        }),
        DensityFunction::Constant { value } => Box::new(*value),
        DensityFunction::Cube { input } => Box::new(Cube(compile_arg(compiler, rand, input))),
        DensityFunction::Div { left, right } => Box::new(Div {
            left: compile_arg(compiler, rand, left),
            right: compile_arg(compiler, rand, right),
        }),
        DensityFunction::Floor { .. } => todo!(),
        DensityFunction::Invert { input } => {
            Box::new(Reciprocal(compile_arg(compiler, rand, input)))
        }
        DensityFunction::Mul { left, right } => Box::new(Mul {
            left: compile_arg(compiler, rand, left),
            right: compile_arg(compiler, rand, right),
        }),
        DensityFunction::Min { left, right } => Box::new(Min {
            left: compile_arg(compiler, rand, left),
            right: compile_arg(compiler, rand, right),
        }),
        DensityFunction::Max { left, right } => Box::new(Max {
            left: compile_arg(compiler, rand, left),
            right: compile_arg(compiler, rand, right),
        }),
        DensityFunction::Negate { input } => Box::new(Negate(compile_arg(compiler, rand, input))),
        DensityFunction::Round { .. } => todo!(),
        DensityFunction::Sub { left, right } => Box::new(Sub {
            left: compile_arg(compiler, rand, left),
            right: compile_arg(compiler, rand, right),
        }),
        DensityFunction::Square { input } => Box::new(Square(compile_arg(compiler, rand, input))),
        DensityFunction::Truncate { .. } => todo!(),
        DensityFunction::YClampedGradient {
            from_y,
            to_y,
            from_value,
            to_value,
        } => Box::new(Gradient {
            axis: Axis::Y,
            tiling: Tiling::Legacy,
            from_coord: *from_y,
            to_coord: *to_y,
            from_value: *from_value,
            to_value: *to_value,
        }),
        DensityFunction::Squeeze { input } => Box::new(Squeeze(compile_arg(compiler, rand, input))),
        DensityFunction::Spline { spline } => Box::new(compile_spline(compiler, rand, spline)),
        DensityFunction::HalfNegative { input } => {
            Box::new(HalfNegative(compile_arg(compiler, rand, input)))
        }
        DensityFunction::QuarterNegative { input } => {
            Box::new(QuarterNegative(compile_arg(compiler, rand, input)))
        }
        DensityFunction::IntervalSelect {
            input,
            thresholds,
            functions,
        } => Box::new(IntervalSelect {
            input: compile_arg(compiler, rand, input),
            thresholds: thresholds.clone(),
            functions: functions
                .iter()
                .map(|v| compile_arg(compiler, rand, v))
                .collect(),
        }),
        DensityFunction::RangeChoice {
            input,
            min_inclusive,
            max_exclusive,
            when_in_range,
            when_out_of_range,
        } => Box::new(RangeChoice {
            input: compile_arg(compiler, rand, input),
            when_in_range: compile_arg(compiler, rand, when_in_range),
            when_out_range: compile_arg(compiler, rand, when_out_of_range),
            range: (*min_inclusive)..(*max_exclusive),
        }),
        DensityFunction::Beardifier => Box::new(0.0f64),
        DensityFunction::BlendAlpha => Box::new(1.0f64),
        DensityFunction::BlendOffset => Box::new(0.0f64),
        DensityFunction::BlendDensity { input } => compile_arg(compiler, rand, input),
        _ => todo!("{:?}", func),
    }
}

fn compile_spline<R: RandomSource, P: PositionalRandom<R>>(
    compiler: &mut Compiler,
    rand: &mut P,
    spline: &DensitySpline,
) -> Spline {
    let coordinate = compile_arg(compiler, rand, &spline.coordinate);
    let locations = spline.points.iter().map(|v| v.location).collect::<Vec<_>>();
    let derivatives = spline
        .points
        .iter()
        .map(|v| v.derivative)
        .collect::<Vec<_>>();
    let values = spline
        .points
        .iter()
        .map(|v| match &v.value {
            ValueOrSpline::Value(v) => Spline::Constant { value: *v },
            ValueOrSpline::Spline(s) => compile_spline(compiler, rand, s),
        })
        .collect::<Vec<_>>();

    Spline::Multipoint {
        coordinate,
        locations,
        derivatives,
        values,
    }
}
