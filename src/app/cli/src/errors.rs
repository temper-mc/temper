use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    /// One or more entries could not be cleared
    #[error("{0} entries could not be cleared")]
    ClearEntries(usize),
    /// IO error    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
