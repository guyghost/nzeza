//! # Coinbase Advanced Trade API Client
//!
//! This module provides a client for the Coinbase Advanced Trade API.
//! It uses JWT authentication with ES256 (ECDSA P-256) signing.
//!
//! ## Authentication
//!
//! The Advanced Trade API uses:
//! - API Key (format: "organizations/{org_id}/apiKeys/{key_id}")
//! - API Secret (PEM-encoded EC private key)
//! - JWT tokens with ES256 algorithm
//!
//! ## References
//!
//! - API Documentation: https://docs.cdp.coinbase.com/advanced-trade/docs/welcome
//! - Authentication: https://docs.cloud.coinbase.com/advanced-trade/docs/rest-api-auth

use crate::domain::entities::order::{Order, OrderSide, OrderType};
use chrono::Utc;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use p256::pkcs8::{DecodePrivateKey, EncodePrivateKey};
use p256::SecretKey;
use rand::RngCore;
use rand::rngs::OsRng;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::info;
use zeroize::Zeroizing;

/// Coinbase Advanced Trade API base URL
const COINBASE_API_BASE: &str = "https://api.coinbase.com";

/// JWT Claims for Coinbase Advanced Trade API
#[derive(Debug, Serialize, Deserialize)]
struct JwtClaims {
    sub: String,        // API key name
    iss: String,        // Always "coinbase-cloud"
    nbf: u64,           // Not before (current time)
    exp: u64,           // Expiration (current time + 2 minutes)
    uri: String,        // Request URI (method + path)
}

/// JWT Header for Coinbase Advanced Trade API
/// Note: jsonwebtoken crate handles header generation, so this is kept for reference
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
struct JwtHeader {
    kid: String,        // API key name
    nonce: String,      // Random hex string
    #[serde(rename = "typ")]
    typ: String,        // "JWT"
    alg: String,        // "ES256"
}

/// Coinbase Advanced Trade API configuration
#[derive(Debug, Clone)]
pub struct CoinbaseAdvancedConfig {
    pub api_base: String,
}

impl Default for CoinbaseAdvancedConfig {
    fn default() -> Self {
        Self {
            api_base: COINBASE_API_BASE.to_string(),
        }
    }
}

/// Coinbase Advanced Trade API client
pub struct CoinbaseAdvancedClient {
    client: Client,
    config: CoinbaseAdvancedConfig,
    api_key: String,
    api_secret: Zeroizing<String>,
}

impl std::fmt::Debug for CoinbaseAdvancedClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoinbaseAdvancedClient")
            .field("config", &self.config)
            .field("api_key", &self.api_key)
            .field("api_secret", &"<REDACTED>")
            .finish()
    }
}

