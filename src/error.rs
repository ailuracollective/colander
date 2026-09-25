//! Error types shared across the colander core.

use std::fmt;

/// A compile/validate/evaluate failure. The contract is deliberately thin:
/// a human-readable message is enough for callers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColanderError {
    pub message: String,
}

impl fmt::Display for ColanderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ColanderError {}

impl ColanderError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, ColanderError>;
