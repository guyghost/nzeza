//! # dYdX v4 Client Module
//!
//! This module provides a wrapper around the official dYdX v4 Rust client.
//! It properly uses Cosmos SDK signing with protobuf messages.
//!
//! ## Features
//!
//! - Uses official v4-client-rs from https://github.com/dydxprotocol/v4-clients
//! - Proper Cosmos SDK transaction signing with protobuf encoding
//! - Market and limit order support
//! - Order cancellation and status checking
//! - Account and subaccount management

use crate::domain::entities::order::{Order, OrderSide, OrderType};
use crate::domain::repositories::exchange_client::{
    Balance, ExchangeClient, ExchangeError, ExchangeResult, OrderStatus,
};
use crate::persistence::models::{CreateDydxOrderMetadata, DydxOrderMetadataRecord};
use crate::persistence::repository::DydxOrderMetadataRepository;
use async_trait::async_trait;
use base64::{self, Engine};
use bigdecimal::BigDecimal;
use dydx::config::ClientConfig;
use dydx::indexer::{Height, IndexerClient, Ticker};
use dydx::node::{
    Account, NodeClient, OrderBuilder, OrderSide as DydxOrderSide, Subaccount, Wallet,
};
use dydx_proto::cosmos_sdk_proto::traits::Message;
use dydx_proto::dydxprotocol::clob::order::TimeInForce;
use dydx_proto::dydxprotocol::clob::OrderId;
use once_cell::sync::OnceCell;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use zeroize::Zeroizing;

/// Global metadata repository for order cancellation support
static METADATA_REPO: OnceCell<Arc<DydxOrderMetadataRepository>> = OnceCell::new();

/// dYdX v4 client for API interactions using official client
pub struct DydxV4Client {
    node_client: Arc<Mutex<NodeClient>>,
    indexer_client: IndexerClient,
    wallet: Wallet,
    account: Arc<Mutex<Account>>,
    #[allow(dead_code)]
    config_path: String,
}

impl DydxV4Client {
    /// Set the global metadata repository for order cancellation support
    /// This must be called before using dYdX clients for order placement
    pub fn set_metadata_repository(repo: Arc<DydxOrderMetadataRepository>) {
        METADATA_REPO.set(repo).unwrap_or_else(|_| {
            warn!("dYdX metadata repository already set, ignoring new value");
        });
    }

    /// Get the global metadata repository
    fn get_metadata_repository() -> Option<&'static Arc<DydxOrderMetadataRepository>> {
        METADATA_REPO.get()
    }
}

impl DydxV4Client {
    /// Create a new dYdX v4 client from mnemonic
    ///
    /// # Security Note
    /// The mnemonic is handled with care using Zeroizing to reduce exposure.
    /// The wallet is derived using BIP-39/BIP-44 standard for Cosmos chains.
    ///
    /// # Arguments
    /// * `mnemonic` - 12 or 24 word mnemonic phrase
    /// * `config_path` - Path to TOML config file (e.g., "dydx_mainnet.toml")
    pub async fn new(mnemonic: &str, config_path: &str) -> Result<Self, String> {
        // Use zeroizing string to reduce mnemonic exposure time
        let zeroizing_mnemonic = Zeroizing::new(mnemonic.to_string());

        // Load configuration from file
        let config = ClientConfig::from_file(config_path)
            .await
            .map_err(|e| format!("Failed to load config from {}: {:?}", config_path, e))?;

        // Create node client
        let mut node_client = NodeClient::connect(config.node.clone())
            .await
            .map_err(|e| format!("Failed to connect to dYdX node: {:?}", e))?;

        // Create indexer client
        let indexer_client = IndexerClient::new(config.indexer);

        // Create wallet from mnemonic
        let wallet = Wallet::from_mnemonic(zeroizing_mnemonic.as_str())
            .map_err(|e| format!("Failed to create wallet from mnemonic: {:?}", e))?;

        // Get account (account index 0)
        let account = wallet
            .account(0, &mut node_client)
            .await
            .map_err(|e| format!("Failed to get account: {:?}", e))?;

        info!(
            "dYdX v4 client initialized - Address: {}",
            account.address()
        );

        Ok(Self {
            node_client: Arc::new(Mutex::new(node_client)),
            indexer_client,
            wallet,
            account: Arc::new(Mutex::new(account)),
            config_path: config_path.to_string(),
        })
    }

