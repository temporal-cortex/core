//! Error types for truth-engine operations.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum TruthError {
    #[error("Invalid RRULE: {0}")]
    InvalidRule(String),

    #[error("Invalid timezone: {0}")]
    InvalidTimezone(String),

    #[error("Expansion error: {0}")]
    Expansion(String),

    #[error("Availability error: {0}")]
    Availability(String),
}

pub type Result<T> = std::result::Result<T, TruthError>;
