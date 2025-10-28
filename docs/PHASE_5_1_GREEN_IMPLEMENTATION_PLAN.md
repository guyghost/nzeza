# Phase 5.1: GREEN Phase Implementation Strategy

**Date**: October 28, 2025  
**Status**: ✅ Ready to Begin GREEN Phase  
**Objective**: Make all 32 RED phase tests pass  

## Current Status

### RED Phase Complete ✅
- 32 tests written and compiled
- Tests failing at assertions (expected RED phase behavior)
- Mock infrastructure fully defined
- Test structure organized by categories

### Investigation Results
After reviewing the codebase:

1. **WebSocketClient is ALREADY PARTIALLY IMPLEMENTED** ✅
   - File: `src/application/actors/websocket_client.rs` (2149 lines)
   - Contains: `pub fn new()`, `pub async fn connect()`, `pub async fn disconnect()`, etc.
   - Contains: Full infrastructure with streams, metrics, circuit breaker logic

2. **Type Definitions EXIST** ✅
   - 40+ public types defined in websocket_client.rs
   - ConnectionState, PriceUpdate, DisconnectEvent, CircuitBreakerEvent, etc.
   - All exported via `pub use websocket_client::*;` in mod.rs

3. **Mock Server Fully Defined** ✅
   - File: `src/application/actors/tests/mock_websocket_server.rs`
   - Implements: MockWebSocketServer, MockWebSocketConnection
   - Methods: start(), stop(), send_price(), send_malformed_json(), etc.

## Why Tests Are Failing: Root Cause Analysis

The 32 tests are RED because:

### Issue 1: Missing Type Exports (HIGH PRIORITY)
Tests import types that aren't re-exported from the actors module:
```rust
// In test: src/application/actors/tests/websocket_connection_tests.rs
use crate::application::actors::{
    WebSocketClient, 
    ConnectionState,          // ✅ Exported
    DisconnectType,          // ❌ NOT exported?
    DisconnectEvent,         // ❌ NOT exported?
    TimeoutMetrics,          // ❌ NOT exported?
    // ... more missing
};
```

**Solution**: Export missing types in `src/application/actors/mod.rs`

### Issue 2: Mock Server May Have Unimplemented Methods
Tests call methods like `mock_server.simulate_connection()` but the method may be incomplete or missing.

**Solution**: Complete mock_websocket_server.rs with all required methods

### Issue 3: Mock Server Not Following Expected Behavior
Tests expect specific return types and timing from mock server.

**Solution**: Verify mock server implementation matches test expectations

### Issue 4: Actual WebSocket Connection vs Mock
Tests try to connect to mock server but real WebSocketClient may use tokio-tungstenite.

**Solution**: Ensure real client can handle mock server responses OR modify client for testing

## GREEN Phase Action Plan

### Phase 5.1.0: Fix Type Exports (15 minutes) - IMMEDIATE

**Goal**: Make all type imports resolve

**Tasks**:

1. **Identify Missing Type Exports**
   ```bash
   grep "pub enum\|pub struct" src/application/actors/websocket_client.rs | \
   while read line; do
     type=$(echo "$line" | grep -o "[A-Z][a-zA-Z]*" | head -1)
     grep -q "pub use websocket_client::$type" src/application/actors/mod.rs || echo "MISSING: $type"
   done
   ```

2. **Update mod.rs to Export All Required Types**
   ```rust
   // src/application/actors/mod.rs
   pub use websocket_client::{
       WebSocketClient,
       ConnectionState,
       CircuitState,
       PriceUpdate,
       DisconnectType,
       DisconnectEvent,
       TimeoutMetrics,
       // ... all other missing types
   };
   ```

3. **Or Use Wildcard (Already Done)**
   - If `pub use websocket_client::*;` exists, just ensure types are public in websocket_client.rs

**Success Criteria**:
- All test imports compile without errors
- No "cannot find type in scope" errors

### Phase 5.1.1: Complete Mock Server Implementation (30 minutes)

**Goal**: Mock server has all methods tests expect

**Required Methods**:

