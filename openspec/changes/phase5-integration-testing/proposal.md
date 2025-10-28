# Phase 5: Integration Layer Testing - Proposal

**Change ID**: `phase5-integration-testing`  
**Status**: PROPOSED  
**Priority**: HIGH  
**Target Release**: October 28, 2025

## Executive Summary

Phase 5 introduces comprehensive integration testing for the nzeza MPC trading bot. Following successful completion of Phase 4's domain layer (129/129 tests passing), this phase validates how the domain services integrate with:

1. **WebSocket Price Feeds** - Real-time market data consumption
2. **Exchange Client Integration** - Multi-exchange order execution
3. **Actor Message Passing** - Asynchronous component communication

## Business Context

The domain layer is fully tested and production-ready. However, integration points require validation:

- WebSocket connections must handle reconnection scenarios
- Exchange clients must properly route orders to correct exchanges
- Actor message passing must maintain correctness under concurrent load
- Error handling must properly cascade from infrastructure to domain

## What is Being Built

### New Testing Infrastructure

| Component | Purpose | Tests |
|-----------|---------|-------|
| WebSocket Tests | Validate price feed consumption | 20+ |
| Exchange Client Tests | Validate multi-exchange routing | 25+ |
| Actor Message Tests | Validate async component interaction | 15+ |
| End-to-End Workflow Tests | Validate complete trading flows | 10+ |

### New Capabilities

| Capability | Description |
|------------|------------|
| `websocket-price-feeds` | Real-time market data ingestion with reconnection |
| `exchange-client-integration` | Multi-exchange client abstraction and routing |
| `actor-message-passing` | Type-safe async message passing between components |

## Impact Assessment

### Positive Impacts
- ✅ Integration layer fully validated before production
- ✅ Identifies missing error handling at boundaries
- ✅ Validates actor model architecture decisions
- ✅ Ensures WebSocket reconnection is robust
- ✅ Confirms multi-exchange routing works correctly

### Risk Mitigation
- 🔒 All domain layer tests continue to pass (regression-free)
- 🔒 Integration tests use test doubles/mocks for external systems
- 🔒 No changes to domain layer implementation
- 🔒 Staged approach (WebSocket → Exchange → Actors)

### No Breaking Changes
- Domain layer API remains unchanged
- Existing tests continue to pass
- Pure additive testing approach

## Scope Definition

### In Scope
✅ WebSocket connection testing (with mocking)  
✅ Exchange client multi-routing validation  
✅ Actor message passing semantics  
✅ Error propagation from infrastructure to domain  
✅ Reconnection logic and backoff strategies  

### Out of Scope
❌ Live exchange connections (use mocks)  
❌ Real WebSocket data processing  
❌ Production deployment configuration  
❌ Performance optimization  

## Implementation Strategy

### Phase 5.1: WebSocket Price Feeds (Est. 2 hours)
1. Create test WebSocket server mock
2. Test connection establishment
3. Test reconnection with exponential backoff
4. Test price data parsing and validation
5. Test circuit breaker on repeated failures

### Phase 5.2: Exchange Client Integration (Est. 2.5 hours)
1. Mock exchange API responses
2. Test order routing to correct exchange
3. Test balance queries across exchanges
4. Test order status checking
5. Test error handling for missing exchanges

### Phase 5.3: Actor Message Passing (Est. 2 hours)
1. Test message delivery between actors
2. Test backpressure handling
3. Test error message propagation
4. Test actor restart scenarios
5. Test concurrent message ordering

### Phase 5.4: End-to-End Workflows (Est. 1.5 hours)
1. Test complete signal → order flow
2. Test portfolio consistency across components
3. Test error recovery scenarios
4. Test multi-symbol concurrent operations

## Success Criteria

### Testing Coverage
- [ ] 70+ new integration tests
- [ ] 100% of integration points covered
- [ ] All error paths validated
- [ ] Reconnection logic fully tested

### Quality Gates
- [ ] All new tests pass (100% success rate)
- [ ] All existing domain tests still pass
- [ ] No regressions introduced
- [ ] Code follows project conventions

### Documentation
- [ ] Integration architecture documented
- [ ] Test structure documented
- [ ] Error handling documented
- [ ] Recovery procedures documented

## Timeline

| Phase | Duration | Deliverable |
|-------|----------|-------------|
| 5.1: WebSocket | 2h | 20 tests passing |
| 5.2: Exchange Client | 2.5h | 25 tests passing |
| 5.3: Actor Message | 2h | 15 tests passing |
| 5.4: End-to-End | 1.5h | 10 tests passing |
| **Total** | **8 hours** | **70+ tests** |

## Files to be Created/Modified

### New Test Files
- `src/application/actors/tests/websocket_integration_tests.rs`
- `src/application/services/tests/exchange_client_integration_tests.rs`
- `src/application/tests/end_to_end_tests.rs`

### New Mock Helpers
- `src/application/actors/tests/mock_websocket_server.rs`
- `src/application/services/tests/mock_exchange_client.rs`
- `src/application/tests/test_fixtures.rs`

### No Changes To
- Domain layer (domain/ directory)
- Any production code
- Existing tests

## Technical Decisions

### Use Mocks, Not Real Connections
- External systems mocked to avoid flakiness
- Tests run fast (<0.1s each)
- CI/CD friendly (no network dependencies)

### Test Actor Model Directly
- Test message passing, not async infrastructure
- Validate actor semantics (delivery, ordering, isolation)
- Mock tokio only when necessary

### Organize by Integration Point
- WebSocket tests grouped together
- Exchange client tests grouped together
- Actors tests grouped together
- E2E tests separate

## Assumptions

1. ✅ Domain layer (129 tests) continues to pass
2. ✅ No breaking changes to domain APIs needed
3. ✅ Mock testing framework (mockito/proptest) available
4. ✅ Tokio actor model remains as-is
5. ✅ Exchange abstraction layer already exists

## Open Questions / Decisions Needed

1. **Q: Should we test actual tokio actor initialization?**
   - A: Yes, but with mocked external I/O

2. **Q: How many exchange mocks do we need?**
   - A: Mock 3 exchanges (dydx, coinbase, hyperliquid) minimum

3. **Q: Should E2E tests use database?**
   - A: No - use in-memory state for speed

## Approval Gate

**Status**: ⏳ AWAITING APPROVAL

This proposal requires approval before implementation begins.

### Reviewers
- [ ] Architecture review
- [ ] Integration point owners
- [ ] Test infrastructure owner

### Sign-Off
- [ ] Technical lead approval
- [ ] Timeline feasibility confirmation

## Next Steps

1. 📋 **Request approval** of this proposal
2. ✅ **Upon approval**: Create detailed `tasks.md` with step-by-step implementation
3. 🔨 **Implementation**: Follow TDD methodology (Red → Green → Refactor)
4. 📝 **Documentation**: Generate comprehensive integration testing guide

---

**Document Generated**: October 28, 2025  
**Change ID**: `phase5-integration-testing`  
**Status**: PROPOSED
