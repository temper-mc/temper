use crate::wrapped::WrappedDensityFunction;
use temper_core::math::TemperMathExt;

pub struct Lerp {
    pub alpha: usize,
    pub first: usize,
    pub second: usize,
}

impl Lerp {
    pub fn compute(&self, func: &WrappedDensityFunction) -> f64 {
        func.execute_inner(self.alpha).lerp(
            func.execute_inner(self.first),
            func.execute_inner(self.second),
        )
    }
}
