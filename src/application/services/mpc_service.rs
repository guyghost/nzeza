use std::collections::HashMap;
use tokio::sync::mpsc;
use crate::domain::entities::exchange::Exchange;
use crate::domain::value_objects::price::Price;
use crate::infrastructure::adapters::exchange_actor::ExchangeMessage;
use crate::domain::services::strategies::{SignalCombiner, TradingSignal};

pub struct MpcService {
    pub senders: HashMap<Exchange, mpsc::Sender<ExchangeMessage>>,
    pub signal_combiner: Option<SignalCombiner>,
}

impl MpcService {
    pub fn new() -> Self {
        Self {
            senders: HashMap::new(),
            signal_combiner: None,
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

    // Pure method: aggregate prices from multiple exchanges
    pub fn aggregate_prices(prices: Vec<Price>) -> Result<Price, String> {
        if prices.is_empty() {
            return Err("Cannot aggregate empty price list".to_string());
        }
        let sum: f64 = prices.iter().map(|p| p.value()).sum();
        let avg = sum / prices.len() as f64;
        Price::new(avg)
    }

    // Generate trading signal using combined strategies
    pub fn generate_trading_signal(&self, candles: &[crate::domain::services::indicators::Candle]) -> Option<TradingSignal> {
        self.signal_combiner.as_ref()?.combine_signals(candles)
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
}