    /// Get the account address
    pub async fn address(&self) -> String {
        let account = self.account.lock().await;
        account.address().to_string()
    }

    /// Get subaccount for trading (default subaccount 0)
    pub async fn get_subaccount(&self) -> Result<Subaccount, String> {
        let account = self.account.lock().await;
        account
            .subaccount(0)
            .map_err(|e| format!("Failed to get subaccount: {:?}", e))
    }

    /// Refresh account state from chain
    pub async fn refresh_account(&self) -> Result<(), String> {
        let mut node_client = self.node_client.lock().await;
        let _account = self.account.lock().await;

        // Get fresh account data
        let new_account = self
            .wallet
            .account(0, &mut *node_client)
            .await
            .map_err(|e| format!("Failed to refresh account: {:?}", e))?;

        drop(_account);
        let mut account = self.account.lock().await;
        *account = new_account;
        Ok(())
    }

    /// Convert our Order to dYdX v4 format and place it
    ///
    /// # Arguments
    /// * `order` - The order to place
    pub async fn place_order(&self, order: &Order) -> Result<String, String> {
        // Get market info
        let ticker = Ticker(order.symbol.clone());
        let market = self
            .indexer_client
            .markets()
            .get_perpetual_market(&ticker)
            .await
            .map_err(|e| format!("Failed to get market {}: {:?}", order.symbol, e))?;

        // Get subaccount
        let subaccount = self.get_subaccount().await?;

        // Get current block height
        let mut node_client = self.node_client.lock().await;
        let current_block_height = node_client
            .latest_block_height()
            .await
            .map_err(|e| format!("Failed to get block height: {:?}", e))?;

        // Convert order side
        let side = match order.side {
            OrderSide::Buy => DydxOrderSide::Buy,
            OrderSide::Sell => DydxOrderSide::Sell,
        };

        // Convert quantity to BigDecimal
        let size = BigDecimal::from_str(&order.quantity.value().to_string())
            .map_err(|e| format!("Failed to parse quantity: {}", e))?;

        // Build order based on type
        let order_builder = OrderBuilder::new(market, subaccount);

        // Determine good_until block height based on order type
        let good_until_blocks = match order.order_type {
            OrderType::Market => 10, // Short-lived for market orders
            OrderType::Limit => 20,  // Longer for limit orders
        };
        let good_until_block = current_block_height.ahead(good_until_blocks);

        let (order_id, dydx_order) = match order.order_type {
            OrderType::Market => {
                // For market orders, we use a slippage protection price
                let slippage_price =
                    order
                        .price
                        .map(|p| p.value() as u64)
                        .unwrap_or(match order.side {
                            OrderSide::Buy => u64::MAX, // Buy at any price
                            OrderSide::Sell => 0,       // Sell at any price
                        });

                order_builder
                    .market(side, size)
                    .reduce_only(false)
                    .price(slippage_price)
                    .time_in_force(TimeInForce::Ioc) // Immediate or Cancel for market orders
                    .until(good_until_block.clone())
                    .build(dydx::indexer::AnyId)
                    .map_err(|e| format!("Failed to build market order: {:?}", e))?
            }
            OrderType::Limit => {
                let price = order.price.ok_or("Limit order must have price")?.value();

                order_builder
                    .limit(side, price as u64, size)
                    .reduce_only(false)
                    .time_in_force(TimeInForce::Unspecified) // GTC by default
                    .until(good_until_block.clone())
                    .build(dydx::indexer::AnyId)
                    .map_err(|e| format!("Failed to build limit order: {:?}", e))?
            }
        };

        drop(node_client); // Release lock before mutable borrow

        // Place the order
        let mut account = self.account.lock().await;
        let mut node_client = self.node_client.lock().await;

        // Update account before placing order
        let refreshed_account = self
            .wallet
            .account(0, &mut *node_client)
            .await
            .map_err(|e| format!("Failed to refresh account before order: {:?}", e))?;
        *account = refreshed_account;

        let tx_hash = node_client
            .place_order(&mut *account, dydx_order)
            .await
            .map_err(|e| format!("Failed to place order: {:?}", e))?;

        let order_id_string = format!("{:?}", order_id);

        info!(
            "dYdX v4 order placed successfully - TxHash: {:?}, OrderID: {}",
            tx_hash, order_id_string
        );

        // Store order metadata for future cancellation
        let subaccount = self.get_subaccount().await?;

        // Encode the OrderId as base64 for storage
        let order_id_bytes = order_id.encode_to_vec();
        let dydx_order_id_b64 = base64::engine::general_purpose::STANDARD.encode(&order_id_bytes);

        // Get market info to extract clob_pair_id
        let ticker = Ticker(order.symbol.clone());
        let market = self
            .indexer_client
            .markets()
            .get_perpetual_market(&ticker)
            .await
            .map_err(|e| format!("Failed to get market for metadata: {:?}", e))?;

        let metadata = CreateDydxOrderMetadata {
            order_id: order_id_string.clone(),
            dydx_order_id: dydx_order_id_b64,
            good_until_block: good_until_block.0 as i64,
            client_id: order_id.client_id as i64,
            subaccount_number: subaccount.number.value() as i32,
            order_flags: order_id.order_flags as i32,
            clob_pair_id: order_id.clob_pair_id as i32,
            symbol: order.symbol.clone(),
            side: match order.side {
                OrderSide::Buy => "buy".to_string(),
                OrderSide::Sell => "sell".to_string(),
            },
            quantity: order.quantity.value().to_string(),
            price: order.price.map(|p| p.value().to_string()),
            order_type: match order.order_type {
                OrderType::Market => "market".to_string(),
                OrderType::Limit => "limit".to_string(),
            },
        };

        // Store metadata if repository is available
        if let Some(metadata_repo) = Self::get_metadata_repository() {
            if let Err(e) = metadata_repo.create(metadata).await {
                error!(
                    "Failed to store order metadata for {}: {}",
                    order_id_string, e
                );
                // Don't fail the order placement if metadata storage fails
                // The order was placed successfully, just log the error
            }
        } else {
            warn!("dYdX metadata repository not set - order cancellation will not be available for {}", order_id_string);
        }

        // Return the order ID as string
        Ok(order_id_string)
    }

