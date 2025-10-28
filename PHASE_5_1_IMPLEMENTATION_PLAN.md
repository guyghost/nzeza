# Next Session: Phase 5.1 Implementation Plan

**Date:** October 28, 2025  
**Current Status:** Phase 4.4 complete ✅ → Phase 5.1 RED (tests written, implementation pending)  
**Estimated Duration:** 3-5 days for one senior developer  
**Priority:** 🔴 CRITICAL - Blocks all trading functionality

---

## Executive Summary

Phase 5.1 is **FULLY SPECIFIED** with 20 comprehensive tests that exactly define the requirements. This document serves as the handoff for the next developer.

**Status:** Tests written (RED phase) - **20 failing tests intentionally**  
**Need:** Implementer to make all 20 tests GREEN ✅

---

## What's Needed: High-Level

```
PROBLEM: System has no price data → can't trade
SOLUTION: Implement WebSocket connections to exchanges
SCOPE: 20 tests defining exact requirements
DELIVERABLE: All 20 tests passing ✅
EFFORT: 3-5 days (1 developer)
BLOCKING: Everything after Phase 5.1
```

---

## Test Files to Review First

### 📁 Read These (in this order)

```
src/application/actors/tests/
├── mod.rs .......................... Module structure (read first)
├── mock_websocket_server.rs ........ Mock server spec (read second)
├── websocket_connection_tests.rs ... 5 connection tests
├── websocket_reconnection_tests.rs . 5 reconnection tests  
├── websocket_price_parsing_tests.rs 5 parsing tests
└── websocket_circuit_breaker_tests. 5 circuit breaker tests
```

### ✅ Read These Docs

```
TDD_RED_PHASE_WEBSOCKET_TESTS_SUMMARY.md  ← RED Phase spec
AGENTS.md ................................. ← Development methodology
PRODUCTION_READINESS_ASSESSMENT.md ......... ← Context on why critical
```

---

## The 20 Tests: What They Expect

### Group 1: Connection Tests (5 tests)
```
✅ test_basic_websocket_connection
   → Establish connection to mock server
   → Verify connection state is "Connected"

✅ test_multiple_concurrent_connections
   → Open 3+ parallel connections
   → Verify all can read messages independently

✅ test_concurrent_message_reading
   → Read from multiple streams at once
   → Verify no message loss or corruption

✅ test_websocket_auth_validation
   → Connection requires bearer token
   → Reject invalid tokens

✅ test_invalid_message_handling
   → Malformed WebSocket frames handled gracefully
   → Error logged, processing continues
```

### Group 2: Reconnection Tests (5 tests)
```
✅ test_exponential_backoff_on_disconnect
   → After disconnect: wait 100ms, retry
   → If fails: wait 200ms, retry
   → If fails: wait 400ms, retry
   → Backoff sequence: 100ms, 200ms, 400ms, 800ms

✅ test_max_retries_enforcement
   → Stop retrying after 5 failed attempts (default)
   → Raise ConnectionAborted error

✅ test_backoff_reset_on_success
   → After successful reconnection
   → Next failure starts at base backoff (100ms)
   → NOT at previous level

✅ test_concurrent_reconnection_attempts
   → Multiple concurrent reconnect attempts
   → Handle without race conditions
   → Only one actual connection at a time

✅ test_connection_state_preservation
   → During reconnection, pending messages preserved
   → Resume processing after reconnect
   → No message loss
```

### Group 3: Price Parsing Tests (5 tests)
```
✅ test_valid_price_message_parsing
   → Parse valid JSON price message
   → Preserve full decimal precision
   → Extract: product_id, price, timestamp

✅ test_malformed_json_handling
   → Invalid JSON handled gracefully
   → Error logged
   → Processing continues

✅ test_missing_required_fields
   → Reject messages missing: product_id, price, timestamp
   → Log specific error about missing field
   → Continue processing next message

✅ test_price_type_validation
   → Price must be numeric
   → Reject strings like "ABC"
   → Reject null/missing price

✅ test_decimal_precision_preservation
   → Full decimal precision preserved (8+ places)
   → 99.12345678901234 stays exact
   → No float rounding errors
```

