//! # dYdX v4 Client Module
//!
//! ⚠️ **CRITICAL WARNING**: This module contains an INCOMPLETE implementation
//! of dYdX v4 integration that will NOT work in production.
//!
//! ## Current Limitations
//!
//! - dYdX v4 is a Cosmos-based blockchain, NOT Ethereum-based
//! - This code incorrectly uses EIP-712-style signing which dYdX v4 does NOT support
//! - Orders placed through this client WILL BE REJECTED by the dYdX v4 API
//!
//! ## Required Changes for Production
//!
//! To properly integrate with dYdX v4, you must:
//! 1. Replace ethers wallet with Cosmos SDK signing (cosmrs crate)
//! 2. Use protobuf message encoding (prost, dydx-proto)
//! 3. Implement proper transaction signing with the dYdX Chain
//! 4. Ideally, use the official v4-client-rs from https://github.com/dydxprotocol/v4-clients
//!
//! ## References
//!
//! - Official dYdX v4 Docs: https://docs.dydx.xyz/
//! - v4 Client Rust: https://github.com/dydxprotocol/v4-clients/tree/main/v4-client-rs
//!
//! **DO NOT USE THIS CODE FOR REAL TRADING WITHOUT FIXING THE SIGNING MECHANISM**

use crate::domain::entities::order::{Order, OrderSide, OrderType};
use ethers::signers::{LocalWallet, MnemonicBuilder, coins_bip39::English, Signer};
use ethers::types::{Address, Signature, H256};
use ethers::utils::keccak256;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;
use zeroize::Zeroizing;

/// dYdX v4 API endpoints
const DYDX_API_BASE: &str = "https://api.dydx.exchange";
const DYDX_INDEXER_BASE: &str = "https://indexer.dydx.trade";

/// dYdX network configuration
#[derive(Debug, Clone)]
pub struct DydxConfig {
    pub api_base: String,
    pub indexer_base: String,
    pub network_id: u32,
}

impl Default for DydxConfig {
    fn default() -> Self {
        Self {
            api_base: DYDX_API_BASE.to_string(),
            indexer_base: DYDX_INDEXER_BASE.to_string(),
            network_id: 1, // mainnet
        }
    }
}

/// dYdX order structure for API
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DydxOrder {
    pub market: String,
    pub side: String,
    pub size: String,
    pub price: String,
    pub r#type: String,
    pub time_in_force: String,
    pub expiration: String,
    pub client_id: String,
    pub signature: String,
    pub reduce_only: bool,
    pub post_only: bool,
}

/// dYdX API response for order placement
#[derive(Debug, Serialize, Deserialize)]
pub struct DydxOrderResponse {
    pub order: DydxOrderDetails,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DydxOrderDetails {
    pub id: String,
    pub client_id: String,
    pub status: String,
}

/// dYdX account information
#[derive(Debug, Serialize, Deserialize)]
pub struct DydxAccount {
    pub account: DydxAccountDetails,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DydxAccountDetails {
    pub subaccounts: Vec<DydxSubaccount>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DydxSubaccount {
    pub subaccount_number: u32,
    pub equity: String,
    pub free_collateral: String,
}

/// dYdX client for API interactions
pub struct DydxClient {
    client: Client,
    config: DydxConfig,
    wallet: LocalWallet,
    sequence_number: Arc<Mutex<u64>>,
}

impl DydxClient {
    /// Create a new dYdX client from mnemonic
    ///
    /// # Security Note
    /// The mnemonic is temporarily stored in memory during wallet creation.
    /// While we use Zeroizing for the input, the ethers library may still
    /// retain copies. For production, consider using hardware wallets or
    /// secure enclaves.
    pub fn new(mnemonic: &str, config: DydxConfig) -> Result<Self, String> {
        // Use zeroizing string to reduce mnemonic exposure time
        let zeroizing_mnemonic = Zeroizing::new(mnemonic.to_string());

        let wallet = MnemonicBuilder::<English>::default()
            .phrase(zeroizing_mnemonic.as_str())
            .build()
            .map_err(|e| format!("Failed to create wallet from mnemonic: {}", e))?;

        // mnemonic is automatically zeroized when zeroizing_mnemonic goes out of scope

        Ok(Self {
            client: Client::new(),
            config,
            wallet,
            sequence_number: Arc::new(Mutex::new(0)),
        })
    }

    /// Get the wallet address
    pub fn address(&self) -> Address {
        self.wallet.address()
    }

    /// Update sequence number from account info
    pub async fn update_sequence_number(&self) -> Result<(), String> {
        let account_info = self.get_account_info().await?;
        let mut seq = self.sequence_number.lock().await;
        *seq = account_info.account.subaccounts.first()
            .map(|sub| sub.subaccount_number as u64)
            .unwrap_or(0);
        Ok(())
    }

    /// Get next sequence number
    pub async fn next_sequence_number(&self) -> u64 {
        let mut seq = self.sequence_number.lock().await;
        let current = *seq;
        *seq += 1;
        current
    }

    /// Get account information from dYdX API
    pub async fn get_account_info(&self) -> Result<DydxAccount, String> {
        let url = format!("{}/v4/accounts/{}", self.config.api_base, self.wallet.address());

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to get account info: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("API error: {}", response.status()));
        }

        let account: DydxAccount = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse account response: {}", e))?;

        Ok(account)
    }

