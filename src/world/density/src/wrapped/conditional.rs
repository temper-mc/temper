use crate::wrapped::WrappedDensityFunction;
use std::ops::Range;

pub enum ConditionalDensityFunction<'a> {
    IntervalSelect {
        thresholds: &'a [f64],
        functions: Vec<usize>,
    },
    RangeChoice {
        range: &'a Range<f64>,
        when_in_range: usize,
        when_out_range: usize,
    },
}

impl ConditionalDensityFunction<'_> {
    pub fn compute(&self, input: f64, func: &WrappedDensityFunction) -> f64 {
        match self {
            Self::IntervalSelect {
                thresholds,
                functions,
            } => {
                for (i, threshold) in thresholds.iter().enumerate() {
                    if input < *threshold {
                        return func.execute_inner(functions[i]);
                    }
                }

                func.execute_inner(*functions.last().unwrap())
            }
            Self::RangeChoice {
                range,
                when_in_range,
                when_out_range,
            } => {
                if range.contains(&input) {
                    func.execute_inner(*when_in_range)
                } else {
                    func.execute_inner(*when_out_range)
                }
            }
        }
    }
}
