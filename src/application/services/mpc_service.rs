use std::collections::HashMap;
use tokio::sync::mpsc;
use crate::domain::entities::exchange::Exchange;
use crate::domain::value_objects::price::Price;
use crate::infrastructure::adapters::exchange_actor::ExchangeMessage;

pub struct MpcService {
    pub senders: HashMap<Exchange, mpsc::Sender<ExchangeMessage>>,
}

impl MpcService {
    pub fn new() -> Self {
        Self {
            senders: HashMap::new(),
        }
    }

    pub fn add_actor(&mut self, exchange: Exchange, sender: mpsc::Sender<ExchangeMessage>) {
        self.senders.insert(exchange, sender);
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
    pub fn aggregate_prices(prices: Vec<Price>) -> Price {
        let sum: f64 = prices.iter().map(|p| p.value()).sum();
        let avg = sum / prices.len() as f64;
        Price(avg) // assuming no error for simplicity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mpc_service_new() {
        let service = MpcService::new();
        assert!(service.senders.is_empty());
    }

    #[test]
    fn test_aggregate_prices_single() {
        let prices = vec![Price::new(100.0).unwrap()];
        let avg = MpcService::aggregate_prices(prices);
        assert_eq!(avg.value(), 100.0);
    }

    #[test]
    fn test_aggregate_prices_multiple() {
        let prices = vec![
            Price::new(100.0).unwrap(),
            Price::new(200.0).unwrap(),
            Price::new(300.0).unwrap(),
        ];
        let avg = MpcService::aggregate_prices(prices);
        assert_eq!(avg.value(), 200.0);
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