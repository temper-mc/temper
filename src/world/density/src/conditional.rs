use crate::wrapped::conditional::ConditionalDensityFunction;
use crate::wrapped::{FlattenedDensityFunction, WrapContext};
use crate::{BoxedDensityFunction, DensityFunction};
use std::ops::Range;

#[derive(Debug)]
pub struct IntervalSelect {
    pub input: BoxedDensityFunction,
    pub thresholds: Vec<f64>,
    pub functions: Vec<BoxedDensityFunction>,
}

#[derive(Debug)]
pub struct RangeChoice {
    pub input: BoxedDensityFunction,
    pub range: Range<f64>,
    pub when_in_range: BoxedDensityFunction,
    pub when_out_range: BoxedDensityFunction,
}

impl DensityFunction for IntervalSelect {
    fn wrap<'a>(&'a self, ctx: &mut WrapContext<'a>) -> usize {
        ctx.push_op(|ctx| FlattenedDensityFunction::Conditional {
            op: ConditionalDensityFunction::IntervalSelect {
                thresholds: &self.thresholds,
                functions: self.functions.iter().map(|v| v.wrap(ctx)).collect(),
            },
            input: self.input.wrap(ctx),
        })
    }
}

impl DensityFunction for RangeChoice {
    fn wrap<'a>(&'a self, ctx: &mut WrapContext<'a>) -> usize {
        ctx.push_op(|ctx| FlattenedDensityFunction::Conditional {
            op: ConditionalDensityFunction::RangeChoice {
                range: &self.range,
                when_in_range: self.when_in_range.wrap(ctx),
                when_out_range: self.when_out_range.wrap(ctx),
            },
            input: self.input.wrap(ctx),
        })
    }
}
