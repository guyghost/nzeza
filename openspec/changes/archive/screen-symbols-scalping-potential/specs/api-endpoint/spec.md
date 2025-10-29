# Specification: API Endpoints

## Overview
The system shall expose symbol screening results via REST API endpoints for consumption by web dashboards, strategy actors, and external tools.

## ADDED Requirements

### Requirement: GET Top Screened Symbols
- **ID**: REQ-API-001
- **Type**: Functional
- **Priority**: High
- **Stability**: Stable

The bot shall provide endpoint to retrieve top candidates by scalping potential.

#### Scenario: Get top 10 candidates with default parameters
**Given** user makes request: `GET /api/screening/symbols/dydx?limit=10`
**When** endpoint processes request
**Then** it shall:
  1. Query latest ranking snapshot
  2. Return top 10 symbols sorted by score descending
  3. Include all component scores and recommendation
  4. Return HTTP 200 with JSON payload

**Response Example:**
```json
{
  "timestamp": "2025-10-29T14:30:00Z",
  "exchange": "dydx",
  "total_symbols_screened": 150,
  "candidates": [
    {
      "rank": 1,
      "symbol": "BTC-USD",
      "scalping_potential_score": 0.91,
      "volatility_score": 0.85,
      "volume_score": 0.90,
      "spread_score": 0.95,
      "momentum_score": 0.88,
      "recommendation": "BestCandidate",
      "current_price": 45230.50,
      "recent_high_24h": 46000.00,
      "recent_low_24h": 44500.00
    },
    {
      "rank": 2,
      "symbol": "ETH-USD",
      "scalping_potential_score": 0.85,
      ...
    }
  ]
}
```

#### Scenario: Apply custom limit parameter
**Given** user requests: `GET /api/screening/symbols/dydx?limit=50`
**When** endpoint processes
**Then** it shall return top 50 candidates (instead of default 10)

#### Scenario: Use default limit when not specified
**Given** user requests: `GET /api/screening/symbols/dydx` (no limit parameter)
**When** endpoint processes
**Then** it shall use default limit: 100 symbols

#### Scenario: Validate limit parameter bounds
**Given** user requests: `GET /api/screening/symbols/dydx?limit=1000`
**When** endpoint validates
**Then** it shall cap limit at maximum: 500
**And** log at WARN level: "Limit capped at max (500), requested 1000"
**And** return capped results

### Requirement: GET Symbols by Category
- **ID**: REQ-API-002
- **Type**: Functional
- **Priority**: High
- **Stability**: Stable

The bot shall provide endpoint to filter symbols by recommendation category.

#### Scenario: Get only best candidates
**Given** request: `GET /api/screening/symbols/dydx?category=best`
**When** endpoint processes
**Then** it shall return only symbols with recommendation="BestCandidate"
**And** sorted by score descending
**And** include count: `"count": 5`

**Response:**
```json
{
  "category": "best",
  "count": 5,
  "candidates": [
    {"rank": 1, "symbol": "BTC-USD", "score": 0.91, ...},
    {"rank": 2, "symbol": "ETH-USD", "score": 0.85, ...}
  ]
}
```

#### Scenario: Get all candidates above minimum threshold
**Given** request: `GET /api/screening/symbols/dydx?category=good`
**When** endpoint processes
**Then** it shall return symbols with score >= 0.60 (GoodCandidate and BestCandidate)
**And** sort by score descending

#### Scenario: Get symbols to avoid
**Given** request: `GET /api/screening/symbols/dydx?category=avoid`
**When** endpoint processes
**Then** it shall return symbols with score < 0.50
**And** indicate risk level

### Requirement: GET Category Summary & Distribution
- **ID**: REQ-API-003
- **Type**: Functional
- **Priority**: Medium
- **Stability**: Stable

The bot shall provide endpoint for ranking statistics and category distribution.

#### Scenario: Get category distribution summary
**Given** request: `GET /api/screening/symbols/dydx/categories`
**When** endpoint processes
**Then** it shall return:

```json
{
  "timestamp": "2025-10-29T14:30:00Z",
  "exchange": "dydx",
  "total_symbols_screened": 150,
  "distribution": {
    "best_candidates": {
      "count": 5,
      "score_range": [0.75, 1.0],
      "avg_score": 0.88,
      "symbols": [
        {"symbol": "BTC-USD", "score": 0.91},
        {"symbol": "ETH-USD", "score": 0.85}
      ]
    },
    "good_candidates": {
      "count": 18,
      "score_range": [0.60, 0.75],
      "avg_score": 0.68,
      "symbols": [...]
    },
    "fair_candidates": {
      "count": 32,
      "score_range": [0.50, 0.60],
      "avg_score": 0.55
    },
    "avoid": {
      "count": 95,
      "score_range": [0.0, 0.50],
      "avg_score": 0.28
    }
  }
}
```

