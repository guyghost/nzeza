use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::mpsc;

use crate::domain::entities::exchange::Exchange;

/// Error type that includes rich context information
#[derive(Debug, Clone)]
pub enum DetailedMpcError {
    /// Order validation failed with specific context
    OrderValidationFailed { symbol: String, reason: String },
    /// Insufficient balance to execute order
    InsufficientBalance {
        required: f64,
        available: f64,
        currency: String,
    },
    /// Position limits exceeded
    PositionLimitExceeded {
        symbol: String,
        limit: u32,
        current: u32,
        limit_type: PositionLimitType,
    },
    /// No traders available to execute orders
    TraderUnavailable {
        reason: String,
        available_traders: Vec<String>,
    },
    /// Exchange connection issue
    ExchangeConnectionLost {
        exchange: String,
        last_contact: std::time::Duration,
        reason: String,
    },
    /// Insufficient data for signal generation
    InsufficientCandles {
        symbol: String,
        required: usize,
        current: usize,
    },
    /// Signal confidence too low (warning, not error)
    LowConfidenceSignal {
        symbol: String,
        confidence: f64,
        threshold: f64,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum PositionLimitType {
    PerSymbol,
    Total,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorSeverity {
    Minor,
    Moderate,
    Critical,
}

impl DetailedMpcError {
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            DetailedMpcError::LowConfidenceSignal { .. } => ErrorSeverity::Minor,
            DetailedMpcError::InsufficientCandles { .. } => ErrorSeverity::Moderate,
            DetailedMpcError::PositionLimitExceeded { .. } => ErrorSeverity::Moderate,
            DetailedMpcError::OrderValidationFailed { .. } => ErrorSeverity::Moderate,
            DetailedMpcError::InsufficientBalance { .. } => ErrorSeverity::Moderate,
            DetailedMpcError::TraderUnavailable { .. } => ErrorSeverity::Critical,
            DetailedMpcError::ExchangeConnectionLost { .. } => ErrorSeverity::Critical,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            DetailedMpcError::OrderValidationFailed { symbol, reason } => {
                format!("Order validation failed for {}: {}", symbol, reason)
            }
            DetailedMpcError::InsufficientBalance {
                required,
                available,
                currency,
            } => {
                format!(
                    "Insufficient {} balance: required {:.2}, available {:.2}",
                    currency, required, available
                )
            }
            DetailedMpcError::PositionLimitExceeded {
                symbol,
                limit,
                current,
                limit_type,
            } => {
                let limit_desc = match limit_type {
                    PositionLimitType::PerSymbol => format!("per symbol {}", symbol),
                    PositionLimitType::Total => "total".to_string(),
                };
                format!(
                    "Position limit exceeded ({}): {} current, {} limit",
                    limit_desc, current, limit
                )
            }
            DetailedMpcError::TraderUnavailable {
                reason,
                available_traders,
            } => {
                format!(
                    "No trader available: {}. Available traders: {:?}",
                    reason, available_traders
                )
            }
            DetailedMpcError::ExchangeConnectionLost {
                exchange,
                last_contact,
                reason,
            } => {
                format!(
                    "Lost connection to {} (last contact: {:?}s ago): {}",
                    exchange,
                    last_contact.as_secs(),
                    reason
                )
            }
            DetailedMpcError::InsufficientCandles {
                symbol,
                required,
                current,
            } => {
                format!(
                    "Insufficient candles for {}: need {}, have {}",
                    symbol, required, current
                )
            }
            DetailedMpcError::LowConfidenceSignal {
                symbol,
                confidence,
                threshold,
            } => {
                format!(
                    "Signal confidence {:.3} below threshold {:.3} for {}",
                    confidence, threshold, symbol
                )
            }
        }
    }
}

#[derive(Debug, Error, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "message")]
pub enum MpcError {
    #[error("Actor not found for exchange: {0:?}")]
    ActorNotFound(Exchange),

    #[error("No response received from actor")]
    NoResponse,

    #[error("Channel send error: {0}")]
    ChannelSendError(String),

    #[error("No prices available for symbol: {symbol}")]
    NoPricesAvailable { symbol: String },

    #[error("Aggregation failed: {0}")]
    AggregationFailed(String),

    #[error("Signal combiner not initialized")]
    SignalCombinerNotInitialized,

    #[error("No signals available")]
    NoSignalsAvailable,

    #[error("Order placement failed: {0}")]
    OrderPlacementFailed(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Timeout waiting for response")]
    Timeout,
}

impl<T> From<mpsc::error::SendError<T>> for MpcError {
    fn from(e: mpsc::error::SendError<T>) -> Self {
        MpcError::ChannelSendError(e.to_string())
    }
}

#[derive(Debug, Error, Clone)]
pub enum ExchangeError {
    #[error("WebSocket connection failed: {0}")]
    WebSocketConnectionFailed(String),

    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    #[error("Failed to parse message: {0}")]
    MessageParseError(String),

    #[error("Order placement failed: {0}")]
    OrderPlacementFailed(String),

    #[error("Authentication not implemented for {exchange}")]
    AuthenticationNotImplemented { exchange: String },

    #[error("Invalid order parameters: {0}")]
    InvalidOrderParameters(String),

    #[error("Connection timeout")]
    ConnectionTimeout,

    #[error("Reconnection limit exceeded")]
    ReconnectionLimitExceeded,
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Internal server error: {0}")]
    InternalServerError(String),

    #[error("Service unavailable")]
    ServiceUnavailable,
}

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Invalid price: {0}")]
    InvalidPrice(String),

    #[error("Invalid quantity: {0}")]
    InvalidQuantity(String),

    #[error("Invalid symbol: {0}")]
    InvalidSymbol(String),

    #[error("Value must be non-negative")]
    MustBeNonNegative,

    #[error("Value must be finite")]
    MustBeFinite,
}

impl From<ValidationError> for String {
    fn from(error: ValidationError) -> Self {
        error.to_string()
    }
}