/// Account information from Coinbase
#[derive(Debug, Serialize, Deserialize)]
pub struct CoinbaseAccount {
    pub uuid: String,
    pub name: String,
    pub currency: String,
    pub available_balance: CoinbaseBalance,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CoinbaseBalance {
    pub value: String,
    pub currency: String,
}

/// Account list response
#[derive(Debug, Serialize, Deserialize)]
pub struct AccountsResponse {
    pub accounts: Vec<CoinbaseAccount>,
    pub has_next: bool,
    pub cursor: String,
    pub size: i32,
}

/// Order request for Coinbase Advanced Trade API
#[derive(Debug, Serialize, Deserialize)]
pub struct CoinbaseOrderRequest {
    pub client_order_id: String,
    pub product_id: String,
    pub side: String,
    pub order_configuration: OrderConfiguration,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OrderConfiguration {
    MarketMarketIoc {
        market_market_ioc: MarketMarketIoc,
    },
    LimitLimitGtc {
        limit_limit_gtc: LimitLimitGtc,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketMarketIoc {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote_size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_size: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LimitLimitGtc {
    pub base_size: String,
    pub limit_price: String,
    pub post_only: bool,
}

/// Order response from Coinbase
#[derive(Debug, Serialize, Deserialize)]
pub struct CoinbaseOrderResponse {
    pub success: bool,
    pub order_id: String,
    pub success_response: Option<SuccessResponse>,
    pub error_response: Option<ErrorResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SuccessResponse {
    pub order_id: String,
    pub product_id: String,
    pub side: String,
    pub client_order_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub error_details: String,
}

/// Order status response
#[derive(Debug, Serialize, Deserialize)]
pub struct OrderStatusResponse {
    pub order: OrderDetails,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderDetails {
    pub order_id: String,
    pub product_id: String,
    pub side: String,
    pub status: String,
}

impl CoinbaseAdvancedClient {
    /// Create a new Coinbase Advanced Trade API client
    ///
    /// # Arguments
    /// * `api_key` - API key in format "organizations/{org_id}/apiKeys/{key_id}"
    /// * `api_secret` - PEM-encoded EC private key
    pub fn new(api_key: &str, api_secret: &str) -> Result<Self, String> {
        Self::new_with_config(api_key, api_secret, CoinbaseAdvancedConfig::default())
    }

    /// Create a new client with custom configuration
    pub fn new_with_config(
        api_key: &str,
        api_secret: &str,
        config: CoinbaseAdvancedConfig,
    ) -> Result<Self, String> {
        // Validate API key format
        if !api_key.starts_with("organizations/") || !api_key.contains("/apiKeys/") {
            return Err(format!(
                "Invalid API key format. Expected 'organizations/{{org_id}}/apiKeys/{{key_id}}', got: {}",
                api_key
            ));
        }

        // Validate API secret (should be PEM-encoded)
        if !api_secret.contains("BEGIN EC PRIVATE KEY") && !api_secret.contains("BEGIN PRIVATE KEY") {
            return Err(
                "Invalid API secret format. Expected PEM-encoded EC private key".to_string()
            );
        }

        let client = Client::builder()
            .user_agent("NZEZA-Trading-Bot/0.1.0")
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            client,
            config,
            api_key: api_key.to_string(),
            api_secret: Zeroizing::new(api_secret.to_string()),
        })
    }

    /// Generate a JWT token for authentication
    fn generate_jwt(&self, method: &str, path: &str) -> Result<String, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("Failed to get current time: {}", e))?
            .as_secs();

        // Build URI (method + host + path)
        let uri = format!("{} api.coinbase.com{}", method, path);

        // Build JWT claims
        let claims = JwtClaims {
            sub: self.api_key.clone(),
            iss: "coinbase-cloud".to_string(),
            nbf: now,
            exp: now + 120, // 2 minutes expiration
            uri,
        };

        // Generate random nonce (not used in this JWT implementation, kept for reference)
        let mut _nonce_bytes = [0u8; 16];
        OsRng.fill_bytes(&mut _nonce_bytes);
        let _nonce = hex::encode(_nonce_bytes);

        // Build JWT header with custom fields
        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(self.api_key.clone());

        // Parse the EC private key from PEM and convert to DER
        let secret_key = SecretKey::from_pkcs8_pem(self.api_secret.as_str())
            .map_err(|e| format!("Failed to parse EC private key from PEM: {}. Make sure the key is in PKCS#8 PEM format.", e))?;

        let der_bytes = secret_key.to_pkcs8_der()
            .map_err(|e| format!("Failed to convert EC key to DER: {}", e))?;

        let encoding_key = EncodingKey::from_ec_der(der_bytes.as_bytes());

        // Encode JWT
        let token = encode(&header, &claims, &encoding_key)
            .map_err(|e| format!("Failed to encode JWT: {}", e))?;

        Ok(token)
    }

    /// Get accounts
    pub async fn get_accounts(&self) -> Result<Vec<CoinbaseAccount>, String> {
        let path = "/api/v3/brokerage/accounts";
        let jwt = self.generate_jwt("GET", path)?;

        let url = format!("{}{}", self.config.api_base, path);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", jwt))
            .send()
            .await
            .map_err(|e| format!("Failed to get accounts: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API error {}: {}", status, error_text));
        }

        let accounts_response: AccountsResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse accounts response: {}", e))?;

        Ok(accounts_response.accounts)
    }

    /// Convert our Order to Coinbase format
    pub fn convert_order(&self, order: &Order) -> Result<CoinbaseOrderRequest, String> {
        let product_id = self.normalize_product_id(&order.symbol)?;
        let side = match order.side {
            OrderSide::Buy => "BUY",
            OrderSide::Sell => "SELL",
        }
        .to_string();

        let client_order_id = format!("nzeza_{}", Utc::now().timestamp_millis());

        let order_configuration = match order.order_type {
            OrderType::Market => {
                // For market orders, use base_size (amount of crypto to buy/sell)
                OrderConfiguration::MarketMarketIoc {
                    market_market_ioc: MarketMarketIoc {
                        base_size: Some(order.quantity.value().to_string()),
                        quote_size: None,
                    },
                }
            }
            OrderType::Limit => {
                let price = order
                    .price
                    .ok_or("Limit order must have price")?
                    .value()
                    .to_string();

                OrderConfiguration::LimitLimitGtc {
                    limit_limit_gtc: LimitLimitGtc {
                        base_size: order.quantity.value().to_string(),
                        limit_price: price,
                        post_only: false,
                    },
                }
            }
        };

        Ok(CoinbaseOrderRequest {
            client_order_id,
            product_id,
            side,
            order_configuration,
        })
    }

    /// Place order on Coinbase
    pub async fn place_order(&self, order: CoinbaseOrderRequest) -> Result<String, String> {
        let path = "/api/v3/brokerage/orders";
        let jwt = self.generate_jwt("POST", path)?;

        let url = format!("{}{}", self.config.api_base, path);

        info!(
            "Placing Coinbase order: {} {} {}",
            order.side, order.product_id, order.client_order_id
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", jwt))
            .json(&order)
            .send()
            .await
            .map_err(|e| format!("Failed to place order: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Order placement failed {}: {}", status, error_text));
        }

        let order_response: CoinbaseOrderResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse order response: {}", e))?;

        if !order_response.success {
            if let Some(error) = order_response.error_response {
                return Err(format!("Order failed: {} - {}", error.error, error.message));
            }
            return Err("Order failed with unknown error".to_string());
        }

        info!("Order placed successfully: {}", order_response.order_id);
        Ok(order_response.order_id)
    }

    /// Cancel order
    pub async fn cancel_order(&self, order_id: &str) -> Result<(), String> {
        let path = format!("/api/v3/brokerage/orders/batch_cancel");
        let jwt = self.generate_jwt("POST", &path)?;

        let url = format!("{}{}", self.config.api_base, path);

        let cancel_request = serde_json::json!({
            "order_ids": [order_id]
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", jwt))
            .json(&cancel_request)
            .send()
            .await
            .map_err(|e| format!("Failed to cancel order: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Order cancellation failed {}: {}", status, error_text));
        }

        info!("Order cancelled successfully: {}", order_id);
        Ok(())
    }

    /// Get order status
    pub async fn get_order_status(&self, order_id: &str) -> Result<String, String> {
        let path = format!("/api/v3/brokerage/orders/historical/{}", order_id);
        let jwt = self.generate_jwt("GET", &path)?;

        let url = format!("{}{}", self.config.api_base, path);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", jwt))
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
                "Failed to get order status {}: {}",
                status, error_text
            ));
        }

