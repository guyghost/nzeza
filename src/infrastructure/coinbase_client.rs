use crate::domain::entities::order::{Order, OrderSide, OrderType};
use crate::domain::repositories::exchange_client::{
    Balance, ExchangeClient, ExchangeError, ExchangeResult, OrderStatus,
};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::info;

/// Coinbase Pro API endpoints
const COINBASE_API_BASE: &str = "https://api.exchange.coinbase.com";
const COINBASE_SANDBOX_BASE: &str = "https://api-public.sandbox.exchange.coinbase.com";

/// Coinbase network configuration
#[derive(Debug, Clone)]
pub struct CoinbaseConfig {
    pub api_base: String,
    pub api_key: String,
    pub api_secret: String,
    pub passphrase: Option<String>, // Make passphrase optional
}

impl CoinbaseConfig {
    pub fn new(api_key: &str, api_secret: &str, passphrase: Option<&str>, sandbox: bool) -> Self {
        Self {
            api_base: if sandbox {
                COINBASE_SANDBOX_BASE.to_string()
            } else {
                COINBASE_API_BASE.to_string()
            },
            api_key: api_key.to_string(),
            api_secret: api_secret.to_string(),
            passphrase: passphrase.map(|s| s.to_string()),
        }
    }
}

/// Coinbase order structure for API
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoinbaseOrder {
    pub product_id: String,
    pub side: String,
    pub size: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_in_force: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_after: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_only: Option<bool>,
}

/// Coinbase API response for order placement
#[derive(Debug, Serialize, Deserialize)]
pub struct CoinbaseOrderResponse {
    pub id: String,
    pub product_id: String,
    pub side: String,
    pub size: String,
    pub price: Option<String>,
    pub status: String,
    pub settled: bool,
    pub done_reason: Option<String>,
    pub created_at: String,
}

/// Coinbase account information
#[derive(Debug, Serialize, Deserialize)]
pub struct CoinbaseAccount {
    pub id: String,
    pub currency: String,
    pub balance: String,
    pub available: String,
    pub hold: String,
}

/// Coinbase client for API interactions
pub struct CoinbaseClient {
    client: Client,
    config: CoinbaseConfig,
}

impl CoinbaseClient {
    /// Create a new Coinbase client from API credentials
    pub fn new(api_key: &str, api_secret: &str, passphrase: Option<&str>) -> Result<Self, String> {
        let config = CoinbaseConfig::new(api_key, api_secret, passphrase, false); // production by default

        Ok(Self {
            client: Client::new(),
            config,
        })
    }

    /// Create a sandbox Coinbase client for testing
    pub fn new_sandbox(
        api_key: &str,
        api_secret: &str,
        passphrase: Option<&str>,
    ) -> Result<Self, String> {
        let config = CoinbaseConfig::new(api_key, api_secret, passphrase, true);

        Ok(Self {
            client: Client::new(),
            config,
        })
    }

