# Symbol Screening for Scalping Potential

## Overview

The Symbol Screening module automatically identifies trading symbols with optimal characteristics for scalping strategies. It analyzes multiple market indicators (volatility, volume, spread, and momentum) to assign a scalping potential score to each symbol across different exchanges.

## Architecture

### Components

1. **Score Calculators** - Individual calculators for each market metric
   - `VolatilityScoreCalculator` - Measures price volatility
   - `VolumeScoreCalculator` - Measures trading volume
   - `SpreadScoreCalculator` - Measures bid-ask spread
   - `MomentumScoreCalculator` - Measures price momentum

2. **Aggregator** - Combines individual scores into overall potential score
   - `ScalpingPotentialAggregator` - Weighted combination of all metrics

3. **Screening Service** - Main service with caching and async support
   - `SymbolScreeningService` - Screens symbols and manages cache

4. **Repository** - Persistence layer
   - `SymbolScreeningRepository` - Stores and retrieves screening results

5. **Actor** - Async execution model
   - `SymbolScreeningActor` - Periodic screening runs

## Scoring Formula

The overall scalping potential score is calculated as:

```
overall_score = 0.3 × volatility + 0.3 × volume + 0.2 × spread + 0.2 × momentum
```

All individual scores are normalized to [0.0, 1.0], where:
- **1.0** = Excellent for scalping
- **0.0** = Poor for scalping

### Recommendation Categories

Based on overall score:

| Score Range | Category | Recommendation |
|-------------|----------|-----------------|
| ≥ 0.75 | BestCandidate | Highly recommended for scalping |
| 0.60 - 0.75 | GoodCandidate | Good opportunity |
| 0.50 - 0.60 | FairCandidate | Acceptable with caution |
| < 0.50 | Avoid | Not suitable for scalping |

## Individual Metrics

### Volatility Score

Measures price movement intensity relative to highs and lows.

**Formula:**
```
volatility_score = min(price_range / max_range, 1.0)
```

**Ideal for scalping:**
- High volatility = more opportunities
- Range: 2-5% intra-period moves

### Volume Score

Measures trading volume and liquidity.

**Formula:**
```
volume_score = min(average_volume / max_volume_threshold, 1.0)
```

**Ideal for scalping:**
- High volume = better execution
- Typically > 500,000 units/period

### Spread Score

Measures bid-ask spread (tightness).

**Formula:**
```
spread_score = max(1.0 - (spread / price), 0.0)
```

**Ideal for scalping:**
- Tight spreads = lower costs
- Typically < 0.1% of price

### Momentum Score

Measures price acceleration and trend strength.

**Formula:**
```
momentum_score = (close - open) / (high - low) normalized to [0.0, 1.0]
```

**Ideal for scalping:**
- Strong directional moves
- Clear trend direction

## Configuration

### Environment Variables

Configure screening behavior via environment variables:

```bash
# Enable/disable symbol screening
SCREENING_ENABLED=true

# Screening run interval (seconds)
SCREENING_INTERVAL_SECONDS=60

# Cache TTL for results (seconds)
SCREENING_CACHE_TTL_SECONDS=300

# Minimum score threshold to consider
SCREENING_SCORE_THRESHOLD=0.50

# Score weights (must sum to 1.0)
SCREENING_VOLATILITY_WEIGHT=0.3
SCREENING_VOLUME_WEIGHT=0.3
SCREENING_SPREAD_WEIGHT=0.2
SCREENING_MOMENTUM_WEIGHT=0.2
```

### Programmatic Configuration

```rust
use nzeza::config::TradingConfig;

let config = TradingConfig::from_env();
println!("Screening enabled: {}", config.screening_enabled);
println!("Screening interval: {} seconds", config.screening_interval_seconds);
println!("Cache TTL: {} seconds", config.screening_cache_ttl_seconds);
println!("Score threshold: {}", config.screening_score_threshold);
```

## API Usage

### Basic Symbol Screening

```rust
use nzeza::domain::services::symbol_screening::{
    SymbolScreeningService, SymbolMarketData
};
use nzeza::domain::services::indicators::Candle;
use std::time::Duration;

#[tokio::main]
async fn main() {
    // Create service with 5-minute cache TTL
    let service = SymbolScreeningService::new(Duration::from_secs(300));

    // Prepare market data
    let candles = vec![
        Candle::new(100.0, 102.0, 98.0, 101.0, 1000.0).unwrap(),
    ];
    let market_data = SymbolMarketData {
        candles,
        volumes: vec![500_000.0],
        bid: 100.0,
        ask: 100.2,
    };

    // Screen single symbol
    let result = service
        .screen_symbol(
            "BTC-USD".to_string(),
            "dydx".to_string(),
            market_data,
        )
        .await;

    println!("Score: {:.2}", result.overall_score);
    println!("Recommendation: {:?}", result.recommendation);
}
```

### Screening Multiple Symbols

```rust
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    let service = SymbolScreeningService::with_default_cache_ttl();
    
    let mut symbols_data = HashMap::new();
    
    // Add multiple symbols...
    symbols_data.insert("BTC-USD".to_string(), market_data_btc);
    symbols_data.insert("ETH-USD".to_string(), market_data_eth);
    
    // Screen all at once
    let ranked_results = service
        .screen_all_symbols("dydx".to_string(), symbols_data)
        .await;
    
    // Results are sorted by score (descending)
    for (rank, result) in ranked_results.iter().enumerate() {
        println!(
            "#{}: {} - {:.2} ({})",
            rank + 1,
            result.symbol,
            result.overall_score,
            result.recommendation
        );
    }
}
```