        let order_status: OrderStatusResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse order status response: {}", e))?;

        Ok(order_status.order.status)
    }

    /// Normalize product ID to Coinbase format
    fn normalize_product_id(&self, symbol: &str) -> Result<String, String> {
        // Coinbase uses format like "BTC-USD", "ETH-USD"
        let normalized = symbol.to_uppercase();
        if normalized.contains('-') {
            Ok(normalized)
        } else {
            // Try to split common pairs
            if normalized.ends_with("USD") {
                let base = &normalized[..normalized.len() - 3];
                Ok(format!("{}-USD", base))
            } else if normalized.ends_with("USDT") {
                let base = &normalized[..normalized.len() - 4];
                Ok(format!("{}-USDT", base))
            } else {
                Err(format!("Cannot normalize symbol: {}", symbol))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_product_id() {
        let config = CoinbaseAdvancedConfig::default();
        let client =
            CoinbaseAdvancedClient::new_with_config("organizations/test/apiKeys/test", "-----BEGIN EC PRIVATE KEY-----\ntest\n-----END EC PRIVATE KEY-----", config)
                .unwrap();

        assert_eq!(
            client.normalize_product_id("BTC-USD").unwrap(),
            "BTC-USD"
        );
        assert_eq!(
            client.normalize_product_id("BTCUSD").unwrap(),
            "BTC-USD"
        );
        assert_eq!(
            client.normalize_product_id("ETH-USD").unwrap(),
            "ETH-USD"
        );
    }

    #[test]
    fn test_config_default() {
        let config = CoinbaseAdvancedConfig::default();
        assert_eq!(config.api_base, COINBASE_API_BASE);
    }

    #[test]
    fn test_invalid_api_key_format() {
        let result = CoinbaseAdvancedClient::new("invalid_key", "-----BEGIN EC PRIVATE KEY-----\ntest\n-----END EC PRIVATE KEY-----");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Invalid API key format"));
    }

    #[test]
    fn test_invalid_api_secret_format() {
        let result =
            CoinbaseAdvancedClient::new("organizations/test/apiKeys/test", "invalid_secret");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Invalid API secret format"));
    }
}