    /// Generate authentication headers for Coinbase API
    fn generate_auth_headers(
        &self,
        method: &str,
        path: &str,
        body: &str,
    ) -> Result<HashMap<String, String>, String> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("Time error: {}", e))?
            .as_secs();

        let message = format!("{}{}{}{}", timestamp, method, path, body);

        // Create HMAC-SHA256 signature
        use base64::{engine::general_purpose, Engine as _};
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(self.config.api_secret.as_bytes())
            .map_err(|e| format!("HMAC error: {}", e))?;

        mac.update(message.as_bytes());
        let signature = general_purpose::STANDARD.encode(mac.finalize().into_bytes());

        let mut headers = HashMap::new();
        headers.insert("CB-ACCESS-KEY".to_string(), self.config.api_key.clone());
        headers.insert("CB-ACCESS-SIGN".to_string(), signature);
        headers.insert("CB-ACCESS-TIMESTAMP".to_string(), timestamp.to_string());

        // Only include passphrase if it exists
        if let Some(ref passphrase) = self.config.passphrase {
            headers.insert("CB-ACCESS-PASSPHRASE".to_string(), passphrase.clone());
        }

        headers.insert("Content-Type".to_string(), "application/json".to_string());

        Ok(headers)
    }

    /// Get account information from Coinbase API
    pub async fn get_accounts(&self) -> Result<Vec<CoinbaseAccount>, String> {
        let path = "/accounts";
        let url = format!("{}{}", self.config.api_base, path);

        let headers = self.generate_auth_headers("GET", path, "")?;

        let mut request = self.client.get(&url);
        // Add required User-Agent header
        request = request.header("User-Agent", "NZEZA-Trading-Bot/0.1.0");
        for (key, value) in headers {
            request = request.header(&key, value);
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Failed to get accounts: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Accounts API error: {} - {}", status, error_text));
        }

        let accounts: Vec<CoinbaseAccount> = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse accounts response: {}", e))?;

        Ok(accounts)
    }

    /// Convert our Order to Coinbase API format
    pub fn convert_order(&self, order: &Order) -> Result<CoinbaseOrder, String> {
        let product_id = self.normalize_product_id(&order.symbol)?;
        let side = match order.side {
            OrderSide::Buy => "buy",
            OrderSide::Sell => "sell",
        }
        .to_string();

        let size = order.quantity.value().to_string();

        let (order_type, price) = match order.order_type {
            OrderType::Market => ("market".to_string(), None),
            OrderType::Limit => {
                let price_str = order
                    .price
                    .map(|p| p.value().to_string())
                    .ok_or("Limit order must have price")?;
                ("limit".to_string(), Some(price_str))
            }
        };

        Ok(CoinbaseOrder {
            product_id,
            side,
            size,
            price,
            r#type: order_type,
            time_in_force: Some("GTC".to_string()), // Good 'Til Canceled
            cancel_after: None,
            post_only: Some(false),
        })
    }

    /// Normalize symbol to Coinbase product ID format
    fn normalize_product_id(&self, symbol: &str) -> Result<String, String> {
        match symbol {
            "BTC-USD" => Ok("BTC-USD".to_string()),
            "ETH-USD" => Ok("ETH-USD".to_string()),
            "SOL-USD" => Ok("SOL-USD".to_string()),
            _ => Err(format!("Unsupported product: {}", symbol)),
        }
    }

    /// Place order on Coinbase
    pub async fn place_order(&self, order: CoinbaseOrder) -> Result<String, String> {
        let path = "/orders";
        let url = format!("{}{}", self.config.api_base, path);

        let body = serde_json::to_string(&order)
            .map_err(|e| format!("Failed to serialize order: {}", e))?;

        let headers = self.generate_auth_headers("POST", path, &body)?;

        let mut request = self.client.post(&url);
        for (key, value) in headers {
            request = request.header(&key, value);
        }

        let response = request
            .body(body)
            .send()
            .await
            .map_err(|e| format!("Failed to place order: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!(
                "Order placement failed: {} - {}",
                status, error_text
            ));
        }

        let order_response: CoinbaseOrderResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse order response: {}", e))?;

        info!("Order placed successfully: {}", order_response.id);
        Ok(order_response.id)
    }

    /// Cancel order on Coinbase
    pub async fn cancel_order(&self, order_id: &str) -> Result<(), String> {
        let path = &format!("/orders/{}", order_id);
        let url = format!("{}{}", self.config.api_base, path);

        let headers = self.generate_auth_headers("DELETE", path, "")?;

        let mut request = self.client.delete(&url);
        for (key, value) in headers {
            request = request.header(&key, value);
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Failed to cancel order: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!(
                "Order cancellation failed: {} - {}",
                status, error_text
            ));
        }

        info!("Order cancelled successfully: {}", order_id);
        Ok(())
    }

    /// Get order status from Coinbase
    pub async fn get_order_status(&self, order_id: &str) -> Result<String, String> {
        let path = &format!("/orders/{}", order_id);
        let url = format!("{}{}", self.config.api_base, path);

        let headers = self.generate_auth_headers("GET", path, "")?;

        let mut request = self.client.get(&url);
        for (key, value) in headers {
            request = request.header(&key, value);
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Failed to get order status: {}", e))?;

        if !response.status().is_success() {
            if response.status() == reqwest::StatusCode::NOT_FOUND {
                return Ok("NOT_FOUND".to_string());
            }
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!(
                "Failed to get order status: {} - {}",
                status, error_text
            ));
        }

        let order_response: CoinbaseOrderResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse order status response: {}", e))?;

        Ok(order_response.status)
    }

    /// Helper to convert Coinbase order status to our OrderStatus enum
    fn parse_order_status(status_str: &str) -> OrderStatus {
        match status_str.to_lowercase().as_str() {
            "pending" | "open" | "active" => OrderStatus::Pending,
            "filled" | "done" => OrderStatus::Filled,
            "cancelled" => OrderStatus::Cancelled,
            "rejected" => OrderStatus::Rejected,
            _ => OrderStatus::Unknown,
        }
    }
}

