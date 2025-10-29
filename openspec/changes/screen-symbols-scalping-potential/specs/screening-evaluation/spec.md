# Specification: Screening Evaluation Engine

## Overview
The system must evaluate each discovered symbol against scalping criteria to determine its potential for profitable scalping trades.

## ADDED Requirements

### Requirement: Calculate Volatility Score
- **ID**: REQ-EVAL-001
- **Type**: Functional
- **Priority**: High
- **Stability**: Stable

The bot shall calculate a volatility score for each symbol indicating price movement intensity.

#### Scenario: Calculate volatility from recent candles
**Given** a symbol has historical candle data (OHLCV)
**When** the bot evaluates volatility score
**Then** it shall calculate: `volatility = (high - low) / close` over recent candles (last 20)
**And** average the individual candle volatilities
**And** normalize result to [0, 1] range using sigmoid function
**And** store score in range [0.0, 1.0]

#### Scenario: Return zero for flat market
**Given** a symbol has minimal price movement
**When** volatility calculation runs
**Then** it shall return score close to 0.0 (< 0.1)
**And** log at DEBUG level: "BTC-USD has low volatility: 0.05"

#### Scenario: Return high score for volatile market
**Given** a symbol has significant price swings (> 3% per candle)
**When** volatility calculation runs
**Then** it shall return score close to 1.0 (> 0.7)
**And** log at DEBUG level: "SOL-USD has high volatility: 0.92"

#### Scenario: Handle insufficient data
**Given** a symbol has fewer than 10 candles of history
**When** volatility calculation runs
**Then** it shall return `None` (insufficient data)
**And** log at DEBUG level: "BTC-USD: insufficient candles for volatility (have 5, need 10)"

### Requirement: Calculate Volume Score
- **ID**: REQ-EVAL-002
- **Type**: Functional
- **Priority**: High
- **Stability**: Stable

The bot shall calculate a volume score indicating trading liquidity.

#### Scenario: Calculate volume from trade history
**Given** a symbol has recent trade volume data
**When** the bot evaluates volume score
**Then** it shall sum recent volume over last 1 hour
**And** normalize against 24-hour average volume
**And** express as fraction of daily volume (0.0 = no volume, 1.0 = very high volume)
**And** floor at $1M USD: `min($1M, volume) / $1M`

#### Scenario: Return high score for liquid symbols
**Given** a symbol is ETH-USD with $50M daily volume
**When** volume score calculated
**Then** it shall return 1.0 (max liquidity)

#### Scenario: Return zero for illiquid symbols
**Given** a symbol is a micro-cap alt coin with $10k daily volume
**When** volume score calculated
**Then** it shall return 0.01 (very illiquid, should be avoided for scalping)

#### Scenario: Handle missing volume data
**Given** a symbol has no volume data yet
**When** volume calculation runs
**Then** it shall return 0.0 (no trades yet)

### Requirement: Calculate Spread Score
- **ID**: REQ-EVAL-003
- **Type**: Functional
- **Priority**: High
- **Stability**: Stable

The bot shall calculate a spread score indicating order book liquidity and execution costs.

#### Scenario: Calculate spread from order book
**Given** an order book with bid/ask prices
**When** the bot evaluates spread score
**Then** it shall calculate: `spread% = (ask - bid) / mid_price * 100`
**And** normalize: `spread_score = 1.0 - min(spread%, 1%) / 1%`
**And** return score in [0, 1] where 1.0 = tight spread, 0.0 = wide spread

#### Scenario: Return high score for tight spreads
**Given** BTC-USD has 0.01% bid-ask spread
**When** spread calculated
**Then** it shall return 0.99 (almost perfect liquidity)

#### Scenario: Return low score for wide spreads
**Given** ALT-USD has 2.0% bid-ask spread (illiquid altcoin)
**When** spread calculated
**Then** it shall return 0.0 (too wide for scalping)

#### Scenario: Handle market orders without order book
**Given** order book data unavailable (only last price available)
**When** spread calculation runs
**Then** it shall estimate spread from recent trades
**And** use trade impact measurement as proxy
**Or** return neutral score 0.5 if no estimate possible

### Requirement: Calculate Momentum Score
- **ID**: REQ-EVAL-004
- **Type**: Functional
- **Priority**: Medium
- **Stability**: Stable

The bot shall calculate momentum score from technical indicators.

#### Scenario: Calculate momentum from RSI
**Given** a symbol has RSI values calculated (period 14)
**When** momentum score calculated
**Then** it shall map RSI to momentum: 
  - RSI < 30: strong downward momentum, score 0.1
  - RSI 30-70: neutral momentum, score 0.5
  - RSI > 70: strong upward momentum, score 0.9

#### Scenario: Calculate momentum from MACD
**Given** a symbol has MACD line and signal line
**When** momentum calculated
**Then** it shall use MACD histogram:
  - Histogram > 0: upward momentum, +0.3 to score
  - Histogram < 0: downward momentum, -0.3 to score
  - Histogram crossing zero: reversal signal, 0.5 score

