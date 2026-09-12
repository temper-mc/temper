use crate::wrapped::binary::BinaryDensityFunction;
use crate::wrapped::unary::UnaryDensityFunction;
use crate::wrapped::{FlattenedDensityFunction, WrapContext};
use crate::{BoxedDensityFunction, DensityFunction};

macro_rules! math_function {
    (binary $(#[$attr:meta])? $name:ident, $wrapped_name:ident, $op:ident) => {
        #[derive(Debug)]
        $(#[$attr])?
        pub struct $name {
            pub left: BoxedDensityFunction,
            pub right: BoxedDensityFunction,
        }

        impl DensityFunction for $name {
            fn wrap<'a>(&'a self, ctx: &mut WrapContext<'a>) -> usize {
                ctx.push_op(|ctx| {
                    FlattenedDensityFunction::Binary {
                        op: BinaryDensityFunction::$op,
                        lhs: self.left.wrap(ctx),
                        rhs: self.right.wrap(ctx),
                    }
                })
            }
        }
    };
    (unary $(#[$attr:meta])? $name:ident, $wrapped_name:ident, $op:ident) => {
        #[derive(Debug)]
        $(#[$attr])?
        pub struct $name(pub BoxedDensityFunction);

        impl DensityFunction for $name {
            fn wrap<'a>(&'a self, ctx: &mut WrapContext<'a>) -> usize {
                ctx.push_op(|ctx| {
                    FlattenedDensityFunction::Unary {
                        op: UnaryDensityFunction::$op,
                        arg: self.0.wrap(ctx),
                    }
                })
            }
        }
    };
    (custom_unary $(#[$attr:meta])? $name:ident, $wrapped_name:ident, $op:ident, $($field:ident: $ty:ty),* $(,)?) => {
        #[derive(Debug)]
        $(#[$attr])?
        pub struct $name {
            pub inner: BoxedDensityFunction,
            $(
            pub $field: $ty,
            )*
        }

        impl DensityFunction for $name {
            fn wrap<'a>(&'a self, ctx: &mut WrapContext<'a>) -> usize {
                ctx.push_op(|ctx| {
                    FlattenedDensityFunction::Unary {
                        op: UnaryDensityFunction::$op($(self.$field),*),
                        arg: self.inner.wrap(ctx),
                    }
                })
            }
        }
    }
}

math_function!(binary Add, WrappedAdd, Add);
math_function!(binary Sub, WrappedSub, Sub);
math_function!(binary Mul, WrappedMul, Mul);
math_function!(binary Div, WrappedDiv, Div);
math_function!(binary Min, WrappedMin, Min);
math_function!(binary Max, WrappedMax, Max);
math_function!(unary Abs, WrappedAbs, Abs);
// math_function!(unary #[expect(dead_code)] Ceil, WrappedCeil, Ceil);
// math_function!(unary #[expect(dead_code)] Floor, WrappedFloor, Floor);
math_function!(unary Square, WrappedSquare, Square);
math_function!(unary Cube, WrappedCube, Cube);
math_function!(unary Negate, WrappedNegate, Negate);
// math_function!(unary #[expect(dead_code)] Round, WrappedRound, f64::round);
math_function!(unary #[expect(dead_code)] Sign, WrappedSign, Sign);
math_function!(unary #[expect(dead_code)] Sqrt, WrappedSqrt, Sqrt);
// math_function!(unary #[expect(dead_code)] Truncate, WrappedTruncate, f64::trunc);
math_function!(unary Reciprocal, WrappedReciprocal, Reciprocal);
math_function!(unary #[expect(dead_code)] Log, WrappedLog, Log);
math_function!(unary Squeeze, WrappedSqueeze, Squeeze);
math_function!(unary HalfNegative, WrappedHalfNegative, HalfNegative);
math_function!(unary QuarterNegative, WrappedQuarterNegative, QuarterNegative);
math_function!(custom_unary Clamp, WrappedClamp, Clamp, min: f64, max: f64);
