# Phase 5: Integration Testing - Design Decisions

**Change ID**: `phase5-integration-testing`

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Integration Testing Layer                 │
├──────────────────┬──────────────────┬──────────────────────┤
│ WebSocket Tests  │ Exchange Tests   │ Actor Message Tests  │
│  (20 tests)      │  (25 tests)      │   (15 tests)         │
└────────┬─────────┴────────┬─────────┴──────────┬────────────┘
         │                  │                    │
┌────────▼────────┐ ┌──────▼────────┐ ┌─────────▼──────────┐
│ Mock WebSocket  │ │ Mock Exchange │ │ Actor Test Utils   │
│   Server        │ │    Clients    │ │   (Message Box)    │
└────────┬────────┘ └──────┬────────┘ └─────────┬──────────┘
         │                  │                    │
┌────────▼──────────────────▼────────────────────▼──────────┐
│              Domain Layer (Unchanged)                      │
│  Order Executor, Portfolio Manager, Position Manager     │
│         All 129 tests continue to pass                     │
└──────────────────────────────────────────────────────────┘
```

## Key Design Principles

### 1. No Production Code Changes
- Integration tests are **purely additive**
- Domain layer implementation remains unchanged
- Tests validate existing abstractions
- Zero risk of regression

### 2. Mock-Based Testing
- **Why**: Speed, consistency, CI/CD friendly
- **How**: Mock external systems at integration boundaries
- **Benefit**: Tests run in <0.1s each, no flakiness
- **Trade-off**: Don't catch network-level issues (but that's infrastructure testing, not integration testing)

### 3. Isolation Between Test Modules
```
5.1 WebSocket Tests
├─ Independent mock WebSocket server
├─ No dependency on 5.2 or 5.3
└─ Can run standalone

5.2 Exchange Client Tests
├─ Independent mock exchange clients
├─ No dependency on 5.1 or 5.3
└─ Can run standalone

5.3 Actor Message Tests
├─ Independent test utilities
├─ No dependency on 5.1 or 5.2
└─ Can run standalone

5.4 End-to-End Tests
├─ Composition of above components
├─ Uses fixtures combining mocks
└─ Validates integration points
```

### 4. Explicit Error Scenarios
Every integration point tested for:
- ✅ Happy path (success case)
- ✅ Connection failure (retry/fallback)
- ✅ Timeout (circuit breaker)
- ✅ Invalid data (parsing error)
- ✅ Resource exhaustion (backpressure)

## Technical Decisions

### Decision 1: Mock Framework Choice
**Option A: Manual mocks** (chosen)
- **Pros**: Simple, explicit, fast to write
- **Cons**: More boilerplate
- **Rationale**: For this project scale, manual mocks are clearer than complex framework mocks

**Option B: mockito/proptest**
- **Pros**: Less boilerplate
- **Cons**: Additional dependency, learning curve
- **Rejected**: Overkill for current scope

### Decision 2: WebSocket Testing Strategy
**Option A: tokio-tungstenite test server** (chosen)
- **Pros**: Real WebSocket protocol, matches production
- **Cons**: Slightly heavier weight
- **Rationale**: Validates actual protocol handling

**Option B: Mock byte stream**
- **Pros**: Faster, simpler
- **Cons**: Doesn't validate actual WebSocket framing
- **Rejected**: Would miss real bugs

### Decision 3: Actor Testing Approach
**Option A: Test via message inbox** (chosen)
- **Pros**: Non-invasive, tests actor semantics
- **Cons**: Requires test helpers
- **Rationale**: Validates behavior without coupling to implementation

**Option B: Direct actor function calls**
- **Pros**: Simpler setup
- **Cons**: Doesn't test message passing
- **Rejected**: Wouldn't validate actor semantics

### Decision 4: Exchange Client Mocking
**Option A: Trait-based mocks** (chosen)
- **Pros**: Polymorphic, can swap implementations
- **Cons**: Requires trait to be public
- **Rationale**: Leverages existing abstraction

**Option B: #[cfg(test)] special handling**
- **Pros**: No production code changes
- **Cons**: Couples test logic to implementation
- **Rejected**: Less flexible

## Test Organization

### File Structure
```
src/
├─ application/
│  ├─ actors/
│  │  └─ tests/
│  │     ├─ websocket_integration_tests.rs
│  │     └─ mock_websocket_server.rs
│  │
│  ├─ services/
│  │  └─ tests/
│  │     ├─ exchange_client_integration_tests.rs
│  │     └─ mock_exchange_client.rs
│  │
│  └─ tests/
│     ├─ end_to_end_tests.rs
│     └─ test_fixtures.rs
│
└─ domain/
   └─ [unchanged]
```

### Module Naming Convention
```
Tests:
- websocket_integration_tests.rs (tests for WebSocket)
- exchange_client_integration_tests.rs (tests for Exchange Client)
- actor_message_passing_tests.rs (tests for Actor Messages)
- end_to_end_tests.rs (tests for workflows)

