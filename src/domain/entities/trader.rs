//! Trader Entity
//!
//! This module defines the `Trader` entity, which is responsible for making trading decisions
//! and executing orders through exchange clients. Traders are decoupled from specific exchanges,
//! allowing them to route orders to multiple exchanges dynamically.
//!
//! ## Key Features
//! - Strategy-based trading logic
//! - Multi-exchange support
//! - Smart order routing
//! - Position tracking
//! - Risk management

use crate::domain::entities::exchange::Exchange;
use crate::domain::entities::order::{Order, OrderSide, OrderType};
use crate::domain::repositories::exchange_client::ExchangeClient;
use crate::domain::services::strategies::{Signal, Strategy, TradingSignal};
use crate::domain::value_objects::price::Price;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

/// Trader entity responsible for trading decisions and execution
pub struct Trader {
    /// Unique identifier for the trader
    pub id: String,
    /// Trading strategy used by this trader
    pub strategy: Box<dyn Strategy + Send + Sync>,
    /// Map of exchange clients available to this trader
    exchange_clients: HashMap<Exchange, Arc<dyn ExchangeClient>>,
    /// Current active exchange for trading
    active_exchange: Option<Exchange>,
    /// Maximum position size allowed
    pub max_position_size: f64,
    /// Minimum confidence threshold for signal execution
    pub min_confidence: f64,
}

impl Trader {
    /// Create a new trader
    ///
    /// # Arguments
    /// * `id` - Unique identifier for the trader (alphanumeric, underscore, hyphen only)
    /// * `strategy` - Trading strategy to use
    /// * `max_position_size` - Maximum position size allowed
    /// * `min_confidence` - Minimum confidence threshold (0.0 to 1.0)
    ///
    /// # Errors
    /// Returns an error if:
    /// - ID is empty or longer than 100 characters
    /// - ID contains invalid characters (only alphanumeric, _, - allowed)
    /// - max_position_size is not positive
    /// - min_confidence is not in range [0.0, 1.0]
    pub fn new(
        id: String,
        strategy: Box<dyn Strategy + Send + Sync>,
        max_position_size: f64,
        min_confidence: f64,
    ) -> Result<Self, String> {
        // Validate ID
        let trimmed_id = id.trim();
        if trimmed_id.is_empty() {
            return Err("Trader ID cannot be empty".to_string());
        }

        if trimmed_id.len() > 100 {
            return Err("Trader ID too long (max 100 characters)".to_string());
        }

        if !trimmed_id.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(
                "Trader ID must contain only alphanumeric characters, underscores, or hyphens"
                    .to_string(),
            );
        }

        // Validate position size
        if max_position_size <= 0.0 {
            return Err("Max position size must be positive".to_string());
        }

        if !max_position_size.is_finite() {
            return Err("Max position size must be finite".to_string());
        }

        // Validate confidence
        if !(0.0..=1.0).contains(&min_confidence) {
            return Err("Min confidence must be between 0.0 and 1.0".to_string());
        }

