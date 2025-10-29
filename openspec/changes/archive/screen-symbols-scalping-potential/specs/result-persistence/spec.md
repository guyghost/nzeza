# Specification: Result Persistence

## Overview
The system must store screening results, ranking snapshots, and historical data in SQLite for analytics, auditing, and trend analysis.

## ADDED Requirements

### Requirement: Persist Screening Results to Database
- **ID**: REQ-PERSIST-001
- **Type**: Functional
- **Priority**: High
- **Stability**: Stable

The bot shall save screening evaluation results to SQLite.

#### Scenario: Store screening result on completion
**Given** screening evaluation completes for BTC-USD
**When** bot processes results
**Then** it shall create record in `symbol_screening_results` table with:
  - symbol: "BTC-USD"
  - exchange: "dydx"
  - timestamp: current UTC time
  - volatility_score: 0.85
  - volume_score: 0.70
  - spread_score: 0.90
  - momentum_score: 0.60
  - scalping_potential_score: 0.755
  - recommendation: "GoodCandidate"

#### Scenario: Update existing result on re-evaluation
**Given** BTC-USD already has screening result from 2 minutes ago
**When** new screening evaluation runs
**Then** it shall:
  1. Check for existing result for same symbol + exchange + day
  2. If exists and < 5 min old: skip (use cache)
  3. If exists and >= 5 min old: update with new scores
  4. Update timestamp to new evaluation time
  5. Log at DEBUG level: "Updated screening for BTC-USD: 0.755 (was 0.750)"

#### Scenario: Store supporting data with result
**Given** screening evaluation completes
**When** saving result
**Then** it shall also save context:
  - current_price: 45230.50
  - recent_high_24h: 46000.00
  - recent_low_24h: 44500.00
  - volume_24h_usd: 50000000.00
  - bid_ask_spread: 0.001
  - timestamp: evaluation time

### Requirement: Persist Ranking Snapshots
- **ID**: REQ-PERSIST-002
- **Type**: Functional
- **Priority**: High
- **Stability**: Stable

The bot shall create snapshots of the complete ranking at each evaluation cycle.

#### Scenario: Create ranking snapshot on completion
**Given** all symbols evaluated and ranked
**When** bot completes screening cycle
**Then** it shall:
  1. Generate unique ranking_id (UUID or timestamp-based)
  2. Create ranking_snapshots record with:
     - ranking_id: unique identifier
     - exchange: "dydx"
     - timestamp: evaluation time
     - total_symbols_screened: 150
  3. Create symbol_ranks record for each symbol with rank, scores, recommendation
  4. Commit transaction atomically

#### Scenario: Query best candidates from latest ranking
**Given** ranking snapshot exists in database
**When** API requests top 10 candidates
**Then** it shall:
  1. Query latest ranking_snapshots by timestamp DESC
  2. Join with symbol_ranks for that ranking_id
  3. Filter recommendation = "BestCandidate"
  4. Sort by rank ASC
  5. Return top 10 results

#### Scenario: Preserve complete ranking history
**Given** multiple ranking cycles completed (e.g., one every 5 minutes)
**When** bot stores new ranking
**Then** it shall:
  1. Keep all previous rankings intact
  2. Not delete old rankings (enable historical analysis)
  3. Create separate ranking_history entries for trend tracking
  4. Log at INFO level: "Stored ranking #247 with 150 symbols"

### Requirement: Maintain Historical Trend Data
- **ID**: REQ-PERSIST-003
- **Type**: Functional
- **Priority**: Medium
- **Stability**: Stable

The bot shall track rank changes and score trends over time.

#### Scenario: Record rank change when symbol's position changes
**Given** 
  - Previous ranking: BTC-USD at rank 1 with score 0.80
  - Current ranking: BTC-USD at rank 2 with score 0.78

**When** new ranking completes
**Then** it shall create ranking_history entry:
  - symbol: "BTC-USD"
  - exchange: "dydx"
  - timestamp: current time
  - rank: 2
  - rank_change: -1 (moved down)
  - score: 0.78
  - score_change: -0.02
  - recommendation: "GoodCandidate"
  - Log at DEBUG level: "BTC-USD moved from rank 1→2, score -0.02"

