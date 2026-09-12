pub enum BinaryDensityFunction {
    Add,
    Sub,
    Mul,
    Div,
    Min,
    Max,
}

impl BinaryDensityFunction {
    pub fn execute(&self, lhs: f64, rhs: f64) -> f64 {
        match self {
            Self::Add => lhs + rhs,
            Self::Sub => lhs - rhs,
            Self::Mul => lhs * rhs,
            Self::Div => lhs / rhs,
            Self::Min => lhs.min(rhs),
            Self::Max => lhs.max(rhs),
        }
    }
}
