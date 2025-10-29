# Design: Portfolio Reconciliation Service

**Change ID:** `add-portfolio-reconciliation`  
**Date:** October 30, 2025  
**Status:** APPROVED  

## Architectural Overview

### System Integration Diagram

```
┌─────────────────────────────────────────────────────┐
│          Trading System                               │
├─────────────────────────────────────────────────────┤
│                                                       │
│  ┌──────────────────┐      ┌────────────────────┐   │
│  │  Position Mgr    │      │  Portfolio Mgr     │   │
│  │  (local state)   │      │  (aggregated)      │   │
│  └────────┬─────────┘      └─────────┬──────────┘   │
│           │                          │                │
│           │  Balances                │                │
│           └──────────────┬───────────┘                │
│                          │                           │
│  ┌──────────────────────▼─────────────────────────┐ │
│  │  PortfolioReconciliationActor                   │ │
│  │  ├─ Fetch exchange balances (async)            │ │
│  │  ├─ Compare with local state                   │ │
│  │  ├─ Detect discrepancies                       │ │
│  │  ├─ Generate reconciliation report             │ │
│  │  └─ Persist audit trail                        │ │
│  └──────────────────────┬─────────────────────────┘ │
│                         │                           │
│           ┌─────────────┼─────────────┐             │
│           │             │             │             │
│  ┌────────▼───────┐  ┌──▼──────────┐ │             │
│  │   Coinbase     │  │   dYdX      │ │  ┌──────┐  │
│  │   Reconciler   │  │ Reconciler  │ │  │More..│  │
│  └────────────────┘  └─────────────┘ │  └──────┘  │
│                                      │             │
│           ┌──────────────────────────▼─┐           │
│           │  ReconciliationRepository   │           │
│           │  (SQLite audit trail)       │           │
│           └─────────────────────────────┘           │
│                                                     │
└─────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility |
|-----------|-----------------|
| **ReconciliationActor** | Orchestration, scheduling, error handling |
| **PortfolioReconciliationService** | Core reconciliation logic |
| **ExchangeReconcilers** | Exchange-specific balance fetching |
| **ReconciliationRepository** | Persist audit trail to database |
| **Configuration** | Load parameters from env vars |

## Data Flow

### Reconciliation Cycle

```
1. Timer triggers (every RECONCILIATION_INTERVAL_SECONDS)
   │
2. ReconciliationActor receives tick
   │
3. For each configured exchange:
   ├─ Fetch real balances from exchange API
   ├─ Get local balances from Position Manager
   ├─ Compare and detect discrepancies
   └─ Generate reconciliation report
   │
4. Evaluate overall status:
   ├─ All balanced? → Status: SUCCESS
   ├─ Minor issues? → Status: WARNING + alert
   └─ Major issues? → Status: ERROR + immediate action
   │
5. Persist to audit trail (ReconciliationRepository)
   │
6. If discrepancy found:
   ├─ Log detailed report
   ├─ Trigger recovery logic
   └─ Alert operational team (if configured)
   │
7. Repeat cycle
```

### Discrepancy Types

```rust
pub enum BalanceDiscrepancy {
    // Missing currency entirely
    Missing {
        currency: String,
        amount: f64,  // Amount in exchange that's missing locally
    },
    
    // Balance mismatch above threshold
    Mismatch {
        currency: String,
        local: f64,
        exchange: f64,
        diff: f64,
    },
    
