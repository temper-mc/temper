use crate::error::{DensityResult, DensityRuntimeError};
use crate::runtime::DensityRuntime;
use std::ops::Range;
use temper_core::pos::BlockPos;
use temper_noise::{BlendedNoise, NormalNoise, SimplexNoise};

/// Represents an opcode and it's arguments for the density runtime to calculate.
#[derive(Debug)]
pub enum DensityOpcode {
    /// Pushes `value` onto the stack.
    ///
    /// This should only be used in cases where [`DensityValueSource`] can't be used, such as
    /// in a density function that always returns a constant.
    PushConstant {
        value: f64,
    },

    /// Pushes the value from the function at 'index' to the stack.
    CallFunction {
        index: usize,
    },

    /// Tells the runtime to jump to `index` as the next opcode to execute. This should be used
    /// to jump over code segments that won't be needed, like for skipping cache values that
    /// have already been calculated.
    JumpTo {
        index: usize,
    },

    /// Adds `lhs` and `rhs`, pushing the result onto the stack.
    Add {
        lhs: DensityValueSource,
        rhs: DensityValueSource,
    },

    /// Multiplies `lhs` and `rhs`, pushing the result onto the stack.
    Mul {
        lhs: DensityValueSource,
        rhs: DensityValueSource,
    },

    /// Subtracts `rhs` from `lhs`, pushing the result onto the stack.
    Sub {
        lhs: DensityValueSource,
        rhs: DensityValueSource,
    },

    /// Divides `lhs` by `rhs`, pushing the result onto the stack.
    Div {
        lhs: DensityValueSource,
        rhs: DensityValueSource,
    },

    /// Finds the minimum value between `a` and `b` and pushes that to the stack.
    Min {
        a: DensityValueSource,
        b: DensityValueSource,
    },

    /// Finds the maximum value between `a` and `b` and pushes that to the stack.
    Max {
        a: DensityValueSource,
        b: DensityValueSource,
    },

    /// Takes the absolute value of the value at the top of the stack.
    Abs,

    /// Squares the value on the top of the stack.
    Square,

    /// Cubes the value on the top of the stack.
    Cube,

    /// Negates the value on the top of the stack.
    Negate,

    /// Finds the sign of the value on the top of the stack.
    #[expect(dead_code)]
    Sign,

    /// Calculates the square root of value on the top of the stack.
    #[expect(dead_code)]
    Sqrt,

    /// Calculates the reciprocal of the value on the top of the stack.
    Reciprocal,

    /// Calculates the natural logarithm of the value on the top of the stack.
    #[expect(dead_code)]
    Log,

    EndIslands {
        island_noise: SimplexNoise,
    },

    /// Calculates the sum of the first two terms of the Maclaurin series for `(1 - cos(x))/x` using
    /// the value on the top of the stack as the input. The input is first clamped between `-1.0`
    /// and `1.0`.
    Squeeze,

    /// If the value on the top of the stack is negative, scales it by `0.5`.
    HalfNegative,

    /// If the value on the top of the stack is negative, scale is by `0.25`.
    QuarterNegative,

    /// Clamps the value on the top of the stack between `min` and `max`.
    Clamp {
        min: f64,
        max: f64,
    },

    /// Calculates the inner value with the axis `axis` of the current block set to `coordinate`.
    /// The result is then stored inside the cache at `cache_index`.
    Slice {
        axis: Axis,
        coordinate: i32,
        cache_index: usize,
        after: usize,
    },

    /// Stores the value at the top of the stack into the cache at `cache_index`.
    StoreSlice {
        cache_index: usize,
    },

    /// Samples the noise map `noise`, first scaling x and z by `xz_scale` and y by `y_scale` as
    /// well as shifting by `shift_x`, `shift_y`, and `shift_z`.
    Noise {
        noise: NormalNoise,
        xz_scale: f64,
        y_scale: f64,
        shift_x: DensityValueSource,
        shift_y: DensityValueSource,
        shift_z: DensityValueSource,
    },

    /// Samples the blended noise map `noise` at the current position.
    BlendedNoise {
        noise: BlendedNoise,
    },

    /// Samples the noise map `noise`, first scaling the position by `0.25` and then scaling the
    /// noise value by `4.0`.
    Shift {
        noise: NormalNoise,
    },

    /// Samples the noise map `noise` at (x, 0, z), first scaling the position by `0.25` and then
    /// scaling the noise value by `4.0`.
    ShiftA {
        noise: NormalNoise,
    },

    /// Samples the noise map `noise` at (z, x, 0), first scaling the position by `0.25` and then
    /// scaling the noise value by `4.0`.
    ShiftB {
        noise: NormalNoise,
    },

    /// Determines the function to execute based off the input value from the stack, jumping to that
    /// code segment and skipping others.
    IntervalSelect {
        input: DensityValueSource,
        thresholds: Vec<f64>,
        indexes: Vec<usize>,
    },