/// Implementation of ExchangeClient trait for Coinbase Pro
#[async_trait]
impl ExchangeClient for CoinbaseClient {
    fn name(&self) -> &str {
        "Coinbase Pro"
    }

    async fn place_order(&self, order: &Order) -> ExchangeResult<String> {
        let coinbase_order = self
            .convert_order(order)
            .map_err(|e| ExchangeError::InvalidOrder(e))?;

        CoinbaseClient::place_order(self, coinbase_order)
            .await
            .map_err(|e| ExchangeError::OrderPlacementFailed(e))
    }

    async fn cancel_order(&self, order_id: &str) -> ExchangeResult<()> {
        CoinbaseClient::cancel_order(self, order_id)
            .await
            .map_err(|e| ExchangeError::OrderCancellationFailed(e))
    }

    async fn get_order_status(&self, order_id: &str) -> ExchangeResult<OrderStatus> {
        let status_str = CoinbaseClient::get_order_status(self, order_id)
            .await
            .map_err(|e| ExchangeError::OrderStatusFailed(e))?;

        Ok(Self::parse_order_status(&status_str))
    }

    async fn get_balance(&self, currency: Option<&str>) -> ExchangeResult<Vec<Balance>> {
        let accounts = self
            .get_accounts()
            .await
            .map_err(|e| ExchangeError::BalanceQueryFailed(e))?;

        let balances: Vec<Balance> = accounts
            .iter()
            .filter(|acc| currency.is_none() || currency == Some(&acc.currency))
            .map(|acc| Balance {
                currency: acc.currency.clone(),
                available: acc.available.parse::<f64>().unwrap_or(0.0),
                total: acc.balance.parse::<f64>().unwrap_or(0.0),
            })
            .collect();

        Ok(balances)
    }

