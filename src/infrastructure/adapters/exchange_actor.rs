use crate::domain::entities::exchange::Exchange;
use crate::domain::value_objects::price::Price;
use futures_util::{SinkExt, StreamExt};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::time::Instant;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
pub enum SubscriptionCommand {
    Subscribe(String),
    Unsubscribe(String),
}

use crate::domain::entities::order::Order;

#[derive(Debug)]
pub enum ExchangeMessage {
    GetPrice {
        symbol: String,
        reply: mpsc::Sender<Result<Price, String>>,
    },
    Subscribe {
        symbol: String,
        reply: mpsc::Sender<Result<(), String>>,
    },
    Unsubscribe {
        symbol: String,
        reply: mpsc::Sender<Result<(), String>>,
    },
    GetSubscriptions {
        reply: mpsc::Sender<Vec<String>>,
    },
    HealthCheck {
        reply: mpsc::Sender<bool>,
    },
    PlaceOrder {
        order: Order,
        reply: mpsc::Sender<Result<String, String>>,
    },
    CancelOrder {
        order_id: String,
        reply: mpsc::Sender<Result<(), String>>,
    },
    GetOrderStatus {
        order_id: String,
        reply: mpsc::Sender<Result<String, String>>,
    },
    Shutdown,
}

pub struct ExchangeActor {
    pub exchange: Exchange,
    pub prices: Arc<Mutex<HashMap<String, Price>>>,
    pub subscriptions: Arc<Mutex<HashSet<String>>>,
    pub shutdown_tx: broadcast::Sender<()>,
    pub subscription_tx: mpsc::Sender<SubscriptionCommand>,
    activity_tracker: ActivityTracker,
}

#[derive(Clone)]
struct ActivityTracker {
    last_activity: Arc<Mutex<Instant>>,
}

impl ActivityTracker {
    fn new() -> Self {
        Self {
            last_activity: Arc::new(Mutex::new(Instant::now())),
        }
    }

    async fn mark_active(&self) {
        let mut guard = self.last_activity.lock().await;
        *guard = Instant::now();
    }

    async fn is_healthy(&self, timeout: std::time::Duration) -> bool {
        let guard = self.last_activity.lock().await;
        guard.elapsed() <= timeout
    }
}

