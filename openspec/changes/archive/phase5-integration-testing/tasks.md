# Phase 5: Integration Testing - Implementation Tasks

**Change ID**: `phase5-integration-testing`  
**Total Estimated Time**: 8 hours  
**Methodology**: Test-Driven Development (TDD)

## Task Breakdown by Phase

### Phase 5.1: WebSocket Price Feeds Integration (2 hours)

#### 5.1.1 Create WebSocket Test Infrastructure
- [ ] Create `src/application/actors/tests/mock_websocket_server.rs`
- [ ] Implement mock WebSocket server that accepts connections
- [ ] Add message queuing for price updates
- [ ] Implement connection drop simulation for reconnection testing
- [ ] Add heartbeat mechanism for keepalive

**Success Criteria**:
- Mock server listens on test port
- Can send/receive WebSocket frames
- Can simulate connection failures
- Tests compile and run

#### 5.1.2 Write WebSocket Connection Tests (20 tests)
- [ ] Test connection establishment succeeds
- [ ] Test connection failure returns error
- [ ] Test graceful disconnect
- [ ] Test reconnection on connection loss
- [ ] Test exponential backoff (1s, 2s, 4s, 8s)
- [ ] Test max reconnection attempts limit
- [ ] Test price message parsing
- [ ] Test multiple price symbols parsing
- [ ] Test invalid message handling
- [ ] Test frame buffering for partial messages
- [ ] Test concurrent message delivery
- [ ] Test connection timeout
- [ ] Test circuit breaker after 5 failures
- [ ] Test circuit breaker recovery after delay
- [ ] Test mixed valid/invalid messages
- [ ] Test large message handling
- [ ] Test message ordering preservation
- [ ] Test connection state transitions
- [ ] Test double-connection prevention
- [ ] Test graceful shutdown

**Success Criteria**:
- All 20 tests passing
- Each test < 0.1s execution
- 100% code coverage for connection logic
- No flakiness in reconnection tests

#### 5.1.3 Validate WebSocket Integration
- [ ] All tests pass in CI/CD
- [ ] Execution time < 2s total
- [ ] No database dependencies
- [ ] No external network calls
- [ ] Mock server cleanup working

**Success Criteria**:
- All 20 tests consistently passing
- Setup/teardown properly scoped
- No resource leaks

---

### Phase 5.2: Exchange Client Integration (2.5 hours)

#### 5.2.1 Create Exchange Client Mocks
- [ ] Create `src/application/services/tests/mock_exchange_client.rs`
- [ ] Implement mock for dYdX v4 client
- [ ] Implement mock for Coinbase Advanced client
- [ ] Implement mock for Hyperliquid client
- [ ] Add configurable response delays
- [ ] Add configurable failure injection

**Success Criteria**:
- All 3 mocks implement ExchangeClient trait
- Can configure success/failure responses
- Can inject delays for latency testing
- Tests compile

#### 5.2.2 Write Exchange Routing Tests (15 tests)
- [ ] Test order routes to correct exchange
- [ ] Test multiple exchanges available
- [ ] Test fallback to secondary exchange
- [ ] Test all exchanges unavailable error
- [ ] Test exchange-specific order types
- [ ] Test balance query across exchanges
- [ ] Test aggregated balance calculation
- [ ] Test order status from correct exchange
- [ ] Test concurrent orders to different exchanges
- [ ] Test exchange-specific errors propagate
- [ ] Test connection timeouts per exchange
- [ ] Test rate limiting response
- [ ] Test malformed response handling
- [ ] Test exchange selection strategy
- [ ] Test graceful degradation

**Success Criteria**:
- All 15 tests passing
- Each test < 0.1s
- Order routing logic proven correct
- All error cases covered

#### 5.2.3 Write Multi-Exchange Scenario Tests (10 tests)
- [ ] Test BTC order on dYdX
- [ ] Test ETH order on Coinbase
- [ ] Test SOL order on Hyperliquid
- [ ] Test balance aggregation 3-way
- [ ] Test one exchange fails, others succeed
- [ ] Test all exchanges fail gracefully
- [ ] Test mixed order types across exchanges
- [ ] Test cancel on specific exchange
- [ ] Test order status sync across exchanges
- [ ] Test portfolio consistency with multi-exchange

**Success Criteria**:
- All 10 tests passing
- Realistic exchange routing validated
- Multi-exchange scenarios covered

#### 5.2.4 Validate Exchange Integration
- [ ] All 25 tests pass
- [ ] Total execution < 3s
- [ ] Mock client covers all scenarios
- [ ] Error handling complete

**Success Criteria**:
- All 25 tests consistently passing
- No flakiness
- Proper cleanup between tests

---

### Phase 5.3: Actor Message Passing (2 hours)

#### 5.3.1 Create Actor Test Utilities
- [ ] Create `src/application/actors/tests/actor_test_utils.rs`
- [ ] Implement actor spawning helper
- [ ] Implement message inbox for testing
- [ ] Implement timeout utilities
- [ ] Implement message ordering assertions

**Success Criteria**:
- Utilities allow easy actor testing
- Message capture working
- Ordering assertions reliable
- Timeout handling correct

