# Specification: Result Ranking

## Overview
The system must rank screened symbols by scalping potential score and maintain ranked lists for strategy actors and API consumers.

## ADDED Requirements

### Requirement: Sort Symbols by Scalping Potential Score
- **ID**: REQ-RANK-001
- **Type**: Functional
- **Priority**: High
- **Stability**: Stable

The bot shall sort and rank all screened symbols by their scalping potential score.

#### Scenario: Rank symbols in descending order
**Given** symbols evaluated with scores: [0.85, 0.42, 0.78, 0.91, 0.55]
**When** bot generates rankings
**Then** it shall sort in descending order: [0.91, 0.85, 0.78, 0.55, 0.42]
**And** assign ranks: [1st, 2nd, 3rd, 4th, 5th]
**And** store rank in each result

#### Scenario: Apply minimum threshold filter
**Given** symbols with scores: [0.85, 0.42, 0.78, 0.91, 0.55]
**When** filtering for candidates (threshold 0.50)
**Then** it shall exclude symbols below threshold: [0.42]
**And** return only: [0.91, 0.85, 0.78, 0.55]
**And** re-rank as: [1st, 2nd, 3rd, 4th]

#### Scenario: Generate top N candidates
**Given** 200 symbols screened and ranked
**When** bot provides "top candidates" list
**Then** it shall return top 10 symbols (or configurable N)
**And** include rank, score, and recommendation for each
**And** sort by score descending within top N

#### Scenario: Maintain ranked history
**Given** symbols ranked at multiple time points
**When** ranking regenerates after new evaluation
**Then** it shall preserve historical rank data
**And** track rank changes over time (e.g., "BTC moved from 3rd to 1st")
**And** log significant rank movements at INFO level: "BTC-USD: rank 5 → 1"

### Requirement: Group Symbols by Recommendation Category
- **ID**: REQ-RANK-002
- **Type**: Functional
- **Priority**: Medium
- **Stability**: Stable

The bot shall organize symbols by their recommendation category.

#### Scenario: Create category lists
**Given** symbols with recommendations:
  - BestCandidate: BTC-USD (0.85), ETH-USD (0.82)
  - GoodCandidate: SOL-USD (0.75), ADA-USD (0.65)
  - FairCandidate: XRP-USD (0.58), DOGE-USD (0.52)
  - Avoid: SHIB-USD (0.35), PEPE-USD (0.20)

**When** bot groups by category
**Then** it shall create lists per category
**And** maintain ranking within each category
**And** return structure:
```json
{
  "best_candidates": [
    {"symbol": "BTC-USD", "score": 0.85, "rank": 1},
    {"symbol": "ETH-USD", "score": 0.82, "rank": 2}
  ],
  "good_candidates": [...],
  "fair_candidates": [...],
  "avoid": [...]
}
```

#### Scenario: Expose category counts
**Given** categorized symbols
**When** API provides summary
**Then** it shall return category distribution:
```json
{
  "total_symbols_screened": 200,
  "best_candidates_count": 2,
  "good_candidates_count": 15,
  "fair_candidates_count": 28,
  "avoid_count": 155
}
```

### Requirement: Support Filtering & Sorting Options
- **ID**: REQ-RANK-003
- **Type**: Functional
- **Priority**: Medium
- **Stability**: Stable

The bot shall support flexible filtering and sorting for different use cases.

#### Scenario: Filter by score range
**Given** client requests symbols with score between 0.60 and 0.80
**When** bot applies filter
**Then** it shall return only symbols in range: `0.60 ≤ score ≤ 0.80`
**And** maintain ranking within filtered set

#### Scenario: Filter by recommendation category
**Given** client requests only "BestCandidate" symbols
**When** bot applies category filter
**Then** it shall return symbols with score ≥ 0.75
**And** sort by score descending within category

#### Scenario: Sort by different criteria
**Given** symbols can be sorted by multiple fields
**When** client specifies sort preference
**Then** bot shall support sorting by:
  - **score** (default, descending): highest potential first
  - **volatility** (descending): most volatile first
  - **volume** (descending): highest liquidity first
  - **spread** (descending): tightest spreads first
  - **momentum** (descending): strongest trends first
  - **name** (ascending): alphabetical order

#### Scenario: Combined filter + sort
**Given** client requests: category=GoodCandidate, sort_by=volatility
**When** bot processes request
**Then** it shall:
  1. Filter to GoodCandidate (0.60-0.75 score)
  2. Sort by volatility descending
  3. Return ranked list in new order

### Requirement: Track Ranking History & Trends
- **ID**: REQ-RANK-004
- **Type**: Functional
- **Priority**: Low
- **Stability**: Stable

The bot shall maintain historical ranking data for trend analysis.

#### Scenario: Record ranking at each evaluation
**Given** screening evaluation completes
**When** ranking generated
**Then** it shall store snapshot of all ranks with timestamp
**And** create ranking_history records in database
**And** enable lookback: "What was BTC's rank 1 hour ago?"

