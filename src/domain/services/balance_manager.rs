//! BalanceManager - Fetches and caches account balance

use crate::domain::entities::balance::BalanceInfo;
use crate::domain::repositories::exchange_client::{Balance, ExchangeClient};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;

/// Cached balance entry with timestamp
#[derive(Clone, Debug)]
struct CachedBalance {
    balance_info: BalanceInfo,
    cached_at: SystemTime,
}

/// Manages fetching and caching of account balance
pub struct BalanceManager {
    exchange_client: Arc<dyn ExchangeClient>,
    cache: Arc<Mutex<Option<CachedBalance>>>,
    cache_ttl: Duration,
}

impl BalanceManager {
    /// Create a new BalanceManager
    ///
    /// # Arguments
    /// * `exchange_client` - The exchange client to fetch balance from
    /// * `cache_ttl` - How long to cache balance (default: 10 seconds)
    pub fn new(exchange_client: Arc<dyn ExchangeClient>, cache_ttl: Duration) -> Self {
        Self {
            exchange_client,
            cache: Arc::new(Mutex::new(None)),
            cache_ttl,
        }
    }

    /// Get the account balance, using cache if available and fresh
    ///
    /// # Returns
    /// BalanceInfo if successful, error message otherwise
    pub async fn get_balance(&self) -> Result<BalanceInfo, String> {
        // Check if cache is valid
        if let Some(cached) = self.cache.lock().await.as_ref() {
            if let Ok(true) = cached.balance_info.is_fresh(self.cache_ttl) {
                tracing::debug!(
                    "Returning cached balance (age: {:?})",
                    cached.cached_at.elapsed().unwrap_or_default()
                );
                return Ok(cached.balance_info.clone());
            }
        }

        // Cache miss or expired - fetch fresh balance
        self.refresh_balance().await
    }

    /// Force refresh of balance from exchange, bypassing cache
    ///
    /// # Returns
    /// BalanceInfo if successful, error message otherwise
    pub async fn refresh_balance(&self) -> Result<BalanceInfo, String> {
        tracing::debug!("Fetching balance from exchange");

        // Fetch balance from exchange
        let balances = self
            .exchange_client
            .get_balance(None)
            .await
            .map_err(|e| format!("Failed to fetch balance: {:?}", e))?;

        if balances.is_empty() {
            return Err("No balances returned from exchange".to_string());
        }

        // Sum up all balances
        let total_balance: f64 = balances.iter().map(|b| b.total).sum();
        let available_balance: f64 = balances.iter().map(|b| b.available).sum();
        let locked_balance: f64 = total_balance - available_balance;

        // Create BalanceInfo
        let balance_info = BalanceInfo::new(total_balance, available_balance, locked_balance)
            .map_err(|e| format!("Invalid balance info: {}", e))?;

        // Cache the result
        let mut cache = self.cache.lock().await;
        *cache = Some(CachedBalance {
            balance_info: balance_info.clone(),
            cached_at: SystemTime::now(),
        });

        tracing::info!(
            "Balance refreshed: total={}, available={}, locked={}",
            total_balance,
            available_balance,
            locked_balance
        );

        Ok(balance_info)
    }

    /// Clear the cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.lock().await;
        *cache = None;
        tracing::debug!("Balance cache cleared");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    /// Mock exchange client for testing
    struct MockExchangeClient {
        balances: Vec<Balance>,
        should_fail: bool,
    }

    #[async_trait]
    impl ExchangeClient for MockExchangeClient {
        fn name(&self) -> &str {
            "MockExchange"
        }

        async fn get_balance(
            &self,
            _currency: Option<&str>,
        ) -> Result<Vec<Balance>, crate::domain::repositories::exchange_client::ExchangeError>
        {
            if self.should_fail {
                return Err(
                    crate::domain::repositories::exchange_client::ExchangeError::BalanceQueryFailed(
                        "Mock error".to_string(),
                    ),
                );
            }
            Ok(self.balances.clone())
        }

        async fn place_order(
            &self,
            _order: &crate::domain::entities::order::Order,
        ) -> Result<String, crate::domain::repositories::exchange_client::ExchangeError> {
            Ok("mock-order-id".to_string())
        }

        async fn cancel_order(
            &self,
            _order_id: &str,
        ) -> Result<(), crate::domain::repositories::exchange_client::ExchangeError> {
            Ok(())
        }