#### Scenario: Combine multiple momentum indicators
**Given** both RSI and MACD available
**When** momentum score calculated
**Then** it shall average with weighting: `0.6*rsi_momentum + 0.4*macd_momentum`
**And** normalize final result to [0, 1]

#### Scenario: Use EMA crossover for trend confirmation
**Given** EMA(3) and EMA(5) calculated
**When** evaluating momentum
**Then** it shall use crossover as confirmation:
  - EMA(3) > EMA(5): uptrend, boost momentum score by 0.1
  - EMA(3) < EMA(5): downtrend, reduce momentum score by 0.1

### Requirement: Aggregate Scores into Scalping Potential
- **ID**: REQ-EVAL-005
- **Type**: Functional
- **Priority**: High
- **Stability**: Stable

The bot shall combine individual scores into overall scalping potential score.

#### Scenario: Combine scores with weighting
**Given** individual scores calculated:
  - Volatility: 0.85
  - Volume: 0.70
  - Spread: 0.90
  - Momentum: 0.60

**When** overall scalping potential calculated
**Then** it shall use formula: 
  `score = 0.30*volatility + 0.30*volume + 0.20*spread + 0.20*momentum`
**And** result: `(0.30*0.85) + (0.30*0.70) + (0.20*0.90) + (0.20*0.60) = 0.755`
**And** round to 2 decimal places

#### Scenario: Generate recommendation category
**Given** overall scalping potential score calculated
**When** bot generates recommendation
**Then** it shall categorize as:
  - Score ≥ 0.75: "BestCandidate"
  - Score 0.60-0.75: "GoodCandidate"  
  - Score 0.50-0.60: "FairCandidate"
  - Score < 0.50: "Avoid"

#### Scenario: Rank symbols by score
**Given** multiple symbols evaluated with scores
**When** bot generates ranking
**Then** it shall sort by score descending: [0.85, 0.78, 0.63, 0.42]
**And** assign rank: [1st, 2nd, 3rd, 4th]
**And** only include symbols with score ≥ 0.50 in top candidates

## Data Structure

```rust
struct ScreeningScore {
    symbol: String,
    exchange: Exchange,
    timestamp: DateTime<Utc>,
    
    // Component scores
    volatility_score: f64,    // [0.0, 1.0]
    volume_score: f64,        // [0.0, 1.0]
    spread_score: f64,        // [0.0, 1.0]
    momentum_score: f64,      // [0.0, 1.0]
    
    // Overall score
    scalping_potential_score: f64,  // [0.0, 1.0]
    recommendation: Recommendation, // BestCandidate, GoodCandidate, FairCandidate, Avoid
    rank: usize,
    
    // Supporting data
    current_price: f64,
    recent_high: f64,
    recent_low: f64,
    recent_volume_1h: f64,  // USD volume in last hour
}

enum Recommendation {
    BestCandidate,   // ≥ 0.75
    GoodCandidate,   // 0.60-0.75
    FairCandidate,   // 0.50-0.60
    Avoid,           // < 0.50
}
```

## Calculation Constraints

| Component | Minimum Data | Update Frequency | Calculation Time |
|-----------|--------------|------------------|------------------|
| Volatility | 10 candles | Every 10s | < 10ms |
| Volume | 1 hour history | Every 1m | < 5ms |
| Spread | 1 order book snapshot | Real-time | < 1ms |
| Momentum | 14 candles for RSI | Every 10s | < 5ms |

## Algorithm Pseudo-code

```
for each symbol in discovered_symbols:
    if insufficient_data(symbol):
        continue  // Skip this cycle
    
    vol_score = calculate_volatility(last_20_candles)
    vol_score = normalize(vol_score)
    
    vol_score = calculate_volume(1h_volume, 24h_average)
    vol_score = normalize(vol_score)
    
    spd_score = calculate_spread(bid, ask)
    spd_score = normalize(spd_score)
    
    mom_score = calculate_momentum(rsi, macd, ema_crossover)
    mom_score = normalize(mom_score)
    
    potential = (0.30 * vol_score + 0.30 * vol_score + 
                 0.20 * spd_score + 0.20 * mom_score)
    
    recommendation = categorize(potential)
    
    store_result(symbol, vol_score, vol_score, spd_score, mom_score, 
                 potential, recommendation)

rank_symbols_by_potential()
```

## Error Handling

| Error | Handling |
|-------|----------|
| Missing candle data | Skip symbol, log DEBUG |
| Volume calculation error | Use previous value, log WARN |
| Order book unavailable | Estimate from last trade, score 0.5 |
| Technical indicator error | Use moving average of recent scores |

## Performance Requirements

- **Evaluation per symbol**: < 50ms
- **All symbols evaluated**: < 10s for 100 symbols
- **Cache frequency**: Calculate fresh every 5 minutes
- **Update on new data**: Incremental updates as new candles arrive

## Related Capabilities
- Symbol Discovery (provides symbol list)
- Result Persistence (stores evaluation results)
- Result Ranking (uses evaluation scores)

## Testing
- Unit: Each score calculation with known inputs
- Integration: Full evaluation pipeline
- Property-based: All scores stay in [0,1] range
