pub enum UnaryDensityFunction {
    Abs,
    Ceil(i32),
    Floor(i32),
    Round(i32),
    Truncate(i32),
    Square,
    Cube,
    Negate,
    Sign,
    Sqrt,
    Reciprocal,
    Log,
    Squeeze,
    HalfNegative,
    QuarterNegative,
    Clamp(f64, f64),
}

impl UnaryDensityFunction {
    pub fn execute(&self, arg: f64) -> f64 {
        match self {
            Self::Abs => arg.abs(),
            Self::Ceil(_) => todo!(),
            Self::Floor(_) => todo!(),
            Self::Round(_) => todo!(),
            Self::Truncate(_) => todo!(),
            Self::Square => arg * arg,
            Self::Cube => arg * arg * arg,
            Self::Negate => -arg,
            Self::Sign => arg.signum(),
            Self::Sqrt => arg.sqrt(),
            Self::Reciprocal => arg.recip(),
            Self::Log => arg.ln(),
            Self::Squeeze => {
                let arg = arg.clamp(-1.0, 1.0);
                arg / 2.0 - arg * arg * arg / 24.0
            }
            Self::HalfNegative => {
                if arg > 0.0 {
                    arg
                } else {
                    arg * 0.5
                }
            }
            Self::QuarterNegative => {
                if arg > 0.0 {
                    arg
                } else {
                    arg * 0.25
                }
            }
            Self::Clamp(min, max) => arg.clamp(*min, *max),
        }
    }
}
