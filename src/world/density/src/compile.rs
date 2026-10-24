use crate::error::{DensityCompileError, DensityCompileResult};
use crate::json::{DensityFunction, DensityFunctionArgument, DensitySpline, ValueOrSpline};
use crate::opcode::{Axis, DensityOpcode, DensityValueSource, Tiling};
use indexmap::IndexMap;
use std::collections::HashMap;
use temper_core::random::{LegacyRandomSource, PositionalRandom, RandomSource};
use temper_noise::params::NoiseParameter;
use temper_noise::{BlendedNoise, NormalNoise, SimplexNoise};

pub struct CompiledDensityFunction {
    pub(crate) main: Box<[DensityOpcode]>,
    pub(crate) functions: Box<[Box<[DensityOpcode]>]>,
    pub(crate) spline_opcodes: Box<[Box<[DensityOpcode]>]>,
    pub(crate) num_caches: usize,
}

pub struct Compiler<'a> {
    externals: &'a HashMap<String, DensityFunctionArgument>,
    spline_opcodes: Vec<Vec<DensityOpcode>>,
    function_opcodes: IndexMap<String, Vec<DensityOpcode>>,
    num_caches: usize,
    legacy_noise: bool,
    seed: u64,
}

impl Compiler<'_> {
    pub fn compile<R: RandomSource, P: PositionalRandom<R>>(
        rand: &mut P,
        externals: &HashMap<String, DensityFunctionArgument>,
        func: DensityFunctionArgument,
        seed: u64,
        legacy_noise: bool,
    ) -> DensityCompileResult<CompiledDensityFunction> {
        let mut this = Compiler {
            externals,
            spline_opcodes: Vec::new(),
            function_opcodes: IndexMap::new(),
            num_caches: 0,
            legacy_noise,
            seed,
        };

        let mut opcodes = Vec::new();

        compile_arg(&mut this, &mut opcodes, rand, &func)?;

        Ok(CompiledDensityFunction {
            main: opcodes.into_boxed_slice(),
            functions: this
                .function_opcodes
                .into_iter()
                .map(|v| v.1.into_boxed_slice())
                .collect(),
            spline_opcodes: this
                .spline_opcodes
                .into_iter()
                .map(|v| v.into_boxed_slice())
                .collect(),
            num_caches: this.num_caches,
        })
    }
}

fn compile_arg<R: RandomSource, P: PositionalRandom<R>>(
    compiler: &mut Compiler,
    opcodes: &mut Vec<DensityOpcode>,
    rand: &mut P,
    arg: &DensityFunctionArgument,
) -> DensityCompileResult<DensityValueSource> {
    match arg {
        DensityFunctionArgument::Constant(val) => Ok(DensityValueSource::Constant(*val)),
        DensityFunctionArgument::Function(val) => {
            compile_to(compiler, opcodes, rand, val.as_ref())?;
            Ok(DensityValueSource::Stack)
        }
        DensityFunctionArgument::External(val) => {
            if let Some((idx, _, _)) = compiler.function_opcodes.get_full(val) {
                Ok(DensityValueSource::Function(idx))
            } else {
                let ops = compiler
                    .externals
                    .get(val)
                    .ok_or(DensityCompileError::MissingExternalFunction(val.clone()))?;

                let mut opcodes = Vec::new();
                let res = compile_arg(compiler, &mut opcodes, rand, ops)?;

                match res {
                    DensityValueSource::Stack => {
                        let (idx, _) = compiler.function_opcodes.insert_full(val.clone(), opcodes);
                        Ok(DensityValueSource::Function(idx))
                    }
                    other => Ok(other),
                }
            }
        }
    }
}