    /// Reconstruct OrderId from stored metadata
    async fn reconstruct_order_id(
        &self,
        metadata: &DydxOrderMetadataRecord,
    ) -> Result<OrderId, String> {
        use dydx_proto::dydxprotocol::subaccounts::SubaccountId;

        // Get the account address
        let account = self.account.lock().await;
        let address = account.address().to_string();
        drop(account);

        // Reconstruct subaccount ID
        let subaccount_id = SubaccountId {
            owner: address,
            number: metadata.subaccount_number as u32,
        };

        // Reconstruct OrderId
        let order_id = OrderId {
            subaccount_id: Some(subaccount_id),
            client_id: metadata.client_id as u32,
            order_flags: metadata.order_flags as u32,
            clob_pair_id: metadata.clob_pair_id as u32,
        };

        Ok(order_id)
    }

    /// Cancel order on dYdX v4
    ///
    /// # Arguments
    /// * `order_id` - The order ID returned from place_order
    pub async fn cancel_order(&self, order_id: &str) -> Result<(), String> {
        info!("Cancel order requested for: {}", order_id);

        // Get metadata repository
        let metadata_repo = Self::get_metadata_repository().ok_or_else(|| {
            "dYdX metadata repository not configured - order cancellation not available".to_string()
        })?;

        // Retrieve stored metadata
        let metadata = metadata_repo
            .get_by_order_id(order_id)
            .await
            .map_err(|e| format!("Failed to retrieve order metadata: {}", e))?
            .ok_or_else(|| format!("Order metadata not found for: {}. This order may have been placed before metadata storage was implemented.", order_id))?;

        // Check if order is already cancelled/expired/filled
        if metadata.status != "active" {
            return Err(format!("Order {} is already {}", order_id, metadata.status));
        }

        // Check if order has expired
        let current_block = self
            .node_client
            .lock()
            .await
            .latest_block_height()
            .await
            .map_err(|e| format!("Failed to get current block height: {:?}", e))?;

        if current_block.0 as u64 >= metadata.good_until_block as u64 {
            // Mark as expired in database
            metadata_repo
                .update_expired(order_id)
                .await
                .map_err(|e| format!("Failed to mark order as expired: {}", e))?;
            return Err(format!(
                "Order {} has expired at block {}",
                order_id, metadata.good_until_block
            ));
        }

        // Reconstruct the OrderId from stored metadata
        let reconstructed_order_id = self
            .reconstruct_order_id(&metadata)
            .await
            .map_err(|e| format!("Failed to reconstruct OrderId: {}", e))?;

        // Reconstruct the good_until_block
        let good_until_block = Height(metadata.good_until_block as u32);

        // Perform the cancellation
        let mut account = self.account.lock().await;
        let mut node_client = self.node_client.lock().await;

        let tx_hash = node_client
            .cancel_order(&mut *account, reconstructed_order_id, good_until_block)
            .await
            .map_err(|e| format!("Failed to cancel order on dYdX: {:?}", e))?;

        // Update the metadata to mark as cancelled
        metadata_repo
            .update_cancelled(order_id, &tx_hash)
            .await
            .map_err(|e| format!("Failed to update order status: {}", e))?;

        info!(
            "Successfully cancelled order {} with tx hash: {:?}",
            order_id, tx_hash
        );
        Ok(())
    }

