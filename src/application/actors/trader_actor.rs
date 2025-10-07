//! Trader Actor
//!
//! This module implements the actor pattern for traders. Each trader runs in its own
//! async task and communicates via message passing. This provides:
//! - Isolated execution context
//! - Concurrent trading operations
//! - Clean separation from exchange actors
//! - Easy scaling (multiple traders per exchange)

use crate::domain::entities::exchange::Exchange;
use crate::domain::entities::order::Order;
use crate::domain::entities::trader::Trader;
use crate::domain::repositories::exchange_client::ExchangeClient;
use crate::domain::services::strategies::TradingSignal;
use crate::domain::value_objects::price::Price;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Messages that can be sent to the trader actor
#[derive(Debug)]
pub enum TraderMessage {
    /// Execute a trading signal
    ExecuteSignal {
        signal: TradingSignal,
        symbol: String,
        price: Price,
        reply: mpsc::Sender<Result<String, String>>,
    },
    /// Place a specific order
    PlaceOrder {
        order: Order,
        reply: mpsc::Sender<Result<String, String>>,
    },
    /// Set active exchange
    SetActiveExchange {
        exchange: Exchange,
        reply: mpsc::Sender<Result<(), String>>,
    },
    /// Get active exchange
    GetActiveExchange {
        reply: mpsc::Sender<Option<Exchange>>,
    },
    /// Check health of all exchanges
    CheckHealth {
        reply: mpsc::Sender<HashMap<Exchange, bool>>,
    },
    /// Get balance from active exchange
    GetBalance {
        currency: Option<String>,
        reply: mpsc::Sender<Result<f64, String>>,
    },
    /// Get trader statistics
    GetStats {
        reply: mpsc::Sender<TraderStats>,
    },
    /// Shutdown the trader
    Shutdown,
}

/// Trader statistics
#[derive(Debug, Clone)]
pub struct TraderStats {
    pub id: String,
    pub active_exchange: Option<Exchange>,
    pub total_orders: u64,
    pub successful_orders: u64,
    pub failed_orders: u64,
    pub available_exchanges: Vec<Exchange>,
}

/// Trader Actor - manages a single trader instance
pub struct TraderActor {
    trader: Trader,
    stats: TraderStats,
}

impl TraderActor {
    /// Spawn a new trader actor
    ///
    /// # Arguments
    /// * `trader` - The trader entity to manage
    ///
    /// # Returns
    /// Message sender to communicate with the actor
    pub fn spawn(trader: Trader) -> mpsc::Sender<TraderMessage> {
        let (tx, rx) = mpsc::channel(100);

        let trader_id = trader.id.clone();
        let stats = TraderStats {
            id: trader_id.clone(),
            active_exchange: trader.get_active_exchange().cloned(),
            total_orders: 0,
            successful_orders: 0,
            failed_orders: 0,
            available_exchanges: trader.get_available_exchanges().iter().map(|e| (*e).clone()).collect(),
        };

        let actor = Self {
            trader,
            stats,
        };

        tokio::spawn(async move {
            actor.run(rx).await;
        });

        info!("TraderActor spawned for trader: {}", trader_id);
        tx
    }

