use crate::compile::CompiledDensityFunction;
use crate::error::{DensityResult, DensityRuntimeError};
use crate::opcode::{DensityOpcode, DensitySpline, Tiling};
use bevy_math::{DVec3, IVec3};
use std::ops::{Div, Neg, Rem};
use temper_core::math::TemperMathExt;
use temper_core::pos::BlockPos;
use temper_noise::SimplexNoise;

pub type DensityStack = Vec<f64>;

#[derive(Clone)]
pub struct DensityCache {
    last_pos: BlockPos,
    last_value: f64,
}

pub struct DensityRuntime<'compiled> {
    main: &'compiled [DensityOpcode],
    pub(crate) functions: &'compiled [Box<[DensityOpcode]>],
    spline_runtimes: &'compiled [Box<[DensityOpcode]>],
    pub(crate) stack: DensityStack,
    pub(crate) function_cache: Box<[Option<f64>]>,
    pos_stack: Vec<BlockPos>,
    caches: Vec<DensityCache>,
}

enum MovementResult {
    Continue,
    JumpTo(usize),
}

impl DensityRuntime<'_> {
    pub fn new(func: &CompiledDensityFunction) -> DensityRuntime<'_> {
        DensityRuntime {
            main: func.main.as_ref(),
            functions: func.functions.as_ref(),
            spline_runtimes: func.spline_opcodes.as_ref(),
            caches: vec![
                DensityCache {
                    last_pos: IVec3::splat(i32::MAX).into(),
                    last_value: 0.0,
                };
                func.num_caches
            ],
            function_cache: vec![None; func.functions.len()].into_boxed_slice(),
            stack: Vec::new(),
            pos_stack: Vec::new(),
        }
    }

    pub fn execute_at(&mut self, pos: BlockPos) -> DensityResult<f64> {
        self.stack.clear();
        self.pos_stack.clear();
        self.function_cache.fill(None);

        self.pos_stack.push(pos);

        let mut position = 0;
        while position < self.main.len() {
            match self.main[position].execute(self)? {
                MovementResult::Continue => position += 1,
                MovementResult::JumpTo(jump) => position = jump,
            }
        }

        self.stack.pop().ok_or(DensityRuntimeError::StackUnderflow)
    }

    fn execute_spline_at(&mut self, index: usize) -> DensityResult<f64> {
        if index >= self.spline_runtimes.len() {
            return Err(DensityRuntimeError::SplineRuntimeOutOfBounds {
                got: index,
                max: self.spline_runtimes.len() - 1,
            });
        }

        let runtime = &self.spline_runtimes[index];
        let mut position = 0;
        while position < runtime.len() {
            match runtime[position].execute(self)? {
                MovementResult::Continue => position += 1,
                MovementResult::JumpTo(jump) => position = jump,
            }
        }

        self.stack.pop().ok_or(DensityRuntimeError::StackUnderflow)
    }

    pub(crate) fn execute_function(&mut self, index: usize) -> DensityResult<f64> {
        if index >= self.functions.len() {
            return Err(DensityRuntimeError::InvalidFunctionId {
                got: index,
                max: self.functions.len() - 1,
            });
        }

        let runtime = &self.functions[index];
        let mut position = 0;
        while position < runtime.len() {
            match runtime[position].execute(self)? {
                MovementResult::Continue => position += 1,
                MovementResult::JumpTo(jump) => position = jump,
            }
        }

        let res = self
            .stack
            .pop()
            .ok_or(DensityRuntimeError::StackUnderflow)?;
        self.function_cache[index] = Some(res);
        Ok(res)
    }

    fn current_pos(&self) -> DensityResult<&BlockPos> {
        self.pos_stack
            .last()
            .ok_or(DensityRuntimeError::PositionStackUnderflow)
    }

    fn pop_pos(&mut self) -> DensityResult<BlockPos> {
        self.pos_stack
            .pop()
            .ok_or(DensityRuntimeError::PositionStackUnderflow)
    }

    fn modify_last(&mut self, f: impl FnOnce(f64) -> f64) -> DensityResult<()> {
        let last = self
            .stack
            .last_mut()
            .ok_or(DensityRuntimeError::StackUnderflow)?;
        *last = f(*last);
        Ok(())
    }
}