#### Scenario: Calculate rank change metrics
**Given** historical ranking data available
**When** bot generates trend report
**Then** it shall calculate for each symbol:
  - Rank change (previous to current)
  - Score trend (moving average over 4 samples)
  - Category changes (e.g., FairCandidate → GoodCandidate)
**And** log significant changes at INFO level

#### Scenario: Identify emerging opportunities
**Given** historical ranking data and trend metrics
**When** bot runs daily analysis
**Then** it shall identify:
  - "Most improved": symbols with best rank improvement
  - "Rising momentum": symbols with increasing scores consistently
  - "Falling risk": symbols dropping in ranking (good for risk avoidance)
**And** emit `RankingTrendAnalysis` event for analytics

## Database Schema

```sql
CREATE TABLE IF NOT EXISTS ranking_snapshots (
    id INTEGER PRIMARY KEY,
    ranking_id TEXT NOT NULL UNIQUE,
    exchange TEXT NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    total_symbols_screened INTEGER NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_ranking_timestamp(timestamp),
    INDEX idx_ranking_exchange(exchange)
);

CREATE TABLE IF NOT EXISTS symbol_ranks (
    id INTEGER PRIMARY KEY,
    ranking_id TEXT NOT NULL,
    symbol TEXT NOT NULL,
    rank INTEGER NOT NULL,
    scalping_potential_score REAL NOT NULL,
    volatility_score REAL,
    volume_score REAL,
    spread_score REAL,
    momentum_score REAL,
    recommendation TEXT NOT NULL,
    price REAL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (ranking_id) REFERENCES ranking_snapshots(ranking_id),
    UNIQUE(ranking_id, symbol),
    INDEX idx_symbol_ranks(rank),
    INDEX idx_symbol_recommendation(recommendation)
);

CREATE TABLE IF NOT EXISTS ranking_history (
    id INTEGER PRIMARY KEY,
    symbol TEXT NOT NULL,
    exchange TEXT NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    rank INTEGER NOT NULL,
    rank_change INTEGER,  -- Positive = improvement, Negative = decline
    score REAL NOT NULL,
    score_change REAL,    -- Score difference from previous
    recommendation TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_history_symbol_time(symbol, timestamp),
    INDEX idx_history_exchange(exchange)
);
```

## Data Structures

```rust
struct RankingSnapshot {
    ranking_id: String,
    exchange: Exchange,
    timestamp: DateTime<Utc>,
    total_symbols_screened: usize,
}

struct SymbolRank {
    ranking_id: String,
    symbol: String,
    rank: usize,
    scalping_potential_score: f64,
    volatility_score: f64,
    volume_score: f64,
    spread_score: f64,
    momentum_score: f64,
    recommendation: Recommendation,
    price: f64,
}

struct RankingTrend {
    symbol: String,
    rank_change: i32,
    score_change: f64,
    previous_rank: usize,
    current_rank: usize,
    trend: TrendDirection,
}

enum TrendDirection {
    Improving,      // Rank moving up (lower number = better)
    Declining,      // Rank moving down
    Stable,         // Rank unchanged
}
```

## API Contracts

### Get Top Candidates
```
GET /api/screening/symbols/dydx?limit=10&filter=best
Response: List[SymbolRank] sorted by score descending
```

### Get Categorized Results
```
GET /api/screening/symbols/dydx/categories
Response: {
  best_candidates: List[SymbolRank],
  good_candidates: List[SymbolRank],
  fair_candidates: List[SymbolRank],
  avoid: List[SymbolRank],
  summary: { counts... }
}
```

### Get Ranking Trends
```
GET /api/screening/symbols/dydx/trends
Response: List[RankingTrend]
```

## Filtering & Sorting Operators

| Operator | Syntax | Example |
|----------|--------|---------|
| Score Range | `score_min` + `score_max` | `?score_min=0.6&score_max=0.8` |
| Category | `category` | `?category=good_candidate` |
| Sort Field | `sort_by` | `?sort_by=volatility` |
| Sort Order | `sort_order` | `?sort_order=asc` (default: desc) |
| Limit Results | `limit` | `?limit=20` (default: 100) |

## Performance Requirements

- **Ranking generation**: < 100ms for 200 symbols
- **Category grouping**: < 50ms
- **Query response time**: < 200ms including database I/O
- **Historical data retention**: Keep 72 hours of snapshots (288 snapshots at 15-min intervals)

## Error Handling

| Error | Handling |
|-------|----------|
| Empty ranking list | Return empty list with count 0, log INFO |
| Invalid filter parameters | Return 400 Bad Request with error details |
| Missing historical data | Omit trend metrics, return partial data |

## Related Capabilities
- Screening Evaluation (provides scores for ranking)
- Result Persistence (stores historical rankings)
- API Endpoint (exposes ranking results)

## Testing
- Unit: Sort algorithm with known inputs
- Integration: Full ranking pipeline with database
- Property-based: Ranks always unique and sequential
- Performance: Benchmark ranking generation < 100ms