    async fn is_healthy(&self) -> bool {
        // Try to get accounts as health check
        self.get_accounts().await.is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::order::{Order, OrderSide, OrderType};

    #[test]
    fn test_coinbase_config_new() {
        let config = CoinbaseConfig::new("key", "secret", Some("passphrase"), false);
        assert_eq!(config.api_base, COINBASE_API_BASE);
        assert_eq!(config.api_key, "key");
        assert_eq!(config.api_secret, "secret");
        assert_eq!(config.passphrase, Some("passphrase".to_string()));
    }

    #[test]
    fn test_coinbase_config_new_without_passphrase() {
        let config = CoinbaseConfig::new("key", "secret", None, false);
        assert_eq!(config.api_base, COINBASE_API_BASE);
        assert_eq!(config.api_key, "key");
        assert_eq!(config.api_secret, "secret");
        assert_eq!(config.passphrase, None);
    }

    #[test]
    fn test_coinbase_config_sandbox() {
        let config = CoinbaseConfig::new("key", "secret", Some("passphrase"), true);
        assert_eq!(config.api_base, COINBASE_SANDBOX_BASE);
    }

    #[test]
    fn test_normalize_product_id() {
        let config = CoinbaseConfig::new("key", "secret", Some("passphrase"), false);
        let client = CoinbaseClient {
            client: Client::new(),
            config,
        };

        assert_eq!(client.normalize_product_id("BTC-USD").unwrap(), "BTC-USD");
        assert_eq!(client.normalize_product_id("ETH-USD").unwrap(), "ETH-USD");
        assert!(client.normalize_product_id("UNKNOWN").is_err());
    }

    #[test]
    fn test_convert_market_order() {
        let config = CoinbaseConfig::new("key", "secret", Some("passphrase"), false);
        let client = CoinbaseClient {
            client: Client::new(),
            config,
        };

        let order = Order::new(
            "test_order".to_string(),
            "BTC-USD".to_string(),
            OrderSide::Buy,
            OrderType::Market,
            None,
            0.01,
        )
        .unwrap();

        let result = client.convert_order(&order);
        assert!(result.is_ok());

        let coinbase_order = result.unwrap();
        assert_eq!(coinbase_order.product_id, "BTC-USD");
        assert_eq!(coinbase_order.side, "buy");
        assert_eq!(coinbase_order.size, "0.01");
        assert_eq!(coinbase_order.r#type, "market");
        assert!(coinbase_order.price.is_none());
    }

    #[test]
    fn test_convert_limit_order() {
        let config = CoinbaseConfig::new("key", "secret", Some("passphrase"), false);
        let client = CoinbaseClient {
            client: Client::new(),
            config,
        };

        let order = Order::new(
            "test_order".to_string(),
            "ETH-USD".to_string(),
            OrderSide::Sell,
            OrderType::Limit,
            Some(3000.0),
            0.5,
        )
        .unwrap();

        let result = client.convert_order(&order);
        assert!(result.is_ok());

        let coinbase_order = result.unwrap();
        assert_eq!(coinbase_order.product_id, "ETH-USD");
        assert_eq!(coinbase_order.side, "sell");
        assert_eq!(coinbase_order.size, "0.5");
        assert_eq!(coinbase_order.r#type, "limit");
        assert_eq!(coinbase_order.price, Some("3000".to_string()));
    }

    #[test]
    fn test_create_limit_order_without_price_fails() {
        // Test that creating a limit order without price fails at the domain level
        let result = Order::new(
            "test_order".to_string(),
            "BTC-USD".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            None, // No price for limit order
            1.0,
        );

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Limit orders must have a price"));
    }

    #[test]
    fn test_generate_auth_headers() {
        let config = CoinbaseConfig::new("test_key", "test_secret", Some("test_passphrase"), false);
        let client = CoinbaseClient {
            client: Client::new(),
            config,
        };

        let headers = client.generate_auth_headers("GET", "/test", "").unwrap();

        assert!(headers.contains_key("CB-ACCESS-KEY"));
        assert!(headers.contains_key("CB-ACCESS-SIGN"));
        assert!(headers.contains_key("CB-ACCESS-TIMESTAMP"));
        assert!(headers.contains_key("CB-ACCESS-PASSPHRASE"));
        assert!(headers.contains_key("Content-Type"));

        assert_eq!(headers.get("CB-ACCESS-KEY").unwrap(), "test_key");
        assert_eq!(
            headers.get("CB-ACCESS-PASSPHRASE").unwrap(),
            "test_passphrase"
        );
        assert_eq!(headers.get("Content-Type").unwrap(), "application/json");
    }

    #[test]
    fn test_coinbase_client_new() {
        let client = CoinbaseClient::new("key", "secret", Some("passphrase"));
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.config.api_key, "key");
        assert_eq!(client.config.api_secret, "secret");
        assert_eq!(client.config.passphrase, Some("passphrase".to_string()));
        assert_eq!(client.config.api_base, COINBASE_API_BASE);
    }

    #[test]
    fn test_coinbase_client_new_without_passphrase() {
        let client = CoinbaseClient::new("key", "secret", None);
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.config.api_key, "key");
        assert_eq!(client.config.api_secret, "secret");
        assert_eq!(client.config.passphrase, None);
        assert_eq!(client.config.api_base, COINBASE_API_BASE);
    }

    #[test]
    fn test_coinbase_client_new_sandbox() {
        let client = CoinbaseClient::new_sandbox("key", "secret", Some("passphrase"));
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.config.api_key, "key");
        assert_eq!(client.config.api_secret, "secret");
        assert_eq!(client.config.passphrase, Some("passphrase".to_string()));
        assert_eq!(client.config.api_base, COINBASE_SANDBOX_BASE);
    }

    #[test]
    fn test_normalize_product_id_unsupported() {
        let config = CoinbaseConfig::new("key", "secret", Some("passphrase"), false);
        let client = CoinbaseClient {
            client: Client::new(),
            config,
        };

        let result = client.normalize_product_id("UNSUPPORTED");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported product"));
    }

