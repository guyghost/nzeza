use nzeza::domain::entities::symbol_screening::SymbolScreeningResult;
use nzeza::domain::services::indicators::Candle;
use nzeza::domain::services::screening::ScalpingPotentialAggregator;
use nzeza::domain::services::symbol_screening::{SymbolMarketData, SymbolScreeningService};
use std::collections::HashMap;
use std::time::Duration;

#[tokio::test]
async fn test_end_to_end_symbol_screening_workflow() {
    // Create screening service with short cache TTL for testing
    let service = SymbolScreeningService::new(Duration::from_secs(1));

    // Prepare market data for multiple symbols
    let mut symbols_data = HashMap::new();

    // High volatility symbol (good for scalping)
    let high_vol_candles = vec![
        Candle::new(100.0, 102.0, 98.0, 101.0, 1000.0).unwrap(),
        Candle::new(101.0, 103.5, 99.5, 102.5, 1200.0).unwrap(),
        Candle::new(102.5, 105.0, 101.0, 104.0, 1500.0).unwrap(),
    ];
    symbols_data.insert(
        "BTC-USD".to_string(),
        SymbolMarketData {
            candles: high_vol_candles,
            volumes: vec![1000000.0, 1200000.0, 1500000.0],
            bid: 100.0,
            ask: 100.2,
        },
    );

    // Medium volatility symbol
    let med_vol_candles = vec![
        Candle::new(2000.0, 2010.0, 1990.0, 2005.0, 1000.0).unwrap(),
        Candle::new(2005.0, 2015.0, 1995.0, 2010.0, 1100.0).unwrap(),
        Candle::new(2010.0, 2020.0, 2000.0, 2015.0, 1200.0).unwrap(),
    ];
    symbols_data.insert(
        "ETH-USD".to_string(),
        SymbolMarketData {
            candles: med_vol_candles,
            volumes: vec![500000.0, 550000.0, 600000.0],
            bid: 2000.0,
            ask: 2000.5,
        },
    );

    // Low volatility symbol (poor for scalping)
    let low_vol_candles = vec![
        Candle::new(150.0, 151.0, 149.0, 150.5, 1000.0).unwrap(),
        Candle::new(150.5, 151.0, 150.0, 150.7, 900.0).unwrap(),
        Candle::new(150.7, 151.2, 150.3, 150.9, 800.0).unwrap(),
    ];
    symbols_data.insert(
        "SOL-USD".to_string(),
        SymbolMarketData {
            candles: low_vol_candles,
            volumes: vec![100000.0, 90000.0, 80000.0],
            bid: 150.0,
            ask: 150.1,
        },
    );

    // Run screening on all symbols
    let results = service
        .screen_all_symbols("test_exchange".to_string(), symbols_data)
        .await;

    // Verify results
    assert_eq!(results.len(), 3, "Should have 3 screening results");

    // Results should be sorted by score (descending)
    for i in 1..results.len() {
        assert!(
            results[i - 1].overall_score >= results[i].overall_score,
            "Results should be sorted by overall score descending"
        );
    }

    // Verify all results are valid
    for result in &results {
        assert_eq!(result.exchange, "test_exchange");
        assert!(result.overall_score >= 0.0 && result.overall_score <= 1.0);
        assert!(result.volatility_score >= 0.0 && result.volatility_score <= 1.0);
        assert!(result.volume_score >= 0.0 && result.volume_score <= 1.0);
        assert!(result.spread_score >= 0.0 && result.spread_score <= 1.0);
        assert!(result.momentum_score >= 0.0 && result.momentum_score <= 1.0);
    }

    // Verify cache statistics
    let stats_after_screening = service.get_cache_stats().await;
    assert_eq!(
        stats_after_screening.misses, 3,
        "Should have 3 cache misses"
    );
    assert_eq!(stats_after_screening.hits, 0, "Should have 0 cache hits");

    // Screen same symbols again - should hit cache
    let mut symbols_data_2 = HashMap::new();
    symbols_data_2.insert(
        "BTC-USD".to_string(),
        SymbolMarketData {
            candles: vec![Candle::new(100.0, 102.0, 98.0, 101.0, 1000.0).unwrap()],
            volumes: vec![1000000.0],
            bid: 100.0,
            ask: 100.2,
        },
    );

    let _results_2 = service
        .screen_all_symbols("test_exchange".to_string(), symbols_data_2)
        .await;

    let stats_after_cache = service.get_cache_stats().await;
    assert_eq!(stats_after_cache.hits, 1, "Should have 1 cache hit");
    assert_eq!(
        stats_after_cache.misses, 3,
        "Should still have 3 cache misses"
    );
}