### Group 4: Circuit Breaker Tests (5 tests)
```
✅ test_circuit_opens_after_threshold
   → After 5 consecutive failures
   → Open circuit (stop trying to connect)
   → Raise CircuitOpen error

✅ test_circuit_half_open_after_timeout
   → Circuit open for 10 seconds
   → Then transition to half-open
   → Try one connection attempt
   → If succeeds: close circuit
   → If fails: back to open (restart timeout)

✅ test_circuit_closes_on_success
   → In half-open state
   → 3 consecutive successful connections
   → Circuit closes (back to normal)

✅ test_exponential_backoff_during_open
   → While circuit is open
   → Timeout increases exponentially
   → Sequence: 10s, 20s, 40s, 80s

✅ test_circuit_metrics_collection
   → Track: failures, successes, open_duration
   → Expose metrics via public interface
   → Timestamps of state transitions
```

---

## Implementation Roadmap

### Day 1: Setup & Mock Server (8 hours)

#### Task 1.1: Understand the Structure
```bash
# Read and understand existing code
1. Read mod.rs - understand module organization
2. Read mock_websocket_server.rs - understand what's needed
3. Read src/application/mod.rs - understand actors
4. Sketch out what MockWebSocketServer needs to do
```

#### Task 1.2: Implement Mock Server
```rust
// src/application/actors/tests/mock_websocket_server.rs

// The mock server needs to be able to:
pub struct MockWebSocketServer {
    // Store: connections, config, state
}

impl MockWebSocketServer {
    pub fn new() -> Self { ... }
    
    // Simulate connection
    pub async fn accept_connection() -> Result<MockConnection> { ... }
    
    // Simulate sending prices
    pub async fn send_price(&self, product_id: &str, price: f64) { ... }
    
    // Simulate disconnection
    pub async fn simulate_disconnect() { ... }
    
    // Simulate malformed data
    pub async fn send_malformed(&self, data: &str) { ... }
    
    // Get metrics
    pub fn metrics(&self) -> CircuitBreakerMetrics { ... }
}

pub struct MockWebSocketConnection {
    // Connection state, pending messages
}
```