fn compile_to<R: RandomSource, P: PositionalRandom<R>>(
    compiler: &mut Compiler,
    opcodes: &mut Vec<DensityOpcode>,
    rand: &mut P,
    func: &DensityFunction,
) -> DensityCompileResult<()> {
    match func {
        DensityFunction::Cache2d { input } => {
            if let Ok(opcode) = compile_arg(compiler, opcodes, rand, input)?.try_into() {
                opcodes.push(opcode);
            }
        }
        DensityFunction::CacheAllInCell { input } => {
            if let Ok(opcode) = compile_arg(compiler, opcodes, rand, input)?.try_into() {
                opcodes.push(opcode);
            }
        }
        DensityFunction::CacheOnce { input } => {
            if let Ok(opcode) = compile_arg(compiler, opcodes, rand, input)?.try_into() {
                opcodes.push(opcode);
            }
        }
        DensityFunction::Interpolated { input } => {
            if let Ok(opcode) = compile_arg(compiler, opcodes, rand, input)?.try_into() {
                opcodes.push(opcode);
            }
        }
        DensityFunction::FlatCache { input } => {
            let idx = opcodes.len();
            let cache_index = compiler.num_caches;
            compiler.num_caches += 1;
            opcodes.push(DensityOpcode::Slice {
                axis: Axis::Y,
                coordinate: 0,
                cache_index,
                after: 0,
            });

            if let Ok(opcode) = compile_arg(compiler, opcodes, rand, input)?.try_into() {
                opcodes.push(opcode);
            }

            opcodes.push(DensityOpcode::StoreSlice { cache_index });

            let len = opcodes.len();
            let DensityOpcode::Slice { after, .. } = &mut opcodes[idx] else {
                unreachable!()
            };
            *after = len;
        }
        DensityFunction::EndIslands => {
            let mut rand = LegacyRandomSource::new(compiler.seed);
            rand.consume_count(17292);

            opcodes.push(DensityOpcode::EndIslands {
                island_noise: SimplexNoise::new(&mut rand),
            });
        }
        DensityFunction::Noise {
            noise,
            xz_scale,
            y_scale,
        } => opcodes.push(DensityOpcode::Noise {
            noise: create_noise(rand, noise.as_str(), compiler.seed)?,
            xz_scale: *xz_scale,
            y_scale: *y_scale,
            shift_x: DensityValueSource::Constant(0.0),
            shift_y: DensityValueSource::Constant(0.0),
            shift_z: DensityValueSource::Constant(0.0),
        }),
        DensityFunction::OldBlendedNoise {
            xz_scale,
            y_scale,
            xz_factor,
            y_factor,
            smear_scale_multiplier,
        } => opcodes.push(if compiler.legacy_noise {
            DensityOpcode::BlendedNoise {
                noise: BlendedNoise::new_seeded(
                    &mut LegacyRandomSource::new(compiler.seed),
                    *xz_scale,
                    *y_scale,
                    *xz_factor,
                    *y_factor,
                    *smear_scale_multiplier,
                ),
            }
        } else {
            DensityOpcode::BlendedNoise {
                noise: BlendedNoise::new_seeded(
                    &mut rand.spawn_from_hash("minecraft:terrain"),
                    *xz_scale,
                    *y_scale,
                    *xz_factor,
                    *y_factor,
                    *smear_scale_multiplier,
                ),
            }
        }),
        DensityFunction::Shift { noise } => opcodes.push(DensityOpcode::Shift {
            noise: create_noise(rand, noise.as_str(), compiler.seed)?,
        }),
        DensityFunction::ShiftA { noise } => opcodes.push(DensityOpcode::ShiftA {
            noise: create_noise(rand, noise.as_str(), compiler.seed)?,
        }),
        DensityFunction::ShiftB { noise } => opcodes.push(DensityOpcode::ShiftB {
            noise: create_noise(rand, noise.as_str(), compiler.seed)?,
        }),
        DensityFunction::ShiftedNoise {
            noise,
            xz_scale,
            y_scale,
            shift_x,
            shift_y,
            shift_z,
        } => {
            let shift_z = compile_arg(compiler, opcodes, rand, shift_z)?;
            let shift_y = compile_arg(compiler, opcodes, rand, shift_y)?;
            let shift_x = compile_arg(compiler, opcodes, rand, shift_x)?;

            opcodes.push(DensityOpcode::Noise {
                noise: create_noise(rand, noise.as_str(), compiler.seed)?,
                xz_scale: *xz_scale,
                y_scale: *y_scale,
                shift_x,
                shift_y,
                shift_z,
            })
        }
        DensityFunction::Abs { input } => {
            if let Ok(opcode) = compile_arg(compiler, opcodes, rand, input)?.try_into() {
                opcodes.push(opcode);
            }

            opcodes.push(DensityOpcode::Abs)
        }
        DensityFunction::Add { left, right } => {
            let rhs = compile_arg(compiler, opcodes, rand, right)?;
            let lhs = compile_arg(compiler, opcodes, rand, left)?;

            opcodes.push(DensityOpcode::Add { lhs, rhs })
        }
        DensityFunction::Clamp { input, min, max } => {
            if let Ok(opcode) = compile_arg(compiler, opcodes, rand, input)?.try_into() {
                opcodes.push(opcode);
            }

            opcodes.push(DensityOpcode::Clamp {
                min: *min,
                max: *max,
            })
        }
        DensityFunction::Constant { value } => {
            opcodes.push(DensityOpcode::PushConstant { value: *value })
        }
        DensityFunction::Cube { input } => {
            if let Ok(opcode) = compile_arg(compiler, opcodes, rand, input)?.try_into() {
                opcodes.push(opcode);
            }

            opcodes.push(DensityOpcode::Cube)
        }
        DensityFunction::Div { left, right } => {
            let rhs = compile_arg(compiler, opcodes, rand, right)?;
            let lhs = compile_arg(compiler, opcodes, rand, left)?;

            opcodes.push(DensityOpcode::Div { lhs, rhs })
        }
        DensityFunction::Invert { input } => {
            if let Ok(opcode) = compile_arg(compiler, opcodes, rand, input)?.try_into() {
                opcodes.push(opcode);
            }

            opcodes.push(DensityOpcode::Reciprocal)
        }
        DensityFunction::Mul { left, right } => {
            let rhs = compile_arg(compiler, opcodes, rand, right)?;
            let lhs = compile_arg(compiler, opcodes, rand, left)?;

            opcodes.push(DensityOpcode::Mul { lhs, rhs })
        }
        DensityFunction::Min { left, right } => {
            let b = compile_arg(compiler, opcodes, rand, right)?;
            let a = compile_arg(compiler, opcodes, rand, left)?;

            opcodes.push(DensityOpcode::Min { a, b })
        }
        DensityFunction::Max { left, right } => {
            let b = compile_arg(compiler, opcodes, rand, right)?;
            let a = compile_arg(compiler, opcodes, rand, left)?;

            opcodes.push(DensityOpcode::Max { a, b })
        }
        DensityFunction::Negate { input } => {
            if let Ok(opcode) = compile_arg(compiler, opcodes, rand, input)?.try_into() {
                opcodes.push(opcode);
            }

            opcodes.push(DensityOpcode::Negate)
        }
        DensityFunction::Sub { left, right } => {
            let rhs = compile_arg(compiler, opcodes, rand, right)?;
            let lhs = compile_arg(compiler, opcodes, rand, left)?;

            opcodes.push(DensityOpcode::Sub { lhs, rhs })
        }
        DensityFunction::Square { input } => {
            if let Ok(opcode) = compile_arg(compiler, opcodes, rand, input)?.try_into() {
                opcodes.push(opcode);
            }

            opcodes.push(DensityOpcode::Square)
        }
        DensityFunction::YClampedGradient {
            from_y,
            to_y,
            from_value,
            to_value,
        } => opcodes.push(DensityOpcode::Gradient {
            axis: Axis::Y,
            tiling: Tiling::Legacy,
            from_value: *from_value,
            to_value: *to_value,
            from_coord: *from_y,
            to_coord: *to_y,
        }),
        DensityFunction::Squeeze { input } => {
            if let Ok(opcode) = compile_arg(compiler, opcodes, rand, input)?.try_into() {
                opcodes.push(opcode);
            }

            opcodes.push(DensityOpcode::Squeeze)
        }
        DensityFunction::Spline { spline } => {
            let spline = compile_spline(compiler, opcodes, rand, spline)?;
            opcodes.push(DensityOpcode::Spline { spline })
        }
        DensityFunction::HalfNegative { input } => {
            if let Ok(opcode) = compile_arg(compiler, opcodes, rand, input)?.try_into() {
                opcodes.push(opcode);
            }

            opcodes.push(DensityOpcode::HalfNegative)
        }
        DensityFunction::QuarterNegative { input } => {
            if let Ok(opcode) = compile_arg(compiler, opcodes, rand, input)?.try_into() {
                opcodes.push(opcode);
            }

            opcodes.push(DensityOpcode::QuarterNegative)
        }
        DensityFunction::IntervalSelect {
            input,
            thresholds,
            functions,
        } => {
            // TODO: if evaluates to constant, then we can inline
            let input = compile_arg(compiler, opcodes, rand, input)?;

            let idx = opcodes.len();
            opcodes.push(DensityOpcode::IntervalSelect {
                input,
                thresholds: thresholds.clone(),
                indexes: Vec::with_capacity(0),
            });

            let (indexes, jumps): (Vec<usize>, Vec<usize>) = functions
                .iter()
                .map(|f| {
                    let idx = opcodes.len();
                    if let Ok(opcode) = compile_arg(compiler, opcodes, rand, f)?.try_into() {
                        opcodes.push(opcode);
                    }

                    let jump = opcodes.len();
                    opcodes.push(DensityOpcode::JumpTo { index: 0 });
                    Ok((idx, jump))
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .unzip();

            let DensityOpcode::IntervalSelect { indexes: val, .. } = &mut opcodes[idx] else {
                unreachable!()
            };

            *val = indexes;

            let jump_to = opcodes.len();
            for jump in jumps {
                let DensityOpcode::JumpTo { index } = &mut opcodes[jump] else {
                    unreachable!()
                };

                *index = jump_to;
            }
        }
        DensityFunction::RangeChoice {
            input,
            min_inclusive,
            max_exclusive,
            when_in_range,
            when_out_of_range,
        } => {
            // TODO: if evaluates to constant, we can inline
            let input = compile_arg(compiler, opcodes, rand, input)?;

            let idx = opcodes.len();
            opcodes.push(DensityOpcode::RangeChoice {
                input,
                range: *min_inclusive..*max_exclusive,
                when_out_of_range: 0,
            });

            if let Ok(opcode) = compile_arg(compiler, opcodes, rand, when_in_range)?.try_into() {
                opcodes.push(opcode);
            }

            let jump_idx = opcodes.len();
            opcodes.push(DensityOpcode::JumpTo { index: 0 });

            if let Ok(opcode) = compile_arg(compiler, opcodes, rand, when_out_of_range)?.try_into()
            {
                opcodes.push(opcode);
            }

            let out_idx = opcodes.len();

            let DensityOpcode::RangeChoice {
                when_out_of_range, ..
            } = &mut opcodes[idx]
            else {
                unreachable!()
            };

            *when_out_of_range = jump_idx + 1;

            let DensityOpcode::JumpTo { index } = &mut opcodes[jump_idx] else {
                unreachable!()
            };

            *index = out_idx;
        }
        DensityFunction::Beardifier => opcodes.push(DensityOpcode::PushConstant { value: 0.0 }),
        DensityFunction::BlendAlpha => opcodes.push(DensityOpcode::PushConstant { value: 1.0 }),
        DensityFunction::BlendOffset => opcodes.push(DensityOpcode::PushConstant { value: 0.0 }),
        DensityFunction::BlendDensity { input } => {
            if let Ok(opcode) = compile_arg(compiler, opcodes, rand, input)?.try_into() {
                opcodes.push(opcode);
            }
        }
        _ => todo!("{:?}", func),
    }

    Ok(())
}

fn compile_spline<R: RandomSource, P: PositionalRandom<R>>(
    compiler: &mut Compiler,
    _opcodes: &mut Vec<DensityOpcode>,
    rand: &mut P,
    spline: &DensitySpline,
) -> DensityCompileResult<crate::opcode::DensitySpline> {
    let spline_idx = compiler.spline_opcodes.len();
    compiler.spline_opcodes.push(Vec::with_capacity(0));
    let mut spline_opcodes = Vec::new();
    if let Ok(opcode) =
        compile_arg(compiler, &mut spline_opcodes, rand, &spline.coordinate)?.try_into()
    {
        spline_opcodes.push(opcode);
    }

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
            ValueOrSpline::Value(v) => Ok(crate::opcode::DensitySpline::Constant(*v)),
            ValueOrSpline::Spline(s) => compile_spline(compiler, _opcodes, rand, s),
        })
        .collect::<Result<Vec<_>, _>>()?;

    compiler.spline_opcodes[spline_idx] = spline_opcodes;
    Ok(crate::opcode::DensitySpline::Multipoint {
        coordinate: spline_idx,
        locations,
        derivatives,
        values,
    })
}

fn create_noise<R: RandomSource, P: PositionalRandom<R>>(
    rand: &mut P,
    name: &str,
    seed: u64,
) -> DensityCompileResult<NormalNoise> {
    Ok(match name {
        "minecraft:nether/temperature" => NormalNoise::new::<LegacyRandomSource, true>(
            &mut LegacyRandomSource::new(seed),
            &NoiseParameter::NETHER_TEMPERATURE,
        ),
        "minecraft:nether/vegetation" => NormalNoise::new::<LegacyRandomSource, true>(
            &mut LegacyRandomSource::new(seed + 1),
            &NoiseParameter::NETHER_VEGETATION,
        ),
        name => NormalNoise::new::<R, false>(
            &mut rand.spawn_from_hash(name),
            NoiseParameter::get_by_name(name)
                .ok_or(DensityCompileError::UnknownNoise(name.to_string()))?,
        ),
    })
}