impl DensityOpcode {
    // We always want this inlined, it should only be used above in the runtime execute functions.
    #[inline(always)]
    fn execute(&self, runtime: &mut DensityRuntime) -> DensityResult<MovementResult> {
        match self {
            DensityOpcode::PushConstant { value } => runtime.stack.push(*value),
            DensityOpcode::CallFunction { index } => {
                if let Some(val) = runtime
                    .function_cache
                    .get(*index)
                    .copied()
                    .unwrap_or_default()
                {
                    runtime.stack.push(val);
                } else {
                    let value = runtime.execute_function(*index)?;
                    runtime.stack.push(value);
                }
            }
            DensityOpcode::JumpTo { index } => return Ok(MovementResult::JumpTo(*index)),
            DensityOpcode::Add { lhs, rhs } => {
                let lhs = lhs.get_value(runtime)?;
                let rhs = rhs.get_value(runtime)?;
                runtime.stack.push(lhs + rhs)
            }
            DensityOpcode::Mul { lhs, rhs } => {
                let lhs = lhs.get_value(runtime)?;
                let rhs = rhs.get_value(runtime)?;
                runtime.stack.push(lhs * rhs)
            }
            DensityOpcode::Sub { lhs, rhs } => {
                let lhs = lhs.get_value(runtime)?;
                let rhs = rhs.get_value(runtime)?;
                runtime.stack.push(lhs - rhs)
            }
            DensityOpcode::Div { lhs, rhs } => {
                let lhs = lhs.get_value(runtime)?;
                let rhs = rhs.get_value(runtime)?;
                runtime.stack.push(lhs / rhs)
            }
            DensityOpcode::Min { a, b } => {
                let a = a.get_value(runtime)?;
                let b = b.get_value(runtime)?;
                runtime.stack.push(a.min(b))
            }
            DensityOpcode::Max { a, b } => {
                let a = a.get_value(runtime)?;
                let b = b.get_value(runtime)?;
                runtime.stack.push(a.max(b))
            }
            DensityOpcode::Abs => runtime.modify_last(f64::abs)?,
            DensityOpcode::Square => runtime.modify_last(|v| v * v)?,
            DensityOpcode::Cube => runtime.modify_last(|v| v * v * v)?,
            DensityOpcode::Negate => runtime.modify_last(f64::neg)?,
            DensityOpcode::Sign => runtime.modify_last(f64::signum)?,
            DensityOpcode::Sqrt => runtime.modify_last(f64::sqrt)?,
            DensityOpcode::Reciprocal => runtime.modify_last(f64::recip)?,
            DensityOpcode::Log => runtime.modify_last(f64::ln)?,
            DensityOpcode::Squeeze => runtime.modify_last(|v| {
                let v = v.clamp(-1.0, 1.0);
                v / 2.0 - v * v * v / 24.0
            })?,
            DensityOpcode::HalfNegative => {
                runtime.modify_last(|v| if v.is_sign_negative() { v * 0.5 } else { v })?
            }
            DensityOpcode::QuarterNegative => {
                runtime.modify_last(|v| if v.is_sign_negative() { v * 0.25 } else { v })?
            }
            DensityOpcode::Clamp { min, max } => runtime.modify_last(|v| v.clamp(*min, *max))?,
            DensityOpcode::Slice {
                axis,
                coordinate,
                cache_index,
                after,
            } => {
                let pos = axis.mask_coordinate(*runtime.current_pos()?, *coordinate);
                let max = runtime.caches.len() - 1;
                let cache = runtime.caches.get_mut(*cache_index).ok_or(
                    DensityRuntimeError::InvalidCache {
                        got: *cache_index,
                        max,
                    },
                )?;

                if cache.last_pos != pos {
                    runtime.pos_stack.push(pos);
                    // we continue here to generate the value
                } else {
                    runtime.stack.push(cache.last_value);
                    return Ok(MovementResult::JumpTo(*after));
                }
            }
            DensityOpcode::StoreSlice { cache_index } => {
                let value = *runtime
                    .stack
                    .last()
                    .ok_or(DensityRuntimeError::StackUnderflow)?;
                let pos = runtime.pop_pos()?;

                let max = runtime.caches.len() - 1;
                let cache = runtime.caches.get_mut(*cache_index).ok_or(
                    DensityRuntimeError::InvalidCache {
                        got: *cache_index,
                        max,
                    },
                )?;

                cache.last_pos = pos;
                cache.last_value = value;

                // value remains on the stack
            }
            DensityOpcode::EndIslands { island_noise } => {
                let pos = runtime.current_pos()?;

                runtime.stack.push(
                    ((get_height_value(island_noise, pos.pos.x / 8, pos.pos.z / 8) - 8.0) / 128.0)
                        as f64,
                )
            }
            DensityOpcode::Noise {
                noise,
                xz_scale,
                y_scale,
                shift_x,
                shift_y,
                shift_z,
            } => {
                let shift_x = shift_x.get_value(runtime)?;
                let shift_y = shift_y.get_value(runtime)?;
                let shift_z = shift_z.get_value(runtime)?;
                let pos = runtime.current_pos()?;

                let val = noise.noise(DVec3::new(
                    (pos.pos.x as f64 * *xz_scale) + shift_x,
                    (pos.pos.y as f64 * *y_scale) + shift_y,
                    (pos.pos.z as f64 * *xz_scale) + shift_z,
                ));
                runtime.stack.push(val);
            }
            DensityOpcode::BlendedNoise { noise } => {
                let val = noise.noise(runtime.current_pos()?.pos.as_dvec3());
                runtime.stack.push(val);
            }
            DensityOpcode::Shift { noise } => {
                let pos = runtime.current_pos()?;
                let val = noise.noise(pos.pos.as_dvec3() * 0.25) * 4.0;
                runtime.stack.push(val);
            }
            DensityOpcode::ShiftA { noise } => {
                let pos = runtime.current_pos()?;
                let val = noise.noise(DVec3::new(
                    pos.pos.x as f64 * 0.25,
                    0.0,
                    pos.pos.z as f64 * 0.25,
                )) * 4.0;
                runtime.stack.push(val);
            }
            DensityOpcode::ShiftB { noise } => {
                let pos = runtime.current_pos()?;
                let val = noise.noise(DVec3::new(
                    pos.pos.z as f64 * 0.25,
                    pos.pos.x as f64 * 0.25,
                    0.0,
                )) * 4.0;
                runtime.stack.push(val);
            }
            DensityOpcode::IntervalSelect {
                input,
                thresholds,
                indexes,
            } => {
                let input = input.get_value(runtime)?;

                for (i, threshold) in thresholds.iter().enumerate() {
                    if input < *threshold {
                        return Ok(MovementResult::JumpTo(indexes[i]));
                    }
                }

                return Ok(MovementResult::JumpTo(*indexes.last().unwrap()));
            }
            DensityOpcode::RangeChoice {
                input,
                range,
                when_out_of_range,
            } => {
                let input = input.get_value(runtime)?;
                if !range.contains(&input) {
                    return Ok(MovementResult::JumpTo(*when_out_of_range));
                }

                // continue here if in range
            }
            DensityOpcode::Spline { spline } => {
                let val = spline.execute(runtime)?;
                runtime.stack.push(val);
            }
            DensityOpcode::Gradient {
                axis,
                tiling,
                from_coord,
                to_coord,
                from_value,
                to_value,
            } => {
                let from_coord = *from_coord as f64;
                let to_coord = *to_coord as f64;

                let coord = axis.select_coordinate(*runtime.current_pos()?) as f64;
                let coord_range = to_coord - from_coord;
                let coord_factor = (to_value - from_value) / coord_range;

                let value = match tiling {
                    Tiling::ClampToEdge => {
                        let rel = coord.clamp(from_coord, to_coord) - from_coord;
                        from_value + rel * coord_factor
                    }
                    Tiling::MirroredRepeat => {
                        let rel = coord - from_coord;
                        let tile_idx = rel.div(coord_range).floor();
                        let local_coord = rel - tile_idx * coord_range;

                        if (tile_idx as i32 & 1) == 0 {
                            from_value + local_coord * coord_factor
                        } else {
                            from_value + (coord_range - local_coord) * coord_factor
                        }
                    }
                    Tiling::Repeat => {
                        let rel = coord - from_coord;
                        from_value + rel.rem(coord_range).floor() * coord_factor
                    }
                    Tiling::Legacy => coord.clamp(from_coord, to_coord).clamped_map(
                        from_coord,
                        to_coord,
                        *from_value,
                        *to_value,
                    ),
                };

                runtime.stack.push(value);
            }
        }

        Ok(MovementResult::Continue)
    }
}