    #[test]
    fn test_convert_order_sell_side() {
        let config = CoinbaseConfig::new("key", "secret", Some("passphrase"), false);
        let client = CoinbaseClient {
            client: Client::new(),
            config,
        };

        let order = Order::new(
            "test_order".to_string(),
            "ETH-USD".to_string(),
            OrderSide::Sell,
            OrderType::Market,
            None,
            0.5,
        )
        .unwrap();

        let result = client.convert_order(&order);
        assert!(result.is_ok());

        let coinbase_order = result.unwrap();
        assert_eq!(coinbase_order.product_id, "ETH-USD");
        assert_eq!(coinbase_order.side, "sell");
        assert_eq!(coinbase_order.size, "0.5");
        assert_eq!(coinbase_order.r#type, "market");
    }

    #[test]
    fn test_coinbase_order_serialization() {
        let order = CoinbaseOrder {
            product_id: "BTC-USD".to_string(),
            side: "buy".to_string(),
            size: "0.01".to_string(),
            price: Some("50000.00".to_string()),
            r#type: "limit".to_string(),
            time_in_force: Some("GTC".to_string()),
            cancel_after: None,
            post_only: Some(false),
        };

        let json = serde_json::to_string(&order).unwrap();
        let deserialized: CoinbaseOrder = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.product_id, order.product_id);
        assert_eq!(deserialized.side, order.side);
        assert_eq!(deserialized.size, order.size);
        assert_eq!(deserialized.price, order.price);
        assert_eq!(deserialized.r#type, order.r#type);
    }

    #[test]
    fn test_coinbase_order_response_deserialization() {
        let json = r#"{
            "id": "test-order-id",
            "product_id": "BTC-USD",
            "side": "buy",
            "size": "0.01",
            "price": "50000.00",
            "status": "pending",
            "settled": false,
            "created_at": "2023-01-01T00:00:00Z"
        }"#;

        let response: CoinbaseOrderResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, "test-order-id");
        assert_eq!(response.product_id, "BTC-USD");
        assert_eq!(response.side, "buy");
        assert_eq!(response.size, "0.01");
        assert_eq!(response.price, Some("50000.00".to_string()));
        assert_eq!(response.status, "pending");
        assert_eq!(response.settled, false);
    }

    #[test]
    fn test_coinbase_account_deserialization() {
        let json = r#"{
            "id": "test-account-id",
            "currency": "BTC",
            "balance": "1.5",
            "available": "1.0",
            "hold": "0.5"
        }"#;

        let account: CoinbaseAccount = serde_json::from_str(json).unwrap();
        assert_eq!(account.id, "test-account-id");
        assert_eq!(account.currency, "BTC");
        assert_eq!(account.balance, "1.5");
        assert_eq!(account.available, "1.0");
        assert_eq!(account.hold, "0.5");
    }

    #[tokio::test]
    async fn test_get_accounts_with_invalid_credentials() {
        let client =
            CoinbaseClient::new("invalid_key", "invalid_secret", Some("invalid_passphrase"))
                .unwrap();

        // This will fail with authentication error, but tests that the method exists and can be called
        let result = client.get_accounts().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_place_order_with_invalid_credentials() {
        let client =
            CoinbaseClient::new("invalid_key", "invalid_secret", Some("invalid_passphrase"))
                .unwrap();

        let order = CoinbaseOrder {
            product_id: "BTC-USD".to_string(),
            side: "buy".to_string(),
            size: "0.01".to_string(),
            price: Some("50000.00".to_string()),
            r#type: "limit".to_string(),
            time_in_force: Some("GTC".to_string()),
            cancel_after: None,
            post_only: Some(false),
        };

        let result = client.place_order(order).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cancel_order_with_invalid_credentials() {
        let client =
            CoinbaseClient::new("invalid_key", "invalid_secret", Some("invalid_passphrase"))
                .unwrap();

        let result = client.cancel_order("invalid-order-id").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_order_status_with_invalid_credentials() {
        let client =
            CoinbaseClient::new("invalid_key", "invalid_secret", Some("invalid_passphrase"))
                .unwrap();

        let result = client.get_order_status("invalid-order-id").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_order_status() {
        assert_eq!(
            CoinbaseClient::parse_order_status("pending"),
            OrderStatus::Pending
        );
        assert_eq!(
            CoinbaseClient::parse_order_status("OPEN"),
            OrderStatus::Pending
        );
        assert_eq!(
            CoinbaseClient::parse_order_status("active"),
            OrderStatus::Pending
        );
        assert_eq!(
            CoinbaseClient::parse_order_status("filled"),
            OrderStatus::Filled
        );
        assert_eq!(
            CoinbaseClient::parse_order_status("DONE"),
            OrderStatus::Filled
        );
        assert_eq!(
            CoinbaseClient::parse_order_status("cancelled"),
            OrderStatus::Cancelled
        );
        assert_eq!(
            CoinbaseClient::parse_order_status("rejected"),
            OrderStatus::Rejected
        );
        assert_eq!(
            CoinbaseClient::parse_order_status("unknown_status"),
            OrderStatus::Unknown
        );
    }
}
