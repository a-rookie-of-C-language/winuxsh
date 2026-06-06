// Error types for WinSH
use thiserror::Error;

/// WinSH error types
#[derive(Error, Debug)]
pub enum ShellError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Command not found: {0}")]
    CommandNotFound(String),

    #[error("Invalid command: {0}")]
    InvalidCommand(String),

    #[error("Job error: {0}")]
    Job(String),

    #[error("Config error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, ShellError>;