**Tests that should pass after this:**
- None yet (they call client methods that don't exist)

---

### Day 2: WebSocket Client (8 hours)

#### Task 2.1: Create WebSocketClient Interface
```rust
// src/application/actors/ - new file

pub struct WebSocketClient {
    // Connection state
    // Reconnection logic
    // Message parsing
    // Circuit breaker
}

impl WebSocketClient {
    pub async fn connect(url: &str, auth_token: &str) -> Result<Self> { ... }
    
    pub async fn read_message(&mut self) -> Result<PriceMessage> {
        // Parse incoming WebSocket message
        // Validate JSON
        // Check required fields
        // Preserve decimal precision
        // Handle errors gracefully
    }
    
    pub async fn disconnect(&mut self) { ... }
    
    pub fn is_connected(&self) -> bool { ... }
    
    pub fn metrics(&self) -> CircuitBreakerMetrics { ... }
}

pub struct PriceMessage {
    pub product_id: String,
    pub price: Decimal,  // Use Decimal for precision
    pub timestamp: DateTime<Utc>,
}
```

**Tests that should pass after this:**
- `test_basic_websocket_connection` ✅
- `test_valid_price_message_parsing` ✅

---

### Day 3: Reconnection Logic (8 hours)

#### Task 3.1: Implement Exponential Backoff
```rust
pub struct ReconnectionConfig {
    pub base_backoff_ms: u64,    // 100ms
    pub max_backoff_ms: u64,     // 800ms
    pub backoff_multiplier: f64, // 2.0
    pub max_retries: u32,        // 5
}

// In WebSocketClient:
impl WebSocketClient {
    async fn reconnect_with_backoff(&mut self) -> Result<()> {
        let mut backoff = self.config.base_backoff_ms;
        let mut retry_count = 0;
        
        loop {
            if retry_count >= self.config.max_retries {
                return Err(ConnectionAborted);
            }
            
            // Wait before retry
            tokio::time::sleep(Duration::from_millis(backoff)).await;
            
            match self.try_connect().await {
                Ok(()) => {
                    self.backoff = self.config.base_backoff_ms; // reset
                    return Ok(());
                }
                Err(_) => {
                    retry_count += 1;
                    backoff = (backoff * 2).min(self.config.max_backoff_ms);
                }
            }
        }
    }
}
```

**Tests that should pass after this:**
- `test_exponential_backoff_on_disconnect` ✅
- `test_max_retries_enforcement` ✅
- `test_backoff_reset_on_success` ✅

---

### Day 4: Circuit Breaker (8 hours)

#### Task 4.1: Implement Circuit Breaker Pattern
```rust
pub enum CircuitState {
    Closed,      // Normal operation
    Open,        // Too many failures, don't try
    HalfOpen,    // Testing if we recovered
}

pub struct CircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure_time: Option<DateTime<Utc>>,
    metrics: CircuitBreakerMetrics,
    
    // Config
    failure_threshold: u32,  // 5
    success_threshold: u32,  // 3
    timeout_base_ms: u64,    // 10000
}

impl CircuitBreaker {
    pub fn check(&mut self) -> Result<()> {
        match self.state {
            CircuitState::Closed => Ok(()),
            CircuitState::Open => {
                // Check if timeout expired
                if self.should_transition_to_half_open() {
                    self.state = CircuitState::HalfOpen;
                    self.success_count = 0;
                    Ok(())
                } else {
                    Err(CircuitOpen)
                }
            }
            CircuitState::HalfOpen => Ok(()), // Try one attempt
        }
    }
    
    pub fn record_success(&mut self) {
        match self.state {
            CircuitState::Closed => {
                self.failure_count = 0;
            }
            CircuitState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.success_threshold {
                    self.state = CircuitState::Closed;
                    self.metrics.record_close();
                }
            }
            CircuitState::Open => {} // Shouldn't happen
        }
    }
    
    pub fn record_failure(&mut self) {
        match self.state {
            CircuitState::Closed => {
                self.failure_count += 1;
                if self.failure_count >= self.failure_threshold {
                    self.state = CircuitState::Open;
                    self.metrics.record_open();
                }
            }
            CircuitState::HalfOpen => {
                self.state = CircuitState::Open;
                self.last_failure_time = Some(Utc::now());
                self.metrics.record_reopen();
            }
            CircuitState::Open => {} // Stay open
        }
    }
}
```

**Tests that should pass after this:**
- `test_circuit_opens_after_threshold` ✅
- `test_circuit_half_open_after_timeout` ✅
- `test_circuit_closes_on_success` ✅
- `test_exponential_backoff_during_open` ✅
- `test_circuit_metrics_collection` ✅

---

### Day 5: Integration & Polish (8 hours)

#### Task 5.1: Get Remaining Tests Green
```bash
# Run tests and see what's failing
cargo test --lib application::actors::tests

# Fix each failing test:
1. test_multiple_concurrent_connections
2. test_concurrent_message_reading
3. test_websocket_auth_validation
4. test_invalid_message_handling
5. test_concurrent_reconnection_attempts
6. test_connection_state_preservation
```

#### Task 5.2: Code Quality
```bash
# Format code
cargo fmt --all

# Check for warnings
cargo clippy --lib

# Run full test suite
cargo test --lib
```

#### Task 5.3: Documentation
```rust
// Add doc comments to all public items
/// WebSocket client for real-time price feeds
pub struct WebSocketClient { ... }

/// Establishes authenticated connection to price feed
pub async fn connect(url: &str, auth_token: &str) -> Result<Self> { ... }
```

---

## Success Criteria

### ✅ All 20 Tests Passing
```bash
cd /Users/guy/Developer/guyghost/nzeza

# Run Phase 5.1 tests
cargo test --lib application::actors::tests

# Expected output:
test result: ok. 20 passed; 0 failed;
```

### ✅ Code Quality
```
✅ cargo fmt --check passes
✅ cargo clippy has no errors
✅ cargo test --lib passes (all 129 domain + 20 new)
✅ No compiler warnings
```

### ✅ Documentation
```
✅ All public functions have doc comments
✅ Module-level documentation present
✅ Usage examples in comments
```

---

## Key Libraries Already Included

```toml
# WebSocket support
tokio-tungstenite = "0.23"  # WebSocket client/server
tokio = "1.0"               # Async runtime

# Decimal precision
bigdecimal = "0.4"          # Preserve decimal precision

# Serialization
serde_json = "1.0"          # JSON parsing
serde = "1.0"               # Serialization framework

# Error handling
thiserror = "1.0"           # Error types

# Time
chrono = "0.4"              # DateTime handling

# Futures
futures-util = "0.3"        # Stream utilities
```

**You probably need to add:**
```toml
# For even better decimal support (optional)
# rust_decimal = "1.33"  # Alternative to bigdecimal
```

---

## Files to Create

```
New files to create:
├─ src/application/actors/websocket_client.rs
│  └─ WebSocketClient struct + implementation
├─ src/application/actors/circuit_breaker.rs
│  └─ CircuitBreaker struct + implementation
└─ (The test files already exist ✅)

Modify existing:
└─ src/application/actors/mod.rs
   └─ Add mod declarations for new modules
```

---

## Debugging Tips

### Test Fails with "unimplemented"
This is expected! The tests call methods that don't exist yet. Implement them one at a time.

### Async Compilation Issues
Remember: Use `#[tokio::test]` for async tests, `#[test]` for sync tests.

### Decimal Precision Issues
```rust
// Use String for parsing to avoid float precision loss
let price: Decimal = price_str.parse()?;

// Or use bigdecimal:
use bigdecimal::BigDecimal;
let price: BigDecimal = price_str.parse()?;
```

### Timeout Issues in Tests
```rust
// Set reasonable timeouts for mock operations
tokio::time::timeout(
    Duration::from_secs(5),
    some_async_operation()
).await?
```

---

## Daily Checklist

### Start of Day
- [ ] Pull latest from main
- [ ] Read today's task in this document
- [ ] Run `cargo test --lib` to see current state

### During Day
- [ ] Implement one feature
- [ ] Run `cargo test --lib` to verify
- [ ] Commit with meaningful message
- [ ] Update status in this doc

### End of Day
- [ ] All tests passing for today's work
- [ ] Code formatted (cargo fmt)
- [ ] No clippy warnings
- [ ] Push to branch

### End of Task
- [ ] All 20 tests green ✅
- [ ] Code reviewed (self + PR)
- [ ] Documentation complete
- [ ] Ready for Phase 5.2

---

## Git Workflow

```bash
# Each day, create meaningful commits
git commit -m "feat(websocket): implement connection logic"
git commit -m "feat(websocket): add exponential backoff"
git commit -m "feat(websocket): implement circuit breaker"
git commit -m "test(websocket): verify all 20 tests passing"

# Push to main when done
git push origin main
```

---

## Next Phase After This

Once all 20 tests pass ✅:

### Phase 5.2: Signal Generation (2-3 days)
- Connect WebSocket price feeds to actor system
- Implement indicator calculations
- Generate trading signals
- Tests for signal accuracy

### Phase 5.3: Exchange Integration (2-5 days)
- Connect to real Coinbase API
- Order placement & tracking
- Position reconciliation
- Tests with live API (testnet)

### Phase 5.4: End-to-End Tests (3-5 days)
- Complete price → signal → order flow
- Error scenarios
- Recovery procedures
- Multi-exchange coordination

---

## Estimated Effort Breakdown

```
Understanding requirements:  2-3 hours
Mock server implementation:   4-5 hours
WebSocket client:            6-8 hours
Reconnection logic:          4-6 hours
Circuit breaker:             4-6 hours
Testing & debugging:         4-6 hours
Code quality & cleanup:      2-3 hours
─────────────────────────────────────
Total:                      26-37 hours ≈ 3-5 days (1 dev)
```

---

## Success Story (Best Case)

```
Day 1: ✅ Mock server ready + 3 tests passing
Day 2: ✅ WebSocket client ready + 7 tests passing
Day 3: ✅ Reconnection logic + 12 tests passing
Day 4: ✅ Circuit breaker + 18 tests passing
Day 5: ✅ All 20 tests green ✅

Result:
- Phase 5.1 COMPLETE
- 149/149 total tests passing (129 domain + 20 new)
- Ready for Phase 5.2
- Timeline on track for production
```

---

## Failure Case (What Could Go Wrong)

```
❌ WebSocket libraries not compatible
   → Solution: Check tokio-tungstenite compatibility
   
❌ Decimal precision issues
   → Solution: Use BigDecimal, not f64
   
❌ Timeout race conditions
   → Solution: Use tokio::time::sleep with proper guards
   
❌ Circuit breaker state management complex
   → Solution: Start simple, add complexity incrementally
   
❌ Tests stay RED
   → Solution: Debug one test at a time, not all 20
```

**Approach:** If stuck more than 2 hours on one issue, ask for help / escalate.

---

## Resources & Context

### Code to Review First
```
1. src/application/actors/tests/mod.rs
   └─ Shows what needs to be tested
   
2. src/application/actors/tests/mock_websocket_server.rs
   └─ Shows mock server interface
   
3. src/application/actors/tests/websocket_connection_tests.rs
   └─ Shows first 5 tests (simpler than others)
```

### Documentation to Read
```
1. TDD_RED_PHASE_WEBSOCKET_TESTS_SUMMARY.md
   └─ Full RED phase specification
   
2. PRODUCTION_READINESS_ASSESSMENT.md
   └─ Why this is critical
   
3. AGENTS.md (sections on TDD)
   └─ Development methodology
```

### Reference Implementation
```
Similar patterns exist in:
- src/domain/services/order_executor.rs (error handling)
- src/domain/services/portfolio_manager.rs (state management)
- src/rate_limit.rs (existing rate limiting)
```

---

## Communication Plan

### Daily Updates
- Commit messages show progress
- One commit per working feature
- Descriptive commit titles

### Blockers
If stuck > 2 hours:
1. Document the issue in comments
2. Create minimal reproducible example
3. Ask for help (escalate immediately)
4. Don't let blockers slow progress

### Completion
Once all 20 tests green:
1. Create PR with full description
2. Show test results
3. Note any decisions made
4. Propose Phase 5.2 timeline

---

## Final Notes

### You Can't Fail
If any 20 tests are passing → progress has been made ✅

### Start Simple
Focus on one test at a time:
1. Make `test_basic_websocket_connection` pass
2. Then `test_valid_price_message_parsing`
3. Then add exponential backoff
4. Then circuit breaker
5. Polish edge cases

### Test-Driven Development
Each test is a specification. If it fails, read the assertion message → understand what's needed → implement exactly that.

---

## Quick Start

```bash
# Clone and setup
cd /Users/guy/Developer/guyghost/nzeza
git pull origin main

# See current status
cargo test --lib application::actors::tests

# Expected: 0 passed; 20 failed; (RED phase)

# Start implementation with first task from Day 1
# Come back to this doc as reference
```

---

**Document Version:** 1.0  
**Created:** October 28, 2025  
**For:** Next developer starting Phase 5.1  
**Status:** Ready for implementation  
**Priority:** 🔴 CRITICAL - Blocks production

**Questions?** See PRODUCTION_READINESS_ASSESSMENT.md or AGENTS.md