impl DensitySpline {
    fn execute(&self, runtime: &mut DensityRuntime) -> DensityResult<f64> {
        match self {
            Self::Multipoint {
                coordinate,
                locations,
                derivatives,
                values,
            } => {
                let input = runtime.execute_spline_at(*coordinate)?;
                let last_index = locations.len() - 1;

                Ok(match Self::find_interval_start(locations, input) {
                    None => {
                        let value = values[0].execute(runtime)?;
                        Self::linear_extend(input, locations, derivatives, value, 0)
                    }
                    Some(index) if index == last_index => {
                        let value = values[last_index].execute(runtime)?;
                        Self::linear_extend(input, locations, derivatives, value, last_index)
                    }
                    Some(start) => {
                        let x1 = locations[start];
                        let x2 = locations[start + 1];
                        let dx = x2 - x1;
                        let t = input.inverse_lerp(x1, x2);
                        let y1 = values[start].execute(runtime)?;
                        let y2 = values[start + 1].execute(runtime)?;
                        let dy = y2 - y1;
                        let d1 = derivatives[start];
                        let d2 = derivatives[start + 1];
                        let a = d1 * dx - dy;
                        let b = -d2 * dx + dy;
                        t.lerp(y1, y2) + t * (1.0 - t) * t.lerp(a, b)
                    }
                })
            }
            Self::Constant(v) => Ok(*v),
        }
    }

