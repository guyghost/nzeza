use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::mpsc;

use crate::domain::entities::exchange::Exchange;

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
