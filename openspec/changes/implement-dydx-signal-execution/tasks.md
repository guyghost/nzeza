# Tasks: Implement dYdX Signal Execution with Balance and Leverage

## Overview
This document breaks down the implementation into small, verifiable work items following TDD (Red → Green → Refactor).

## Phase 1: BalanceManager Implementation (2-3 days)

### Task 1.1: Define BalanceInfo and BalanceCache Data Structures
**Type**: Feature  
**Priority**: High  
**Estimated**: 2 hours  
**Dependencies**: None

**Red Phase**: Write tests for BalanceInfo
- Test that BalanceInfo can be constructed with valid values
- Test that invariant `total_balance == available_balance + locked_balance` is enforced
- Test that timestamp is set to current time

**Green Phase**: Implement BalanceInfo
```rust
pub struct BalanceInfo {
    pub total_balance: f64,
    pub available_balance: f64,
    pub locked_balance: f64,
    pub timestamp: SystemTime,
}
```

**Refactor**: Add validation methods
- `validate()` -> Result<(), String>
- `is_fresh(ttl: Duration)` -> bool

**Acceptance Criteria**:
- ✅ BalanceInfo struct defined in `src/domain/entities/balance.rs`
- ✅ All tests pass
- ✅ Invariants enforced

---

### Task 1.2: Implement BalanceManager with Caching
**Type**: Feature  
**Priority**: High  
**Estimated**: 1 day  
**Dependencies**: 1.1

**Red Phase**: Write tests for BalanceManager
- Test `get_balance()` returns cached value within TTL
- Test `get_balance()` fetches fresh after TTL expires
- Test `refresh_balance()` forces API call regardless of cache
- Test cache initialization and eviction

**Green Phase**: Implement BalanceManager
```rust
pub struct BalanceManager {
    exchange_client: Arc<dyn ExchangeClient>,
    cache: Arc<Mutex<Option<CachedBalance>>>,
    cache_ttl: Duration,
}

impl BalanceManager {
    pub async fn get_balance(&self) -> Result<BalanceInfo, String>;
    pub async fn refresh_balance(&self) -> Result<BalanceInfo, String>;
}
```

**Refactor**: Optimize cache lookup, add metrics

**Acceptance Criteria**:
- ✅ BalanceManager implemented in `src/domain/services/balance_manager.rs`
- ✅ Caching works correctly (TTL-based)
- ✅ Thread-safe with Arc<Mutex>
- ✅ All tests pass (including cache TTL tests)
- ✅ Added to `src/domain/services/mod.rs`

---

### Task 1.3: Integration Test: BalanceManager with Mock dYdX Client
**Type**: Test  
**Priority**: High  
**Estimated**: 1 day  
**Dependencies**: 1.2

**Red Phase**: Write integration tests
- Test `get_balance()` with mock dYdX client returning valid balance
- Test error handling when mock returns authentication error
- Test error handling when mock returns network error

**Green Phase**: Create mock dYdX client for testing
```rust
pub struct MockDydxClient {
    balance_response: Option<Vec<Balance>>,
    error_response: Option<String>,
}
```

**Refactor**: Reuse mock across other integration tests

**Acceptance Criteria**:
- ✅ Mock dYdX client implemented
- ✅ Integration tests in `tests/balance_manager_e2e.rs`
- ✅ All 5+ test cases pass
- ✅ Coverage > 80%

---

## Phase 2: LeverageCalculator Implementation (2-3 days)

### Task 2.1: Define LeverageInfo Data Structure
**Type**: Feature  
**Priority**: High  
**Estimated**: 1 hour  
**Dependencies**: None

**Red Phase**: Write tests for LeverageInfo
- Test construction with valid values
- Test that `available_leverage = max_leverage - current_leverage`

**Green Phase**: Implement LeverageInfo
```rust
pub struct LeverageInfo {
    pub max_leverage: f64,
    pub current_leverage: f64,
    pub available_leverage: f64,
    pub margin_ratio: f64,
    pub maintenance_margin_ratio: f64,
    pub timestamp: SystemTime,
}
```

**Refactor**: Add helper methods for margin health checks

**Acceptance Criteria**:
- ✅ LeverageInfo struct in `src/domain/entities/leverage.rs`
- ✅ All tests pass
- ✅ Invariants enforced

---

### Task 2.2: Implement LeverageCalculator
**Type**: Feature  
**Priority**: High  
**Estimated**: 1 day  
**Dependencies**: 2.1, 1.2

**Red Phase**: Write tests for LeverageCalculator
- Test `get_leverage_info()` with no positions
- Test `get_leverage_info()` with open positions
- Test calculation: `available_leverage = max_leverage - current_leverage`
- Test margin ratio calculation

**Green Phase**: Implement LeverageCalculator
```rust
pub struct LeverageCalculator {
    exchange_client: Arc<dyn ExchangeClient>,
}

impl LeverageCalculator {
    pub async fn get_leverage_info(&self, account: &Account) -> Result<LeverageInfo, String>;
}
```

