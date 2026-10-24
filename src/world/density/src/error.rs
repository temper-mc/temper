pub type DensityResult<T> = Result<T, DensityRuntimeError>;
pub type DensityCompileResult<T> = Result<T, DensityCompileError>;

#[derive(Debug)]
pub enum DensityRuntimeError {
    StackUnderflow,
    PositionStackUnderflow,
    SplineRuntimeOutOfBounds { got: usize, max: usize },
    InvalidCache { got: usize, max: usize },
    InvalidFunctionId { got: usize, max: usize },
}

#[derive(Debug)]
pub enum DensityCompileError {
    MissingExternalFunction(String),
    UnknownNoise(String),
}