```rust
impl MockWebSocketServer {
    // Connection management
    pub async fn simulate_connection(&mut self) -> Result<()> { ... }
    pub async fn simulate_drop(&mut self, connection_id: u64) -> Result<()> { ... }
    
    // Price data
    pub async fn send_price(&mut self, product_id: &str, price: &str) -> Result<()> { ... }
    pub async fn send_malformed_json(&mut self) -> Result<()> { ... }
    
    // Failure scenarios
    pub async fn set_failure_mode(&mut self, mode: FailureMode) { ... }
    pub async fn reject_next_connection(&mut self) { ... }
    
    // Reconnection scenarios
    pub async fn simulate_timeout(&mut self) { ... }
    pub async fn simulate_slow_response(&mut self, delay: Duration) { ... }
    
    // Connection tracking
    pub async fn next_connection(&mut self) -> Option<MockWebSocketConnection> { ... }
    pub fn connection_count(&self) -> usize { ... }
    pub async fn stop(&mut self) -> Result<()> { ... }
}
```

**Success Criteria**:
- All methods compile
- Methods have proper async/await
- Methods return expected types
- No `unimplemented!()` in critical paths

### Phase 5.1.2: Fix WebSocketClient Implementation (1-2 hours)

**Goal**: WebSocketClient methods fully functional for tests

**Key Methods to Review/Fix**:

1. **Connection Management**
   - [ ] `pub fn new(url: &str) -> Self` - Create client
   - [ ] `pub async fn connect(&self) -> Result<(), String>` - Establish connection
   - [ ] `pub async fn disconnect(&self)` - Close connection
   - [ ] `pub fn is_connected(&self) -> bool` - Check connection state

2. **State Management**
   - [ ] `pub fn connection_state(&self) -> ConnectionState` - Get current state
   - [ ] `pub fn last_heartbeat(&self) -> Option<Instant>` - Get heartbeat
   - [ ] `pub fn connection_id(&self) -> Option<String>` - Get connection ID
   - [ ] State transitions: Disconnected → Connecting → Connected

3. **Reconnection Logic**
   - [ ] `pub async fn reconnect(&self) -> Result<(), String>` - Reconnect with backoff
   - [ ] Exponential backoff: 100ms → 200ms → 400ms → 800ms
   - [ ] Max retry enforcement (stop after N attempts)
   - [ ] Backoff reset on success

4. **Price Parsing**
   - [ ] JSON parsing from incoming messages
   - [ ] Field validation (product_id, price, timestamp)
   - [ ] Type validation (price must be numeric)
   - [ ] Decimal precision preservation (8+ places)
   - [ ] Error handling for malformed data

5. **Circuit Breaker**
   - [ ] Open after N consecutive failures
   - [ ] Half-open after timeout
   - [ ] Close after M consecutive successes
   - [ ] Timeout escalation in open state

**Success Criteria**:
- All methods implemented (no `unimplemented!()`)
- Async properly handled with `.await`
- Error paths return Err(String)
- Success paths return Ok(T)

### Phase 5.1.3: Validate Connection Between Client and Mock Server (1 hour)

**Goal**: Real WebSocketClient can connect to mock server

**Tasks**:

1. **Test Real Connection**
   - Mock server listens on localhost:9001
   - WebSocketClient connects to `ws://127.0.0.1:9001`
   - Verify connection succeeds

2. **Debug Connection Issues**
   - Check tokio-tungstenite compatibility
   - Verify mock server WebSocket protocol compliance
   - Add tracing for connection lifecycle

3. **Fix Compatibility Issues**
   - If mock server too simple, upgrade to real WebSocket protocol
   - If client has issues, adjust client connection logic

**Success Criteria**:
- WebSocketClient can connect to MockWebSocketServer
- Connection state transitions work correctly
- Messages flow between client and server

### Phase 5.1.4: Run Tests Progressively (30 minutes)

**Goal**: Gradually get tests from RED to PASSING

**Test Groups** (in order):

1. **Connection Tests** (8 tests)
   ```bash
   cargo test --lib websocket_connection_tests -- --test-threads=1 --nocapture
   ```
   - Expected: Tests pass one by one
   - Debug: Any connection failures

2. **Reconnection Tests** (8 tests)
   ```bash
   cargo test --lib websocket_reconnection_tests -- --test-threads=1 --nocapture
   ```
   - Expected: Backoff timings validated
   - Debug: Timing precision issues

3. **Price Parsing Tests** (8 tests)
   ```bash
   cargo test --lib websocket_price_parsing_tests -- --test-threads=1 --nocapture
   ```
   - Expected: JSON parsing validated
   - Debug: Field validation failures