        async fn get_order_status(
            &self,
            _order_id: &str,
        ) -> Result<
            crate::domain::repositories::exchange_client::OrderStatus,
            crate::domain::repositories::exchange_client::ExchangeError,
        > {
            Ok(crate::domain::repositories::exchange_client::OrderStatus::Pending)
        }
    }

    #[tokio::test]
    async fn test_balance_manager_fetch_balance() {
        let balances = vec![
            Balance {
                currency: "USD".to_string(),
                available: 1000.0,
                total: 1000.0,
            },
            Balance {
                currency: "BTC".to_string(),
                available: 0.5,
                total: 0.5,
            },
        ];

        let mock_client = Arc::new(MockExchangeClient {
            balances,
            should_fail: false,
        });

        let manager = BalanceManager::new(mock_client, Duration::from_secs(10));
        let balance = manager.get_balance().await;

        assert!(balance.is_ok());
        let balance = balance.unwrap();
        assert_eq!(balance.total_balance, 1000.5);
        assert_eq!(balance.available_balance, 1000.5);
        assert_eq!(balance.locked_balance, 0.0);
    }

    #[tokio::test]
    async fn test_balance_manager_caching() {
        let balances = vec![Balance {
            currency: "USD".to_string(),
            available: 1000.0,
            total: 1000.0,
        }];

        let mock_client = Arc::new(MockExchangeClient {
            balances,
            should_fail: false,
        });

        let manager = BalanceManager::new(mock_client, Duration::from_secs(60));

        // First call
        let balance1 = manager.get_balance().await.unwrap();

        // Second call (should be cached)
        let balance2 = manager.get_balance().await.unwrap();

        assert_eq!(balance1, balance2);
    }

    #[tokio::test]
    async fn test_balance_manager_refresh_balance() {
        let balances = vec![Balance {
            currency: "USD".to_string(),
            available: 2000.0,
            total: 2500.0,
        }];

        let mock_client = Arc::new(MockExchangeClient {
            balances,
            should_fail: false,
        });

        let manager = BalanceManager::new(mock_client, Duration::from_secs(0));

        let balance = manager.refresh_balance().await.unwrap();
        assert_eq!(balance.total_balance, 2500.0);
        assert_eq!(balance.available_balance, 2000.0);
        assert_eq!(balance.locked_balance, 500.0);
    }

    #[tokio::test]
    async fn test_balance_manager_fetch_error() {
        let mock_client = Arc::new(MockExchangeClient {
            balances: vec![],
            should_fail: true,
        });

        let manager = BalanceManager::new(mock_client, Duration::from_secs(10));
        let result = manager.get_balance().await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to fetch balance"));
    }

    #[tokio::test]
    async fn test_balance_manager_empty_balances() {
        let mock_client = Arc::new(MockExchangeClient {
            balances: vec![],
            should_fail: false,
        });

        let manager = BalanceManager::new(mock_client, Duration::from_secs(10));
        let result = manager.get_balance().await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No balances returned"));
    }

    #[tokio::test]
    async fn test_balance_manager_clear_cache() {
        let balances = vec![Balance {
            currency: "USD".to_string(),
            available: 1000.0,
            total: 1000.0,
        }];

        let mock_client = Arc::new(MockExchangeClient {
            balances,
            should_fail: false,
        });

        let manager = BalanceManager::new(mock_client, Duration::from_secs(60));

        // Cache balance
        let _balance1 = manager.get_balance().await.unwrap();

        // Clear cache
        manager.clear_cache().await;

        // Verify cache is empty
        let cache = manager.cache.lock().await;
        assert!(cache.is_none());
    }

    #[tokio::test]
    async fn test_balance_manager_with_locked_balance() {
        let balances = vec![
            Balance {
                currency: "USD".to_string(),
                available: 5000.0,
                total: 7000.0,
            },
            Balance {
                currency: "ETH".to_string(),
                available: 1.0,
                total: 1.5,
            },
        ];

        let mock_client = Arc::new(MockExchangeClient {
            balances,
            should_fail: false,
        });

        let manager = BalanceManager::new(mock_client, Duration::from_secs(10));
        let balance = manager.get_balance().await.unwrap();

        assert_eq!(balance.total_balance, 7001.5);
        assert_eq!(balance.available_balance, 5001.0);
        assert_eq!(balance.locked_balance, 2000.5);
    }
}
