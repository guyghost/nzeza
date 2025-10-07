//! Exchange Client Factory
//!
//! This module provides a factory for creating exchange client instances.
//! Clients are created once and shared between different parts of the system
//! (e.g., ExchangeActor for market data, TraderActor for execution).

use crate::domain::entities::exchange::Exchange;
use crate::domain::repositories::exchange_client::ExchangeClient;
use crate::infrastructure::coinbase_advanced_client::CoinbaseAdvancedClient;
use crate::infrastructure::coinbase_client::CoinbaseClient;
use crate::infrastructure::dydx_v4_client::DydxV4Client;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, warn};

/// Factory for creating exchange clients
pub struct ExchangeClientFactory;

impl ExchangeClientFactory {
    /// Create all available exchange clients based on environment configuration
    ///
    /// # Returns
    /// HashMap of Exchange to ExchangeClient instances
    pub async fn create_all() -> HashMap<Exchange, Arc<dyn ExchangeClient>> {
        let mut clients: HashMap<Exchange, Arc<dyn ExchangeClient>> = HashMap::new();

        // Try to create dYdX v4 client
        if let Some(client) = Self::create_dydx_client().await {
            clients.insert(Exchange::Dydx, client);
        }

        // Try to create Coinbase Advanced client
        if let Some(client) = Self::create_coinbase_advanced_client().await {
            clients.insert(Exchange::Coinbase, client);
        }

        // Try to create Coinbase Pro (legacy) client
        // Note: Typically you'd use either Advanced OR Pro, not both
        // This is here for backwards compatibility
        if let Some(client) = Self::create_coinbase_pro_client().await {
            // Only add if we don't already have Coinbase Advanced
            if !clients.contains_key(&Exchange::Coinbase) {
                clients.insert(Exchange::Coinbase, client);
            }
        }

        info!(
            "ExchangeClientFactory created {} exchange clients",
            clients.len()
        );

        clients
    }

    /// Create a dYdX v4 client from environment variables
    async fn create_dydx_client() -> Option<Arc<dyn ExchangeClient>> {
        match std::env::var("DYDX_MNEMONIC") {
            Ok(mnemonic) => {
                // Initialize rustls crypto provider (required for TLS)
                if rustls::crypto::CryptoProvider::get_default().is_none() {
                    let _ = rustls::crypto::ring::default_provider()
                        .install_default()
                        .map_err(|_| warn!("Rustls crypto provider already installed"));
                }

                // Use dydx_mainnet.toml for mainnet configuration
                let config_path = std::env::var("DYDX_CONFIG_PATH")
                    .unwrap_or_else(|_| "dydx_mainnet.toml".to_string());

                match DydxV4Client::new(&mnemonic, &config_path).await {
                    Ok(client) => {
                        info!("✓ dYdX v4 client created successfully");
                        Some(Arc::new(client) as Arc<dyn ExchangeClient>)
                    }
                    Err(e) => {
                        error!("✗ Failed to create dYdX v4 client: {}", e);
                        None
                    }
                }
            }
            Err(_) => {
                warn!("DYDX_MNEMONIC not set, dYdX client not created");
                None
            }
        }
    }

    /// Create a Coinbase Advanced Trade client from environment variables
    async fn create_coinbase_advanced_client() -> Option<Arc<dyn ExchangeClient>> {
        let api_key = std::env::var("COINBASE_ADVANCED_API_KEY").ok()?;
        let api_secret = std::env::var("COINBASE_ADVANCED_API_SECRET").ok()?;

        match CoinbaseAdvancedClient::new(&api_key, &api_secret) {
            Ok(client) => {
                info!("✓ Coinbase Advanced Trade client created successfully");
                Some(Arc::new(client) as Arc<dyn ExchangeClient>)
            }
            Err(e) => {
                error!("✗ Failed to create Coinbase Advanced client: {}", e);
                None
            }
        }
    }

    /// Create a Coinbase Pro (legacy) client from environment variables
    async fn create_coinbase_pro_client() -> Option<Arc<dyn ExchangeClient>> {
        let api_key = std::env::var("COINBASE_API_KEY").ok()?;
        let api_secret = std::env::var("COINBASE_API_SECRET").ok()?;
        let passphrase = std::env::var("COINBASE_PASSPHRASE").ok();

        match CoinbaseClient::new(&api_key, &api_secret, passphrase.as_deref()) {
            Ok(client) => {
                info!("✓ Coinbase Pro client created successfully");
                Some(Arc::new(client) as Arc<dyn ExchangeClient>)
            }
            Err(e) => {
                error!("✗ Failed to create Coinbase Pro client: {}", e);
                None
            }
        }
    }

    /// Create a specific exchange client
    ///
    /// # Arguments
    /// * `exchange` - The exchange to create a client for
    ///
    /// # Returns
    /// ExchangeClient instance if credentials are available
    pub async fn create(exchange: Exchange) -> Option<Arc<dyn ExchangeClient>> {
        match exchange {
            Exchange::Dydx => Self::create_dydx_client().await,
            Exchange::Coinbase => {
                // Try Advanced first, then fall back to Pro
                if let Some(client) = Self::create_coinbase_advanced_client().await {
                    Some(client)
                } else {
                    Self::create_coinbase_pro_client().await
                }
            }
            Exchange::Binance => {
                warn!("Binance client not yet implemented");
                None
            }
            Exchange::Hyperliquid => {
                warn!("Hyperliquid client not yet implemented");
                None
            }
            Exchange::Kraken => {
                warn!("Kraken client not yet implemented");
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_all_without_env() {
        // Without environment variables, should return empty map
        let clients = ExchangeClientFactory::create_all().await;
        // This will vary depending on what env vars are actually set
        // Just verify it doesn't panic
        assert!(clients.len() <= 3); // At most Dydx, Coinbase, and one legacy
    }

    #[tokio::test]
    async fn test_create_specific_without_credentials() {
        // Should return None when credentials aren't available
        // (unless they happen to be in the environment)
        let result = ExchangeClientFactory::create(Exchange::Binance).await;
        assert!(result.is_none()); // Binance not implemented
    }
}
