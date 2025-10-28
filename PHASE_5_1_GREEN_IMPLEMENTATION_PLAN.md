# Phase 5.1 GREEN Implementation Plan

**Status**: Agent Implementer in Progress  
**Date**: October 28, 2025  
**Target**: Make all 31 RED tests pass  

## Key Issues Identified for Implementer

### 1. ReconnectionConfig Field Mismatch
**Tests expect:**
```rust
ReconnectionConfig {
    base_backoff: Duration,
    max_backoff: Duration,
    max_retries: u32,
    backoff_multiplier: f64,
}
```

**Current code has:**
```rust
ReconnectionConfig {
    max_attempts: u32,
    base_delay: Duration,
    max_delay: Duration,
    backoff_multiplier: f64,
}
```

**Fix**: Update struct field names to match test expectations.

### 2. ReconnectionEvent Variants Mismatch
**Tests expect:**
- `ReconnectionEvent::AttemptStarted { attempt_number, delay }`
- `ReconnectionEvent::MaxRetriesExceeded { total_attempts }`
- Other variants as needed

**Current code has:**
- `ReconnectionEvent::Started { attempt }`
- `ReconnectionEvent::Succeeded { attempt, duration }`
- `ReconnectionEvent::Failed { attempt, error }`

**Fix**: Update enum variants to match test expectations.

### 3. Bearer Token Validation
**Tests expect:**
- Token validation in `connect()` method
- Specific token: `"valid_bearer_token_abcdef123456"`
- Rejected if missing or invalid

**Current code**: Has validation but might need tweaking.

### 4. Exponential Backoff Implementation
**Tests expect:**
- Base delay: 100ms
- Multiplier: 2.0
- Pattern: 100ms, 200ms, 400ms, 800ms (for attempts 1-4)
- Max backoff enforcement

**Formula**: delay = base_backoff * multiplier^(attempt-1), capped at max_backoff

### 5. Mock Server Integration
**Tests expect:**
- Mock server at ports 9001-9020
- Proper connection simulation
- Disconnect/reconnect scenarios

**Current code**: Already has mock_websocket_server.rs with basic structure.

## Test Execution Plan

### Phase 1: Connection Tests (15 tests)
- Basic connection
- Multiple concurrent connections
- Connection failure handling
- Graceful disconnect
- Connection timeout
- Force disconnect
- State transitions
- Double connection prevention
- Auth validation
- Frame buffering
- Concurrent message reading
- Invalid message handling
- Large message handling
- Message ordering
- Keepalive/heartbeat

### Phase 2: Reconnection Tests (7 tests)
- Reconnection on loss
- Exponential backoff
- Max retries enforcement
- Backoff reset on success
- Concurrent reconnection
- State preservation
- Adaptive backoff

### Phase 3: Price Parsing Tests (5 tests)
- Valid price message parsing
- Malformed JSON handling
- Missing required fields
- Price type validation
- Decimal precision preservation (18 places)

### Phase 4: Circuit Breaker Tests (5 tests)
- Circuit opens after threshold
- Half-open state transition
- Circuit closes on success
- Exponential backoff during open
- Metrics collection

## Implementation Checklist

### ReconnectionConfig
- [ ] Rename `max_attempts` → use as needed
- [ ] Rename `base_delay` → `base_backoff`
- [ ] Rename `max_delay` → `max_backoff`
- [ ] Ensure `max_retries` field exists
- [ ] Ensure `backoff_multiplier` field exists

### ReconnectionEvent
- [ ] Add/update `AttemptStarted { attempt_number, delay }` variant
- [ ] Add/update `MaxRetriesExceeded { total_attempts }` variant
- [ ] Remove or repurpose old variants as needed

### Reconnection Logic
- [ ] Implement exponential backoff calculation
- [ ] Implement max retry enforcement
- [ ] Implement backoff reset on success
- [ ] Implement state preservation
- [ ] Handle concurrent reconnection attempts

### Message Processing
- [ ] Implement frame buffering
- [ ] Implement concurrent delivery
- [ ] Ensure message ordering
- [ ] Handle large messages

### Circuit Breaker
- [ ] Implement state transitions
- [ ] Implement failure tracking
- [ ] Implement timeout-based transitions
- [ ] Implement metrics collection

## Success Criteria

- [ ] All 31 RED tests pass
- [ ] Test execution < 2 seconds
- [ ] No compiler warnings
- [ ] All domain tests still pass (2 existing)
- [ ] Code compiles cleanly
- [ ] Ready for REFACTOR phase

## Expected Output

When complete, test output should show:
```
test result: ok. 33 passed; 0 failed
```

(31 new tests + 2 existing domain tests)

## Next Steps

1. Implementer completes all 31 tests passing
2. Reviewer reviews code quality and runs git commit
3. REFACTOR phase improves code without changing behavior
4. Move to Phase 5.2: Exchange Clients Integration Testing

---

**Estimated Time**: 2 hours  
**Current Time Elapsed**: ~5 minutes  
**Time Remaining**: ~115 minutes