    #[inline]
    fn linear_extend(
        input: f64,
        locations: &[f64],
        derivatives: &[f64],
        value: f64,
        index: usize,
    ) -> f64 {
        let derivative = derivatives[index];

        if derivative == 0.0 {
            value
        } else {
            value + derivative * (input - locations[index])
        }
    }

    #[inline]
    fn find_interval_start(locations: &[f64], input: f64) -> Option<usize> {
        locations
            .partition_point(|&location| input >= location)
            .checked_sub(1)
    }
}

fn get_height_value(island_noise: &SimplexNoise, section_x: i32, section_z: i32) -> f32 {
    let chunk_x = section_x / 2;
    let chunk_z = section_z / 2;
    let sub_section_x = section_x % 2;
    let sub_section_z = section_z % 2;
    let mut doffs = (100.0
        - f32::sqrt((section_x * section_x + section_z * section_z) as f32) * 8.0)
        .clamp(-100.0, 80.0);

    for xo in -12..=12 {
        for zo in -12..=12 {
            let total_chunk_x = chunk_x as i64 + xo as i64;
            let total_chunk_z = chunk_z as i64 + zo as i64;

            if total_chunk_x * total_chunk_x + total_chunk_z * total_chunk_z > 4096
                && island_noise.get_value(total_chunk_x as f64, total_chunk_z as f64) < -0.9
            {
                let island_size = (f32::abs(total_chunk_x as f32) * 3439.0
                    + f32::abs(total_chunk_z as f32) * 147.0)
                    % 13.0
                    + 9.0;
                let xd = (sub_section_x - xo * 2) as f32;
                let zd = (sub_section_z - zo * 2) as f32;
                let new_doffs =
                    (100.0 - f32::sqrt(xd * xd + zd * zd) * island_size).clamp(-100.0, 80.0);
                doffs = doffs.max(new_doffs);
            }
        }
    }

    doffs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opcode::DensityValueSource;

    fn test_simple(ops: &[DensityOpcode], pos: BlockPos) -> f64 {
        let spline_runtimes = [];
        let mut runtime = DensityRuntime {
            main: ops,
            functions: &[],
            spline_runtimes: &spline_runtimes,
            function_cache: Box::new([]),
            stack: Vec::new(),
            caches: Vec::new(),
            pos_stack: Vec::new(),
        };

        let value = runtime.execute_at(pos);
        assert!(value.is_ok());

        value.unwrap()
    }

    #[test]
    fn test_add() {
        let ops = [
            DensityOpcode::PushConstant { value: 5.0 },
            DensityOpcode::Add {
                lhs: DensityValueSource::Stack,
                rhs: DensityValueSource::Constant(5.0),
            },
        ];

        let res = test_simple(&ops, BlockPos::of(0, 0, 0));
        assert_eq!(res, 10.0);
    }

    #[test]
    fn test_mul() {
        let ops = [
            DensityOpcode::PushConstant { value: 5.0 },
            DensityOpcode::Mul {
                lhs: DensityValueSource::Stack,
                rhs: DensityValueSource::Constant(5.0),
            },
        ];

        let res = test_simple(&ops, BlockPos::of(0, 0, 0));
        assert_eq!(res, 25.0);
    }

    #[test]
    fn test_sub() {
        let ops = [
            DensityOpcode::PushConstant { value: 5.0 },
            DensityOpcode::Sub {
                lhs: DensityValueSource::Stack,
                rhs: DensityValueSource::Constant(5.0),
            },
        ];

        let res = test_simple(&ops, BlockPos::of(0, 0, 0));
        assert_eq!(res, 0.0);
    }

    #[test]
    fn test_div() {
        let ops = [
            DensityOpcode::PushConstant { value: 5.0 },
            DensityOpcode::Div {
                lhs: DensityValueSource::Stack,
                rhs: DensityValueSource::Constant(5.0),
            },
        ];

        let res = test_simple(&ops, BlockPos::of(0, 0, 0));
        assert_eq!(res, 1.0);
    }

    #[test]
    fn test_add_mul() {
        let ops = [
            DensityOpcode::PushConstant { value: 5.0 },
            DensityOpcode::Add {
                lhs: DensityValueSource::Stack,
                rhs: DensityValueSource::Constant(5.0),
            },
            DensityOpcode::Mul {
                lhs: DensityValueSource::Stack,
                rhs: DensityValueSource::Constant(5.0),
            },
        ];

        let res = test_simple(&ops, BlockPos::of(0, 0, 0));
        assert_eq!(res, 50.0);
    }
}