4. **Circuit Breaker Tests** (8 tests)
   ```bash
   cargo test --lib websocket_circuit_breaker_tests -- --test-threads=1 --nocapture
   ```
   - Expected: Failure detection works
   - Debug: State transitions

**Success Criteria**:
- 32/32 tests passing
- Each test completes in < 1s
- Total suite runs in < 2min

## Implementation Priority Matrix

| Priority | Component | Est. Time | Impact |
|----------|-----------|-----------|--------|
| 🔴 CRITICAL | Type Exports | 15min | All tests compile |
| 🔴 CRITICAL | Mock Server Methods | 30min | Tests can run |
| 🟡 HIGH | WebSocketClient Core | 1h | Basic connection |
| 🟡 HIGH | Reconnection Logic | 45min | 8/32 tests pass |
| 🟡 HIGH | Price Parsing | 45min | 16/32 tests pass |
| 🟢 MEDIUM | Circuit Breaker | 30min | 24/32 tests pass |
| 🟢 MEDIUM | Refactoring | 30min | Code quality |

**Total Estimated Time**: 3.5 - 4 hours

## Risk Assessment

### Risk 1: Mock Server WebSocket Compatibility ⚠️
**Likelihood**: Medium  
**Impact**: Blocking - tests can't run  
**Mitigation**: Test mock server independently first

### Risk 2: Async/Await Deadlocks 🟡
**Likelihood**: Low  
**Impact**: Flaky tests  
**Mitigation**: Use synchronous locks where possible

### Risk 3: Timing-Based Test Flakiness 🟡
**Likelihood**: Medium (for reconnection tests)  
**Impact**: Intermittent failures  
**Mitigation**: Add generous timeouts, use `tokio::time::advance()`

### Risk 4: Mock Server Cleanup ⚠️
**Likelihood**: Low  
**Impact**: Port conflicts between tests  
**Mitigation**: Each test uses unique port, proper teardown

## Success Metrics

### After GREEN Phase Completion:
- ✅ 32/32 tests passing
- ✅ All 129 domain tests still passing (no regressions)
- ✅ Each test execution < 1s
- ✅ Total suite execution < 30s
- ✅ No test flakiness (100% pass rate across 3 runs)

## Next Steps for Session

### Immediate (Next 30 minutes):
1. **Fix Type Exports** (Phase 5.1.0)
   - Verify all types are exported
   - Commit: `fix(websocket): export all required types for tests`

2. **Complete Mock Server** (Phase 5.1.1)
   - Add any missing methods
   - Commit: `fix(websocket): complete mock server implementation`

### Follow-up (Next 1.5-2 hours):
3. **Review WebSocketClient** (Phase 5.1.2)
   - Verify all methods implemented
   - Fix any stubs or incomplete implementations
   - Multiple commits as needed

4. **Run Tests** (Phase 5.1.3-5.1.4)
   - Run test groups progressively
   - Debug failures
   - Final validation

## Command Reference

```bash
# Check compilation
cargo build --lib

# Run all WebSocket tests
cargo test --lib websocket -- --test-threads=1 --nocapture

# Run by category
cargo test --lib websocket_connection_tests
cargo test --lib websocket_reconnection_tests
cargo test --lib websocket_price_parsing_tests
cargo test --lib websocket_circuit_breaker_tests

# Single test
cargo test --lib test_basic_websocket_connection -- --nocapture

# With logging
RUST_LOG=nzeza=debug cargo test --lib websocket_connection_tests -- --nocapture

# Check for missing types
cargo check 2>&1 | grep "cannot find"

# Verify domain tests still pass
cargo test --lib concurrency_tests
cargo test --lib order_execution_tests
cargo test --lib portfolio_consistency_tests
```

## Documentation & References

- Phase 5.1 RED Phase: `docs/PHASE_5_1_RED_PHASE_STATUS.md`
- Phase 5 Proposal: `openspec/changes/phase5-integration-testing/proposal.md`
- WebSocketClient Impl: `src/application/actors/websocket_client.rs`
- Mock Server: `src/application/actors/tests/mock_websocket_server.rs`
- Test Cases: `src/application/actors/tests/websocket_*_tests.rs`

---

**Document Generated**: October 28, 2025  
**Phase**: GREEN (Implementation Planning)  
**Status**: Ready to Execute  
**Estimated Completion**: 4 hours