    /// Determines the function to execute based off the input value and `range`, jumping to the
    /// proper function. The function for when the input value is in range should follow this
    /// opcode.
    RangeChoice {
        input: DensityValueSource,
        range: Range<f64>,
        when_out_of_range: usize,
    },

    /// Executes a spline.
    Spline {
        spline: DensitySpline,
    },

    /// Computes a continuous gradient of values from `from_value` to `to_value` using the
    /// coordinate described by `axis` as the selector for the value in between.
    Gradient {
        axis: Axis,
        tiling: Tiling,
        from_coord: i32,
        to_coord: i32,
        from_value: f64,
        to_value: f64,
    },
}

/// Represents a source for a value.
#[derive(Debug)]
pub enum DensityValueSource {
    /// Pops a value off of the runtime stack.
    Stack,

    /// Returns the wrapped constant.
    Constant(f64),

    /// References a value from a function.
    Function(usize),
}

/// Represents an axis, either X, Y, or Z.
#[derive(Debug)]
pub enum Axis {
    /// Represents the x coordinate.
    #[expect(dead_code)]
    X,

    /// Represents the y coordinate.
    Y,

    /// Represents the z coordinate.
    #[expect(dead_code)]
    Z,
}

/// Represents a tiling mode to be used with the gradient instruction.
#[derive(Debug)]
pub enum Tiling {
    #[expect(dead_code)]
    ClampToEdge,
    #[expect(dead_code)]
    Repeat,
    #[expect(dead_code)]
    MirroredRepeat,

    /// Used for the legacy `y_clamped_gradient` density function.
    Legacy,
}

/// Nodes for a spline object.
#[derive(Debug)]
pub enum DensitySpline {
    /// Represents a multipoint spline. Coordinate should be the index into the runtime's spline
    /// code segments.
    Multipoint {
        coordinate: usize,
        locations: Vec<f64>,
        derivatives: Vec<f64>,
        values: Vec<DensitySpline>,
    },
    Constant(f64),
}

impl TryInto<DensityOpcode> for DensityValueSource {
    type Error = ();

    /// Attempts to convert this value source into an opcode.
    ///
    /// [DensityValueSource::Constant] will convert to [DensityOpcode::PushConstant].
    /// [DensityValueSource::Function] will convert to [DensityOpcode::CallFunction].
    /// [DensityValueSource::Stack] will result in `Err(())`.
    fn try_into(self) -> Result<DensityOpcode, Self::Error> {
        match self {
            Self::Constant(value) => Ok(DensityOpcode::PushConstant { value }),
            Self::Function(index) => Ok(DensityOpcode::CallFunction { index }),
            Self::Stack => Err(()),
        }
    }
}

impl DensityValueSource {
    /// Returns the value represented by this value source.
    ///
    /// # Arguments
    ///  * `stack`: the current runtime's stack.
    ///
    /// # Returns
    ///  * `Ok(value)`: the value represented by this value source.
    ///  * `Err(StackUnderflow)`: the stack didn't contain a value but one was expected.
    pub fn get_value(&self, runtime: &mut DensityRuntime) -> DensityResult<f64> {
        match self {
            DensityValueSource::Stack => runtime
                .stack
                .pop()
                .ok_or(DensityRuntimeError::StackUnderflow),
            DensityValueSource::Constant(c) => Ok(*c),
            DensityValueSource::Function(index) => {
                if let Some(value) = runtime
                    .function_cache
                    .get(*index)
                    .copied()
                    .unwrap_or_default()
                {
                    Ok(value)
                } else {
                    runtime.execute_function(*index)
                }
            }
        }
    }
}

impl Axis {
    /// Masks the coordinate covered by `self`, replacing it with `with_coord`.
    ///
    /// # Arguments
    ///  * `pos`: the position to mask the coordinate of.
    ///  * `with_coord`: the value to replace the coordinate described by `self` with.
    ///
    /// # Returns
    /// A [BlockPos] that contains `with_coord` on the axis described by `self` and the values from
    /// `pos` for the remaining coordinates.
    #[inline]
    pub fn mask_coordinate(&self, pos: BlockPos, with_coord: i32) -> BlockPos {
        match self {
            Self::X => pos.pos.with_x(with_coord).into(),
            Self::Y => pos.pos.with_y(with_coord).into(),
            Self::Z => pos.pos.with_z(with_coord).into(),
        }
    }

    /// Returns the coordinate covered by `self` from `pos`.
    ///
    /// # Arguments
    ///  * `pos`: the position to grab the coordinate from.
    ///
    /// # Returns
    /// The coordinate from `pos` described by `self`.
    #[inline]
    pub const fn select_coordinate(&self, pos: BlockPos) -> i32 {
        match self {
            Self::X => pos.pos.x,
            Self::Y => pos.pos.y,
            Self::Z => pos.pos.z,
        }
    }
}
