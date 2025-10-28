# Phase 5.2: WebSocket Client Implementation Plan - PHASE 1 Type System Fixes

**Date:** October 28, 2025  
**Status:** Starting PHASE 1 - Type System Fixes  
**Based on:** PHASE_5_1_WEBSOCKET_ANALYSIS.md  

---

## Overview

This document outlines the step-by-step implementation plan to complete the WebSocket client for Phase 5.1. Work will be done in 5 phases following TDD RED → GREEN → REFACTOR cycle.

**All phases follow Trunk-Based Development with frequent commits.**

---

## PHASE 1: Type System Fixes (CRITICAL) ⏱️ 4-6 hours

### Objectives
1. Define 3 missing stream types
2. Fix 5 async/sync method inconsistencies
3. Update 12 metrics struct definitions with missing fields
4. Add new fields to WebSocketClientInner
5. Update initialization code

### Tasks

#### Task 1.1: Define DisconnectEventStream, StateChangeStream, ProgressStream
**File:** `src/application/actors/websocket_client.rs`  
**Lines:** Insert after line 348 (after existing stream types)  
**Effort:** 30 minutes

- [ ] Create `DisconnectEventStream` struct with broadcast sender
- [ ] Create `DisconnectEventReceiver` with async recv()
- [ ] Create `StateChangeStream` struct with broadcast sender
- [ ] Create `StateChangeReceiver` with async recv()
- [ ] Create `ProgressStream` struct with broadcast sender
- [ ] Create `ProgressReceiver` with async recv()

#### Task 1.2: Update Event Type Definitions
**File:** `src/application/actors/websocket_client.rs`  
**Lines:** 359-432 (existing definitions)  
**Effort:** 30 minutes

**DisconnectEvent updates (line 359-363):**
- [ ] Add `clean_shutdown: bool` field
- [ ] Add `duration: Duration` field

**StateChangeEvent updates (line 384-388):**
- [ ] Change `timestamp: Instant` to `timestamp: Option<Instant>`
- [ ] Add `duration: Duration` field
- [ ] Add `trigger_reason: String` field

**ProgressEvent updates (line 428-432):**
- [ ] Rename `progress_percentage: u8` to `percentage: f64`
- [ ] Add `completed: bool` field

#### Task 1.3: Update Metrics Struct Definitions
**File:** `src/application/actors/websocket_client.rs`  
**Lines:** 365-448 (metrics structs)  
**Effort:** 1 hour

Updates required:

1. **TimeoutMetrics (line 365-371):**
   - [ ] Rename `timeout_count` to `connection_timeouts`
   - [ ] Add `last_timeout_timestamp: Option<SystemTime>`

2. **DisconnectMetrics (line 374-380):**
   - [ ] Add `average_disconnect_time: Duration`

3. **StateTransitionMetrics (line 391-395):**
   - [ ] Add `average_transition_duration: Duration`
   - [ ] Add `current_state: ConnectionState`
   - [ ] Add `time_in_connected_state: Duration`
   - [ ] Add `time_in_connecting_state: Duration`
   - [ ] Add `time_in_disconnected_state: Duration`
   - [ ] Add `time_in_reconnecting_state: Duration`
   - [ ] Add `time_in_failed_state: Duration`

4. **ConnectionPreventionMetrics (line 398-402):**
   - [ ] Replace `prevention_count: u64` with `duplicate_connection_attempts: u64`
   - [ ] Replace `reason_counts` with `prevention_reasons: Vec<(String, u64)>`
   - [ ] Add `last_prevention_timestamp: Option<SystemTime>`
   - [ ] Add `current_connections: u32`
   - [ ] Add `max_concurrent_connections: u32`