### Monitoring Cache Performance

```rust
#[tokio::main]
async fn main() {
    let service = SymbolScreeningService::with_default_cache_ttl();
    
    // Screen symbols...
    
    // Get cache statistics
    let stats = service.get_cache_stats().await;
    println!("Cache hits: {}", stats.hits);
    println!("Cache misses: {}", stats.misses);
    println!("Hit rate: {:.2}%", stats.hit_rate());
    
    // Clear cache if needed
    service.clear_cache().await;
    
    // Reset statistics
    service.reset_cache_stats().await;
}
```

## Logging

The screening module uses structured logging with `tracing`. Control verbosity with the `RUST_LOG` environment variable:

```bash
# Debug level - see all score calculations
RUST_LOG=nzeza=debug cargo run

# Info level - see results only
RUST_LOG=nzeza=info cargo run

# Very verbose - all internal details
RUST_LOG=nzeza=trace cargo run
```

### Log Examples

**Cache hit:**
```
DEBUG nzeza::domain::services::symbol_screening: Cache hit - returning cached screening result
    symbol: BTC-USD
    exchange: dydx
    cache_age_ms: 125
    cache_hit_rate: 50.00%
```

**Calculation complete:**
```
DEBUG nzeza::domain::services::screening::aggregator: Completed scalping potential calculation
    symbol: BTC-USD
    exchange: dydx
    overall_score: 0.65
    volatility_score: 0.75
    volume_score: 0.80
    spread_score: 0.50
    momentum_score: 0.60
    recommendation: GoodCandidate
```

**Results published:**
```
INFO nzeza::domain::services::symbol_screening: Screening result calculated and cached
    symbol: BTC-USD
    exchange: dydx
    overall_score: 0.65
    recommendation: GoodCandidate
    cache_size: 5
```

## Integration with Actor System

The screening runs periodically via the `SymbolScreeningActor`:

```rust
use nzeza::application::actors::screening_actor::SymbolScreeningActor;

#[tokio::main]
async fn main() {
    // Actor runs screening at configured interval
    let actor = SymbolScreeningActor::new(
        screening_service,
        symbol_data_provider,
        repository,
    );
    
    // Start periodic screening
    actor.run().await;
}
```

## Performance Considerations

### Caching

- **Default TTL**: 5 minutes (300 seconds)
- **Cache Key**: `{exchange}:{symbol}`
- **Hit Rate Target**: 70%+ (depends on symbol freshness requirements)

### Database Persistence

- Screening results are stored with timestamp
- Useful for backtesting and analysis
- Index on `(symbol, exchange, screened_at)`

### Optimization Tips

1. **Increase Cache TTL** if symbol characteristics don't change frequently
2. **Batch screening** multiple symbols together
3. **Use `reset_cache_stats()`** periodically to monitor performance
4. **Filter symbols** before screening to reduce load

## Testing

### Unit Tests

```bash
# Run screening tests
cargo test symbol_screening --lib

# Run with backtrace
RUST_BACKTRACE=1 cargo test symbol_screening --lib
```

### Integration Tests

```bash
# Run end-to-end tests
cargo test --test symbol_screening_e2e
```

### Test Coverage

Key test scenarios:

1. ✅ Score calculation with various market conditions
2. ✅ Ranking and sorting by score
3. ✅ Cache hit/miss tracking
4. ✅ Cache expiration and refresh
5. ✅ Multi-symbol screening workflow
6. ✅ Recommendation categorization
7. ✅ Multiple exchange support

## Troubleshooting

### Low/Inconsistent Scores

- **Check market data**: Ensure candles have sufficient volatility
- **Check volumes**: Ensure volume data is realistic
- **Check spread**: Bid-ask spread might be too wide

### High Cache Miss Rate

- **Reduce cache TTL**: `SCREENING_CACHE_TTL_SECONDS`
- **Increase screening interval**: `SCREENING_INTERVAL_SECONDS`
- **Check symbol diversity**: If screening too many unique symbols

### Performance Issues

- **Profile**: Use `RUST_LOG=nzeza=trace` to identify bottlenecks
- **Reduce symbol count**: Screen fewer symbols per run
- **Increase interval**: Reduce screening frequency
- **Use database indices**: Ensure database queries are optimized

## Best Practices

1. **Use default cache TTL** for most use cases (5 minutes)
2. **Screen symbols in batches** for better performance
3. **Monitor cache statistics** to optimize TTL
4. **Store results in database** for historical analysis
5. **Use info-level logs** in production, debug in development
6. **Validate market data** before screening
7. **Test with diverse symbols** to ensure robust scoring

## Future Enhancements

- [ ] Machine learning scoring optimization
- [ ] Real-time symbol discovery from exchange APIs
- [ ] Advanced technical indicators (RSI, MACD, Bollinger Bands)
- [ ] Multi-timeframe analysis
- [ ] Correlation analysis between symbols
- [ ] Risk-adjusted scoring
- [ ] WebSocket real-time updates

## Related Documentation

- [AGENTS.md](./AGENTS.md) - Architecture and development guidelines
- [TDD_WORKFLOW.md](./TDD_WORKFLOW.md) - Test-driven development approach
- [README.md](./README.md) - General project documentation
