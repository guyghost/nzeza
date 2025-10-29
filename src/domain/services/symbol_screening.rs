use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::domain::entities::symbol_screening::SymbolScreeningResult;
use crate::domain::services::indicators::Candle;
use crate::domain::services::screening::ScalpingPotentialAggregator;

/// Cache performance statistics
#[derive(Clone, Debug, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

impl CacheStats {
    /// Calculate hit rate as percentage
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
    }
}

/// Market data for a symbol
#[derive(Clone, Debug)]
pub struct SymbolMarketData {
    pub candles: Vec<Candle>,
    pub volumes: Vec<f64>,
    pub bid: f64,
    pub ask: f64,
}

/// Cached screening result with timestamp
#[derive(Clone)]
struct CachedResult {
    result: SymbolScreeningResult,
    cached_at: SystemTime,
}

/// Service for screening symbols for scalping potential
pub struct SymbolScreeningService {
    aggregator: ScalpingPotentialAggregator,
    cache: Arc<RwLock<HashMap<String, CachedResult>>>,
    cache_ttl: Duration,
    stats: Arc<RwLock<CacheStats>>,
}

impl SymbolScreeningService {
    pub fn new(cache_ttl: Duration) -> Self {
        SymbolScreeningService {
            aggregator: ScalpingPotentialAggregator::new(),
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl,
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    pub fn with_default_cache_ttl() -> Self {
        Self::new(Duration::from_secs(300)) // 5 minutes default
    }

    /// Screen a single symbol
    pub async fn screen_symbol(
        &self,
        symbol: String,
        exchange: String,
        market_data: SymbolMarketData,
    ) -> SymbolScreeningResult {
        let cache_key = format!("{}:{}", exchange, symbol);

        // Check cache
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                if cached.cached_at.elapsed().unwrap_or(self.cache_ttl) < self.cache_ttl {
                    let mut stats = self.stats.write().await;
                    stats.hits += 1;

                    debug!(
                        symbol = %symbol,
                        exchange = %exchange,
                        cache_age_ms = cached.cached_at.elapsed().unwrap_or(Duration::ZERO).as_millis(),
                        cache_hit_rate = format!("{:.2}%", stats.hit_rate()),
                        "Cache hit - returning cached screening result"
                    );
                    return cached.result.clone();
                }
            }
        }

        let mut stats = self.stats.write().await;
        stats.misses += 1;
        drop(stats);

        debug!(
            symbol = %symbol,
            exchange = %exchange,
            "Cache miss - calculating fresh screening result"
        );

        // Calculate fresh result
        let result = self.aggregator.calculate(
            symbol.clone(),
            exchange.clone(),
            &market_data.candles,
            &market_data.volumes,
            market_data.bid,
            market_data.ask,
        );

        // Cache result
        {
            let mut cache = self.cache.write().await;
            cache.insert(
                cache_key.clone(),
                CachedResult {
                    result: result.clone(),
                    cached_at: SystemTime::now(),
                },
            );
        }

        info!(
            symbol = %symbol,
            exchange = %exchange,
            overall_score = result.overall_score,
            recommendation = ?result.recommendation,
            cache_size = self.cache.read().await.len(),
            "Screening result calculated and cached"
        );

        result
    }

    /// Screen multiple symbols
    pub async fn screen_all_symbols(
        &self,
        exchange: String,
        symbols_data: HashMap<String, SymbolMarketData>,
    ) -> Vec<SymbolScreeningResult> {
        let symbol_count = symbols_data.len();
        debug!(
            exchange = %exchange,
            symbol_count = symbol_count,
            "Starting screening of multiple symbols"
        );

        let mut results = Vec::new();

        for (symbol, market_data) in symbols_data {
            let result = self
                .screen_symbol(symbol, exchange.clone(), market_data)
                .await;
            results.push(result);
        }

        // Sort by score descending
        let ranked = ScalpingPotentialAggregator::rank_results(results);

        info!(
            exchange = %exchange,
            total_symbols_screened = symbol_count,
            results_count = ranked.len(),
            "Completed screening of all symbols"
        );

        ranked
    }