#### Scenario: Calculate moving average score trend
**Given** historical scores for a symbol over multiple cycles:
  - t-40min: 0.80
  - t-30min: 0.81
  - t-20min: 0.79
  - t-10min: 0.78
  - t-now: 0.77

**When** bot analyzes trends
**Then** it shall calculate:
  - Score trend: -0.075 (declining)
  - 4-period moving average: (0.81+0.79+0.78+0.77)/4 = 0.7875
  - Trend direction: "Declining"
  - Log analysis at INFO level if significant change

#### Scenario: Clean up old historical records
**Given** database contains screening results older than retention period (72 hours)
**When** daily maintenance task runs
**Then** it shall:
  1. Delete ranking_snapshots older than 72 hours
  2. Keep symbol_screening_results for 30 days (enable longer trends)
  3. Aggregate ranking_history into daily summaries after 7 days
  4. Log at INFO level: "Cleaned up 1,200 old ranking records"

### Requirement: Query & Retrieve Results Efficiently
- **ID**: REQ-PERSIST-004
- **Type**: Functional
- **Priority**: Medium
- **Stability**: Stable

The bot shall retrieve screening results with good query performance.

#### Scenario: Get latest screening for a symbol
**Given** multiple screening results exist for BTC-USD
**When** strategy actor queries for current score
**Then** it shall:
  1. Query `SELECT * FROM symbol_screening_results WHERE symbol='BTC-USD' AND exchange='dydx' ORDER BY timestamp DESC LIMIT 1`
  2. Return most recent result
  3. Execute < 10ms (with index on symbol + exchange + timestamp)

#### Scenario: Get all current rankings
**Given** latest ranking snapshot ID is known
**When** API requests current rankings
**Then** it shall:
  1. Query symbol_ranks by ranking_id
  2. Return 100+ symbols within < 100ms
  3. Support pagination: ?limit=20&offset=40

#### Scenario: Retrieve symbol's rank history
**Given** user requests rank trend for BTC-USD over last 24 hours
**When** query executes
**Then** it shall:
  1. Query ranking_history for BTC-USD, last 24h
  2. Return time-series data: [(timestamp, rank, score), ...]
  3. Execute < 50ms with proper indexing

#### Scenario: Get trending symbols
**Given** ranking history available
**When** API requests "most improved symbols"
**Then** it shall:
  1. Calculate rank improvement for each symbol (current - previous)
  2. Sort by improvement descending
  3. Return top 10 with greatest positive rank change
  4. Execute < 200ms

### Requirement: Handle Concurrent Writes Safely
- **ID**: REQ-PERSIST-005
- **Type**: Functional
- **Priority**: High
- **Stability**: Stable

The bot shall handle concurrent database writes without corruption or data loss.

#### Scenario: Atomic ranking snapshot storage
**Given** screening completes on 150 symbols simultaneously
**When** bot stores ranking snapshot
**Then** it shall:
  1. Use database transaction (BEGIN...COMMIT)
  2. Create ranking_snapshots record
  3. Create all 150 symbol_ranks records
  4. Commit atomically: all succeed or all rollback
  5. Never leave partial ranking in database

#### Scenario: Prevent duplicate screening results
**Given** screening for BTC-USD completes
**When** multiple tasks attempt to save same symbol
**Then** database constraint shall prevent duplicates:
  - UNIQUE(symbol, exchange) per day
  - Later write updates existing record
  - No duplicate entries created

#### Scenario: Maintain ordering guarantees
**Given** concurrent writes to ranking_history
**When** multiple rank changes recorded
**Then** all records stored with timestamp
**And** queries sorted by timestamp guarantee order
**And** no out-of-order records possible

## Database Schema

