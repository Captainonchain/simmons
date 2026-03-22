//! Error types for the Simmons trading system

use thiserror::Error;

/// Main error type for Simmons
#[derive(Error, Debug)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Feed error: {0}")]
    Feed(String),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("JSON parsing error: {0}")]
    Json(String),

    #[error("Risk limit exceeded: {0}")]
    RiskLimit(String),

    #[error("Insufficient capital: need {needed}, have {available}")]
    InsufficientCapital {
        needed: rust_decimal::Decimal,
        available: rust_decimal::Decimal,
    },

    #[error("Position not found: {0}")]
    PositionNotFound(String),

    #[error("Execution error: {0}")]
    Execution(String),

    #[error("Brain timeout: no decision within {0}s")]
    BrainTimeout(u64),

    #[error("Brain error: {0}")]
    Brain(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e.to_string())
    }
}

impl From<toml::de::Error> for Error {
    fn from(e: toml::de::Error) -> Self {
        Error::Config(e.to_string())
    }
}

/// Result type alias for Simmons operations
pub type Result<T> = std::result::Result<T, Error>;

/// Risk-specific errors
#[derive(Error, Debug)]
pub enum RiskError {
    #[error("Max drawdown exceeded: {current}% > {limit}%")]
    MaxDrawdownExceeded {
        current: rust_decimal::Decimal,
        limit: rust_decimal::Decimal,
    },

    #[error("Max position size exceeded: {size} > {limit}")]
    MaxPositionExceeded {
        size: rust_decimal::Decimal,
        limit: rust_decimal::Decimal,
    },

    #[error("Daily loss limit reached: ${loss}")]
    DailyLossLimit { loss: rust_decimal::Decimal },

    #[error("Correlation limit exceeded")]
    CorrelationLimit,

    #[error("Trading halted")]
    TradingHalted,
}

impl From<RiskError> for Error {
    fn from(e: RiskError) -> Self {
        Error::RiskLimit(e.to_string())
    }
}