### Requirement: GET Symbol Detail & Scoring Breakdown
- **ID**: REQ-API-004
- **Type**: Functional
- **Priority**: Medium
- **Stability**: Stable

The bot shall provide endpoint for detailed breakdown of a specific symbol's scores.

#### Scenario: Get detailed score breakdown for symbol
**Given** request: `GET /api/screening/symbols/dydx/BTC-USD`
**When** endpoint processes
**Then** it shall return detailed breakdown:

```json
{
  "symbol": "BTC-USD",
  "exchange": "dydx",
  "timestamp": "2025-10-29T14:30:00Z",
  "current_rank": 1,
  "current_price": 45230.50,
  
  "scoring": {
    "volatility": {
      "score": 0.85,
      "weight": 0.30,
      "contribution": 0.255,
      "interpretation": "High volatility - good for scalping"
    },
    "volume": {
      "score": 0.90,
      "weight": 0.30,
      "contribution": 0.270,
      "interpretation": "Excellent liquidity"
    },
    "spread": {
      "score": 0.95,
      "weight": 0.20,
      "contribution": 0.190,
      "interpretation": "Tight bid-ask spreads"
    },
    "momentum": {
      "score": 0.88,
      "weight": 0.20,
      "contribution": 0.176,
      "interpretation": "Strong upward trend"
    }
  },
  
  "overall_score": 0.91,
  "recommendation": "BestCandidate",
  
  "market_metrics": {
    "high_24h": 46000.00,
    "low_24h": 44500.00,
    "volume_24h_usd": 50000000.00,
    "bid_ask_spread_percent": 0.001
  },
  
  "technical_indicators": {
    "rsi_14": 65.2,
    "macd_histogram": 125.3,
    "ema_3": 45250.00,
    "ema_5": 45200.00
  }
}
```

#### Scenario: Handle missing symbol error
**Given** request: `GET /api/screening/symbols/dydx/INVALID-USD`
**When** endpoint processes
**Then** it shall return HTTP 404 with error:
```json
{
  "error": "Symbol not found",
  "symbol": "INVALID-USD",
  "exchange": "dydx"
}
```

### Requirement: GET Ranking Trends & History
- **ID**: REQ-API-005
- **Type**: Functional
- **Priority**: Medium
- **Stability**: Stable

The bot shall provide endpoint for historical ranking trends and analysis.

#### Scenario: Get rank history for symbol
**Given** request: `GET /api/screening/symbols/dydx/BTC-USD/history?period=24h`
**When** endpoint processes
**Then** it shall return time-series rank data:

```json
{
  "symbol": "BTC-USD",
  "exchange": "dydx",
  "period": "24h",
  "history": [
    {"timestamp": "2025-10-28T14:30:00Z", "rank": 3, "score": 0.81},
    {"timestamp": "2025-10-28T14:35:00Z", "rank": 3, "score": 0.82},
    {"timestamp": "2025-10-28T14:40:00Z", "rank": 2, "score": 0.84},
    {"timestamp": "2025-10-28T14:45:00Z", "rank": 2, "score": 0.86},
    {"timestamp": "2025-10-28T14:50:00Z", "rank": 1, "score": 0.88},
    {"timestamp": "2025-10-29T14:30:00Z", "rank": 1, "score": 0.91}
  ],
  "trend": {
    "best_rank_achieved": 1,
    "worst_rank_achieved": 5,
    "current_trend": "improving",
    "avg_score_24h": 0.85,
    "score_change_24h": 0.10
  }
}
```

#### Scenario: Get most improved symbols
**Given** request: `GET /api/screening/symbols/dydx/trends?type=most_improved`
**When** endpoint processes
**Then** it shall return symbols with best rank improvement:

```json
{
  "type": "most_improved",
  "period": "24h",
  "trends": [
    {
      "symbol": "SOL-USD",
      "rank_improvement": 42,  // Moved up 42 positions
      "previous_rank": 45,
      "current_rank": 3,
      "score_change": 0.28,
      "interpretation": "Rapid improvement - high volatility increased"
    }
  ]
}
```

### Requirement: Support Query Filtering & Sorting
- **ID**: REQ-API-006
- **Type**: Functional
- **Priority**: Medium
- **Stability**: Stable

The bot shall support flexible filtering and sorting parameters.