5. **ConnectionErrorMetrics (line 405-409):**
   - [ ] Replace `total_errors: u64` with `total_connection_failures: u64`
   - [ ] Add `rejection_errors: u64`
   - [ ] Add `timeout_errors: u64`
   - [ ] Add `dns_resolution_errors: u64`
   - [ ] Add `handshake_errors: u64`
   - [ ] Add `last_error_timestamp: Option<SystemTime>`
   - [ ] Add `last_error_message: String`
   - [ ] Change `error_types: Vec<(String, u64)>` to `error_type_distribution: HashMap<String, u64>`

6. **FrameBufferMetrics (line 412-417):**
   - [ ] Rename `frames_buffered` to `total_frames_buffered`
   - [ ] Add `messages_reassembled: u64`
   - [ ] Add `buffer_utilization_max: f64`
   - [ ] Add `average_buffer_time: Duration`
   - [ ] Add `buffer_overflows: u32`
   - [ ] Add `concurrent_buffer_operations: u32`
   - [ ] Add `buffer_size_limit: usize`

7. **MixedMessageMetrics (line 420-424):**
   - [ ] Add `total_messages_processed: u64`
   - [ ] Add `valid_messages_processed: u64`
   - [ ] Add `invalid_messages_processed: u64`
   - [ ] Add `error_tolerance_activated: bool`
   - [ ] Add `connection_stability_maintained: bool`

8. **LargeMessageMetrics (line 435-440):**
   - [ ] Rename `large_message_count` to `total_large_messages`
   - [ ] Add `largest_message_size: u64`
   - [ ] Add `average_large_message_time: Duration`
   - [ ] Add `max_message_processing_time: Duration`
   - [ ] Add `oversized_message_rejections: u64`
   - [ ] Add `concurrent_large_messages: u32`
   - [ ] Add `streaming_operations: u32`

9. **MessageOrderingMetrics (line 443-448):**
   - [ ] Rename `total_messages` to `total_messages_processed`
   - [ ] Add `sequence_violations: u32`
   - [ ] Add `gap_detections: u32`
   - [ ] Add `ordering_preserved_percentage: f64`
   - [ ] Add `average_message_latency: Duration`
   - [ ] Add `concurrent_sender_ordering_maintained: bool`
   - [ ] Add `ordering_algorithm: String`

#### Task 1.4: Update WebSocketClientInner Struct
**File:** `src/application/actors/websocket_client.rs`  
**Lines:** 450-510 (inner struct definition)  
**Effort:** 30 minutes

- [ ] Add `disconnect_event_stream: DisconnectEventStream` field
- [ ] Add `state_change_stream: StateChangeStream` field
- [ ] Add `progress_stream: ProgressStream` field
- [ ] Add builder config fields:
  - `connection_timeout: Duration`
  - `handshake_timeout: Duration`
  - `graceful_disconnect_enabled: bool`
  - `disconnect_timeout: Duration`
  - `forced_disconnect_timeout: Duration`
  - `state_monitoring_enabled: bool`
  - `retry_on_failure_enabled: bool`
  - `frame_buffering_enabled: bool`
  - `partial_frame_handling_enabled: bool`
  - `error_tolerance_enabled: bool`
  - `message_validation_enabled: bool`
  - `max_message_size: u64`
  - `large_message_streaming_enabled: bool`
  - `progress_reporting_enabled: bool`
  - `message_ordering_enabled: bool`
  - `sequence_tracking_enabled: bool`
  - `order_verification_enabled: bool`

#### Task 1.5: Update WebSocketClient::new() Initialization
**File:** `src/application/actors/websocket_client.rs`  
**Lines:** 741-897 (new() method)  
**Effort:** 30 minutes

- [ ] Initialize new stream senders:
  - `let (disconnect_event_tx, _) = broadcast::channel(100);`
  - `let (state_change_tx, _) = broadcast::channel(100);`
  - `let (progress_tx, _) = broadcast::channel(100);`
- [ ] Add all new fields to WebSocketClientInner initialization
- [ ] Set default values for all new config fields

#### Task 1.6: Fix Async/Sync Inconsistencies
**File:** `src/application/actors/websocket_client.rs`  
**Lines:** 1619-1637 (circuit state methods)  
**Effort:** 30 minutes