    // Precision/rounding differences within tolerance
    Precision {
        currency: String,
        tolerance: f64,
    },
}
```

### Severity Levels

```rust
pub enum ReconciliationSeverity {
    Info,       // Minor rounding differences
    Warning,    // 1-5% balance mismatch
    Error,      // >5% balance mismatch
    Critical,   // Missing large amounts
}
```

## Database Schema

### `reconciliation_audit` Table

```sql
CREATE TABLE reconciliation_audit (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    
    -- Identification
    reconciliation_id TEXT NOT NULL UNIQUE,
    exchange_id TEXT NOT NULL,
    
    -- Timing
    reconciliation_timestamp DATETIME NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    -- Status & Results
    status TEXT NOT NULL,        -- 'SUCCESS', 'WARNING', 'ERROR', 'CRITICAL'
    discrepancy_count INTEGER NOT NULL,
    
    -- Balances (JSON)
    local_balances_json TEXT NOT NULL,     -- Serialized JSON
    exchange_balances_json TEXT NOT NULL,  -- Serialized JSON
    
    -- Discrepancies (JSON array)
    discrepancies_json TEXT NOT NULL,      -- Array of discrepancies
    
    -- Recovery
    recovery_attempted BOOLEAN DEFAULT 0,
    recovery_status TEXT,                  -- 'SUCCESS', 'FAILED', 'PENDING'
    recovery_details_json TEXT,
    
    -- Audit
    operator_notes TEXT,
    INDEX idx_exchange_time (exchange_id, reconciliation_timestamp),
    INDEX idx_status (status)
);
```

## API Design

### Trait Interface

```rust
#[async_trait]
pub trait PortfolioReconciliationService: Send + Sync {
    /// Fetch balances from exchange
    async fn fetch_exchange_balances(
        &self,
        exchange: &Exchange,
    ) -> Result<Vec<Balance>, ReconciliationError>;
    
    /// Compare local vs exchange balances
    fn detect_discrepancies(
        &self,
        local: &[Balance],
        exchange: &[Balance],
    ) -> Result<Vec<BalanceDiscrepancy>, ReconciliationError>;
    
    /// Generate reconciliation report
    fn generate_report(
        &self,
        exchange: Exchange,
        local_balances: Vec<Balance>,
        exchange_balances: Vec<Balance>,
        discrepancies: Vec<BalanceDiscrepancy>,
    ) -> Result<ReconciliationReport, ReconciliationError>;
    
    /// Execute reconciliation cycle
    async fn reconcile(
        &self,
        exchange: Exchange,
    ) -> Result<ReconciliationReport, ReconciliationError>;
}
```

### Configuration Parameters

```rust
pub struct TradingConfig {
    // ... existing fields ...
    
    /// Enable/disable reconciliation service
    pub reconciliation_enabled: bool,
    
    /// How often to run reconciliation (seconds)
    pub reconciliation_interval_seconds: u64,
    
    /// Dollar threshold for flagging discrepancies
    pub reconciliation_threshold_usd: f64,
    
    /// Number of retries on transient failures
    pub reconciliation_retry_count: u32,
    
    /// API call timeout (seconds)
    pub reconciliation_timeout_seconds: u64,
}
```

### Actor Messages

```rust
pub enum ReconciliationMessage {
    /// Trigger reconciliation for specific exchange
    ReconcileExchange {
        exchange: Exchange,
        reply: oneshot::Sender<Result<ReconciliationReport, ReconciliationError>>,
    },
    
    /// Trigger reconciliation for all exchanges
    ReconcileAll {
        reply: oneshot::Sender<Vec<ReconciliationReport>>,
    },
    
    /// Get reconciliation history for exchange
    GetHistory {
        exchange: Exchange,
        limit: u32,
        reply: oneshot::Sender<Vec<ReconciliationReport>>,
    },
    
    /// Manually override and sync balances
    SyncBalances {
        exchange: Exchange,
        reply: oneshot::Sender<Result<(), ReconciliationError>>,
    },
    
    /// Shutdown signal
    Shutdown,
}
```

## Error Handling

### Error Types

```rust
#[derive(thiserror::Error, Debug)]
pub enum ReconciliationError {
    #[error("Exchange API error: {0}")]
    ExchangeApi(String),
    
    #[error("Network timeout after {0}s")]
    Timeout(u64),
    
    #[error("Balance calculation error: {0}")]
    CalculationError(String),
    