**Refactor**: Add caching for leverage info (30s TTL)

**Acceptance Criteria**:
- ✅ LeverageCalculator in `src/domain/services/leverage_calculator.rs`
- ✅ Correctly calculates current leverage from positions
- ✅ Tests pass (8+ test cases)
- ✅ Coverage > 80%

---

### Task 2.3: Integration Test: LeverageCalculator with Mock Account Data
**Type**: Test  
**Priority**: High  
**Estimated**: 1 day  
**Dependencies**: 2.2, 1.3

**Red Phase**: Write integration tests
- Test leverage calculation with mock account (no positions)
- Test leverage calculation with mock account (5x current leverage)
- Test margin ratio calculations

**Green Phase**: Create mock account data for testing
- Mock Account struct with positions
- Mock exchange client returning account info

**Refactor**: Parameterize test data for reusability

**Acceptance Criteria**:
- ✅ Integration tests in `tests/leverage_calculator_e2e.rs`
- ✅ 5+ test cases pass
- ✅ Coverage > 80%

---

## Phase 3: PositionSizer Implementation (1-2 days)

### Task 3.1: Define PositionSizingRequest and PositionSizingResult
**Type**: Feature  
**Priority**: High  
**Estimated**: 2 hours  
**Dependencies**: None

**Red Phase**: Write tests for data structures
- Test PositionSizingRequest construction
- Test PositionSizingResult invariants

**Green Phase**: Implement structures
```rust
pub struct PositionSizingRequest { ... }
pub struct PositionSizingResult { ... }
```

**Refactor**: Add validation methods

**Acceptance Criteria**:
- ✅ Structures in `src/domain/value_objects/position_sizing.rs`
- ✅ All tests pass

---

### Task 3.2: Implement PositionSizer Algorithm
**Type**: Feature  
**Priority**: High  
**Estimated**: 1 day  
**Dependencies**: 3.1

**Red Phase**: Write comprehensive unit tests
- Test: balance + leverage → max quantity
- Test: portfolio exposure limit applied
- Test: min/max order size constraints
- Test: edge cases (zero balance, high price, etc.)
- Test: sizing justification in result

**Green Phase**: Implement PositionSizer
```rust
pub struct PositionSizer;

impl PositionSizer {
    pub fn size_position(req: &PositionSizingRequest) 
        -> Result<PositionSizingResult, String>;
}
```

**Refactor**: Extract helper functions for each constraint

**Acceptance Criteria**:
- ✅ PositionSizer in `src/domain/services/position_sizer.rs`
- ✅ 10+ unit tests pass
- ✅ All edge cases handled
- ✅ Coverage > 90%
- ✅ Result includes clear `reason` field

---

### Task 3.3: Property-Based Tests for PositionSizer
**Type**: Test  
**Priority**: Medium  
**Estimated**: 1 day  
**Dependencies**: 3.2

**Red Phase**: Write property tests using `proptest`
- Property: `quantity * price <= available_balance * leverage`
- Property: `quantity * price <= available_balance * max_portfolio_exposure`
- Property: `quantity_usd >= min_order_size || quantity == 0`
- Property: `quantity_usd <= max_order_size`

**Green Phase**: Add property-based test suite

**Refactor**: Consolidate properties

**Acceptance Criteria**:
- ✅ Property tests in `src/domain/services/position_sizer.rs` (property tests section)
- ✅ 1000+ random scenarios tested without failures
- ✅ All properties hold

---

## Phase 4: OrderExecutor Integration (1-2 days)

### Task 4.1: Enhance OrderExecutor with Balance/Leverage Checks
**Type**: Feature  
**Priority**: High  
**Estimated**: 1 day  
**Dependencies**: 1.2, 2.2, 3.2

**Red Phase**: Write tests for enhanced OrderExecutor
- Test: signal execution fetches balance
- Test: signal execution checks leverage
- Test: signal execution sizes position
- Test: insufficient balance prevents execution
- Test: insufficient leverage prevents execution
- Test: position placed with calculated quantity

**Green Phase**: Modify OrderExecutor
```rust
pub struct OrderExecutor {
    // ... existing fields ...
    balance_manager: Arc<BalanceManager>,      // NEW
    leverage_calculator: Arc<LeverageCalculator>, // NEW
}

impl OrderExecutor {
    pub async fn execute_signal(
        &mut self,
        symbol: &str,
        signal: &TradingSignal,
        current_price: f64,
    ) -> Result<Order, String>;
}
```

**Refactor**: Extract common validation logic

**Acceptance Criteria**:
- ✅ OrderExecutor enhanced in `src/domain/services/order_executor.rs`
- ✅ Constructor updated to accept balance_manager and leverage_calculator
- ✅ execute_signal() now fetches balance/leverage before execution
- ✅ 8+ unit tests pass
- ✅ Coverage > 80%

---

### Task 4.2: Handle Error Cases in Signal Execution
**Type**: Feature  
**Priority**: High  
**Estimated**: 1 day  
**Dependencies**: 4.1

