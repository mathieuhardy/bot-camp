//! Error types for the bot-camp server.

use thiserror::Error;

/// A specialized `Result` type for bot-camp operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in the bot-camp server.
#[derive(Debug, Error)]
pub enum Error {
    /// I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