    #[error("Database error: {0}")]
    DatabaseError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Recovery failed: {0}")]
    RecoveryFailed(String),
}
```

### Retry Strategy

```
Attempt 1: Immediate retry
Attempt 2: Wait 100ms
Attempt 3: Wait 500ms
Attempt 4: Wait 2.5s (if enabled)
Failure: Return error, continue cycle
```

## Testing Strategy

### Test Categories & Coverage

#### Unit Tests (5-6 tests)
```
- Balance comparison with exact match
- Balance comparison with minor differences
- Precision/rounding tolerance handling
- Discrepancy detection algorithm
- Severity classification logic
```

#### Integration Tests (5-7 tests)
```
- Multi-exchange reconciliation
- Actor message handling
- Repository persistence and retrieval
- Audit trail creation
- Configuration loading
```

#### Error Handling (3-4 tests)
```
- Exchange API timeout
- Network failure recovery
- Malformed balance data
- Database transaction rollback
```

#### Edge Cases (2-3 tests)
```
- Zero balances
- Missing currencies
- Empty portfolio
- Concurrent reconciliation attempts
```

**Total Expected: 15-20 tests**

## Performance Considerations

### Optimization Strategies

1. **Async Fetching:** All exchange API calls run concurrently
2. **Connection Pooling:** Reuse HTTP connections
3. **Batch Updates:** Write audit trail in batches
4. **Caching:** Cache recent exchange balance responses (short TTL)
5. **Non-blocking:** Reconciliation happens in separate task

### Performance Targets

| Operation | Target | Method |
|-----------|--------|--------|
| Balance fetch | < 1s/exchange | Concurrent requests |
| Reconciliation cycle | < 2s total | Batched operations |
| Audit write | < 100ms | Transaction batching |
| Memory overhead | < 10MB | Streaming large datasets |

## Security Considerations

1. **No Balance Logging:** Sensitive numbers not logged directly
2. **API Isolation:** Each exchange credentials separate
3. **Audit Trail Integrity:** Database constraints prevent modification
4. **Permission Checks:** Only authorized operations allowed
5. **Timeout Protection:** Prevent hanging requests

## Integration Points

### Input Sources
- Position Manager (local balances)
- Exchange Clients (real balances)
- Configuration (parameters)

### Output Targets
- ReconciliationRepository (audit trail)
- Alert System (if configured)
- Metrics Collector (for monitoring)

## Deployment Considerations

### Database Migration
```sql
-- Create reconciliation_audit table on first deployment
```

### Configuration
- All parameters configurable via environment variables
- Defaults allow immediate use
- No manual configuration required

### Monitoring
- Reconciliation success/failure rate
- Average reconciliation time
- Discrepancy frequency
- Recovery success rate

## Future Extensions

### Phase 2 Potential
- Machine learning to predict likely discrepancies
- Automatic position adjustment when discrepancies detected
- Reconciliation report export/API
- Webhook alerts for external monitoring
- Historical reconciliation analysis

### Phase 3 Potential
- Multi-user reconciliation tracking
- Reconciliation approval workflow
- Compliance report generation
- Integration with risk management system

## Decision Rationale

### Why Async Actor?
✅ Non-blocking - doesn't impact trading  
✅ Isolated failures - system continues if reconciliation fails  
✅ Built-in retry logic - Tokio handles task respawning  

### Why Repository Pattern?
✅ Testable - can mock database  
✅ Flexible - can swap backends  
✅ Audit trail - immutable record  

### Why Configurable Thresholds?
✅ Different exchanges have different precision  
✅ Different traders have different risk tolerance  
✅ Different trading styles have different reconciliation needs  

## Implementation Notes

1. **Use existing exchange clients** - Don't duplicate API code
2. **Leverage Position Manager** - It already has local balances
3. **Follow DDD patterns** - Keep domain logic separate from infrastructure
4. **Comprehensive logging** - Help debugging in production
5. **Graceful degradation** - System continues if reconciliation fails

---

**Status:** Design Complete ✅  
**Ready for Implementation:** YES ✅