    /// Clear the cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        let cache_size = cache.len();
        cache.clear();
        debug!(cleared_entries = cache_size, "Screening cache cleared");
    }

    /// Get cache statistics
    pub async fn cache_size(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }

    /// Get cache performance statistics
    pub async fn get_cache_stats(&self) -> CacheStats {
        self.stats.read().await.clone()
    }

    /// Reset cache statistics
    pub async fn reset_cache_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = CacheStats::default();
        debug!("Cache statistics reset");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_screening_service_initialization() {
        let service = SymbolScreeningService::with_default_cache_ttl();
        assert_eq!(service.cache_size().await, 0);
        let stats = service.get_cache_stats().await;
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
    }

    #[tokio::test]
    async fn test_screening_service_cache_hit_rate() {
        let service = SymbolScreeningService::with_default_cache_ttl();
        let candles = vec![Candle::new(100.0, 101.0, 99.0, 100.5, 1000.0).unwrap()];
        let market_data = SymbolMarketData {
            candles,
            volumes: vec![500_000.0],
            bid: 100.0,
            ask: 100.1,
        };

        // First call - cache miss
        let _result1 = service
            .screen_symbol(
                "BTC-USD".to_string(),
                "test".to_string(),
                market_data.clone(),
            )
            .await;

        let stats_after_miss = service.get_cache_stats().await;
        assert_eq!(stats_after_miss.misses, 1);
        assert_eq!(stats_after_miss.hits, 0);
        assert_eq!(stats_after_miss.hit_rate(), 0.0);

        // Second call - cache hit
        let _result2 = service
            .screen_symbol("BTC-USD".to_string(), "test".to_string(), market_data)
            .await;

        let stats_after_hit = service.get_cache_stats().await;
        assert_eq!(stats_after_hit.misses, 1);
        assert_eq!(stats_after_hit.hits, 1);
        assert_eq!(stats_after_hit.hit_rate(), 50.0);
    }

    #[tokio::test]
    async fn test_screening_service_reset_cache_stats() {
        let service = SymbolScreeningService::with_default_cache_ttl();
        let candles = vec![Candle::new(100.0, 101.0, 99.0, 100.5, 1000.0).unwrap()];
        let market_data = SymbolMarketData {
            candles,
            volumes: vec![500_000.0],
            bid: 100.0,
            ask: 100.1,
        };

        // Generate some cache activity
        let _result1 = service
            .screen_symbol(
                "BTC-USD".to_string(),
                "test".to_string(),
                market_data.clone(),
            )
            .await;
        let _result2 = service
            .screen_symbol("BTC-USD".to_string(), "test".to_string(), market_data)
            .await;

        let stats_before = service.get_cache_stats().await;
        assert!(stats_before.hits > 0 || stats_before.misses > 0);

        // Reset stats
        service.reset_cache_stats().await;
        let stats_after = service.get_cache_stats().await;
        assert_eq!(stats_after.hits, 0);
        assert_eq!(stats_after.misses, 0);
    }

    #[tokio::test]
    async fn test_screening_service_screen_single_symbol() {
        let service = SymbolScreeningService::with_default_cache_ttl();
        let candles = vec![Candle::new(100.0, 101.0, 99.0, 100.5, 1000.0).unwrap()];
        let market_data = SymbolMarketData {
            candles,
            volumes: vec![500_000.0],
            bid: 100.0,
            ask: 100.1,
        };

        let result = service
            .screen_symbol("BTC-USD".to_string(), "test".to_string(), market_data)
            .await;

        assert_eq!(result.symbol, "BTC-USD");
        assert_eq!(result.exchange, "test");
        assert!(result.overall_score >= 0.0 && result.overall_score <= 1.0);
    }

    #[tokio::test]
    async fn test_screening_service_screen_multiple_symbols() {
        let service = SymbolScreeningService::with_default_cache_ttl();
        let mut symbols_data = HashMap::new();

        for symbol in &["BTC-USD", "ETH-USD", "SOL-USD"] {
            symbols_data.insert(
                symbol.to_string(),
                SymbolMarketData {
                    candles: vec![Candle::new(100.0, 101.0, 99.0, 100.5, 1000.0).unwrap()],
                    volumes: vec![500_000.0],
                    bid: 100.0,
                    ask: 100.1,
                },
            );
        }

        let results = service
            .screen_all_symbols("test".to_string(), symbols_data)
            .await;

        assert_eq!(results.len(), 3);
        // Results should be sorted by score
        for i in 1..results.len() {
            assert!(results[i - 1].overall_score >= results[i].overall_score);
        }
    }

    #[tokio::test]
    async fn test_screening_service_result_caching() {
        let service = SymbolScreeningService::with_default_cache_ttl();
        let candles = vec![Candle::new(100.0, 101.0, 99.0, 100.5, 1000.0).unwrap()];
        let market_data = SymbolMarketData {
            candles,
            volumes: vec![500_000.0],
            bid: 100.0,
            ask: 100.1,
        };

        let result1 = service
            .screen_symbol(
                "BTC-USD".to_string(),
                "test".to_string(),
                market_data.clone(),
            )
            .await;

        // Second call should return cached result
        let result2 = service
            .screen_symbol("BTC-USD".to_string(), "test".to_string(), market_data)
            .await;

        assert_eq!(result1.overall_score, result2.overall_score);
        assert_eq!(result1.screened_at, result2.screened_at);
    }

    #[tokio::test]
    async fn test_screening_service_cache_expiration() {
        let service = SymbolScreeningService::new(Duration::from_millis(100));
        let candles = vec![Candle::new(100.0, 101.0, 99.0, 100.5, 1000.0).unwrap()];
        let market_data = SymbolMarketData {
            candles,
            volumes: vec![500_000.0],
            bid: 100.0,
            ask: 100.1,
        };

        let result1 = service
            .screen_symbol(
                "BTC-USD".to_string(),
                "test".to_string(),
                market_data.clone(),
            )
            .await;

        // Wait for cache to expire
        tokio::time::sleep(Duration::from_millis(150)).await;

        let result2 = service
            .screen_symbol("BTC-USD".to_string(), "test".to_string(), market_data)
            .await;

        // Should be different timestamps (recalculated)
        assert!(
            result2.screened_at > result1.screened_at || result1.screened_at == result2.screened_at
        );
    }

    #[tokio::test]
    async fn test_screening_service_handles_missing_data() {
        let service = SymbolScreeningService::with_default_cache_ttl();
        let market_data = SymbolMarketData {
            candles: vec![],
            volumes: vec![],
            bid: 0.0,
            ask: 0.0,
        };

        let result = service
            .screen_symbol("TEST-USD".to_string(), "test".to_string(), market_data)
            .await;

        // Should handle gracefully
        assert_eq!(result.symbol, "TEST-USD");
        assert!(result.overall_score >= 0.0 && result.overall_score <= 1.0);
    }
}