**Red Phase**: Write error handling tests
- Test: insufficient balance error
- Test: insufficient leverage error
- Test: balance fetch failure
- Test: leverage calculation failure
- Test: position sizing failure
- Test: order placement failure (from dYdX)

**Green Phase**: Implement error handling
- Create `TradeExecutionError` enum with variants
- Implement proper error propagation

**Refactor**: Unify error logging and metrics

**Acceptance Criteria**:
- ✅ TradeExecutionError enum defined
- ✅ All error cases tested (6+ tests)
- ✅ Error messages are clear and actionable
- ✅ Errors logged at appropriate level

---

### Task 4.3: Metrics and Logging for Signal Execution
**Type**: Feature  
**Priority**: Medium  
**Estimated**: 1 day  
**Dependencies**: 4.2

**Red Phase**: Write tests for logging/metrics
- Test: successful execution is logged
- Test: failed execution is logged
- Test: metrics recorded correctly (execution time, quantity, etc.)

**Green Phase**: Add logging and metrics
- Use `tracing::info!`, `tracing::warn!`, `tracing::error!`
- Record execution metrics to TradingMetrics

**Refactor**: Consolidate logging patterns

**Acceptance Criteria**:
- ✅ Logging implemented for all paths (success/failure)
- ✅ Metrics recorded for execution time, quantity, fill price
- ✅ Tests verify logging/metrics

---

## Phase 5: Integration Testing (1-2 days)

### Task 5.1: End-to-End Test: Signal → Execution
**Type**: Test  
**Priority**: High  
**Estimated**: 1 day  
**Dependencies**: 4.3, 3.3

**Red Phase**: Write end-to-end test
```rust
#[tokio::test]
async fn test_signal_execution_e2e() {
    // Setup: Create mock dYdX client with balance + positions
    // Act: Generate signal and execute
    // Assert: Order is placed with correct quantity
}
```

**Green Phase**: Create E2E test
- Create complete mock environment
- Execute signal from generation to order placement
- Verify correct quantity, margin, etc.

**Refactor**: Parameterize test for multiple scenarios

**Acceptance Criteria**:
- ✅ E2E test in `tests/signal_execution_e2e.rs`
- ✅ 5+ scenarios pass:
  - Successful execution with balance
  - Insufficient balance
  - Insufficient leverage
  - Portfolio exposure limit
  - Min/max order size

---

### Task 5.2: Stress Test: Multiple Concurrent Signals
**Type**: Test  
**Priority**: Medium  
**Estimated**: 1 day  
**Dependencies**: 5.1

**Red Phase**: Write concurrent signal test
- Test: multiple signals processed simultaneously
- Test: balance is correctly tracked across concurrent executions
- Test: no double-spending of balance

**Green Phase**: Implement concurrent test
- Spawn 10+ tokio tasks with signals
- Verify no balance violations

**Refactor**: Add configurable concurrency level

**Acceptance Criteria**:
- ✅ Concurrent test in `tests/signal_execution_concurrent.rs`
- ✅ 100 concurrent signals processed without errors
- ✅ Balance consistency maintained

---

### Task 5.3: Documentation and Examples
**Type**: Docs  
**Priority**: Medium  
**Estimated**: 1 day  
**Dependencies**: 5.2

**Content**:
- How to use BalanceManager, LeverageCalculator, PositionSizer
- Configuration examples (min/max order sizes, portfolio exposure)
- Error handling examples
- Troubleshooting guide

**Acceptance Criteria**:
- ✅ README updated in `src/domain/services/README.md`
- ✅ Code examples in doc comments
- ✅ Configuration options documented

---

## Phase 6: Code Review and Finalization (1 day)

### Task 6.1: Code Quality Review
**Type**: Review  
**Priority**: High  
**Estimated**: 1 day  
**Dependencies**: All phases

**Checklist**:
- ✅ All tests pass: `cargo test`
- ✅ No clippy warnings: `cargo clippy`
- ✅ Code formatted: `cargo fmt`
- ✅ 80%+ test coverage
- ✅ DDD patterns followed
- ✅ No hardcoded values (use config)
- ✅ Error messages are clear
- ✅ Logging is structured

**Acceptance Criteria**:
- ✅ All checks pass
- ✅ Ready for merge

---

## Summary

**Total Estimated Time**: 9-13 days  
**Phases**:
1. Phase 1 (Balance): 2-3 days
2. Phase 2 (Leverage): 2-3 days
3. Phase 3 (Sizing): 1-2 days
4. Phase 4 (Integration): 2-3 days
5. Phase 5 (E2E Testing): 1-2 days
6. Phase 6 (Review): 1 day

**Total Tasks**: 17  
**Total Tests**: 40+  
**Total Coverage**: 80%+

**Parallelizable**:
- Phases 1 and 2 can start simultaneously (no dependencies)
- Phase 3 can start after 2.1 is done (independent of balance/leverage implementations)
- Phase 5 requires all of Phases 1-4

**Critical Path**: Phase 1 → Phase 2 → Phase 4 → Phase 5 → Phase 6