Convert from `async` to synchronous:
- [ ] `circuit_state()` - change from `async fn` to `fn`, use `.try_lock().unwrap()`
- [ ] `is_circuit_open()` - change from `async fn` to `fn`, use `.try_lock().unwrap()`
- [ ] `is_circuit_closed()` - change from `async fn` to `fn`, use `.try_lock().unwrap()`

Also check and fix if needed:
- [ ] `reconnection_metrics()` - verify sync/async consistency
- [ ] `circuit_breaker_metrics()` - verify sync/async consistency

### Validation Checklist
- [ ] Code compiles: `cargo build`
- [ ] No clippy warnings: `cargo clippy`
- [ ] Formatting correct: `cargo fmt --check`
- [ ] All tests compile: `cargo test --no-run`

### Commit Strategy
After completing each task, commit with message format:
```
feat(websocket): [PHASE 1 - Task 1.X] <description>

- Add DisconnectEventStream type definition
- Add StateChangeStream type definition
- Add ProgressStream type definition
```

---

## PHASE 2: Builder Methods (HIGH) ⏱️ 2-3 hours
*Scheduled after PHASE 1 completion*

---

## PHASE 3: Query/Metrics Methods (HIGH) ⏱️ 6-8 hours
*Scheduled after PHASE 2 completion*

---

## PHASE 4: Stream Methods (MEDIUM) ⏱️ 1-2 hours
*Scheduled after PHASE 3 completion*

---

## PHASE 5: Action Methods (MEDIUM) ⏱️ 3-4 hours
*Scheduled after PHASE 4 completion*

---

## Overall Progress Tracking

| Phase | Status | ETA | Tasks | Commits |
|-------|--------|-----|-------|---------|
| 1 - Type System | IN_PROGRESS | 4-6h | 6/6 | 0/6 |
| 2 - Builder Methods | PENDING | 2-3h | 0/17 | 0/1 |
| 3 - Query Methods | PENDING | 6-8h | 0/11 | 0/1 |
| 4 - Stream Methods | PENDING | 1-2h | 0/3 | 0/1 |
| 5 - Action Methods | PENDING | 3-4h | 0/2 | 0/1 |

**Total Effort:** 16-23 hours  
**Total LOC Added:** 3,500-4,500  
**Remaining Methods:** 89+

---

## Key Implementation Patterns

### Stream Pattern (Already Used)
```rust
#[derive(Clone)]
pub struct XxxStream {
    pub sender: broadcast::Sender<XxxEvent>,
}

impl XxxStream {
    pub fn subscribe(&self) -> XxxReceiver {
        XxxReceiver {
            receiver: self.sender.subscribe(),
        }
    }
}

pub struct XxxReceiver {
    pub receiver: broadcast::Receiver<XxxEvent>,
}

impl XxxReceiver {
    pub async fn recv(&mut self) -> Result<XxxEvent, String> {
        self.receiver.recv().await.map_err(|e| e.to_string())
    }
}
```

### Builder Pattern (Already Used)
```rust
pub fn with_xxx(mut self, value: Type) -> Self {
    self.inner.xxx = value;
    self
}
```

### Metrics Query Pattern (Must Replace Hardcoded Values)
```rust
pub fn xxx_metrics(&self) -> XxxMetrics {
    let inner = self.inner.try_lock().unwrap();
    XxxMetrics {
        field1: inner.field1_value,
        field2: inner.field2_value,
        // etc.
    }
}
```

---

## Notes

1. **Import Management:** Ensure `HashMap` is imported from `std::collections`
2. **Backward Compatibility:** Some field renames may break existing code - check for usage
3. **Metrics Collection:** Currently returns hardcoded values - will be replaced in PHASE 3
4. **Testing:** All tests are in RED phase and expect these exact types
5. **Trunk-Based Development:** Frequent commits to main after each task with passing tests

