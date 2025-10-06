use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::domain::entities::exchange::Exchange;
use crate::domain::value_objects::price::Price;

#[derive(Debug)]
pub enum ExchangeMessage {
    GetPrice { symbol: String, reply: mpsc::Sender<Result<Price, String>> },
}

pub struct ExchangeActor {
    pub exchange: Exchange,
    pub last_price: Arc<Mutex<Option<Price>>>,
}

impl ExchangeActor {
    pub fn spawn(exchange: Exchange) -> mpsc::Sender<ExchangeMessage> {
        let last_price = Arc::new(Mutex::new(None));
        let (tx, rx) = mpsc::channel(100);
        let exchange_clone = exchange.clone();
        let actor = Self {
            exchange,
            last_price: last_price.clone(),
        };
        tokio::spawn(async move {
            actor.run(rx).await;
        });
        // Spawn WebSocket task
        tokio::spawn(async move {
            Self::run_websocket(exchange_clone, last_price).await;
        });
        tx
    }

    async fn run(self, mut rx: mpsc::Receiver<ExchangeMessage>) {
        while let Some(msg) = rx.recv().await {
            match msg {
                ExchangeMessage::GetPrice { symbol: _, reply } => {
                    let price = self.last_price.lock().await.clone();
                    let result = price.ok_or("No price available".to_string());
                    let _ = reply.send(result).await;
                }
            }
        }
    }

    async fn run_websocket(exchange: Exchange, last_price: Arc<Mutex<Option<Price>>>) {
        let (ws_url, subscribe_msg) = match exchange {
            Exchange::Binance => {
                ("wss://stream.binance.com:9443/ws/btcusdt@ticker".to_string(), None) // For demo, hardcoded BTCUSDT
            }
            Exchange::Dydx => {
                ("wss://api.dydx.exchange/v4/ws".to_string(), Some(r#"{"type":"subscribe","channel":"v4_markets","id":"BTC-USD"}"#.to_string()))
            }
            Exchange::Hyperliquid => {
                ("wss://api.hyperliquid.xyz/ws".to_string(), Some(r#"{"method":"subscribe","subscription":{"type":"allMids"}}"#.to_string()))
            }
            Exchange::Coinbase => {
                ("wss://ws-feed.pro.coinbase.com".to_string(), Some(r#"{"type":"subscribe","product_ids":["BTC-USD"],"channels":["ticker"]}"#.to_string()))
            }
            Exchange::Kraken => {
                ("wss://ws.kraken.com".to_string(), Some(r#"{"event":"subscribe","pair":["BTC/USD"],"subscription":{"name":"ticker"}}"#.to_string()))
            }
        };

        let (ws_stream, _) = connect_async(&ws_url).await.expect("Failed to connect");
        let (mut write, mut read) = ws_stream.split();

        // Send subscribe message if needed
        if let Some(msg) = subscribe_msg {
            write.send(Message::Text(msg)).await.unwrap();
        }

        while let Some(message) = read.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) {
                        let price_opt = Self::parse_price(&exchange, &data);
                        if let Some(price) = price_opt {
                            *last_price.lock().await = Some(price);
                        }
                    }
                }
                Ok(Message::Close(_)) => break,
                _ => {}
            }
        }
    }

    fn parse_price(exchange: &Exchange, data: &serde_json::Value) -> Option<Price> {
        match exchange {
            Exchange::Binance => {
                data["c"].as_str().and_then(|s| s.parse::<f64>().ok()).and_then(|p| Price::new(p).ok())
            }
            Exchange::Dydx => {
                data["contents"]["markets"]["BTC-USD"]["oraclePrice"].as_str().and_then(|s| s.parse::<f64>().ok()).and_then(|p| Price::new(p).ok())
            }
            Exchange::Hyperliquid => {
                // For allMids, find BTC
                if let Some(arr) = data.as_array() {
                    for item in arr {
                        if item[0] == "BTC" {
                            return item[1].as_f64().and_then(|p| Price::new(p).ok());
                        }
                    }
                }
                None
            }
            Exchange::Coinbase => {
                data["price"].as_str().and_then(|s| s.parse::<f64>().ok()).and_then(|p| Price::new(p).ok())
            }
            Exchange::Kraken => {
                data[1]["c"][0].as_str().and_then(|s| s.parse::<f64>().ok()).and_then(|p| Price::new(p).ok())
            }
        }
    }
}

#[cfg(test)]
pub struct MockExchangeActor {
    pub exchange: Exchange,
    pub mock_price: Price,
}

#[cfg(test)]
impl MockExchangeActor {
    pub fn spawn(exchange: Exchange, mock_price: Price) -> mpsc::Sender<ExchangeMessage> {
        let (tx, rx) = mpsc::channel(100);
        let actor = Self { exchange, mock_price };
        tokio::spawn(async move {
            actor.run(rx).await;
        });
        tx
    }

    async fn run(self, mut rx: mpsc::Receiver<ExchangeMessage>) {
        while let Some(msg) = rx.recv().await {
            match msg {
                ExchangeMessage::GetPrice { symbol: _, reply } => {
                    let result = Ok(self.mock_price.clone());
                    let _ = reply.send(result).await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_price_binance() {
        let data = json!({"c": "50000.00"});
        let price = ExchangeActor::parse_price(&Exchange::Binance, &data);
        assert!(price.is_some());
        assert_eq!(price.unwrap().value(), 50000.0);
    }

    #[test]
    fn test_parse_price_dydx() {
        let data = json!({"contents": {"markets": {"BTC-USD": {"oraclePrice": "51000.00"}}}});
        let price = ExchangeActor::parse_price(&Exchange::Dydx, &data);
        assert!(price.is_some());
        assert_eq!(price.unwrap().value(), 51000.0);
    }

    #[test]
    fn test_parse_price_hyperliquid() {
        let data = json!([["BTC", 52000.0], ["ETH", 3000.0]]);
        let price = ExchangeActor::parse_price(&Exchange::Hyperliquid, &data);
        assert!(price.is_some());
        assert_eq!(price.unwrap().value(), 52000.0);
    }

    #[test]
    fn test_parse_price_coinbase() {
        let data = json!({"price": "53000.00"});
        let price = ExchangeActor::parse_price(&Exchange::Coinbase, &data);
        assert!(price.is_some());
        assert_eq!(price.unwrap().value(), 53000.0);
    }

    #[test]
    fn test_parse_price_kraken() {
        let data = json!([{}, {"c": ["54000.00", "54001.00"]}]);
        let price = ExchangeActor::parse_price(&Exchange::Kraken, &data);
        assert!(price.is_some());
        assert_eq!(price.unwrap().value(), 54000.0);
    }

    #[test]
    fn test_parse_price_invalid() {
        let data = json!({"invalid": "data"});
        let price = ExchangeActor::parse_price(&Exchange::Binance, &data);
        assert!(price.is_none());
    }

    #[tokio::test]
    async fn test_mock_exchange_actor() {
        let mock_price = Price::new(60000.0).unwrap();
        let sender = MockExchangeActor::spawn(Exchange::Binance, mock_price.clone());

        let (reply_tx, mut reply_rx) = mpsc::channel(1);
        let msg = ExchangeMessage::GetPrice {
            symbol: "BTCUSDT".to_string(),
            reply: reply_tx,
        };
        sender.send(msg).await.unwrap();

        let result = reply_rx.recv().await.unwrap();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), 60000.0);
    }
}