#### Scenario: Filter by score range
**Given** request: `GET /api/screening/symbols/dydx?score_min=0.6&score_max=0.8`
**When** endpoint processes
**Then** it shall return symbols with score in range [0.6, 0.8]
**And** sort by score descending

#### Scenario: Sort by different criteria
**Given** request: `GET /api/screening/symbols/dydx?sort_by=volatility&sort_order=desc`
**When** endpoint processes
**Then** it shall return symbols sorted by volatility descending
**And** support sort_by values: score, volatility, volume, spread, momentum, rank, symbol

#### Scenario: Combined filtering & sorting
**Given** request: `GET /api/screening/symbols/dydx?category=good&sort_by=momentum&limit=20`
**When** endpoint processes
**Then** it shall:
  1. Filter to GoodCandidate (score 0.60-0.75)
  2. Sort by momentum descending
  3. Return top 20 results

#### Scenario: Pagination support
**Given** request: `GET /api/screening/symbols/dydx?limit=10&offset=20`
**When** endpoint processes
**Then** it shall skip first 20 results and return next 10
**And** include pagination metadata:
```json
{
  "pagination": {
    "limit": 10,
    "offset": 20,
    "total": 150,
    "has_more": true
  }
}
```

### Requirement: Error Handling & Response Codes
- **ID**: REQ-API-007
- **Type**: Functional
- **Priority**: High
- **Stability**: Stable

The bot shall return appropriate HTTP status codes and error messages.

#### Scenario: No screening data available yet
**Given** screening hasn't run yet (fresh bot start)
**When** client requests: `GET /api/screening/symbols/dydx`
**Then** it shall return HTTP 202 Accepted:
```json
{
  "status": "pending",
  "message": "Screening in progress, please retry in 30 seconds",
  "retry_after_seconds": 30
}
```

#### Scenario: Invalid query parameters
**Given** request: `GET /api/screening/symbols/dydx?score_min=abc`
**When** endpoint validates
**Then** it shall return HTTP 400 Bad Request:
```json
{
  "error": "Bad Request",
  "details": {
    "score_min": "Invalid numeric value: 'abc'"
  }
}
```

#### Scenario: Database error during query
**Given** database becomes unavailable
**When** client requests data
**Then** it shall return HTTP 503 Service Unavailable:
```json
{
  "error": "Service Unavailable",
  "message": "Database connection failed, please retry later"
}
```

## API Endpoint Summary

| Endpoint | Method | Purpose | Response |
|----------|--------|---------|----------|
| `/api/screening/symbols/dydx` | GET | Get top candidates | List[SymbolRank] |
| `/api/screening/symbols/dydx/categories` | GET | Get category distribution | CategoryDistribution |
| `/api/screening/symbols/dydx/{symbol}` | GET | Get symbol details | SymbolDetail |
| `/api/screening/symbols/dydx/{symbol}/history` | GET | Get rank history | TimeSeriesData |
| `/api/screening/symbols/dydx/trends` | GET | Get trending analysis | TrendAnalysis |

## Query Parameters

| Parameter | Type | Default | Example | Notes |
|-----------|------|---------|---------|-------|
| `limit` | int | 100 | 20 | Max 500 |
| `offset` | int | 0 | 40 | For pagination |
| `category` | enum | - | best, good, fair, avoid | Filter by recommendation |
| `sort_by` | enum | score | volatility | Sort column |
| `sort_order` | enum | desc | asc | ASC or DESC |
| `score_min` | float | - | 0.60 | Min score [0,1] |
| `score_max` | float | - | 0.80 | Max score [0,1] |
| `period` | enum | 24h | 7d, 30d | For trends/history |

## Response Format

All responses use consistent format:
```json
{
  "status": "success",  // or "error", "pending"
  "timestamp": "2025-10-29T14:30:00Z",
  "exchange": "dydx",
  "data": { ... },
  "pagination": { ... },  // if applicable
  "error": null  // if status != "error", null
}
```

## Performance Requirements

| Endpoint | Target Latency |
|----------|-----------------|
| `/api/screening/symbols/dydx` | < 200ms |
| `/api/screening/symbols/dydx/categories` | < 150ms |
| `/api/screening/symbols/dydx/{symbol}` | < 100ms |
| `/api/screening/symbols/dydx/{symbol}/history` | < 250ms |

## Rate Limiting

- Default: 100 requests per minute per IP
- Implementation: Middleware with token bucket algorithm
- Header: `X-RateLimit-Remaining`, `X-RateLimit-Reset`

## Related Capabilities
- Result Ranking (provides sorted data)
- Result Persistence (queries from database)

## Testing
- Unit: Response formatting with test data
- Integration: Full API endpoint tests with mock database
- E2E: Test with real screening data