    /// Convert our Order to dYdX API format
    pub async fn convert_order(&self, order: &Order) -> Result<DydxOrder, String> {
        let market = self.normalize_market(&order.symbol)?;
        let side = match order.side {
            OrderSide::Buy => "BUY",
            OrderSide::Sell => "SELL",
        }.to_string();

        let size = order.quantity.value().to_string();

        let price = match order.order_type {
            OrderType::Market => "0".to_string(),
            OrderType::Limit => order.price
                .map(|p| p.value().to_string())
                .ok_or("Limit order must have price")?,
        };

        let order_type = match order.order_type {
            OrderType::Market => "MARKET",
            OrderType::Limit => "LIMIT",
        }.to_string();

        let client_id = format!("client_{}", chrono::Utc::now().timestamp_millis());

        // Create order hash for signing
        let order_hash = self.create_order_hash(&market, &side, &size, &price, &client_id)?;

        // Sign the order
        let signature = self.sign_order(&order_hash).await?;

        let expiration = (chrono::Utc::now() + chrono::Duration::minutes(5))
            .timestamp()
            .to_string();

        Ok(DydxOrder {
            market,
            side,
            size,
            price,
            r#type: order_type,
            time_in_force: "GTT".to_string(), // Good 'Til Time
            expiration,
            client_id,
            signature: format!("0x{}", hex::encode(signature.to_vec())),
            reduce_only: false,
            post_only: false,
        })
    }

    /// Create order hash for signing
    ///
    /// ⚠️ CRITICAL WARNING: This implementation is INCOMPLETE and will NOT work with dYdX v4.
    ///
    /// dYdX v4 is a Cosmos-based blockchain and does NOT use EIP-712 signing.
    /// This simplified hash is a placeholder only.
    ///
    /// To properly integrate with dYdX v4, you MUST:
    /// 1. Use the official Cosmos signing mechanism with protobuf messages
    /// 2. Integrate with cosmos-sdk-proto and cosmrs crates
    /// 3. Use the official v4-client-rs when dependency issues are resolved
    ///
    /// Current status: Orders placed through this client WILL BE REJECTED by dYdX v4 API.
    fn create_order_hash(&self, market: &str, side: &str, size: &str, price: &str, client_id: &str) -> Result<H256, String> {
        // ⚠️ PLACEHOLDER ONLY - NOT VALID FOR dYdX v4
        let message = format!("{}:{}:{}:{}:{}", market, side, size, price, client_id);
        let hash = keccak256(message.as_bytes());
        Ok(H256::from(hash))
    }

    /// Sign order hash with wallet
    async fn sign_order(&self, hash: &H256) -> Result<Signature, String> {
        self.wallet.sign_hash(*hash)
            .map_err(|e| format!("Failed to sign order: {}", e))
    }

    /// Normalize market symbol to dYdX format
    fn normalize_market(&self, symbol: &str) -> Result<String, String> {
        match symbol {
            "BTC-USD" => Ok("BTC-USD".to_string()),
            "ETH-USD" => Ok("ETH-USD".to_string()),
            "SOL-USD" => Ok("SOL-USD".to_string()),
            _ => Err(format!("Unsupported market: {}", symbol)),
        }
    }