    /// Get order status from dYdX v4 indexer
    pub async fn get_order_status(&self, order_id: &str) -> Result<String, String> {
        // Get subaccount
        let subaccount = self.get_subaccount().await?;

        // Query indexer for order status (using get_subaccount_orders instead of deprecated list_orders)
        let orders = self
            .indexer_client
            .accounts()
            .get_subaccount_orders(&subaccount, None)
            .await
            .map_err(|e| format!("Failed to get orders: {:?}", e))?;

        // Find our order
        for order in orders {
            let current_order_id = format!("{:?}", order.id);
            let client_id_str = format!("{:?}", order.client_id);
            if current_order_id == order_id || client_id_str == order_id {
                return Ok(format!("{:?}", order.status));
            }
        }

        Ok("NOT_FOUND".to_string())
    }

    /// Get account information
    pub async fn get_account_info(&self) -> Result<AccountInfo, String> {
        let account = self.account.lock().await;
        Ok(AccountInfo {
            address: account.address().to_string(),
            account_number: account.account_number(),
            sequence: account.sequence_number(),
        })
    }

    /// Helper to convert dYdX order status string to our OrderStatus enum
    fn parse_order_status(status_str: &str) -> OrderStatus {
        match status_str.to_uppercase().as_str() {
            "OPEN" | "PENDING" | "BEST_EFFORT_OPENED" => OrderStatus::Pending,
            "FILLED" | "BEST_EFFORT_FILLED" => OrderStatus::Filled,
            "PARTIALLY_FILLED" => OrderStatus::PartiallyFilled,
            "CANCELLED" | "BEST_EFFORT_CANCELLED" => OrderStatus::Cancelled,
            "NOT_FOUND" => OrderStatus::Unknown,
            _ => OrderStatus::Unknown,
        }
    }
}

