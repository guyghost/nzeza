# Specification: Symbol Discovery from dYdX

## Overview
The system must discover all available trading pairs (symbols) from dYdX exchange via API and maintain a current list of tradeable symbols.

## ADDED Requirements

### Requirement: Fetch Available Symbols from dYdX API
- **ID**: REQ-DISCOVERY-001
- **Type**: Functional
- **Priority**: High
- **Stability**: Stable

The bot shall fetch all available trading pairs from dYdX API with the following characteristics:

#### Scenario: Successfully retrieve markets list
**Given** the dYdX API is available and returns valid market data
**When** the bot initiates symbol discovery
**Then** it shall parse the market data and extract all symbol names
**And** store them in a list sorted alphabetically
**And** log the count of discovered symbols at INFO level

#### Scenario: Handle API errors gracefully
**Given** the dYdX API returns an error (5xx, connection timeout, etc.)
**When** the bot attempts symbol discovery
**Then** it shall log the error at WARN level
**And** it shall retry using exponential backoff (base 2 seconds, max 30 seconds)
**And** it shall fall back to last known symbol list from cache/database
**And** it shall not crash or block other bot functionality

#### Scenario: Update symbol list on schedule
**Given** the bot has initialized symbol discovery
**When** periodic re-discovery interval occurs (configurable, default: 1 hour)
**Then** it shall fetch fresh symbol list from dYdX
**And** detect any new symbols added to exchange
**And** detect any symbols delisted from exchange
**And** log changes at INFO level: "New symbols discovered: BTC-USD, ETH-USD"

#### Scenario: Cache symbol list
**Given** symbol discovery has completed successfully
**When** another component requests symbol list
**Then** it shall return cached list without API call
**And** cache shall be valid for 5 minutes
**And** after 5 minutes, next request shall trigger fresh API fetch

#### Data Structure
```rust
struct DiscoveredSymbol {
    symbol: String,           // e.g., "BTC-USD", "ETH-USD"
    exchange: Exchange,       // Exchange::Dydx
    discovered_at: DateTime<Utc>,
    base_asset: String,       // e.g., "BTC"
    quote_asset: String,      // e.g., "USD"
    min_order_size: f64,      // Minimum order quantity on dYdX
}
```

### Requirement: Normalize Symbol Naming
- **ID**: REQ-DISCOVERY-002
- **Type**: Functional
- **Priority**: Medium
- **Stability**: Stable

The bot shall normalize symbol names to a consistent format for internal use.

#### Scenario: Normalize dYdX symbols
**Given** dYdX returns symbols in format "BTC-USD"
**When** the bot processes discovered symbols
**Then** it shall normalize to consistent format: "BTC-USD" (no change for dYdX)
**And** store both original (exchange-specific) and normalized formats

#### Scenario: Support future exchange symbol formats
**Given** symbols from different exchanges have different formats (e.g., "BTCUSDT" on Binance)
**When** multiple exchanges are integrated
**Then** the system shall normalize all to "BTC-USD" format internally
**And** maintain mapping between normalized and exchange-specific formats

### Requirement: Validate Discovered Symbols
- **ID**: REQ-DISCOVERY-003
- **Type**: Functional
- **Priority**: Medium
- **Stability**: Stable

The bot shall validate discovered symbols to ensure data quality.

#### Scenario: Validate symbol format
**Given** symbols discovered from dYdX API
**When** the bot processes them
**Then** it shall validate format matches pattern: `[A-Z]+-[A-Z]+`
**And** reject invalid symbols and log at WARN level
**And** only add valid symbols to symbol list

#### Scenario: Filter out non-tradeable symbols
**Given** dYdX returns market data for all symbols
**When** the bot evaluates tradeable status
**Then** it shall filter out symbols with zero volume or not in tradeable status
**And** only include symbols with min_order_size <= $10

## Database Schema

```sql
CREATE TABLE IF NOT EXISTS discovered_symbols (
    id INTEGER PRIMARY KEY,
    symbol TEXT NOT NULL,
    exchange TEXT NOT NULL,
    base_asset TEXT NOT NULL,
    quote_asset TEXT NOT NULL,
    discovered_at TIMESTAMP NOT NULL,
    min_order_size REAL NOT NULL,
    status TEXT NOT NULL DEFAULT 'active', -- 'active', 'delisted'
    last_verified TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(symbol, exchange)
);

CREATE INDEX idx_symbols_exchange ON discovered_symbols(exchange);
CREATE INDEX idx_symbols_status ON discovered_symbols(status);
CREATE INDEX idx_symbols_discovered_at ON discovered_symbols(discovered_at);
```

## API Contract

### Discovery Flow
```
ExchangeActor (dYdX)
    ↓
    → Fetch `/v4/markets` (dYdX REST API)
    ↓
SymbolDiscoveryService.discover_symbols() 
    ↓
    → Parse markets
    → Validate symbols
    → Store in database
    → Emit `SymbolsDiscovered` event
    ↓
ScreeningActor (consumes list)
```

## Error Handling

| Error | Handling | Recovery |
|-------|----------|----------|
| API timeout | Log WARN, retry exponential backoff | Fall back to cached list |
| Invalid JSON | Log ERROR, skip this discovery cycle | Use last known list |
| No symbols found | Log ERROR (critical) | Alert operator, use last known list |
| Connectivity lost | Log WARN, periodic retry | Automatic retry when restored |

## Performance Requirements

- **Latency**: Symbol discovery should complete within 5 seconds
- **Frequency**: Re-discovery every 1 hour (configurable)
- **Cache lifetime**: 5 minutes for no-change case
- **Storage**: Support up to 10,000 symbols (easily handles dYdX + 4 other exchanges)

## Dependencies
- dYdX API `/v4/markets` endpoint
- SQLite database
- Tokio for async operations
- Serde for JSON parsing

## Related Capabilities
- Screening Evaluation (consumes discovered symbols)
- Result Persistence (stores discovered symbols in database)

## Testing
- Unit: Symbol validation logic
- Integration: Full discovery pipeline with mock dYdX API
- E2E: Production dYdX API (with test account)