    /// Place order on dYdX
    pub async fn place_order(&self, order: DydxOrder) -> Result<String, String> {
        let url = format!("{}/v4/orders", self.config.api_base);

        let response = self.client
            .post(&url)
            .json(&order)
            .send()
            .await
            .map_err(|e| format!("Failed to place order: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Order placement failed: {} - {}", status, error_text));
        }

        let order_response: DydxOrderResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse order response: {}", e))?;

        info!("Order placed successfully: {}", order_response.order.id);
        Ok(order_response.order.id)
    }

    /// Cancel order on dYdX
    pub async fn cancel_order(&self, order_id: &str) -> Result<(), String> {
        let url = format!("{}/v4/orders/{}", self.config.api_base, order_id);

        let cancel_payload = serde_json::json!({
            "order_id": order_id
        });

        let response = self.client
            .delete(&url)
            .json(&cancel_payload)
            .send()
            .await
            .map_err(|e| format!("Failed to cancel order: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Order cancellation failed: {} - {}", status, error_text));
        }

        info!("Order cancelled successfully: {}", order_id);
        Ok(())
    }

    /// Get order status from dYdX
    pub async fn get_order_status(&self, order_id: &str) -> Result<String, String> {
        let url = format!("{}/v4/orders/{}", self.config.api_base, order_id);

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to get order status: {}", e))?;

        if !response.status().is_success() {
            if response.status() == reqwest::StatusCode::NOT_FOUND {
                return Ok("NOT_FOUND".to_string());
            }
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Failed to get order status: {} - {}", status, error_text));
        }

        let order_response: DydxOrderResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse order status response: {}", e))?;

        Ok(order_response.order.status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::order::{Order, OrderSide, OrderType};

    #[test]
    fn test_dydx_config_default() {
        let config = DydxConfig::default();
        assert_eq!(config.api_base, DYDX_API_BASE);
        assert_eq!(config.indexer_base, DYDX_INDEXER_BASE);
        assert_eq!(config.network_id, 1);
    }

    #[test]
    fn test_normalize_market() {
        let config = DydxConfig::default();
        let client = DydxClient::new("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about", config).unwrap();

        assert_eq!(client.normalize_market("BTC-USD").unwrap(), "BTC-USD");
        assert_eq!(client.normalize_market("ETH-USD").unwrap(), "ETH-USD");
        assert!(client.normalize_market("UNKNOWN").is_err());
    }

    #[test]
    fn test_create_order_hash() {
        let config = DydxConfig::default();
        let client = DydxClient::new("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about", config).unwrap();

        let hash = client.create_order_hash("BTC-USD", "BUY", "0.01", "50000", "client_123").unwrap();
        assert!(!hash.is_zero());
    }

    #[tokio::test]
    async fn test_convert_market_order() {
        let config = DydxConfig::default();
        let client = DydxClient::new("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about", config).unwrap();

        let order = Order::new(
            "test_order".to_string(),
            "BTC-USD".to_string(),
            OrderSide::Buy,
            OrderType::Market,
            None,
            0.01,
        ).unwrap();

        let result = client.convert_order(&order).await;
        assert!(result.is_ok());

        let dydx_order = result.unwrap();
        assert_eq!(dydx_order.market, "BTC-USD");
        assert_eq!(dydx_order.side, "BUY");
        assert_eq!(dydx_order.size, "0.01");
        assert_eq!(dydx_order.price, "0");
        assert_eq!(dydx_order.r#type, "MARKET");
        assert!(!dydx_order.signature.is_empty());
    }

    #[tokio::test]
    async fn test_convert_limit_order() {
        let config = DydxConfig::default();
        let client = DydxClient::new("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about", config).unwrap();

        let order = Order::new(
            "test_order".to_string(),
            "ETH-USD".to_string(),
            OrderSide::Sell,
            OrderType::Limit,
            Some(3000.0),
            0.5,
        ).unwrap();

        let result = client.convert_order(&order).await;
        assert!(result.is_ok());

        let dydx_order = result.unwrap();
        assert_eq!(dydx_order.market, "ETH-USD");
        assert_eq!(dydx_order.side, "SELL");
        assert_eq!(dydx_order.size, "0.5");
        assert_eq!(dydx_order.price, "3000");
        assert_eq!(dydx_order.r#type, "LIMIT");
        assert!(!dydx_order.signature.is_empty());
    }
}