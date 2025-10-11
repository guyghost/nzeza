# Design for Order Execution Fix

## Current Architecture
The order execution pipeline follows this flow:

```
Signal Generation Task (10s interval)
    ↓
generate_signal_for_symbol()
    ↓ (if ≥5 candles)
TradingSignal generated
    ↓
store_signal() → LRU cache
    ↓
Order Execution Task (30s interval)
    ↓
check_and_execute_orders()
    ↓
get_all_last_signals()
    ↓
execute_order_from_signal()
    ↓
select_trader_sender()
    ↓
TraderActor::PlaceOrder
    ↓
Exchange API call
```

## Identified Issues

### 1. Signal Storage Race Condition
- Signals are generated asynchronously but execution happens on different intervals
- LRU cache may evict signals before execution
- No guarantee signals persist between tasks

### 2. Insufficient Diagnostics
- Limited logging in execution path
- Hard to distinguish between "no signals" vs "signals not executing"
- Error messages lack context for debugging

### 3. Trader Availability
- Traders initialized once at startup
- No runtime validation of trader health
- Silent failures when traders unavailable

## Proposed Design Changes

### Enhanced Logging Architecture
```
ExecutionLogger {
    signal_count: AtomicUsize,
    execution_attempts: AtomicUsize,
    execution_successes: AtomicUsize,
    last_execution_time: AtomicU64,
}
```

### Signal Persistence Guarantee
- Use persistent storage for critical signals
- Implement signal deduplication to prevent duplicate executions
- Add signal expiration with configurable TTL

### Trader Health Monitoring
```
TraderHealthChecker {
    async fn check_trader(&self, trader_id: &str) -> Result<(), Error>
    async fn get_available_traders(&self) -> Vec<String>
    async fn report_trader_failure(&self, trader_id: &str, error: Error)
}
```

### Execution Pipeline Improvements
```
OrderExecutionPipeline {
    async fn diagnose_execution_readiness(&self) -> ExecutionStatus
    async fn execute_with_diagnostics(&self, signal: TradingSignal) -> ExecutionResult
    async fn log_execution_metrics(&self, result: &ExecutionResult)
}
```

## Implementation Strategy

### Phase 1: Diagnostics (Low Risk)
- Add comprehensive logging without changing logic
- Implement execution status reporting
- Add signal lifecycle tracking

### Phase 2: Reliability (Medium Risk)
- Improve signal persistence
- Add trader health checks
- Implement execution retries with backoff

### Phase 3: Optimization (Low Risk)
- Optimize signal storage and retrieval
- Reduce logging overhead in production
- Add execution performance metrics

## Error Handling Design

### ExecutionError Types
```rust
enum ExecutionError {
    NoSignalsAvailable,
    TraderUnavailable { trader_id: String },
    SignalExpired { signal_id: String },
    ConfidenceTooLow { confidence: f64, threshold: f64 },
    PositionLimitsExceeded { current: u32, max: u32 },
    TradingLimitsExceeded { limit_type: String },
    ExchangeError { exchange: String, error: String },
}
```

### Error Recovery Strategies
- **Retry Logic**: For transient failures (network timeouts)
- **Circuit Breaker**: For persistent trader failures
- **Fallback Execution**: Use alternative traders when primary fails
- **Signal Requeuing**: For temporary execution failures

## Monitoring and Observability

### Metrics to Track
- Signals generated per minute
- Execution attempts per minute
- Execution success rate
- Average execution latency
- Trader availability percentage
- Signal-to-execution conversion rate

### Alert Conditions
- Execution success rate < 90%
- No executions in 5 minutes when signals present
- Trader unavailable for > 1 minute
- Signal queue growing without processing

## Backward Compatibility
- All changes maintain existing API contracts
- Configuration defaults unchanged
- Logging can be disabled via environment variables
- No breaking changes to external interfaces