        Ok(Self {
            id: trimmed_id.to_string(),
            strategy,
            exchange_clients: HashMap::new(),
            active_exchange: None,
            max_position_size,
            min_confidence,
        })
    }

    /// Add an exchange client to this trader
    ///
    /// # Arguments
    /// * `exchange` - Exchange identifier
    /// * `client` - Exchange client implementation
    pub fn add_exchange(&mut self, exchange: Exchange, client: Arc<dyn ExchangeClient>) {
        info!("Adding exchange {} to trader {}", exchange.name(), self.id);

        // Set as active if this is the first exchange
        if self.active_exchange.is_none() {
            self.active_exchange = Some(exchange.clone());
        }

        self.exchange_clients.insert(exchange, client);
    }

    /// Remove an exchange client from this trader
    pub fn remove_exchange(&mut self, exchange: &Exchange) {
        self.exchange_clients.remove(exchange);

        // If we removed the active exchange, select another one
        if self.active_exchange.as_ref() == Some(exchange) {
            self.active_exchange = self.exchange_clients.keys().next().cloned();
        }
    }

    /// Set the active exchange for trading
    pub fn set_active_exchange(&mut self, exchange: Exchange) -> Result<(), String> {
        if !self.exchange_clients.contains_key(&exchange) {
            return Err(format!("Exchange {} not available to this trader", exchange.name()));
        }

        self.active_exchange = Some(exchange);
        Ok(())
    }

    /// Get the currently active exchange
    pub fn get_active_exchange(&self) -> Option<&Exchange> {
        self.active_exchange.as_ref()
    }

    /// Get list of all available exchanges
    pub fn get_available_exchanges(&self) -> Vec<&Exchange> {
        self.exchange_clients.keys().collect()
    }

    /// Execute a trading signal
    ///
    /// # Arguments
    /// * `signal` - Trading signal to execute
    /// * `symbol` - Trading symbol
    /// * `price` - Current market price
    ///
    /// # Returns
    /// Order ID if successful
    pub async fn execute_signal(
        &self,
        signal: &TradingSignal,
        symbol: &str,
        _price: Price,
    ) -> Result<Option<String>, String> {
        // Check confidence threshold
        if signal.confidence < self.min_confidence {
            return Err(format!(
                "Signal confidence {:.2} below minimum {:.2}",
                signal.confidence, self.min_confidence
            ));
        }

        // Check if we have an active exchange
        let exchange = self.active_exchange.as_ref()
            .ok_or("No active exchange set for trader")?;

        let client = self.exchange_clients.get(exchange)
            .ok_or_else(|| format!("Exchange client not found for {:?}", exchange))?;

        // Determine order side and quantity based on signal
        let (side, quantity) = match signal.signal {
            Signal::Buy => (OrderSide::Buy, self.max_position_size),
            Signal::Sell => (OrderSide::Sell, self.max_position_size),
            Signal::Hold => {
                info!(
                    "Trader {} skipping HOLD signal for {} (confidence {:.2})",
                    self.id, symbol, signal.confidence
                );
                return Ok(None);
            }
        };

        // Create order with safe timestamp generation
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("System time error: {}", e))?
            .as_millis();
        let order_id = format!("trader_{}_{}", self.id, timestamp);
        let order = Order::new(
            order_id,
            symbol.to_string(),
            side,
            OrderType::Market,
            None, // Market order has no limit price
            quantity,
        )?;

        info!(
            "Trader {} executing {:?} order on {} for {} with confidence {:.2}",
            self.id, signal.signal, exchange.name(), symbol, signal.confidence
        );

        // Execute order through exchange client
        client.place_order(&order).await.map(|id| Some(id)).map_err(|e| {
            warn!(
                "Trader {} failed to place order on {}: {}",
                self.id, exchange.name(), e
            );
            format!("Failed to place order: {}", e)
        })
    }

    /// Route order to the best exchange based on criteria
    ///
    /// This is a placeholder for future smart order routing logic
    /// Currently just uses the active exchange
    pub async fn route_order(&self, order: &Order) -> Result<String, String> {
        let exchange = self.active_exchange.as_ref()
            .ok_or("No active exchange set for trader")?;

        let client = self.exchange_clients.get(exchange)
            .ok_or_else(|| format!("Exchange client not found for {:?}", exchange))?;

        info!(
            "Trader {} routing order to {}",
            self.id, exchange.name()
        );

        client.place_order(order).await.map_err(|e| {
            format!("Failed to route order: {}", e)
        })
    }

    /// Check if all exchange clients are healthy
    pub async fn check_health(&self) -> HashMap<Exchange, bool> {
        let mut health_status = HashMap::new();

        for (exchange, client) in &self.exchange_clients {
            let is_healthy = client.is_healthy().await;
            health_status.insert(exchange.clone(), is_healthy);
        }

        health_status
    }

    /// Get balance from the active exchange
    pub async fn get_balance(&self, currency: Option<&str>) -> Result<f64, String> {
        let exchange = self.active_exchange.as_ref()
            .ok_or("No active exchange set for trader")?;

        let client = self.exchange_clients.get(exchange)
            .ok_or_else(|| format!("Exchange client not found for {:?}", exchange))?;

        let balances = client.get_balance(currency).await
            .map_err(|e| format!("Failed to get balance: {}", e))?;

        // Sum up available balance for requested currency
        let total_available = balances.iter()
            .map(|b| b.available)
            .sum();

        Ok(total_available)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repositories::exchange_client::{Balance, ExchangeError, ExchangeResult, OrderStatus};
    use async_trait::async_trait;
    use crate::domain::services::strategies::FastScalping;

    // Mock ExchangeClient for testing
    struct MockExchangeClient {
        name: String,
        should_fail: bool,
    }

    #[async_trait]
    impl ExchangeClient for MockExchangeClient {
        fn name(&self) -> &str {
            &self.name
        }

        async fn place_order(&self, _order: &Order) -> ExchangeResult<String> {
            if self.should_fail {
                Err(ExchangeError::OrderPlacementFailed("Mock error".to_string()))
            } else {
                Ok("mock_order_id".to_string())
            }
        }

        async fn cancel_order(&self, _order_id: &str) -> ExchangeResult<()> {
            Ok(())
        }

        async fn get_order_status(&self, _order_id: &str) -> ExchangeResult<OrderStatus> {
            Ok(OrderStatus::Filled)
        }

        async fn get_balance(&self, _currency: Option<&str>) -> ExchangeResult<Vec<Balance>> {
            Ok(vec![Balance {
                currency: "USD".to_string(),
                available: 10000.0,
                total: 10000.0,
            }])
        }

        async fn is_healthy(&self) -> bool {
            !self.should_fail
        }
    }

    #[test]
    fn test_trader_new() {
        let strategy = Box::new(FastScalping::new());
        let trader = Trader::new("trader1".to_string(), strategy, 100.0, 0.7);

        assert!(trader.is_ok());
        let trader = trader.unwrap();
        assert_eq!(trader.id, "trader1");
        assert_eq!(trader.max_position_size, 100.0);
        assert_eq!(trader.min_confidence, 0.7);
    }

    #[test]
    fn test_trader_new_invalid_params() {
        let strategy = Box::new(FastScalping::new());

        // Invalid position size
        let result = Trader::new("trader1".to_string(), strategy, -100.0, 0.7);
        assert!(result.is_err());

        // Invalid confidence
        let strategy = Box::new(FastScalping::new());
        let result = Trader::new("trader1".to_string(), strategy, 100.0, 1.5);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_exchange() {
        let strategy = Box::new(FastScalping::new());
        let mut trader = Trader::new("trader1".to_string(), strategy, 100.0, 0.7).unwrap();

        let mock_client = Arc::new(MockExchangeClient {
            name: "MockExchange".to_string(),
            should_fail: false,
        });

        trader.add_exchange(Exchange::Binance, mock_client);
        assert_eq!(trader.get_available_exchanges().len(), 1);
        assert_eq!(trader.get_active_exchange(), Some(&Exchange::Binance));
    }

    #[tokio::test]
    async fn test_execute_signal() {
        let strategy = Box::new(FastScalping::new());
        let mut trader = Trader::new("trader1".to_string(), strategy, 0.01, 0.7).unwrap();

        let mock_client = Arc::new(MockExchangeClient {
            name: "MockExchange".to_string(),
            should_fail: false,
        });

        trader.add_exchange(Exchange::Binance, mock_client);

        let signal = TradingSignal {
            signal: Signal::Buy,
            confidence: 0.8,
        };

        let price = Price::new(50000.0).unwrap();
        let result = trader.execute_signal(&signal, "BTC-USD", price).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("mock_order_id".to_string()));
    }

    #[tokio::test]
    async fn test_execute_signal_low_confidence() {
        let strategy = Box::new(FastScalping::new());
        let mut trader = Trader::new("trader1".to_string(), strategy, 0.01, 0.7).unwrap();

        let mock_client = Arc::new(MockExchangeClient {
            name: "MockExchange".to_string(),
            should_fail: false,
        });

        trader.add_exchange(Exchange::Binance, mock_client);

        let signal = TradingSignal {
            signal: Signal::Buy,
            confidence: 0.5, // Below threshold
        };

        let price = Price::new(50000.0).unwrap();
        let result = trader.execute_signal(&signal, "BTC-USD", price).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("confidence"));
    }

    #[tokio::test]
    async fn test_execute_signal_hold() {
        let strategy = Box::new(FastScalping::new());
        let mut trader = Trader::new("trader1".to_string(), strategy, 0.01, 0.7).unwrap();

        let mock_client = Arc::new(MockExchangeClient {
            name: "MockExchange".to_string(),
            should_fail: false,
        });

        trader.add_exchange(Exchange::Binance, mock_client);

        let signal = TradingSignal {
            signal: Signal::Hold,
            confidence: 0.9,
        };

        let price = Price::new(50000.0).unwrap();
        let result = trader.execute_signal(&signal, "BTC-USD", price).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_balance() {
        let strategy = Box::new(FastScalping::new());
        let mut trader = Trader::new("trader1".to_string(), strategy, 0.01, 0.7).unwrap();

        let mock_client = Arc::new(MockExchangeClient {
            name: "MockExchange".to_string(),
            should_fail: false,
        });

        trader.add_exchange(Exchange::Binance, mock_client);

        let balance = trader.get_balance(Some("USD")).await;
        assert!(balance.is_ok());
        assert_eq!(balance.unwrap(), 10000.0);
    }

    #[tokio::test]
    async fn test_check_health() {
        let strategy = Box::new(FastScalping::new());
        let mut trader = Trader::new("trader1".to_string(), strategy, 0.01, 0.7).unwrap();

        let mock_client = Arc::new(MockExchangeClient {
            name: "MockExchange".to_string(),
            should_fail: false,
        });

        trader.add_exchange(Exchange::Binance, mock_client);

        let health = trader.check_health().await;
        assert_eq!(health.len(), 1);
        assert_eq!(health.get(&Exchange::Binance), Some(&true));
    }
}