impl ExchangeActor {
    pub fn spawn(exchange: Exchange) -> mpsc::Sender<ExchangeMessage> {
        let prices = Arc::new(Mutex::new(HashMap::new()));
        let subscriptions = Arc::new(Mutex::new(HashSet::new()));
        let (tx, rx) = mpsc::channel(100);
        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
        let (subscription_tx, subscription_rx) = mpsc::channel(100);
        let exchange_clone = exchange.clone();
        let activity_tracker = ActivityTracker::new();

        let actor = Self {
            exchange,
            prices: prices.clone(),
            subscriptions: subscriptions.clone(),
            shutdown_tx: shutdown_tx.clone(),
            subscription_tx: subscription_tx.clone(),
            activity_tracker: activity_tracker.clone(),
        };

        tokio::spawn(async move {
            actor.run(rx).await;
        });

        // Spawn WebSocket task with shutdown receiver
        tokio::spawn(async move {
            Self::run_websocket(
                exchange_clone,
                prices,
                subscriptions,
                subscription_rx,
                shutdown_rx,
                activity_tracker,
            )
            .await;
        });

        tx
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

    async fn run(self, mut rx: mpsc::Receiver<ExchangeMessage>) {
        info!(
            "Actor started for exchange: {}",
            Self::get_exchange_name(&self.exchange)
        );

        while let Some(msg) = rx.recv().await {
            self.activity_tracker.mark_active().await;

            match msg {
                ExchangeMessage::GetPrice { symbol, reply } => {
                    let prices = self.prices.lock().await;
                    let result = prices
                        .get(&symbol)
                        .cloned()
                        .ok_or_else(|| format!("No price available for symbol: {}", symbol));
                    if let Err(e) = reply.send(result).await {
                        warn!("Failed to send reply: {:?}", e);
                    }
                }
                ExchangeMessage::Subscribe { symbol, reply } => {
                    info!(
                        "Subscribe request for {}: {}",
                        Self::get_exchange_name(&self.exchange),
                        symbol
                    );
                    let mut subscriptions = self.subscriptions.lock().await;

                    if subscriptions.contains(&symbol) {
                        let _ = reply
                            .send(Err(format!("Already subscribed to {}", symbol)))
                            .await;
                    } else {
                        subscriptions.insert(symbol.clone());

                        // Send subscription command to WebSocket task
                        let result = self
                            .subscription_tx
                            .send(SubscriptionCommand::Subscribe(symbol.clone()))
                            .await
                            .map_err(|e| format!("Failed to send subscription command: {}", e));

                        if let Err(e) = reply.send(result).await {
                        warn!("Failed to send reply: {:?}", e);
                    }
                        info!(
                            "Subscribed to {}: {}",
                            Self::get_exchange_name(&self.exchange),
                            symbol
                        );
                    }
                }
                ExchangeMessage::Unsubscribe { symbol, reply } => {
                    info!(
                        "Unsubscribe request for {}: {}",
                        Self::get_exchange_name(&self.exchange),
                        symbol
                    );
                    let mut subscriptions = self.subscriptions.lock().await;

                    if !subscriptions.contains(&symbol) {
                        let _ = reply
                            .send(Err(format!("Not subscribed to {}", symbol)))
                            .await;
                    } else {
                        subscriptions.remove(&symbol);

                        // Send unsubscription command to WebSocket task
                        let result = self
                            .subscription_tx
                            .send(SubscriptionCommand::Unsubscribe(symbol.clone()))
                            .await
                            .map_err(|e| format!("Failed to send unsubscription command: {}", e));

                        if let Err(e) = reply.send(result).await {
                        warn!("Failed to send reply: {:?}", e);
                    }
                        info!(
                            "Unsubscribed from {}: {}",
                            Self::get_exchange_name(&self.exchange),
                            symbol
                        );
                    }
                }
                ExchangeMessage::GetSubscriptions { reply } => {
                    let subscriptions = self.subscriptions.lock().await;
                    let symbols: Vec<String> = subscriptions.iter().cloned().collect();
                    if let Err(e) = reply.send(symbols).await {
                        warn!("Failed to send symbols reply: {:?}", e);
                    }
                }
                ExchangeMessage::HealthCheck { reply } => {
                    let is_healthy = self
                        .activity_tracker
                        .is_healthy(std::time::Duration::from_secs(30))
                        .await;
                    if let Err(e) = reply.send(is_healthy).await {
                        warn!("Failed to send health check reply: {:?}", e);
                    }
                    debug!(
                        "Health check for {}: {}",
                        Self::get_exchange_name(&self.exchange),
                        is_healthy
                    );
                }
                ExchangeMessage::PlaceOrder { order, reply } => {
                    info!(
                        "Place order request for {}: {:?}",
                        Self::get_exchange_name(&self.exchange),
                        order.id
                    );
                    let result = Self::place_order(&self.exchange, &order).await;
                    if let Err(e) = reply.send(result).await {
                        warn!("Failed to send reply: {:?}", e);
                    }
                }
                ExchangeMessage::CancelOrder { order_id, reply } => {
                    info!(
                        "Cancel order request for {}: {}",
                        Self::get_exchange_name(&self.exchange),
                        order_id
                    );
                    let result = Self::cancel_order(&self.exchange, &order_id).await;
                    if let Err(e) = reply.send(result).await {
                        warn!("Failed to send reply: {:?}", e);
                    }
                }
                ExchangeMessage::GetOrderStatus { order_id, reply } => {
                    debug!(
                        "Get order status request for {}: {}",
                        Self::get_exchange_name(&self.exchange),
                        order_id
                    );
                    let result = Self::get_order_status(&self.exchange, &order_id).await;
                    if let Err(e) = reply.send(result).await {
                        warn!("Failed to send reply: {:?}", e);
                    }
                }
                ExchangeMessage::Shutdown => {
                    info!(
                        "Shutdown signal received for exchange: {}",
                        Self::get_exchange_name(&self.exchange)
                    );
                    // Signal the WebSocket task to shutdown
                    let _ = self.shutdown_tx.send(());
                    break;
                }
            }
        }

        info!(
            "Actor stopped for exchange: {}",
            Self::get_exchange_name(&self.exchange)
        );
    }

    async fn run_websocket(
        exchange: Exchange,
        prices: Arc<Mutex<HashMap<String, Price>>>,
        subscriptions: Arc<Mutex<HashSet<String>>>,
        mut subscription_rx: mpsc::Receiver<SubscriptionCommand>,
        mut shutdown_rx: broadcast::Receiver<()>,
        activity_tracker: ActivityTracker,
    ) {
        use std::time::Duration;

        let mut backoff = Duration::from_secs(1);
        let max_backoff = Duration::from_secs(60);

        loop {
            info!(
                "Starting WebSocket connection for exchange: {}",
                Self::get_exchange_name(&exchange)
            );

            tokio::select! {
                result = Self::try_websocket_connection(&exchange, &prices, &subscriptions, &mut subscription_rx, &activity_tracker) => {
                    match result {
                        Ok(()) => {
                            info!("WebSocket connection ended normally for {}, reconnecting...", Self::get_exchange_name(&exchange));
                            backoff = Duration::from_secs(1); // Reset on successful connection
                        }
                        Err(e) => {
                            error!("WebSocket error for {}: {}, retrying in {:?}", Self::get_exchange_name(&exchange), e, backoff);
                        }
                    }

                    // Check for shutdown before sleeping
                    tokio::select! {
                        _ = tokio::time::sleep(backoff) => {
                            backoff = (backoff * 2).min(max_backoff);
                        }
                        _ = shutdown_rx.recv() => {
                            info!("WebSocket task received shutdown signal for {}", Self::get_exchange_name(&exchange));
                            return;
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("WebSocket task received shutdown signal for {}", Self::get_exchange_name(&exchange));
                    return;
                }
            }
        }
    }

    async fn try_websocket_connection(
        exchange: &Exchange,
        prices: &Arc<Mutex<HashMap<String, Price>>>,
        subscriptions: &Arc<Mutex<HashSet<String>>>,
        subscription_rx: &mut mpsc::Receiver<SubscriptionCommand>,
        activity_tracker: &ActivityTracker,
    ) -> Result<(), String> {
        let ws_url = Self::get_websocket_url(exchange);

        // Add timeout for WebSocket connection (10 seconds)
        let (stream, _) = tokio::time::timeout(
            tokio::time::Duration::from_secs(10),
            connect_async(&ws_url)
        )
        .await
        .map_err(|_| "WebSocket connection timeout (10s)".to_string())?
        .map_err(|e| format!("Failed to connect: {}", e))?;
        info!(
            "Successfully connected to {} WebSocket",
            Self::get_exchange_name(exchange)
        );
        activity_tracker.mark_active().await;

        let (mut write, mut read) = stream.split();

        // Subscribe to any existing symbols
        let current_subscriptions = subscriptions.lock().await.clone();
        for symbol in current_subscriptions {
            if let Some(msg) = Self::build_subscribe_message(exchange, &symbol) {
                write
                    .send(Message::Text(msg.clone()))
                    .await
                    .map_err(|e| format!("Failed to send subscribe message: {}", e))?;
                info!(
                    "Sent subscribe message to {} for {}: {}",
                    Self::get_exchange_name(exchange),
                    symbol,
                    msg
                );
            }
        }

        loop {
            tokio::select! {
                // Handle incoming WebSocket messages
                message = read.next() => {
                    match message {
                        Some(Ok(Message::Text(text))) => {
                            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) {
                                if let Some((symbol, price)) = Self::parse_price_with_symbol(exchange, &data) {
                                    prices.lock().await.insert(symbol.clone(), price);
                                    debug!("Prix mis à jour pour {} {}: {:.2}", Self::get_exchange_name(exchange), symbol, price.value());
                                    activity_tracker.mark_active().await;
                                }
                            } else {
                                warn!("Received invalid JSON from {:?}: {}", exchange, text);
                            }
                        }
                        Some(Ok(Message::Close(frame))) => {
                            info!("WebSocket connection closed for {}: {:?}", Self::get_exchange_name(exchange), frame);
                            return Ok(());
                        }
                        Some(Ok(Message::Ping(payload))) => {
                            debug!("Received ping from {}, responding with pong", Self::get_exchange_name(exchange));
                            write.send(Message::Pong(payload)).await
                                .map_err(|e| format!("Failed to send pong: {}", e))?;
                        }
                        Some(Ok(other)) => {
                            debug!("Received other message type from {}: {:?}", Self::get_exchange_name(exchange), other);
                        }
                        Some(Err(e)) => {
                            return Err(format!("WebSocket read error: {}", e));
                        }
                        None => {
                            info!("WebSocket stream ended for {}", Self::get_exchange_name(exchange));
                            return Ok(());
                        }
                    }
                }
                // Handle subscription commands
                Some(cmd) = subscription_rx.recv() => {
                    match cmd {
                        SubscriptionCommand::Subscribe(symbol) => {
                            if let Some(msg) = Self::build_subscribe_message(exchange, &symbol) {
                                write.send(Message::Text(msg.clone())).await
                                    .map_err(|e| format!("Failed to send subscribe message: {}", e))?;
                                info!("Subscribed to {} {}: {}", Self::get_exchange_name(exchange), symbol, msg);
                                activity_tracker.mark_active().await;
                            }
                        }
                        SubscriptionCommand::Unsubscribe(symbol) => {
                            if let Some(msg) = Self::build_unsubscribe_message(exchange, &symbol) {
                                write.send(Message::Text(msg.clone())).await
                                    .map_err(|e| format!("Failed to send unsubscribe message: {}", e))?;
                                info!("Unsubscribed from {} {}: {}", Self::get_exchange_name(exchange), symbol, msg);
                            }
                            // Remove from prices map
                            prices.lock().await.remove(&symbol);
                            activity_tracker.mark_active().await;
                        }
                    }
                }
            }
        }
    }

    fn get_websocket_url(exchange: &Exchange) -> String {
        match exchange {
            Exchange::Binance => "wss://stream.binance.com:9443/ws".to_string(),
            Exchange::Dydx => "wss://indexer.dydx.trade/v4/ws".to_string(),
            Exchange::Hyperliquid => "wss://api.hyperliquid.xyz/ws".to_string(),
            Exchange::Coinbase => "wss://ws-feed.exchange.coinbase.com".to_string(),
            Exchange::Kraken => "wss://ws.kraken.com".to_string(),
        }
    }

    fn build_subscribe_message(exchange: &Exchange, symbol: &str) -> Option<String> {
        match exchange {
            Exchange::Binance => {
                let stream = format!("{}@ticker", symbol.to_lowercase());
                Some(format!(
                    r#"{{"method":"SUBSCRIBE","params":["{}"],"id":1}}"#,
                    stream
                ))
            }
            Exchange::Dydx => Some(format!(
                r#"{{"type":"subscribe","channel":"v4_markets","id":"{}"}}"#,
                symbol
            )),
            Exchange::Hyperliquid => {
                Some(r#"{"method":"subscribe","subscription":{"type":"allMids"}}"#.to_string())
            }
            Exchange::Coinbase => Some(format!(
                r#"{{"type":"subscribe","product_ids":["{}"],"channels":["ticker"]}}"#,
                symbol
            )),
            Exchange::Kraken => Some(format!(
                r#"{{"event":"subscribe","pair":["{}"],"subscription":{{"name":"ticker"}}}}"#,
                symbol
            )),
        }
    }

    fn build_unsubscribe_message(exchange: &Exchange, symbol: &str) -> Option<String> {
        match exchange {
            Exchange::Binance => {
                let stream = format!("{}@ticker", symbol.to_lowercase());
                Some(format!(
                    r#"{{"method":"UNSUBSCRIBE","params":["{}"],"id":1}}"#,
                    stream
                ))
            }
            Exchange::Dydx => Some(format!(
                r#"{{"type":"unsubscribe","channel":"v4_markets","id":"{}"}}"#,
                symbol
            )),
            Exchange::Hyperliquid => {
                Some(r#"{"method":"unsubscribe","subscription":{"type":"allMids"}}"#.to_string())
            }
            Exchange::Coinbase => Some(format!(
                r#"{{"type":"unsubscribe","product_ids":["{}"],"channels":["ticker"]}}"#,
                symbol
            )),
            Exchange::Kraken => Some(format!(
                r#"{{"event":"unsubscribe","pair":["{}"],"subscription":{{"name":"ticker"}}}}"#,
                symbol
            )),
        }
    }

    fn parse_price_with_symbol(
        exchange: &Exchange,
        data: &serde_json::Value,
    ) -> Option<(String, Price)> {
        match exchange {
            Exchange::Binance => {
                let symbol = data["s"].as_str()?.to_string();
                let price = data["c"].as_str()?.parse::<f64>().ok()?;
                Some((symbol, Price::new(price).ok()?))
            }
            Exchange::Dydx => {
                if let Some(markets) = data["contents"]["markets"].as_object() {
                    for (symbol, market_data) in markets {
                        if let Some(price_str) = market_data["oraclePrice"].as_str() {
                            if let Ok(price_val) = price_str.parse::<f64>() {
                                if let Ok(price) = Price::new(price_val) {
                                    return Some((symbol.clone(), price));
                                }
                            }
                        }
                    }
                }
                None
            }
            Exchange::Hyperliquid => {
                if let Some(arr) = data["data"]["mids"].as_object() {
                    for (symbol, price_val) in arr {
                        if let Some(price_str) = price_val.as_str() {
                            if let Ok(price_f64) = price_str.parse::<f64>() {
                                if let Ok(price) = Price::new(price_f64) {
                                    return Some((symbol.clone(), price));
                                }
                            }
                        }
                    }
                }
                None
            }
            Exchange::Coinbase => {
                let symbol = data["product_id"].as_str()?.to_string();
                let price = data["price"].as_str()?.parse::<f64>().ok()?;
                Some((symbol, Price::new(price).ok()?))
            }
            Exchange::Kraken => {
                if let Some(arr) = data.as_array() {
                    if arr.len() >= 4 {
                        let symbol = arr[3].as_str()?.to_string();
                        let price = arr[1]["c"][0].as_str()?.parse::<f64>().ok()?;
                        return Some((symbol, Price::new(price).ok()?));
                    }
                }
                None
            }
        }
    }

    #[allow(dead_code)]
    fn parse_price(exchange: &Exchange, data: &serde_json::Value) -> Option<Price> {
        match exchange {
            Exchange::Binance => data["c"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .and_then(|p| Price::new(p).ok()),
            Exchange::Dydx => data["contents"]["markets"]["BTC-USD"]["oraclePrice"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .and_then(|p| Price::new(p).ok()),
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
            Exchange::Coinbase => data["price"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .and_then(|p| Price::new(p).ok()),
            Exchange::Kraken => data[1]["c"][0]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .and_then(|p| Price::new(p).ok()),
        }
    }

    /// Place an order on the exchange
    async fn place_order(exchange: &Exchange, order: &Order) -> Result<String, String> {
        match exchange {
            Exchange::Dydx => Self::place_order_dydx(order).await,
            _ => Err(format!(
                "Order placement not implemented for {:?}",
                exchange
            )),
        }
    }

    /// Cancel an order on the exchange
    async fn cancel_order(exchange: &Exchange, order_id: &str) -> Result<(), String> {
        match exchange {
            Exchange::Dydx => Self::cancel_order_dydx(order_id).await,
            _ => Err(format!(
                "Order cancellation not implemented for {:?}",
                exchange
            )),
        }
    }

    /// Get order status from the exchange
    async fn get_order_status(exchange: &Exchange, order_id: &str) -> Result<String, String> {
        match exchange {
            Exchange::Dydx => Self::get_order_status_dydx(order_id).await,
            _ => Err(format!("Order status not implemented for {:?}", exchange)),
        }
    }

    /// Get wallet from mnemonic for dYdX authentication
    fn get_dydx_wallet() -> Result<ethers::signers::LocalWallet, String> {
        let mnemonic = std::env::var("DYDX_MNEMONIC")
            .map_err(|_| "DYDX_MNEMONIC environment variable not set".to_string())?;

        ethers::signers::MnemonicBuilder::<ethers::signers::coins_bip39::English>::default()
            .phrase(mnemonic.as_str())
            .build()
            .map_err(|e| format!("Failed to create wallet from mnemonic: {}", e))
    }

    /// Place order on dYdX v4 using mnemonic wallet
    async fn place_order_dydx(order: &Order) -> Result<String, String> {
        let _wallet = Self::get_dydx_wallet()?;

        // For now, return a mock order ID since full dYdX v4 integration requires
        // more complex setup with order signing, sequence numbers, etc.
        // This is a placeholder that will be expanded with full implementation
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("System time error: {}", e))?
            .as_millis();

        let order_id = format!("dydx_order_{}", timestamp);

        info!(
            "dYdX order placement requested: {:?} {} {} (wallet ready)",
            order.side,
            order.quantity.value(),
            order.symbol
        );

        // TODO: Implement full dYdX v4 order placement with proper signing
        Ok(order_id)
    }

    /// Cancel order on dYdX v4 using mnemonic wallet
    async fn cancel_order_dydx(order_id: &str) -> Result<(), String> {
        let _wallet = Self::get_dydx_wallet()?;

        info!("dYdX order cancellation requested: {}", order_id);

        // TODO: Implement full dYdX v4 order cancellation
        Ok(())
    }

    /// Get order status from dYdX v4 using mnemonic wallet
    async fn get_order_status_dydx(order_id: &str) -> Result<String, String> {
        let _wallet = Self::get_dydx_wallet()?;

        info!("dYdX order status requested: {}", order_id);

        // TODO: Implement full dYdX v4 order status checking
        Ok("PENDING".to_string())
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
        let actor = Self {
            exchange,
            mock_price,
        };
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
                    if let Err(e) = reply.send(result).await {
                        warn!("Failed to send reply: {:?}", e);
                    }
                }
                ExchangeMessage::Subscribe { symbol: _, reply } => {
                    let _ = reply.send(Ok(())).await;
                }
                ExchangeMessage::Unsubscribe { symbol: _, reply } => {
                    let _ = reply.send(Ok(())).await;
                }
                ExchangeMessage::GetSubscriptions { reply } => {
                    let _ = reply.send(vec![]).await;
                }
                ExchangeMessage::HealthCheck { reply } => {
                    let _ = reply.send(true).await;
                }
                ExchangeMessage::PlaceOrder { order: _, reply } => {
                    let _ = reply.send(Ok("mock_order_id".to_string())).await;
                }
                ExchangeMessage::CancelOrder { order_id: _, reply } => {
                    let _ = reply.send(Ok(())).await;
                }
                ExchangeMessage::GetOrderStatus { order_id: _, reply } => {
                    let _ = reply.send(Ok("FILLED".to_string())).await;
                }
                ExchangeMessage::Shutdown => {
                    break;
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

    #[test]
    fn test_get_websocket_url() {
        assert_eq!(
            ExchangeActor::get_websocket_url(&Exchange::Binance),
            "wss://stream.binance.com:9443/ws"
        );
        assert_eq!(
            ExchangeActor::get_websocket_url(&Exchange::Dydx),
            "wss://indexer.dydx.trade/v4/ws"
        );
        assert_eq!(
            ExchangeActor::get_websocket_url(&Exchange::Hyperliquid),
            "wss://api.hyperliquid.xyz/ws"
        );
        assert_eq!(
            ExchangeActor::get_websocket_url(&Exchange::Coinbase),
            "wss://ws-feed.exchange.coinbase.com"
        );
        assert_eq!(
            ExchangeActor::get_websocket_url(&Exchange::Kraken),
            "wss://ws.kraken.com"
        );
    }

    #[test]
    fn test_build_subscribe_message() {
        let msg = ExchangeActor::build_subscribe_message(&Exchange::Binance, "BTCUSDT");
        assert!(msg.is_some());
        let msg_str = msg.unwrap();
        assert!(msg_str.contains("SUBSCRIBE"));
        assert!(msg_str.contains("btcusdt@ticker"));

        let msg = ExchangeActor::build_subscribe_message(&Exchange::Coinbase, "BTC-USD");
        assert!(msg.is_some());
        let msg_str = msg.unwrap();
        assert!(msg_str.contains("subscribe"));
        assert!(msg_str.contains("BTC-USD"));
    }

    #[test]
    fn test_build_unsubscribe_message() {
        let msg = ExchangeActor::build_unsubscribe_message(&Exchange::Binance, "BTCUSDT");
        assert!(msg.is_some());
        assert!(msg.unwrap().contains("UNSUBSCRIBE"));

        let msg = ExchangeActor::build_unsubscribe_message(&Exchange::Coinbase, "BTC-USD");
        assert!(msg.is_some());
        assert!(msg.unwrap().contains("unsubscribe"));
    }

    #[test]
    fn test_parse_price_with_symbol_binance() {
        let data = json!({"s": "BTCUSDT", "c": "50000.00"});
        let result = ExchangeActor::parse_price_with_symbol(&Exchange::Binance, &data);
        assert!(result.is_some());
        let (symbol, price) = result.unwrap();
        assert_eq!(symbol, "BTCUSDT");
        assert_eq!(price.value(), 50000.0);
    }

    #[test]
    fn test_parse_price_with_symbol_coinbase() {
        let data = json!({"product_id": "BTC-USD", "price": "53000.00"});
        let result = ExchangeActor::parse_price_with_symbol(&Exchange::Coinbase, &data);
        assert!(result.is_some());
        let (symbol, price) = result.unwrap();
        assert_eq!(symbol, "BTC-USD");
        assert_eq!(price.value(), 53000.0);
    }

    #[tokio::test]
    async fn test_activity_tracker_detects_staleness() {
        let tracker = ActivityTracker::new();
        assert!(tracker.is_healthy(std::time::Duration::from_millis(50)).await);
        tokio::time::sleep(std::time::Duration::from_millis(80)).await;
        assert!(!tracker.is_healthy(std::time::Duration::from_millis(50)).await);
    }
}