    /// Main actor loop
    async fn run(mut self, mut rx: mpsc::Receiver<TraderMessage>) {
        info!("TraderActor {} started", self.stats.id);

        while let Some(msg) = rx.recv().await {
            match msg {
                TraderMessage::ExecuteSignal {
                    signal,
                    symbol,
                    price,
                    reply,
                } => {
                    debug!(
                        "Trader {} received ExecuteSignal for {} with confidence {:.2}",
                        self.stats.id, symbol, signal.confidence
                    );

                    self.stats.total_orders += 1;
                    let result = self.trader.execute_signal(&signal, &symbol, price).await;

                    match &result {
                        Ok(order_id) => {
                            self.stats.successful_orders += 1;
                            info!(
                                "Trader {} successfully executed signal for {}: order_id={}",
                                self.stats.id, symbol, order_id
                            );
                        }
                        Err(e) => {
                            self.stats.failed_orders += 1;
                            warn!(
                                "Trader {} failed to execute signal for {}: {}",
                                self.stats.id, symbol, e
                            );
                        }
                    }

                    if let Err(e) = reply.send(result).await {
                        error!("Failed to send ExecuteSignal reply: {:?}", e);
                    }
                }

                TraderMessage::PlaceOrder { order, reply } => {
                    debug!(
                        "Trader {} received PlaceOrder for {}",
                        self.stats.id, order.symbol
                    );

                    self.stats.total_orders += 1;
                    let result = self.trader.route_order(&order).await;

                    match &result {
                        Ok(order_id) => {
                            self.stats.successful_orders += 1;
                            info!(
                                "Trader {} successfully placed order for {}: order_id={}",
                                self.stats.id, order.symbol, order_id
                            );
                        }
                        Err(e) => {
                            self.stats.failed_orders += 1;
                            warn!(
                                "Trader {} failed to place order for {}: {}",
                                self.stats.id, order.symbol, e
                            );
                        }
                    }

                    if let Err(e) = reply.send(result).await {
                        error!("Failed to send PlaceOrder reply: {:?}", e);
                    }
                }

                TraderMessage::SetActiveExchange { exchange, reply } => {
                    debug!(
                        "Trader {} setting active exchange to {}",
                        self.stats.id,
                        exchange.name()
                    );

                    let result = self.trader.set_active_exchange(exchange.clone());
                    if result.is_ok() {
                        self.stats.active_exchange = Some(exchange);
                    }

                    if let Err(e) = reply.send(result).await {
                        error!("Failed to send SetActiveExchange reply: {:?}", e);
                    }
                }

                TraderMessage::GetActiveExchange { reply } => {
                    let active = self.trader.get_active_exchange().cloned();
                    if let Err(e) = reply.send(active).await {
                        error!("Failed to send GetActiveExchange reply: {:?}", e);
                    }
                }

                TraderMessage::CheckHealth { reply } => {
                    debug!("Trader {} checking health", self.stats.id);
                    let health = self.trader.check_health().await;
                    if let Err(e) = reply.send(health).await {
                        error!("Failed to send CheckHealth reply: {:?}", e);
                    }
                }

                TraderMessage::GetBalance { currency, reply } => {
                    debug!(
                        "Trader {} getting balance for {:?}",
                        self.stats.id, currency
                    );
                    let result = self
                        .trader
                        .get_balance(currency.as_deref())
                        .await;
                    if let Err(e) = reply.send(result).await {
                        error!("Failed to send GetBalance reply: {:?}", e);
                    }
                }

                TraderMessage::GetStats { reply } => {
                    if let Err(e) = reply.send(self.stats.clone()).await {
                        error!("Failed to send GetStats reply: {:?}", e);
                    }
                }

                TraderMessage::Shutdown => {
                    info!("Trader {} received shutdown signal", self.stats.id);
                    break;
                }
            }
        }

        info!("TraderActor {} stopped", self.stats.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::order::{OrderSide, OrderType};
    use crate::domain::repositories::exchange_client::{Balance, ExchangeError, ExchangeResult, OrderStatus};
    use crate::domain::services::strategies::{FastScalping, Signal};
    use async_trait::async_trait;

    // Mock ExchangeClient for testing
    struct MockExchangeClient {
        name: String,
    }

    #[async_trait]
    impl ExchangeClient for MockExchangeClient {
        fn name(&self) -> &str {
            &self.name
        }

        async fn place_order(&self, _order: &Order) -> ExchangeResult<String> {
            Ok("test_order_id".to_string())
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
            true
        }
    }

    #[tokio::test]
    async fn test_trader_actor_spawn() {
        let strategy = Box::new(FastScalping::new());
        let mut trader = Trader::new("test_trader".to_string(), strategy, 0.01, 0.7).unwrap();

        let mock_client = Arc::new(MockExchangeClient {
            name: "MockExchange".to_string(),
        });
        trader.add_exchange(Exchange::Binance, mock_client);

        let sender = TraderActor::spawn(trader);

        // Test that we can send a message
        let (reply_tx, mut reply_rx) = mpsc::channel(1);
        sender
            .send(TraderMessage::GetActiveExchange { reply: reply_tx })
            .await
            .unwrap();

        let response = reply_rx.recv().await.unwrap();
        assert_eq!(response, Some(Exchange::Binance));

        // Shutdown
        sender.send(TraderMessage::Shutdown).await.unwrap();
    }

    #[tokio::test]
    async fn test_trader_actor_execute_signal() {
        let strategy = Box::new(FastScalping::new());
        let mut trader = Trader::new("test_trader".to_string(), strategy, 0.01, 0.7).unwrap();

        let mock_client = Arc::new(MockExchangeClient {
            name: "MockExchange".to_string(),
        });
        trader.add_exchange(Exchange::Binance, mock_client);

        let sender = TraderActor::spawn(trader);

        let signal = TradingSignal {
            signal: Signal::Buy,
            confidence: 0.8,
        };

        let (reply_tx, mut reply_rx) = mpsc::channel(1);
        sender
            .send(TraderMessage::ExecuteSignal {
                signal,
                symbol: "BTC-USD".to_string(),
                price: Price::new(50000.0).unwrap(),
                reply: reply_tx,
            })
            .await
            .unwrap();

        let response = reply_rx.recv().await.unwrap();
        assert!(response.is_ok());
        assert_eq!(response.unwrap(), "test_order_id");

        // Shutdown
        sender.send(TraderMessage::Shutdown).await.unwrap();
    }

    #[tokio::test]
    async fn test_trader_actor_get_stats() {
        let strategy = Box::new(FastScalping::new());
        let mut trader = Trader::new("test_trader".to_string(), strategy, 0.01, 0.7).unwrap();

        let mock_client = Arc::new(MockExchangeClient {
            name: "MockExchange".to_string(),
        });
        trader.add_exchange(Exchange::Binance, mock_client);

        let sender = TraderActor::spawn(trader);

        // Execute a signal first
        let signal = TradingSignal {
            signal: Signal::Buy,
            confidence: 0.8,
        };

        let (reply_tx, mut reply_rx) = mpsc::channel(1);
        sender
            .send(TraderMessage::ExecuteSignal {
                signal,
                symbol: "BTC-USD".to_string(),
                price: Price::new(50000.0).unwrap(),
                reply: reply_tx,
            })
            .await
            .unwrap();

        let _ = reply_rx.recv().await.unwrap();

        // Get stats
        let (stats_tx, mut stats_rx) = mpsc::channel(1);
        sender
            .send(TraderMessage::GetStats { reply: stats_tx })
            .await
            .unwrap();

        let stats = stats_rx.recv().await.unwrap();
        assert_eq!(stats.id, "test_trader");
        assert_eq!(stats.total_orders, 1);
        assert_eq!(stats.successful_orders, 1);
        assert_eq!(stats.failed_orders, 0);

        // Shutdown
        sender.send(TraderMessage::Shutdown).await.unwrap();
    }

    #[tokio::test]
    async fn test_trader_actor_set_active_exchange() {
        let strategy = Box::new(FastScalping::new());
        let mut trader = Trader::new("test_trader".to_string(), strategy, 0.01, 0.7).unwrap();

        let mock_client1 = Arc::new(MockExchangeClient {
            name: "MockExchange1".to_string(),
        });
        let mock_client2 = Arc::new(MockExchangeClient {
            name: "MockExchange2".to_string(),
        });

        trader.add_exchange(Exchange::Binance, mock_client1);
        trader.add_exchange(Exchange::Coinbase, mock_client2);

        let sender = TraderActor::spawn(trader);

        // Change active exchange
        let (reply_tx, mut reply_rx) = mpsc::channel(1);
        sender
            .send(TraderMessage::SetActiveExchange {
                exchange: Exchange::Coinbase,
                reply: reply_tx,
            })
            .await
            .unwrap();

        let result = reply_rx.recv().await.unwrap();
        assert!(result.is_ok());

        // Verify active exchange changed
        let (check_tx, mut check_rx) = mpsc::channel(1);
        sender
            .send(TraderMessage::GetActiveExchange { reply: check_tx })
            .await
            .unwrap();

        let active = check_rx.recv().await.unwrap();
        assert_eq!(active, Some(Exchange::Coinbase));

        // Shutdown
        sender.send(TraderMessage::Shutdown).await.unwrap();
    }
}