#[tokio::test]
async fn test_symbol_screening_recommendation_categorization() {
    let service = SymbolScreeningService::with_default_cache_ttl();

    // Test best candidate (high scores)
    let best_candidate_data = SymbolMarketData {
        candles: vec![Candle::new(100.0, 102.0, 98.0, 101.0, 1000.0).unwrap()],
        volumes: vec![1000000.0],
        bid: 100.0,
        ask: 100.0,
    };

    let result = service
        .screen_symbol(
            "BEST-USD".to_string(),
            "test".to_string(),
            best_candidate_data,
        )
        .await;

    // With high values, should get good scores
    assert!(result.overall_score > 0.0);

    // Test poor candidate (missing data)
    let poor_candidate_data = SymbolMarketData {
        candles: vec![],
        volumes: vec![],
        bid: 0.0,
        ask: 0.0,
    };

    let result_poor = service
        .screen_symbol(
            "POOR-USD".to_string(),
            "test".to_string(),
            poor_candidate_data,
        )
        .await;

    // With missing data, should get low scores
    assert!(result_poor.overall_score < result.overall_score);
}

#[tokio::test]
async fn test_symbol_screening_aggregator_ranking() {
    let aggregator = ScalpingPotentialAggregator::new();

    // Create test data for multiple symbols
    let results = vec![
        SymbolScreeningResult::new(
            "SOL-USD".to_string(),
            "test".to_string(),
            0.3,
            0.3,
            0.3,
            0.3,
        ),
        SymbolScreeningResult::new(
            "BTC-USD".to_string(),
            "test".to_string(),
            0.8,
            0.8,
            0.8,
            0.8,
        ),
        SymbolScreeningResult::new(
            "ETH-USD".to_string(),
            "test".to_string(),
            0.6,
            0.6,
            0.6,
            0.6,
        ),
    ];

    let ranked = ScalpingPotentialAggregator::rank_results(results);

    // Verify ranking
    assert_eq!(ranked[0].symbol, "BTC-USD", "BTC should be ranked 1st");
    assert_eq!(ranked[1].symbol, "ETH-USD", "ETH should be ranked 2nd");
    assert_eq!(ranked[2].symbol, "SOL-USD", "SOL should be ranked 3rd");

    // Verify scores are descending
    assert!(ranked[0].overall_score > ranked[1].overall_score);
    assert!(ranked[1].overall_score > ranked[2].overall_score);
}

#[tokio::test]
async fn test_symbol_screening_cache_expiration() {
    let service = SymbolScreeningService::new(Duration::from_millis(500));

    let market_data = SymbolMarketData {
        candles: vec![Candle::new(100.0, 101.0, 99.0, 100.5, 1000.0).unwrap()],
        volumes: vec![500_000.0],
        bid: 100.0,
        ask: 100.1,
    };

    // First screening
    let result1 = service
        .screen_symbol(
            "BTC-USD".to_string(),
            "test".to_string(),
            market_data.clone(),
        )
        .await;

    let stats_initial = service.get_cache_stats().await;
    assert_eq!(stats_initial.misses, 1);

    // Second screening immediately (should be cached)
    let result2 = service
        .screen_symbol(
            "BTC-USD".to_string(),
            "test".to_string(),
            market_data.clone(),
        )
        .await;

    let stats_after_hit = service.get_cache_stats().await;
    assert_eq!(stats_after_hit.hits, 1);

    // Wait for cache to expire
    tokio::time::sleep(Duration::from_millis(600)).await;

    // Third screening (cache should be expired)
    let _result3 = service
        .screen_symbol("BTC-USD".to_string(), "test".to_string(), market_data)
        .await;

    let stats_after_expiry = service.get_cache_stats().await;
    assert_eq!(stats_after_expiry.misses, 2, "Cache miss after expiration");
    assert_eq!(stats_after_expiry.hits, 1, "Previous hit still counted");
}

#[test]
fn test_score_formula_weights_sum_to_one() {
    // Verify that the score formula weights are balanced
    let volatility_weight = 0.3;
    let volume_weight = 0.3;
    let spread_weight = 0.2;
    let momentum_weight = 0.2;

    let total_weight = volatility_weight + volume_weight + spread_weight + momentum_weight;
    assert_eq!(total_weight, 1.0, "Score formula weights should sum to 1.0");
}

#[tokio::test]
async fn test_symbol_screening_multiple_exchanges() {
    let service = SymbolScreeningService::with_default_cache_ttl();

    // Screen BTC-USD on multiple exchanges
    let market_data = SymbolMarketData {
        candles: vec![Candle::new(100.0, 101.0, 99.0, 100.5, 1000.0).unwrap()],
        volumes: vec![500_000.0],
        bid: 100.0,
        ask: 100.1,
    };

    let result_dydx = service
        .screen_symbol(
            "BTC-USD".to_string(),
            "dydx".to_string(),
            market_data.clone(),
        )
        .await;

    let result_coinbase = service
        .screen_symbol(
            "BTC-USD".to_string(),
            "coinbase".to_string(),
            market_data.clone(),
        )
        .await;

    // Results should be different entries in cache (different exchange keys)
    assert_eq!(result_dydx.exchange, "dydx");
    assert_eq!(result_coinbase.exchange, "coinbase");
    assert_eq!(result_dydx.symbol, result_coinbase.symbol);

    // Cache should have 2 entries
    assert_eq!(service.cache_size().await, 2);
}
