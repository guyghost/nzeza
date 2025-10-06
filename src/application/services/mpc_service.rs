use crate::domain::entities::exchange::Exchange;
use crate::infrastructure::adapters::exchange_actor::ExchangeMessage;
use crate::domain::services::strategies::{SignalCombiner, TradingSignal};
use crate::domain::services::candle_builder::CandleBuilder;
use crate::domain::value_objects::price::Price;
use crate::domain::entities::order::Order;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, info, warn, error};

pub struct MpcService {
    pub senders: HashMap<Exchange, mpsc::Sender<ExchangeMessage>>,
    pub signal_combiner: Option<SignalCombiner>,
    pub candle_builder: Arc<Mutex<CandleBuilder>>,
    pub last_signals: Arc<Mutex<HashMap<String, TradingSignal>>>,
}

impl MpcService {
    pub fn new() -> Self {
        // 1 minute candles, keep 100 candles in history
        let candle_builder = Arc::new(Mutex::new(
            CandleBuilder::new(Duration::from_secs(60), 100)
        ));

        Self {
            senders: HashMap::new(),
            signal_combiner: None,
            candle_builder,
            last_signals: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn get_exchange_name(exchange: &Exchange) -> &'static str {
        match exchange {
            Exchange::Binance => "Binance",
            Exchange::Dydx => "dYdX",
            Exchange::Hyperliquid => "Hyperliquid",
            Exchange::Coinbase => "Coinbase",
            Exchange::Kraken => "Kraken",
        }
    }

    pub fn add_actor(&mut self, exchange: Exchange, sender: mpsc::Sender<ExchangeMessage>) {
        self.senders.insert(exchange, sender);
    }

    pub fn set_signal_combiner(&mut self, combiner: SignalCombiner) {
        self.signal_combiner = Some(combiner);
    }

    /// Check health of a specific actor
    pub async fn check_actor_health(&self, exchange: &Exchange) -> Result<bool, String> {
        if let Some(sender) = self.senders.get(exchange) {
            let (reply_tx, mut reply_rx) = mpsc::channel(1);

            sender.send(ExchangeMessage::HealthCheck { reply: reply_tx })
                .await
                .map_err(|e| format!("Failed to send health check: {}", e))?;

            reply_rx.recv().await
                .ok_or_else(|| "No health check response".to_string())
        } else {
            Err(format!("No actor found for exchange: {:?}", exchange))
        }
    }

    /// Check health of all actors
    pub async fn check_all_actors_health(&self) -> HashMap<Exchange, bool> {
        use tracing::info;
        let mut health_status = HashMap::new();

        for exchange in self.senders.keys() {
            match self.check_actor_health(exchange).await {
                Ok(is_healthy) => {
                    health_status.insert(exchange.clone(), is_healthy);
                    if !is_healthy {
                        use tracing::warn;
                        warn!("Actor {:?} is unhealthy", exchange);
                    }
                }
                Err(e) => {
                    use tracing::error;
                    error!("Failed to check health of {:?}: {}", exchange, e);
                    health_status.insert(exchange.clone(), false);
                }
            }
        }

        info!("Health check complete: {:?}", health_status);
        health_status
    }

    /// Shutdown all actors gracefully
    pub async fn shutdown(&self) {
        use tracing::info;

        info!("Shutting down all exchange actors...");

        for (exchange, sender) in &self.senders {
            info!("Sending shutdown signal to {:?}", exchange);
            if let Err(e) = sender.send(ExchangeMessage::Shutdown).await {
                use tracing::error;
                error!("Failed to send shutdown to {:?}: {}", exchange, e);
            }
        }

        // Give actors time to shutdown gracefully
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        info!("All actors shutdown complete");
    }

    #[allow(dead_code)]
    pub async fn get_price(&self, exchange: &Exchange, symbol: &str) -> Result<Price, String> {
        if let Some(sender) = self.senders.get(exchange) {
            let (reply_tx, mut reply_rx) = mpsc::channel(1);
            let msg = ExchangeMessage::GetPrice {
                symbol: symbol.to_string(),
                reply: reply_tx,
            };
            sender.send(msg).await.map_err(|e| e.to_string())?;
            reply_rx.recv().await.ok_or("No response from actor".to_string())?
        } else {
            Err(format!("No actor for {:?}", exchange))
        }
    }

    #[allow(dead_code)]
    pub async fn subscribe(&self, exchange: &Exchange, symbol: &str) -> Result<(), String> {
        if let Some(sender) = self.senders.get(exchange) {
            let (reply_tx, mut reply_rx) = mpsc::channel(1);
            let msg = ExchangeMessage::Subscribe {
                symbol: symbol.to_string(),
                reply: reply_tx,
            };
            sender.send(msg).await.map_err(|e| e.to_string())?;
            reply_rx.recv().await.ok_or("No response from actor".to_string())?
        } else {
            Err(format!("No actor for {:?}", exchange))
        }
    }

    #[allow(dead_code)]
    pub async fn unsubscribe(&self, exchange: &Exchange, symbol: &str) -> Result<(), String> {
        if let Some(sender) = self.senders.get(exchange) {
            let (reply_tx, mut reply_rx) = mpsc::channel(1);
            let msg = ExchangeMessage::Unsubscribe {
                symbol: symbol.to_string(),
                reply: reply_tx,
            };
            sender.send(msg).await.map_err(|e| e.to_string())?;
            reply_rx.recv().await.ok_or("No response from actor".to_string())?
        } else {
            Err(format!("No actor for {:?}", exchange))
        }
    }

    #[allow(dead_code)]
    pub async fn get_subscriptions(&self, exchange: &Exchange) -> Result<Vec<String>, String> {
        if let Some(sender) = self.senders.get(exchange) {
            let (reply_tx, mut reply_rx) = mpsc::channel(1);
            let msg = ExchangeMessage::GetSubscriptions { reply: reply_tx };
            sender.send(msg).await.map_err(|e| e.to_string())?;
            reply_rx.recv().await.ok_or("No response from actor".to_string())
        } else {
            Err(format!("No actor for {:?}", exchange))
        }
    }

    // Pure method: aggregate prices from multiple exchanges
    #[allow(dead_code)]
    pub fn aggregate_prices(prices: Vec<Price>) -> Result<Price, String> {
        if prices.is_empty() {
            return Err("Cannot aggregate empty price list".to_string());
        }
        let sum: f64 = prices.iter().map(|p| p.value()).sum();
        let avg = sum / prices.len() as f64;
        Price::new(avg)
    }

    // Generate trading signal using combined strategies
    #[allow(dead_code)]
    pub fn generate_trading_signal(&self, candles: &[crate::domain::services::indicators::Candle]) -> Option<TradingSignal> {
        self.signal_combiner.as_ref()?.combine_signals(candles)
    }

    /// Get aggregated price for a symbol across all exchanges (using normalized symbols)
    #[allow(dead_code)]
    pub async fn get_aggregated_price(&self, symbol: &str) -> Result<Price, String> {
        use crate::config::TradingConfig;
        let normalized_symbol = TradingConfig::normalize_symbol(symbol);
        
        let mut prices = Vec::new();

        for (exchange, sender) in &self.senders {
            // Get subscriptions for this exchange
            let (sub_tx, mut sub_rx) = mpsc::channel(1);
            sender.send(ExchangeMessage::GetSubscriptions { reply: sub_tx })
                .await
                .map_err(|e| format!("Failed to get subscriptions: {}", e))?;

            if let Some(subs) = sub_rx.recv().await {
                // Check if any of the subscribed symbols normalize to our target symbol
                for sub_symbol in &subs {
                    let normalized_sub = TradingConfig::normalize_symbol(sub_symbol);
                    if normalized_sub == normalized_symbol {
                        // Get price for this symbol on this exchange
                        let (price_tx, mut price_rx) = mpsc::channel(1);
                        sender.send(ExchangeMessage::GetPrice {
                            symbol: sub_symbol.to_string(),
                            reply: price_tx,
                        })
                        .await
                        .map_err(|e| format!("Failed to request price: {}", e))?;

                    if let Some(Ok(price)) = price_rx.recv().await {
                        debug!("Prix obtenu {:.2} depuis {} pour {} (normalisé: {})", price.value(), Self::get_exchange_name(exchange), sub_symbol, normalized_symbol);
                        prices.push(price);
                    } else {
                        debug!("Aucun prix disponible depuis {} pour {}", Self::get_exchange_name(exchange), sub_symbol);
                    }
                        break; // Found a matching symbol, no need to check others
                    }
                }
            }
        }

        if prices.is_empty() {
            return Err(format!("No prices available for symbol: {} (normalized: {})", symbol, normalized_symbol));
        }

        let aggregated = Self::aggregate_prices(prices.clone())?;
        debug!("Prix agrégé calculé {:.2} pour {} (basé sur {} échanges)", aggregated.value(), normalized_symbol, prices.len());
        Ok(aggregated)
    }

    /// Get all tracked symbols across all exchanges
    #[allow(dead_code)]
    pub async fn get_all_symbols(&self) -> Vec<String> {
        let mut all_symbols = std::collections::HashSet::new();

        for sender in self.senders.values() {
            let (reply_tx, mut reply_rx) = mpsc::channel(1);
            if sender.send(ExchangeMessage::GetSubscriptions { reply: reply_tx }).await.is_ok() {
                if let Some(symbols) = reply_rx.recv().await {
                    for symbol in symbols {
                        all_symbols.insert(symbol);
                    }
                }
            }
        }

        all_symbols.into_iter().collect()
    }

    /// Update candle builder with new price
    #[allow(dead_code)]
    pub async fn update_candle(&self, symbol: String, price: Price) {
        let mut builder = self.candle_builder.lock().await;
        builder.add_price(symbol.clone(), price);
        debug!("Bougie mise à jour pour {} avec prix {:.2}", symbol, price.value());
    }

    /// Get candles for a symbol
    #[allow(dead_code)]
    pub async fn get_candles(&self, symbol: &str) -> Vec<crate::domain::services::indicators::Candle> {
        let builder = self.candle_builder.lock().await;
        builder.get_candles(symbol)
    }

    /// Generate trading signal for a specific symbol
    #[allow(dead_code)]
    pub async fn generate_signal_for_symbol(&self, symbol: &str) -> Option<TradingSignal> {
        let candles = self.get_candles(symbol).await;
        if candles.len() >= 10 { // Need minimum candles for signal generation
            self.generate_trading_signal(&candles)
        } else {
            None
        }
    }

    /// Place an order on a specific exchange
    #[allow(dead_code)]
    pub async fn place_order(&self, exchange: &Exchange, order: Order) -> Result<String, String> {
        if let Some(sender) = self.senders.get(exchange) {
            let (reply_tx, mut reply_rx) = mpsc::channel(1);
            let msg = ExchangeMessage::PlaceOrder {
                order,
                reply: reply_tx,
            };
            sender.send(msg).await.map_err(|e| e.to_string())?;
            reply_rx.recv().await.ok_or("No response from actor".to_string())?
        } else {
            Err(format!("No actor for {:?}", exchange))
        }
    }

    /// Cancel an order on a specific exchange
    #[allow(dead_code)]
    pub async fn cancel_order(&self, exchange: &Exchange, order_id: &str) -> Result<(), String> {
        if let Some(sender) = self.senders.get(exchange) {
            let (reply_tx, mut reply_rx) = mpsc::channel(1);
            let msg = ExchangeMessage::CancelOrder {
                order_id: order_id.to_string(),
                reply: reply_tx,
            };
            sender.send(msg).await.map_err(|e| e.to_string())?;
            reply_rx.recv().await.ok_or("No response from actor".to_string())?
        } else {
            Err(format!("No actor for {:?}", exchange))
        }
    }

    /// Get order status from a specific exchange
    #[allow(dead_code)]
    pub async fn get_order_status(&self, exchange: &Exchange, order_id: &str) -> Result<String, String> {
        if let Some(sender) = self.senders.get(exchange) {
            let (reply_tx, mut reply_rx) = mpsc::channel(1);
            let msg = ExchangeMessage::GetOrderStatus {
                order_id: order_id.to_string(),
                reply: reply_tx,
            };
            sender.send(msg).await.map_err(|e| e.to_string())?;
            reply_rx.recv().await.ok_or("No response from actor".to_string())?
        } else {
            Err(format!("No actor for {:?}", exchange))
        }
    }

    /// Get last signal for a symbol
    #[allow(dead_code)]
    pub async fn get_last_signal(&self, symbol: &str) -> Option<TradingSignal> {
        let last_signals = self.last_signals.lock().await;
        last_signals.get(symbol).cloned()
    }

    /// Get all last signals
    #[allow(dead_code)]
    pub async fn get_all_last_signals(&self) -> HashMap<String, TradingSignal> {
        let last_signals = self.last_signals.lock().await;
        last_signals.clone()
    }

    /// Execute order based on signal
    #[allow(dead_code)]
    pub async fn execute_order_from_signal(&self, symbol: &str, signal: &TradingSignal) -> Result<String, String> {
        use crate::domain::entities::order::{Order, OrderType, OrderSide};
        use std::time::{SystemTime, UNIX_EPOCH};

        // Get current price (for logging purposes)
        let _current_price = self.get_aggregated_price(symbol).await?;
        
        // Determine order side and quantity based on signal
        let (order_side, quantity) = match signal.signal {
            crate::domain::services::strategies::Signal::Buy => {
                (OrderSide::Buy, 0.001) // Small test quantity
            },
            crate::domain::services::strategies::Signal::Sell => {
                (OrderSide::Sell, 0.001) // Small test quantity
            },
            crate::domain::services::strategies::Signal::Hold => {
                return Ok("No order executed - signal is HOLD".to_string());
            }
        };

        // Only execute if confidence is high enough
        if signal.confidence < 0.7 {
            return Ok(format!("Signal confidence {:.2} too low for execution (minimum 0.7)", signal.confidence));
        }

        // Generate unique order ID
        let order_id = format!("order_{}_{}", 
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis(),
            symbol
        );

        // Create the order
        let order = Order::new(
            order_id,
            symbol.to_string(),
            order_side.clone(),
            OrderType::Market,
            None, // Market order has no price
            quantity,
        ).map_err(|e| format!("Failed to create order: {}", e))?;

        // Place order on dYdX (for now, only dYdX is implemented)
        let order_result = self.place_order(&Exchange::Dydx, order).await;

        match order_result {
            Ok(order_id) => {
                info!("🚀 ORDER EXECUTED on dYdX: {:?} {} {} (confidence: {:.2}) - Order ID: {}",
                      order_side, quantity, symbol, signal.confidence, order_id);
                Ok(format!("Order executed on dYdX: {:?} {} {} - Order ID: {}", 
                          order_side, quantity, symbol, order_id))
            },
            Err(e) => {
                error!("Failed to execute order on dYdX: {}", e);
                Err(format!("Failed to execute order: {}", e))
            }
        }
    }

    /// Check and execute orders for all symbols with signals
    #[allow(dead_code)]
    pub async fn check_and_execute_orders(&self) -> Vec<Result<String, String>> {
        let mut results = Vec::new();
        let last_signals = self.get_all_last_signals().await;

        for (symbol, signal) in last_signals {
            match self.execute_order_from_signal(&symbol, &signal).await {
                Ok(msg) => {
                    results.push(Ok(msg));
                },
                Err(e) => {
                    warn!("Failed to execute order for {}: {}", symbol, e);
                    results.push(Err(format!("{}: {}", symbol, e)));
                }
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::services::strategies::Strategy;
    use super::*;

    #[test]
    fn test_mpc_service_new() {
        let service = MpcService::new();
        assert!(service.senders.is_empty());
    }

    #[test]
    fn test_aggregate_prices_single() {
        let prices = vec![Price::new(100.0).unwrap()];
        let avg = MpcService::aggregate_prices(prices).unwrap();
        assert_eq!(avg.value(), 100.0);
    }

    #[test]
    fn test_generate_trading_signal() {
        use crate::domain::services::strategies::{FastScalping, MomentumScalping};
        use crate::domain::services::indicators::Candle;

        let mut service = MpcService::new();
        let strategies: Vec<Box<dyn Strategy + Send + Sync>> = vec![
            Box::new(FastScalping::new()),
            Box::new(MomentumScalping::new()),
        ];
        let weights = vec![0.5, 0.5];
        let combiner = SignalCombiner::new(strategies, weights)
            .expect("Failed to create signal combiner");
        service.set_signal_combiner(combiner);

        let candles = vec![
            Candle::new(100.0, 105.0, 95.0, 102.0, 1000.0).unwrap(),
            Candle::new(102.0, 108.0, 98.0, 105.0, 1100.0).unwrap(),
            Candle::new(105.0, 110.0, 100.0, 108.0, 1200.0).unwrap(),
            Candle::new(108.0, 112.0, 103.0, 106.0, 1300.0).unwrap(),
            Candle::new(106.0, 111.0, 102.0, 109.0, 1400.0).unwrap(),
            Candle::new(109.0, 115.0, 105.0, 112.0, 1500.0).unwrap(),
            Candle::new(112.0, 118.0, 108.0, 115.0, 1600.0).unwrap(),
            Candle::new(115.0, 120.0, 110.0, 117.0, 1700.0).unwrap(),
            Candle::new(117.0, 122.0, 112.0, 119.0, 1800.0).unwrap(),
            Candle::new(119.0, 125.0, 115.0, 122.0, 1900.0).unwrap(),
            Candle::new(122.0, 128.0, 118.0, 125.0, 2000.0).unwrap(),
            Candle::new(125.0, 130.0, 120.0, 127.0, 2100.0).unwrap(),
            Candle::new(127.0, 132.0, 122.0, 129.0, 2200.0).unwrap(),
            Candle::new(129.0, 135.0, 125.0, 132.0, 2300.0).unwrap(),
            Candle::new(132.0, 138.0, 128.0, 135.0, 2400.0).unwrap(),
            Candle::new(135.0, 140.0, 130.0, 137.0, 2500.0).unwrap(),
            Candle::new(137.0, 142.0, 132.0, 139.0, 2600.0).unwrap(),
            Candle::new(139.0, 144.0, 134.0, 141.0, 2700.0).unwrap(),
            Candle::new(141.0, 146.0, 136.0, 143.0, 2800.0).unwrap(),
            Candle::new(143.0, 148.0, 138.0, 145.0, 2900.0).unwrap(),
            Candle::new(145.0, 150.0, 140.0, 147.0, 3000.0).unwrap(),
            Candle::new(147.0, 152.0, 142.0, 149.0, 3100.0).unwrap(),
            Candle::new(149.0, 154.0, 144.0, 151.0, 3200.0).unwrap(),
            Candle::new(151.0, 156.0, 146.0, 153.0, 3300.0).unwrap(),
            Candle::new(153.0, 158.0, 148.0, 155.0, 3400.0).unwrap(),
            Candle::new(155.0, 160.0, 150.0, 157.0, 3500.0).unwrap(),
            Candle::new(157.0, 162.0, 152.0, 159.0, 3600.0).unwrap(),
            Candle::new(159.0, 164.0, 154.0, 161.0, 3700.0).unwrap(),
            Candle::new(161.0, 166.0, 156.0, 163.0, 3800.0).unwrap(),
            Candle::new(163.0, 168.0, 158.0, 165.0, 3900.0).unwrap(),
        ];

        let signal = service.generate_trading_signal(&candles);
        assert!(signal.is_some());
        let s = signal.unwrap();
        assert!(s.confidence >= 0.0 && s.confidence <= 1.0);
    }

    #[tokio::test]
    async fn test_mpc_service_with_mock_actor() {
        let mut service = MpcService::new();

        // Use mock actor for predictable testing
        let mock_price = crate::domain::value_objects::price::Price::new(55000.0).unwrap();
        let sender = crate::infrastructure::adapters::exchange_actor::MockExchangeActor::spawn(
            crate::domain::entities::exchange::Exchange::Binance,
            mock_price.clone(),
        );
        service.add_actor(crate::domain::entities::exchange::Exchange::Binance, sender);

        // Get price from mock actor
        let result = service.get_price(&crate::domain::entities::exchange::Exchange::Binance, "BTCUSDT").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), 55000.0);
    }

    #[tokio::test]
    async fn test_subscribe_unsubscribe() {
        let mut service = MpcService::new();

        let mock_price = crate::domain::value_objects::price::Price::new(50000.0).unwrap();
        let sender = crate::infrastructure::adapters::exchange_actor::MockExchangeActor::spawn(
            crate::domain::entities::exchange::Exchange::Binance,
            mock_price,
        );
        service.add_actor(crate::domain::entities::exchange::Exchange::Binance, sender);

        // Test subscribe
        let result = service.subscribe(&crate::domain::entities::exchange::Exchange::Binance, "BTCUSDT").await;
        assert!(result.is_ok());

        // Test unsubscribe
        let result = service.unsubscribe(&crate::domain::entities::exchange::Exchange::Binance, "BTCUSDT").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_subscriptions() {
        let mut service = MpcService::new();

        let mock_price = crate::domain::value_objects::price::Price::new(50000.0).unwrap();
        let sender = crate::infrastructure::adapters::exchange_actor::MockExchangeActor::spawn(
            crate::domain::entities::exchange::Exchange::Binance,
            mock_price,
        );
        service.add_actor(crate::domain::entities::exchange::Exchange::Binance, sender);

        // Get subscriptions
        let result = service.get_subscriptions(&crate::domain::entities::exchange::Exchange::Binance).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0); // Mock actor returns empty list
    }
}