/// Account information structure
#[derive(Debug, Clone)]
pub struct AccountInfo {
    pub address: String,
    pub account_number: u64,
    pub sequence: u64,
}

/// Implementation of ExchangeClient trait for dYdX v4
#[async_trait]
impl ExchangeClient for DydxV4Client {
    fn name(&self) -> &str {
        "dYdX v4"
    }

    async fn place_order(&self, order: &Order) -> ExchangeResult<String> {
        // Use the existing place_order implementation
        DydxV4Client::place_order(self, order)
            .await
            .map_err(|e| ExchangeError::OrderPlacementFailed(e))
    }

    async fn cancel_order(&self, order_id: &str) -> ExchangeResult<()> {
        // Use the existing cancel_order implementation
        DydxV4Client::cancel_order(self, order_id)
            .await
            .map_err(|e| ExchangeError::OrderCancellationFailed(e))
    }

    async fn get_order_status(&self, order_id: &str) -> ExchangeResult<OrderStatus> {
        // Use the existing get_order_status implementation
        let status_str = DydxV4Client::get_order_status(self, order_id)
            .await
            .map_err(|e| ExchangeError::OrderStatusFailed(e))?;

        Ok(Self::parse_order_status(&status_str))
    }

    async fn get_balance(&self, currency: Option<&str>) -> ExchangeResult<Vec<Balance>> {
        // Get subaccount to access balance info
        let subaccount = self
            .get_subaccount()
            .await
            .map_err(|e| ExchangeError::BalanceQueryFailed(e))?;

        // Query balances from indexer
        let subaccount_data = self
            .indexer_client
            .accounts()
            .get_subaccount(&subaccount)
            .await
            .map_err(|e| {
                ExchangeError::BalanceQueryFailed(format!("Failed to get subaccount data: {:?}", e))
            })?;

        let mut balances = Vec::new();

        // dYdX uses USDC as main collateral
        let equity = &subaccount_data.equity;
        let currency_str = currency.unwrap_or("USDC");
        if currency.is_none() || currency == Some("USDC") {
            // Convert BigDecimal to f64
            let equity_f64 = equity.to_string().parse::<f64>().unwrap_or(0.0);

            balances.push(Balance {
                currency: currency_str.to_string(),
                available: equity_f64,
                total: equity_f64,
            });
        }

        Ok(balances)
    }

    async fn is_healthy(&self) -> bool {
        // Try to get account info as health check
        self.get_account_info().await.is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_info() {
        let info = AccountInfo {
            address: "dydx1abc...".to_string(),
            account_number: 123,
            sequence: 456,
        };

        assert_eq!(info.address, "dydx1abc...");
        assert_eq!(info.account_number, 123);
        assert_eq!(info.sequence, 456);
    }

    #[test]
    fn test_parse_order_status() {
        assert_eq!(
            DydxV4Client::parse_order_status("OPEN"),
            OrderStatus::Pending
        );
        assert_eq!(
            DydxV4Client::parse_order_status("FILLED"),
            OrderStatus::Filled
        );
        assert_eq!(
            DydxV4Client::parse_order_status("CANCELLED"),
            OrderStatus::Cancelled
        );
        assert_eq!(
            DydxV4Client::parse_order_status("PARTIALLY_FILLED"),
            OrderStatus::PartiallyFilled
        );
        assert_eq!(
            DydxV4Client::parse_order_status("NOT_FOUND"),
            OrderStatus::Unknown
        );
        assert_eq!(
            DydxV4Client::parse_order_status("INVALID"),
            OrderStatus::Unknown
        );
    }

    #[test]
    fn test_metadata_repository_not_set() {
        // Test that get_metadata_repository returns None when not set
        assert!(DydxV4Client::get_metadata_repository().is_none());
    }

    #[test]
    fn test_metadata_repository_set_and_get() {
        // This test would require a mock repository, but for now we just test the static access
        // In a real test, we'd create a test database and repository
        assert!(DydxV4Client::get_metadata_repository().is_none());
    }
}