```sql
-- Main screening results table
CREATE TABLE IF NOT EXISTS symbol_screening_results (
    id INTEGER PRIMARY KEY,
    symbol TEXT NOT NULL,
    exchange TEXT NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    
    -- Scoring components
    volatility_score REAL NOT NULL CHECK(volatility_score >= 0 AND volatility_score <= 1),
    volume_score REAL NOT NULL CHECK(volume_score >= 0 AND volume_score <= 1),
    spread_score REAL NOT NULL CHECK(spread_score >= 0 AND spread_score <= 1),
    momentum_score REAL NOT NULL CHECK(momentum_score >= 0 AND momentum_score <= 1),
    
    -- Overall score
    scalping_potential_score REAL NOT NULL CHECK(scalping_potential_score >= 0 AND scalping_potential_score <= 1),
    recommendation TEXT NOT NULL,
    
    -- Supporting data
    current_price REAL NOT NULL,
    recent_high_24h REAL,
    recent_low_24h REAL,
    volume_24h_usd REAL,
    bid_ask_spread REAL,
    
    -- Metadata
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    -- Constraints
    UNIQUE(symbol, exchange, DATE(timestamp)),
    CHECK(current_price >= 0),
    CHECK(volume_24h_usd >= 0)
);

CREATE INDEX idx_screening_symbol_exchange ON symbol_screening_results(symbol, exchange);
CREATE INDEX idx_screening_timestamp ON symbol_screening_results(timestamp);
CREATE INDEX idx_screening_recommendation ON symbol_screening_results(recommendation);
CREATE INDEX idx_screening_score ON symbol_screening_results(scalping_potential_score);

-- Ranking snapshots (one per evaluation cycle)
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

-- Individual symbol ranks in a snapshot
CREATE TABLE IF NOT EXISTS symbol_ranks (
    id INTEGER PRIMARY KEY,
    ranking_id TEXT NOT NULL,
    symbol TEXT NOT NULL,
    rank INTEGER NOT NULL CHECK(rank >= 1),
    
    -- Scores for this rank
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
    
    INDEX idx_symbol_ranks_rank(rank),
    INDEX idx_symbol_ranks_recommendation(recommendation)
);

-- Historical rank changes and trends
CREATE TABLE IF NOT EXISTS ranking_history (
    id INTEGER PRIMARY KEY,
    symbol TEXT NOT NULL,
    exchange TEXT NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    
    rank INTEGER NOT NULL,
    rank_change INTEGER,
    score REAL NOT NULL,
    score_change REAL,
    recommendation TEXT NOT NULL,
    
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    INDEX idx_history_symbol_timestamp(symbol, timestamp),
    INDEX idx_history_exchange(exchange),
    INDEX idx_history_rank_change(rank_change)
);
```

## Data Retention Policy

| Table | Retention Period | Rationale |
|-------|-----------------|-----------|
| `symbol_screening_results` | 30 days | Enable trend analysis, not needed longer |
| `ranking_snapshots` | 72 hours | Keep recent snapshots, archive older |
| `symbol_ranks` | 72 hours | Follow snapshot retention |
| `ranking_history` | Indefinite | Aggregated, lightweight storage for trends |

## Performance Requirements

| Operation | Target Latency | Data Size |
|-----------|---------------|-----------| 
| Insert screening result | < 5ms | 1-2 KB per record |
| Create ranking snapshot | < 100ms | 150 symbols × 2KB = 300 KB |
| Query latest rank | < 10ms | Single row |
| Query ranking history | < 50ms | ~288 samples (72h at 15min) |
| Full rankings export | < 200ms | 150 symbols |

## Backup & Recovery

- Database file: `data/nzeza.db`
- Backup strategy: Copy database to cold storage weekly
- Recovery: SQLite supports point-in-time recovery via WAL (Write-Ahead Logging)
- Test recovery: Monthly verification of backup integrity

## Related Capabilities
- Screening Evaluation (generates results to persist)
- Result Ranking (uses persisted results)
- API Endpoint (queries persisted data)

## Testing
- Unit: CRUD operations on each table
- Integration: Full persistence pipeline with transactions
- Property-based: Constraints always enforced
- Performance: Benchmark all queries meet latency targets