Mocks/Fixtures:
- mock_websocket_server.rs (WebSocket mock)
- mock_exchange_client.rs (Exchange client mock)
- test_fixtures.rs (Reusable test setup)
```

## Mock Design Patterns

### Pattern 1: Configurable Mock Response
```rust
pub struct MockExchangeClient {
    responses: Arc<Mutex<HashMap<String, Result<...>>>>,
    delay: Duration,
    failure_rate: f32,
}

impl MockExchangeClient {
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
    
    pub fn with_failure_rate(mut self, rate: f32) -> Self {
        self.failure_rate = rate;
        self
    }
}
```

### Pattern 2: Message Capture for Assertions
```rust
pub struct MessageCapture<T> {
    messages: Arc<Mutex<Vec<T>>>,
}

impl MessageCapture<T> {
    pub fn assert_count(&self, expected: usize) {
        let messages = self.messages.lock().unwrap();
        assert_eq!(messages.len(), expected);
    }
    
    pub fn assert_ordered(&self, expected: Vec<T>) {
        let messages = self.messages.lock().unwrap();
        assert_eq!(*messages, expected);
    }
}
```

### Pattern 3: Scenario-Based Test Fixtures
```rust
struct TraderScenario {
    initial_balance: f64,
    active_exchanges: Vec<Exchange>,
    mock_prices: HashMap<String, f64>,
    market_conditions: MarketCondition,
}

impl TraderScenario {
    fn bullish() -> Self { /* ... */ }
    fn bearish() -> Self { /* ... */ }
    fn high_volatility() -> Self { /* ... */ }
}
```

## Error Handling Strategy

### Propagation Path
```
Mock Client Error
    ↓
Integration Boundary
    ↓
Domain Service (OrderExecutor/PortfolioManager)
    ↓
Error Type Conversion
    ↓
Test Assertion
```

### Test Assertions
```rust
// Test 1: Error propagates correctly
let result = executor.place_order(...);
assert!(result.is_err());
assert_eq!(result.unwrap_err(), "Exchange unavailable");

// Test 2: Fallback works
let result = executor_with_fallback.place_order(...);
assert!(result.is_ok()); // Falls back to secondary exchange

// Test 3: Circuit breaker engages
for _ in 0..5 {
    executor.place_order(...); // All fail
}
assert!(executor.is_circuit_open()); // Circuit engaged
```

## Performance Characteristics

### Target Execution Times
```
Per Test:        < 100ms (fast enough for TDD)
Per Module:      < 2s (5.1), < 3s (5.2), < 2s (5.3), < 2s (5.4)
All Tests:       < 10s (acceptable for CI/CD)
```

### No Flakiness Guarantee
- All tests deterministic (no timing dependencies)
- All tests isolated (no shared state)
- All tests use mock time (no real delays)
- All tests clean up resources properly

## Traceability & Debugging

### Test Naming Convention
```
test_[component]_[scenario]_[expected_outcome]

Examples:
- test_websocket_connection_established_sends_ready_message
- test_exchange_client_multiple_available_routes_to_primary
- test_actor_message_delivery_orders_messages_fifo
- test_end_to_end_signal_creates_order
```

### Logging Strategy
```rust
#[test]
fn test_websocket_reconnection() {
    eprintln!("[TEST] Starting WebSocket reconnection test");
    
    let result = websocket.reconnect();
    eprintln!("[TEST] Reconnection attempts: {:?}", result);
    
    assert!(result.is_ok());
    eprintln!("[TEST] ✅ Test passed");
}
```

## Regression Prevention

### Why Domain Tests Still Pass
1. **No domain code changes** → No behavior changes
2. **Domain tests still run** → Regression caught immediately
3. **Integration tests validate contracts** → Breaks notify of API changes

### Validation Checklist
- [ ] Run: `cargo test --lib domain` → All 129 pass
- [ ] Run: `cargo test --lib phase5` → All 70 pass
- [ ] Run: `cargo test --lib` → All 199+ pass

## Future Extensibility

### Phase 6 Can Add
- Performance benchmarks (latency/throughput)
- Load testing (concurrent operations)
- Stress testing (error recovery under pressure)
- Integration with real exchanges (infrastructure testing)

### This Phase Provides Foundation For
- Clear integration point tests as baseline
- Mock implementations for easy substitution
- Test fixture patterns for new test types
- Actor testing utilities for future actors

## Related Specifications

- `specs/order-execution/spec.md` - Order execution requirements
- `specs/order-cancellation/spec.md` - Cancellation requirements
- Domain Layer Specs (TDD green phase) - Domain services

---

**Change ID**: `phase5-integration-testing`  
**Last Updated**: October 28, 2025  
**Status**: DESIGN FINALIZED
