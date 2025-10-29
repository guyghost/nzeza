# Design Document: Symbol Screening for Scalping Potential

## Architecture Overview

### Components
```
┌─────────────────────────────────────────────────────────────┐
│ dYdX Exchange Actor (existing)                              │
│ - Connects to WebSocket                                      │
│ - Streams market data (prices, volumes, spreads)            │
└──────────────────┬──────────────────────────────────────────┘
                   │
                   ├─────────────────────────────────────────────┐
                   ↓                                             │
┌──────────────────────────────────────┐                       │
│ Symbol Screening Service (new)       │                       │
│ - Evaluates scalping criteria        │                       │
│ - Scores each symbol                 │                       │
│ - Ranks by potential                 │                       │
└──────────────────┬───────────────────┘                       │
                   │                                           │
                   ├──→ Database (SQLite)                      │
                   │    - Symbol screening metadata            │
                   │    - Historical scores & trends           │
                   │                                           │
                   └──→ Strategy Actors (existing)             │
                        - Prioritize high-potential symbols ←──┘
```

### Key Metrics for Scalping Potential

1. **Volatility Score** (0-1)
   - Formula: `(High-Low) / Close` over recent candles
   - Higher volatility = more price movement opportunities
   - Target: > 0.02 (2% volatility in recent candles)

2. **Volume Score** (0-1)
   - Formula: Normalized recent volume vs. average
   - Higher volume = better liquidity, easier entry/exit
   - Target: > $1M daily volume on dYdX

3. **Spread Score** (0-1)
   - Formula: `(Ask - Bid) / Mid` 
   - Tighter spreads = lower execution slippage
   - Target: < 0.1% spread for liquid pairs

4. **Momentum Score** (0-1)
   - Based on combined technical signals (RSI, MACD, EMA crossover)
   - Indicates current trend strength
   - Used as multiplier on other scores

5. **Overall Scalping Potential Score** (0-1)
   - Weighted combination: `0.3*volatility + 0.3*volume + 0.2*spread + 0.2*momentum`
   - Only symbols with score > 0.5 are candidates

### Data Model

```rust
struct SymbolScreeningResult {
    symbol: String,
    exchange: Exchange,
    timestamp: DateTime<Utc>,
    
    // Scoring components
    volatility_score: f64,
    volume_score: f64,
    spread_score: f64,
    momentum_score: f64,
    
    // Overall score
    scalping_potential_score: f64,
    
    // Metadata
    rank: usize,
    recommendation: ScreeningRecommendation, // Buy, Avoid, Hold
    last_trade_price: f64,
    recent_high: f64,
    recent_low: f64,
}

enum ScreeningRecommendation {
    BestCandidate,  // Score > 0.75
    GoodCandidate,  // Score 0.6-0.75
    FairCandidate,  // Score 0.5-0.6
    Avoid,          // Score < 0.5
}
```

### Integration with Existing Systems

#### 1. Exchange Actor Integration
- Exchange actors continue feeding market data via existing channels
- Screening service consumes price/volume data from existing candle builder
- No changes to existing WebSocket connections

#### 2. Database Integration
- New table: `symbol_screening_results` for storing results
- Tracks historical screening scores to detect emerging opportunities
- Enables analytics on which symbols were most profitable

#### 3. Strategy Integration
- Strategy actors can query current screening results
- Prioritize trading signals from high-potential symbols
- Optional: weight signals by screening score

#### 4. API Integration
- New endpoint: `GET /api/screening/symbols/dydx` → ranked symbols
- Optional: `GET /api/screening/symbols/dydx/{symbol}` → detailed scoring breakdown

## Implementation Strategy

### Phase 1: Core Screening (Red-Green-Refactor TDD)
1. **Data Model**: `SymbolScreeningResult` structure with validation
2. **Scoring Engine**: Individual score calculation functions (volatility, volume, spread, momentum)
3. **Aggregation**: Combined scoring with weighting
4. **Persistence**: Database table and repository methods
5. **Tests**: Unit tests for each scoring component, integration tests for ranking

### Phase 2: Screening Service Actor
1. Spawn screening actor at startup
2. Periodic task: evaluate all dYdX symbols every 5 minutes
3. Cache results to avoid redundant calculations
4. Update database with new scores
5. Emit `ScreeningCompleted` event for downstream consumption

### Phase 3: API & Integration
1. Add API endpoints for retrieving screening results
2. Expose results to strategy actors
3. Update strategy logic to consider screening scores
4. Documentation and examples

## Trade-offs & Decisions

### Why Periodic Screening (vs. Continuous)?
- **Rationale**: Screening is compute-intensive (evaluates all symbols)
  - Continuous screening = constant CPU usage
  - Periodic (every 5 min) = efficient, captures trends
  - Interval configurable in `config.toml`

### Why Multiple Scoring Components?
- **Rationale**: Single metric insufficient for scalping
  - Volatility alone = choppy/whipsaw risk
  - Volume alone = can't account for micro-cap illiquid pairs
  - Combined signals = more robust decisions

### Why Cache Results?
- **Rationale**: Avoid redundant API/computation costs
  - dYdX market data updates at ~5-second intervals
  - Screening score changes slowly (minutes-scale)
  - Cache for 1-5 minutes sufficient

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Screening identifies "hot" symbols that crash | High loss | Add risk management: position sizing, stop-loss enforcement |
| API rate limits if screening polls too often | Service | Start with 5-min intervals, monitor, adjust |
| Compute overhead degrades other systems | Medium | Run screening on separate thread/actor, profile |
| Symbols disappear from exchange mid-trade | Medium | Graceful handling in order executor for delisted pairs |
| False signals from technical indicators | Medium | Use multiple indicators, require confluence, backtest |

## Testing Strategy

### Unit Tests
- ✅ Volatility score calculation with known data
- ✅ Volume score normalization
- ✅ Spread calculation and weighting
- ✅ Momentum score from technical indicators
- ✅ Overall score aggregation and ranking
- ✅ Recommendation categorization

### Integration Tests
- ✅ Screening actor spawning and lifecycle
- ✅ Database persistence and retrieval
- ✅ End-to-end scoring with mock exchange data
- ✅ API endpoint responses

### Property-Based Tests
- ✅ All scores in [0, 1] range
- ✅ Ranking order maintained
- ✅ Score aggregation commutative/associative properties

## Future Extensions
- **Multi-Exchange Screening**: Extend to Binance, Kraken, Coinbase, Hyperliquid
- **Custom Weightings**: Allow per-strategy scoring preferences
- **Machine Learning**: Predict scalping success using historical screening + performance
- **Anomaly Detection**: Flag unusual volume/spread conditions