#### 5.3.2 Write Actor Message Tests (15 tests)
- [ ] Test message delivery to actor
- [ ] Test message ordering with queue
- [ ] Test concurrent message handling
- [ ] Test actor handles backpressure
- [ ] Test message timeouts
- [ ] Test actor restart recovery
- [ ] Test error message propagation
- [ ] Test actor crash handling
- [ ] Test cross-actor message passing
- [ ] Test message priority ordering
- [ ] Test actor state isolation
- [ ] Test actor cleanup on shutdown
- [ ] Test concurrent actor operations
- [ ] Test mailbox overflow handling
- [ ] Test deadletter queue fallback

**Success Criteria**:
- All 15 tests passing
- Each test < 0.1s
- Actor semantics validated
- No flakiness

#### 5.3.3 Validate Actor Integration
- [ ] All tests pass in isolation
- [ ] All tests pass when run together
- [ ] Total execution < 2s
- [ ] No actor resource leaks

**Success Criteria**:
- Consistent test results
- Proper actor cleanup
- No hanging tokio tasks

---

### Phase 5.4: End-to-End Workflow Tests (1.5 hours)

#### 5.4.1 Create E2E Test Fixtures
- [ ] Create `src/application/tests/test_fixtures.rs`
- [ ] Implement complete trader setup
- [ ] Implement mock market data
- [ ] Implement mock exchange responses
- [ ] Implement portfolio state initialization

**Success Criteria**:
- Fixtures easily create realistic scenarios
- Quick setup (<10ms)
- Clean teardown

#### 5.4.2 Write Workflow Tests (10 tests)
- [ ] Test signal → order flow (complete)
- [ ] Test price feed → indicator → signal pipeline
- [ ] Test multi-symbol concurrent trading
- [ ] Test position open → close cycle
- [ ] Test portfolio consistency maintained
- [ ] Test error recovery (exchange fails)
- [ ] Test error recovery (signal fails)
- [ ] Test concurrent signal processing
- [ ] Test trade history recording
- [ ] Test metrics collection

**Success Criteria**:
- All 10 tests passing
- Each test < 0.5s
- Complete workflows validated
- End-to-end correctness proven

#### 5.4.3 Validate E2E Integration
- [ ] All workflows pass
- [ ] Portfolio remains consistent
- [ ] Error scenarios handled
- [ ] Metrics correct

**Success Criteria**:
- All 10 tests consistently passing
- Real-world scenarios validated

---

## Quality Gates & Verification

### After Each Phase

- [ ] Run all new tests: `cargo test --lib [phase]`
- [ ] Verify no regressions: `cargo test --lib domain`
- [ ] Check execution time < target
- [ ] Review test coverage
- [ ] Update this checklist

### Final Verification (After All Phases)

- [ ] All 70+ integration tests pass
- [ ] All 129 domain tests still pass
- [ ] Total execution time < 10s
- [ ] No test flakiness
- [ ] Code follows conventions
- [ ] Documentation complete

---

## TDD Workflow per Test

For each test implementation:

1. **RED**: Write test that fails (should compile but fail at assert)
   ```bash
   cargo test --lib [test_name] 2>&1 | grep -A 5 "FAILED\|panicked"
   ```

2. **GREEN**: Implement minimum code to make test pass
   ```bash
   cargo test --lib [test_name] 2>&1 | grep "ok\|passed"
   ```

3. **REFACTOR**: Improve code without changing test outcome
   ```bash
   cargo test --lib [test_name] 2>&1 | grep "ok\|passed"
   ```

4. **COMMIT**: Create atomic commit
   ```bash
   git add .
   git commit -m "test(phase5): [phase] - [test description]"
   ```

---

## Git Commit Strategy

### Pattern: One commit per test or feature group

```bash
test(phase5.1): websocket - connection establishment
test(phase5.1): websocket - reconnection with backoff
test(phase5.2): exchange-client - order routing validation
feat(phase5.1): implement websocket reconnection logic
test(phase5.2): exchange-client - multi-exchange routing
feat(phase5.2): implement exchange client multiplexing
...
```

Total expected: ~40-50 commits (1-2 per test group)

---

## Estimated Time Breakdown

| Phase | Component | Est. Time | Target Tests |
|-------|-----------|-----------|--------------|
| 5.1 | WebSocket | 2.0h | 20 |
| 5.2 | Exchange | 2.5h | 25 |
| 5.3 | Actors | 2.0h | 15 |
| 5.4 | E2E | 1.5h | 10 |
| **Total** | **Integration** | **8.0h** | **70** |

---

## Success Metrics

By completion, we should have:

✅ **70+ integration tests** all passing  
✅ **100% of integration points** tested  
✅ **All domain tests** still passing (129)  
✅ **Fast execution** (all tests < 10s total)  
✅ **Zero flakiness** (100% consistency)  
✅ **Production-ready** (ready for Phase 6 performance testing)  

---

## Check-In Points

- After 5.1: 20 tests, 2h elapsed
- After 5.2: 45 tests, 4.5h elapsed
- After 5.3: 60 tests, 6.5h elapsed
- After 5.4: 70 tests, 8h elapsed

---

**Change ID**: `phase5-integration-testing`  
**Last Updated**: October 28, 2025  
**Status**: READY FOR IMPLEMENTATION (After Approval)
