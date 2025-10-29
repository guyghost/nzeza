//! Exchange Client Trait
//!
//! This module defines the `ExchangeClient` trait, which provides a common interface
//! for all exchange implementations. This abstraction allows traders to operate
//! independently of specific exchange implementations.
//!
//! ## Benefits
//! - Decouples trading logic from exchange-specific code
//! - Enables easy mocking for testing
//! - Allows traders to work with multiple exchanges
//! - Simplifies adding new exchange support

use crate::domain::entities::order::Order;
use async_trait::async_trait;

/// Common result type for exchange operations
pub type ExchangeResult<T> = Result<T, ExchangeError>;

/// Errors that can occur during exchange operations
#[derive(Debug, Clone)]
pub enum ExchangeError {
    /// Order placement failed
    OrderPlacementFailed(String),
    /// Order cancellation failed
    OrderCancellationFailed(String),
    /// Order status query failed
    OrderStatusFailed(String),
    /// Balance query failed
    BalanceQueryFailed(String),
    /// Authentication error
    AuthenticationError(String),
    /// Network error
    NetworkError(String),
    /// Invalid order parameters
    InvalidOrder(String),
    /// Exchange-specific error
    ExchangeSpecific(String),
}

impl std::fmt::Display for ExchangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExchangeError::OrderPlacementFailed(msg) => {
                write!(f, "Order placement failed: {}", msg)
            }
            ExchangeError::OrderCancellationFailed(msg) => {
                write!(f, "Order cancellation failed: {}", msg)
            }
            ExchangeError::OrderStatusFailed(msg) => {
                write!(f, "Order status query failed: {}", msg)
            }
            ExchangeError::BalanceQueryFailed(msg) => write!(f, "Balance query failed: {}", msg),
            ExchangeError::AuthenticationError(msg) => write!(f, "Authentication error: {}", msg),
            ExchangeError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            ExchangeError::InvalidOrder(msg) => write!(f, "Invalid order: {}", msg),
            ExchangeError::ExchangeSpecific(msg) => write!(f, "Exchange error: {}", msg),
        }
    }
}

impl std::error::Error for ExchangeError {}

/// Account balance information
#[derive(Debug, Clone)]
pub struct Balance {
    pub currency: String,
    pub available: f64,
    pub total: f64,
}

/// Order status from exchange
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderStatus {
    /// Order is pending execution
    Pending,
    /// Order is partially filled
    PartiallyFilled,
    /// Order is fully filled
    Filled,
    /// Order was cancelled
    Cancelled,
    /// Order was rejected
    Rejected,
    /// Order expired
    Expired,
    /// Status unknown or not found
    Unknown,
}

impl std::fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderStatus::Pending => write!(f, "PENDING"),
            OrderStatus::PartiallyFilled => write!(f, "PARTIALLY_FILLED"),
            OrderStatus::Filled => write!(f, "FILLED"),
            OrderStatus::Cancelled => write!(f, "CANCELLED"),
            OrderStatus::Rejected => write!(f, "REJECTED"),
            OrderStatus::Expired => write!(f, "EXPIRED"),
            OrderStatus::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// Exchange client trait providing common interface for all exchanges
#[async_trait]
pub trait ExchangeClient: Send + Sync {
    /// Get the name of this exchange
    fn name(&self) -> &str;

    /// Place an order on the exchange
    ///
    /// # Arguments
    /// * `order` - The order to place
    ///
    /// # Returns
    /// The exchange-assigned order ID
    async fn place_order(&self, order: &Order) -> ExchangeResult<String>;

    /// Cancel an order on the exchange
    ///
    /// # Arguments
    /// * `order_id` - The exchange-assigned order ID
    async fn cancel_order(&self, order_id: &str) -> ExchangeResult<()>;

    /// Get the status of an order
    ///
    /// # Arguments
    /// * `order_id` - The exchange-assigned order ID
    ///
    /// # Returns
    /// The current status of the order
    async fn get_order_status(&self, order_id: &str) -> ExchangeResult<OrderStatus>;

    /// Get account balance
    ///
    /// # Arguments
    /// * `currency` - Optional currency filter (None returns all balances)
    ///
    /// # Returns
    /// List of balances
    async fn get_balance(&self, currency: Option<&str>) -> ExchangeResult<Vec<Balance>>;

    /// Check if the exchange client is healthy and connected
    async fn is_healthy(&self) -> bool {
        // Default implementation - can be overridden
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_status_display() {
        assert_eq!(OrderStatus::Pending.to_string(), "PENDING");
        assert_eq!(OrderStatus::Filled.to_string(), "FILLED");
        assert_eq!(OrderStatus::Cancelled.to_string(), "CANCELLED");
    }

    #[test]
    fn test_exchange_error_display() {
        let error = ExchangeError::OrderPlacementFailed("Test error".to_string());
        assert_eq!(error.to_string(), "Order placement failed: Test error");
    }

    #[test]
    fn test_balance_creation() {
        let balance = Balance {
            currency: "USD".to_string(),
            available: 1000.0,
            total: 1500.0,
        };
        assert_eq!(balance.currency, "USD");
        assert_eq!(balance.available, 1000.0);
        assert_eq!(balance.total, 1500.0);
    }
}
