use ionic_config::errors::ConfigError;
use ionic_logging::errors::LoggingError;
use ionic_profiling::errors::ProfilingError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UtilsError {
    #[error("Something failed lol")]
    SomeError,

    #[error("Logging error: {0}")]
    LoggingError(#[from] LoggingError),

    #[error("Profiling error: {0}")]
    ProfilingError(#[from] ProfilingError),

    #[error("Config error: {0}")]
    ConfigError(#[from] ConfigError),
}
