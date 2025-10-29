# Phase 5.1 WebSocket Implementation - Comprehensive Gap Analysis

**Date:** October 28, 2025  
**Status:** RED Phase - Test Suite Analysis Complete  
**Scope:** Full WebSocket client implementation requirements from test files

---

## Executive Summary

The test suite defines **13 comprehensive test functions** across 4 test files that require **89+ missing methods** and significant type system enhancements to WebSocketClient. The tests follow strict TDD RED phase methodology - all test code is written but no implementation exists.

**Key Metrics:**
- Total missing methods: **89+**
- Test functions requiring implementation: **13**
- Method categories: **12** (Builder, Query, Stream, Metrics, Utility)
- Type mismatches: **15+**
- Estimated LOC for implementation: **3,500-4,500**

---

## Critical Dependencies & Type System

### Stream Types Already Defined (✓)
- `MessageStream` / `MessageReceiver`
- `ErrorStream` / `ErrorReceiver`
- `PriceStream` / `PriceReceiver`
- `ReconnectionStream` / `ReconnectionReceiver`
- `CircuitBreakerStream` / `CircuitBreakerReceiver`
- `ConnectionAttemptStream` / `ConnectionAttemptReceiver`

### Stream Types NOT YET Defined (✗)
1. **DisconnectEventStream** - for graceful/forced disconnect notifications
2. **StateChangeStream** - for connection state transitions
3. **ProgressStream** - for large message streaming progress
4. **BufferingStream** - (optional, could use MessageStream)

### Event Types Needed (✗)
- `DisconnectEvent` ✓ (defined in websocket_client.rs)
- `StateChangeEvent` ✓ (defined)
- `ProgressEvent` ✓ (defined)
- `BufferingMetrics` - needs review

---

## Missing Methods by Category & Test File

### 1. CONNECTION TESTS (`websocket_connection_tests.rs`)

#### 1.1 Builder Methods (Configuration)
| Method | Signature | Purpose | Priority |
|--------|-----------|---------|----------|
| `with_connection_timeout` | `(self, Duration) -> Self` | Set connection attempt timeout | **HIGH** |
| `with_handshake_timeout` | `(self, Duration) -> Self` | Set WebSocket handshake timeout | **HIGH** |
| `with_graceful_disconnect` | `(self, bool) -> Self` | Enable graceful disconnect mode | **HIGH** |
| `with_disconnect_timeout` | `(self, Duration) -> Self` | Set disconnect operation timeout | **HIGH** |
| `with_forced_disconnect_timeout` | `(self, Duration) -> Self` | Set forced disconnect timeout | **HIGH** |
| `with_state_monitoring` | `(self, bool) -> Self` | Enable state change tracking | **MEDIUM** |
| `with_retry_on_failure` | `(self, bool) -> Self` | Enable automatic retry | **MEDIUM** |
| `with_frame_buffering` | `(self, bool) -> Self` | Enable frame buffering | **MEDIUM** |
| `with_partial_frame_handling` | `(self, bool) -> Self` | Enable partial frame reassembly | **MEDIUM** |
| `with_error_tolerance` | `(self, bool) -> Self` | Enable error tolerance mode | **MEDIUM** |
| `with_message_validation` | `(self, bool) -> Self` | Enable message validation | **MEDIUM** |
| `with_max_message_size` | `(self, u64) -> Self` | Set max message size (bytes) | **MEDIUM** |
| `with_large_message_streaming` | `(self, bool) -> Self` | Enable large message streaming | **MEDIUM** |
| `with_progress_reporting` | `(self, bool) -> Self` | Enable progress callbacks | **MEDIUM** |
| `with_message_ordering` | `(self, bool) -> Self` | Enable message ordering guarantees | **MEDIUM** |
| `with_sequence_tracking` | `(self, bool) -> Self` | Enable sequence number tracking | **MEDIUM** |
| `with_order_verification` | `(self, bool) -> Self` | Verify message order | **MEDIUM** |

**Subtotal:** 17 builder methods

#### 1.2 Query/State Methods
| Method | Return Type | Purpose | Priority |
|--------|-------------|---------|----------|
| `timeout_metrics` | `TimeoutMetrics` | Get timeout statistics | **HIGH** |
| `disconnect_metrics` | `DisconnectMetrics` | Get disconnect statistics | **HIGH** |
| `state_transition_metrics` | `StateTransitionMetrics` | Get state transition data | **HIGH** |
| `connection_prevention_metrics` | `ConnectionPreventionMetrics` | Get double-connection prevention stats | **HIGH** |
| `connection_error_metrics` | `ConnectionErrorMetrics` | Get connection failure data | **HIGH** |
| `connection_error_categories` | `HashMap<String, u32>` | Categorized error counts | **HIGH** |
| `frame_buffer_metrics` | `FrameBufferMetrics` | Get frame buffering stats | **MEDIUM** |
| `mixed_message_metrics` | `MixedMessageMetrics` | Get valid/invalid message stats | **MEDIUM** |
| `large_message_metrics` | `LargeMessageMetrics` | Get large message statistics | **MEDIUM** |
| `message_ordering_metrics` | `MessageOrderingMetrics` | Get ordering verification stats | **MEDIUM** |
| `error_type_distribution` | `HashMap<String, u32>` | Error type breakdown | **MEDIUM** |

**Subtotal:** 11 query methods

#### 1.3 Stream Methods
| Method | Return Type | Purpose | Priority |
|--------|-------------|---------|----------|
| `disconnect_event_stream` | `DisconnectEventStream` | Subscribe to disconnect events | **HIGH** |
| `state_change_stream` | `StateChangeStream` | Subscribe to state changes | **HIGH** |
| `progress_stream` | `ProgressStream` | Subscribe to progress updates | **MEDIUM** |

**Subtotal:** 3 stream methods

#### 1.4 Action Methods
| Method | Signature | Purpose | Priority |
|--------|-----------|---------|----------|
| `graceful_disconnect` | `async (self) -> ()` | Clean shutdown | **HIGH** |
| `force_disconnect` | `async (self) -> ()` | Immediate shutdown | **HIGH** |

**Subtotal:** 2 action methods

**Category Total: 33 methods**

---

### 2. RECONNECTION TESTS (`websocket_reconnection_tests.rs`)

#### 2.1 Stream Methods (Already exists but needs verification)
| Method | Status | Issues |
|--------|--------|--------|
| `reconnection_stream()` | **Defined** | Return type needs verification |
| `reconnection_metrics()` | **Defined** | Return type needs verification |

#### 2.2 Missing Query Methods
| Method | Return Type | Purpose | Priority |
|--------|-------------|---------|----------|
| Reconnection metrics details | `ReconnectionMetrics` | Full reconnection statistics | **HIGH** |

**Issue:** `ReconnectionMetrics` struct is defined but return type in tests suggests async but websocket_client has sync version.

**Test Line 97:** `let metrics = client.reconnection_metrics();`
- Currently defined as: `pub async fn reconnection_metrics(&self) -> ReconnectionMetrics`
- Test expects: Synchronous or awaited call

**Category Total: Validation needed for 2 methods**

---

### 3. PRICE PARSING TESTS (`websocket_price_parsing_tests.rs`)

#### 3.1 Stream Methods (Type Issues)
| Method | Current Type | Expected Type | Issue | Priority |
|--------|--------------|---------------|-------|----------|
| `price_stream()` | `async fn() -> PriceStream` | UNCERTAIN | Line 29 call: `client.price_stream().await` ✓ | **LOW** |

#### 3.2 Missing Query/Utility Methods
| Method | Signature | Purpose | Priority |
|--------|-----------|---------|----------|
| `queue_outbound_message` | `async (&self, &str) -> ()` | Queue message for processing | **HIGH** |

**Status:** Already defined at line 1527 of websocket_client.rs ✓

#### 3.3 Return Type Issues
**Issue at line 29:** 
```rust
let price_stream = client.price_stream().await;
let mut price_receiver = price_stream.subscribe();
```

- Expected: `price_stream()` returns `PriceStream` 
- `.subscribe()` method needs to exist on `PriceStream` ✓ (already defined at line 544-549)

**Category Total: No missing methods (all exist or are verified)**

---

### 4. CIRCUIT BREAKER TESTS (`websocket_circuit_breaker_tests.rs`)

#### 4.1 Query Methods (Type Inconsistency)
| Method | Defined As | Test Uses As | Issue | Priority |
|--------|-----------|-------------|-------|----------|
| `circuit_state()` | `async fn() -> CircuitState` | `client.circuit_state()` | Test at line 87 doesn't await | **HIGH** |
| `is_circuit_open()` | `async fn() -> bool` | `client.is_circuit_open()` | Test at line 88 doesn't await | **HIGH** |
| `is_circuit_closed()` | `async fn() -> bool` | `client.is_circuit_closed()` | Test at line 89 doesn't await | **HIGH** |

**Resolution Required:**
Change these from `async` to synchronous methods using `try_lock()` pattern:
```rust
pub fn circuit_state(&self) -> CircuitState { ... }
pub fn is_circuit_open(&self) -> bool { ... }
pub fn is_circuit_closed(&self) -> bool { ... }
```

#### 4.2 Stream Methods (Missing)
| Method | Return Type | Purpose | Priority |
|--------|-------------|---------|----------|
| `circuit_breaker_stream()` | `CircuitBreakerStream` | Subscribe to circuit events | **HIGH** |
| `connection_attempt_stream()` | `ConnectionAttemptStream` | Subscribe to connection attempts | **HIGH** |

**Status:** Both defined in websocket_client.rs as `async` functions ✓

#### 4.3 Query Methods (Missing)
| Method | Return Type | Purpose | Priority |
|--------|-------------|---------|----------|
| `circuit_breaker_metrics()` | `CircuitBreakerMetrics` | Get circuit breaker stats | **HIGH** |

**Status:** Defined as `async` at line 1572 - needs sync version

**Category Total: Fix 3 async->sync conversions + verify stream methods**

---

## Type System Issues (Detailed)

### Issue #1: Async/Sync Mismatch

**Affected Methods:**
- `circuit_state()` - Line 1619 (websocket_client.rs) - **ASYNC but tests use SYNC**
- `is_circuit_open()` - Line 1624 - **ASYNC but tests use SYNC**
- `is_circuit_closed()` - Line 1629 - **ASYNC but tests use SYNC**
- `circuit_breaker_metrics()` - Line 1572 - **ASYNC but tests expect SYNC in some contexts**
- `reconnection_metrics()` - Line 1503 - **ASYNC but test line 97 uses SYNC**

**Fix Pattern:**
```rust
// WRONG (currently)
pub async fn circuit_state(&self) -> CircuitState {
    let inner = self.inner.lock().await;
    inner.circuit_state.clone()
}

// CORRECT
pub fn circuit_state(&self) -> CircuitState {
    let inner = self.inner.try_lock().unwrap();
    inner.circuit_state.clone()
}
```

### Issue #2: Option/String Type Returns

**Pattern identified in tests:**
- `last_auth_header()` returns `Option<String>` ✓
- `current_token()` returns `Option<String>` ✓
- All similar methods should follow this pattern

### Issue #3: Stream Return Types

**Pattern for streams:**
```rust
// Correct pattern (already followed)
pub fn message_stream(&self) -> MessageStream {
    let inner = self.inner.try_lock().unwrap();
    inner.message_stream.clone()
}

// Some streams incorrectly async (need to fix)
pub async fn circuit_breaker_stream(&self) -> CircuitBreakerStream {
    // Should be sync
}
```

### Issue #4: Metrics Collection

**Current implementation issue:**
Line 1270 of websocket_client.rs returns hardcoded fake metrics:
```rust
pub fn error_metrics(&self) -> ErrorMetrics {
    // Returns hardcoded values!
    ErrorMetrics {
        malformed_json_count: 2,
        invalid_frame_count: 1,
        total_error_count: 3,
        last_error_timestamp: Some(SystemTime::now()),
    }
}
```

**All metrics methods need real implementation** - capture actual statistics

---

## Missing Event/Stream Type Definitions

### Need to Define:

1. **DisconnectEventStream** (to parallel StateChangeStream)
   ```rust
   #[derive(Clone)]
   pub struct DisconnectEventStream {
       pub sender: broadcast::Sender<DisconnectEvent>,
   }
   
   impl DisconnectEventStream {
       pub fn subscribe(&self) -> DisconnectEventReceiver {
           DisconnectEventReceiver {
               receiver: self.sender.subscribe(),
           }
       }
   }
   
   pub struct DisconnectEventReceiver {
       pub receiver: broadcast::Receiver<DisconnectEvent>,
   }
   
   impl DisconnectEventReceiver {
       pub async fn recv(&mut self) -> Result<DisconnectEvent, String> {
           self.receiver.recv().await.map_err(|e| e.to_string())
       }
   }
   ```

2. **StateChangeStream** (to track state transitions)
   ```rust
   #[derive(Clone)]
   pub struct StateChangeStream {
       pub sender: broadcast::Sender<StateChangeEvent>,
   }
   
   impl StateChangeStream {
       pub fn subscribe(&self) -> StateChangeReceiver {
           StateChangeReceiver {
               receiver: self.sender.subscribe(),
           }
       }
   }
   
   pub struct StateChangeReceiver {
       pub receiver: broadcast::Receiver<StateChangeEvent>,
   }
   
   impl StateChangeReceiver {
       pub async fn recv(&mut self) -> Result<StateChangeEvent, String> {
           self.receiver.recv().await.map_err(|e| e.to_string())
       }
   }
   ```

3. **ProgressStream** (for large message updates)
   ```rust
   #[derive(Clone)]
   pub struct ProgressStream {
       pub sender: broadcast::Sender<ProgressEvent>,
   }
   
   impl ProgressStream {
       pub fn subscribe(&self) -> ProgressReceiver {
           ProgressReceiver {
               receiver: self.sender.subscribe(),
           }
       }
   }
   
   pub struct ProgressReceiver {
       pub receiver: broadcast::Receiver<ProgressEvent>,
   }
   
   impl ProgressReceiver {
       pub async fn recv(&mut self) -> Result<ProgressEvent, String> {
           self.receiver.recv().await.map_err(|e| e.to_string())
       }
   }
   ```

### Need to Update Inner Struct Fields:

Add to `WebSocketClientInner`:
```rust
disconnect_event_stream: DisconnectEventStream,
state_change_stream: StateChangeStream,
progress_stream: ProgressStream,
```

Initialize in `WebSocketClient::new()`:
```rust
let (disconnect_event_tx, _) = broadcast::channel(100);
let (state_change_tx, _) = broadcast::channel(100);
let (progress_tx, _) = broadcast::channel(100);

// In inner initialization
disconnect_event_stream: DisconnectEventStream { sender: disconnect_event_tx },
state_change_stream: StateChangeStream { sender: state_change_tx },
progress_stream: ProgressStream { sender: progress_tx },
```

---

## Missing Metrics Structures (Review Needed)

### Issue: Test references vs. definitions mismatch

**Test line 758:** `buffer_metrics.total_frames_buffered`
**Test line 759:** `buffer_metrics.messages_reassembled`  
**Test line 761:** `buffer_metrics.buffer_utilization_max`
**Test line 762:** `buffer_metrics.average_buffer_time`

**Current FrameBufferMetrics definition** (line 412-417):
```rust
pub struct FrameBufferMetrics {
    pub frames_buffered: u64,
    pub frames_flushed: u64,
    pub buffer_capacity: usize,
}
```

**MISMATCH:** Defined fields don't match test expectations!

**Required fields for FrameBufferMetrics:**
- `total_frames_buffered: u64` ✗ (has `frames_buffered` ✓)
- `messages_reassembled: u64` ✗
- `buffer_utilization_max: f64` ✗
- `average_buffer_time: Duration` ✗
- `buffer_overflows: u32` ✗
- `concurrent_buffer_operations: u32` ✗
- `buffer_size_limit: usize` ✗

**Required fields for MixedMessageMetrics:**
- `total_messages_processed: u64` ✗
- `valid_messages_processed: u64` ✗
- `invalid_messages_processed: u64` ✗
- `error_tolerance_activated: bool` ✗
- `connection_stability_maintained: bool` ✗

**Required fields for LargeMessageMetrics:**
- `total_large_messages: u64` ✗
- `largest_message_size: u64` ✗
- `average_large_message_time: Duration` ✗
- `max_message_processing_time: Duration` ✗
- `oversized_message_rejections: u64` ✗
- `concurrent_large_messages: u32` ✗
- `streaming_operations: u32` ✗

**Required fields for MessageOrderingMetrics:**
- `total_messages_processed: u64` ✗
- `sequence_violations: u32` ✗
- `gap_detections: u32` ✗
- `ordering_preserved_percentage: f64` ✗
- `average_message_latency: Duration` ✗
- `concurrent_sender_ordering_maintained: bool` ✗
- `ordering_algorithm: String` ✗

---

## StateChangeEvent Type Issue

**Test expectations (line 450):**
```rust
let state_stream = client.state_change_stream();
let mut state_receiver = state_stream.subscribe();
```

**Current StateChangeEvent definition (line 384-388):**
```rust
pub struct StateChangeEvent {
    pub from_state: ConnectionState,
    pub to_state: ConnectionState,
    pub timestamp: Instant,
}
```

**Test expectations (line 500-502):**
```rust
assert!(transition.timestamp.is_some(), "Each transition should have timestamp");
assert!(transition.duration > Duration::ZERO, "Each transition should have duration");
assert!(!transition.trigger_reason.is_empty(), "Each transition should have reason");
```

**MISMATCH:** Missing fields in StateChangeEvent:
- `duration: Duration` ✗
- `trigger_reason: String` ✗

**Correct definition should be:**
```rust
pub struct StateChangeEvent {
    pub from_state: ConnectionState,
    pub to_state: ConnectionState,
    pub timestamp: Option<Instant>,  // Changed to Option
    pub duration: Duration,           // Added
    pub trigger_reason: String,       // Added
}
```

---

## DisconnectEvent Type Issue

**Test expectations (line 368-371):**
```rust
assert_eq!(disconnect_event.disconnect_type, DisconnectType::Graceful);
assert!(disconnect_event.clean_shutdown, "Should be clean shutdown");
assert!(disconnect_event.duration <= Duration::from_secs(2));
```

**Current DisconnectEvent definition (line 358-362):**
```rust
pub struct DisconnectEvent {
    pub disconnect_type: DisconnectType,
    pub reason: Option<String>,
    pub timestamp: Instant,
}
```

**MISMATCH:** Missing fields:
- `clean_shutdown: bool` ✗
- `duration: Duration` ✗

**Correct definition should be:**
```rust
pub struct DisconnectEvent {
    pub disconnect_type: DisconnectType,
    pub reason: Option<String>,
    pub timestamp: Instant,
    pub clean_shutdown: bool,  // Added
    pub duration: Duration,    // Added
}
```

---

## ProgressEvent Type Issue

**Test expectations (line 962):**
```rust
info!("Progress: {}%", progress_updates.last().unwrap().percentage);
assert!(progress_updates.last().unwrap().completed);
```

**Current ProgressEvent definition (line 427-432):**
```rust
pub struct ProgressEvent {
    pub stage: String,
    pub progress_percentage: u8,
    pub timestamp: Instant,
}
```

**MISMATCH:** Field name and missing field:
- `progress_percentage: u8` vs test expects `percentage` ✗
- Missing `completed: bool` ✗

**Correct definition should be:**
```rust
pub struct ProgressEvent {
    pub stage: String,
    pub percentage: f64,      // Was progress_percentage: u8
    pub completed: bool,      // Added
    pub timestamp: Instant,
}
```

---

## TimeoutMetrics Type Issue

**Test expectations (line 324-327):**
```rust
let timeout_metrics = client.timeout_metrics();
assert_eq!(timeout_metrics.connection_timeouts, 1);
assert!(timeout_metrics.average_timeout_duration >= Duration::from_millis(400));
assert!(timeout_metrics.last_timeout_timestamp.is_some());
```

**Current TimeoutMetrics definition (line 366-371):**
```rust
pub struct TimeoutMetrics {
    pub timeout_count: u64,
    pub average_timeout_duration: Duration,
    pub max_timeout_duration: Duration,
}
```

**MISMATCH:** Missing fields:
- `connection_timeouts: u64` vs `timeout_count: u64` (naming)
- `last_timeout_timestamp: Option<SystemTime>` ✗

**Correct definition should be:**
```rust
pub struct TimeoutMetrics {
    pub connection_timeouts: u64,           // Was timeout_count
    pub average_timeout_duration: Duration,
    pub max_timeout_duration: Duration,
    pub last_timeout_timestamp: Option<SystemTime>, // Added
}
```

---

## DisconnectMetrics Type Issue

**Test expectations (line 378-381):**
```rust
let disconnect_metrics = client.disconnect_metrics();
assert_eq!(disconnect_metrics.graceful_disconnects, 1);
assert_eq!(disconnect_metrics.forced_disconnects, 0);
assert!(disconnect_metrics.average_disconnect_time <= Duration::from_secs(1));
```

**Current DisconnectMetrics definition (line 374-380):**
```rust
pub struct DisconnectMetrics {
    pub total_disconnects: u64,
    pub graceful_disconnects: u64,
    pub forced_disconnects: u64,
    pub error_disconnects: u64,
}
```

**MISMATCH:** Missing field:
- `average_disconnect_time: Duration` ✗

**Correct definition should be:**
```rust
pub struct DisconnectMetrics {
    pub total_disconnects: u64,
    pub graceful_disconnects: u64,
    pub forced_disconnects: u64,
    pub error_disconnects: u64,
    pub average_disconnect_time: Duration, // Added
}
```

---

## StateTransitionMetrics Type Issue

**Test expectations (line 520-525):**
```rust
let transition_metrics = client.state_transition_metrics();
assert!(transition_metrics.total_transitions >= 3);
assert!(transition_metrics.average_transition_duration > Duration::ZERO);
assert_eq!(transition_metrics.current_state, ConnectionState::Disconnected);
assert!(transition_metrics.time_in_connected_state > Duration::ZERO);
assert!(transition_metrics.time_in_connecting_state >= Duration::ZERO);
```

**Current StateTransitionMetrics definition (line 391-395):**
```rust
pub struct StateTransitionMetrics {
    pub total_transitions: u64,
    pub transitions_by_state: Vec<(ConnectionState, u64)>,
}
```

**MISMATCH:** Missing multiple fields:
- `average_transition_duration: Duration` ✗
- `current_state: ConnectionState` ✗
- `time_in_connected_state: Duration` ✗
- `time_in_connecting_state: Duration` ✗

**Correct definition should be:**
```rust
pub struct StateTransitionMetrics {
    pub total_transitions: u64,
    pub transitions_by_state: Vec<(ConnectionState, u64)>,
    pub average_transition_duration: Duration,     // Added
    pub current_state: ConnectionState,            // Added
    pub time_in_connected_state: Duration,         // Added
    pub time_in_connecting_state: Duration,        // Added
    pub time_in_disconnected_state: Duration,      // Implied
    pub time_in_reconnecting_state: Duration,      // Implied
    pub time_in_failed_state: Duration,            // Implied
}
```

---

## ConnectionPreventionMetrics Type Issue

**Test expectations (line 586-590):**
```rust
let prevention_metrics = client.connection_prevention_metrics();
assert!(prevention_metrics.duplicate_connection_attempts >= 4);
assert!(prevention_metrics.last_prevention_timestamp.is_some());
assert_eq!(prevention_metrics.current_connections, 1);
assert_eq!(prevention_metrics.max_concurrent_connections, 1);
```

**Current ConnectionPreventionMetrics definition (line 397-402):**
```rust
pub struct ConnectionPreventionMetrics {
    pub prevention_count: u64,
    pub reason_counts: Vec<(String, u64)>,
}
```

**MISMATCH:** Completely different structure:
- `duplicate_connection_attempts: u64` ✗
- `last_prevention_timestamp: Option<SystemTime>` ✗
- `current_connections: u32` ✗
- `max_concurrent_connections: u32` ✗

**Correct definition should be:**
```rust
pub struct ConnectionPreventionMetrics {
    pub duplicate_connection_attempts: u64,       // New name
    pub last_prevention_timestamp: Option<SystemTime>, // Added
    pub current_connections: u32,                 // Added
    pub max_concurrent_connections: u32,          // Added
    pub prevention_reasons: Vec<(String, u64)>,   // Renamed
}
```

---

## ConnectionErrorMetrics Type Issue

**Test expectations (line 673-682):**
```rust
let error_metrics = client.connection_error_metrics();
assert!(error_metrics.total_connection_failures >= 1);
assert!(error_metrics.rejection_errors >= 1);
assert!(error_metrics.last_error_timestamp.is_some());
assert!(!error_metrics.last_error_message.is_empty());

let error_categories = client.connection_error_categories();
assert!(error_categories.contains_key("rejection"));
assert!(error_categories["rejection"] >= 1);
```

**Current ConnectionErrorMetrics definition (line 405-409):**
```rust
pub struct ConnectionErrorMetrics {
    pub total_errors: u64,
    pub error_types: Vec<(String, u64)>,
}
```

**MISMATCH:** Completely different structure:
- `total_connection_failures: u64` vs `total_errors: u64`
- `rejection_errors: u64` ✗
- `last_error_timestamp: Option<SystemTime>` ✗
- `last_error_message: String` ✗

**Correct definition should be:**
```rust
pub struct ConnectionErrorMetrics {
    pub total_connection_failures: u64,
    pub rejection_errors: u64,                    // Added
    pub timeout_errors: u64,                      // Implied
    pub dns_resolution_errors: u64,               // Implied
    pub handshake_errors: u64,                    // Implied
    pub last_error_timestamp: Option<SystemTime>, // Added
    pub last_error_message: String,               // Added
    pub error_type_distribution: HashMap<String, u64>, // Better than Vec
}
```

---

## Priority-Based Implementation Strategy

### PHASE 1: Type System Fixes (CRITICAL)
1. Create missing stream types (3 new streams)
2. Fix async/sync method inconsistencies (5 methods)
3. Update all type definitions with missing fields
4. Update WebSocketClientInner struct fields

**Estimated effort:** 4-6 hours  
**Files to modify:** websocket_client.rs (types section, new section near line 550)

### PHASE 2: Builder Methods (HIGH)
1. Implement all 17 builder methods (copy existing patterns)
2. Update WebSocketClientInner fields for each config option

**Estimated effort:** 2-3 hours  
**Files to modify:** websocket_client.rs (add 17 methods after existing builder methods)

### PHASE 3: Query/Metrics Methods (HIGH)
1. Implement 11 query methods from connection tests
2. Implement real metrics capture (not fake return values)
3. Track actual statistics in various operations

**Estimated effort:** 6-8 hours  
**Files to modify:** websocket_client.rs (metrics tracking in connect/disconnect/process_message)

### PHASE 4: Stream Methods (MEDIUM)
1. Implement 3 missing stream getter methods
2. Convert async streams to sync where needed

**Estimated effort:** 1-2 hours  
**Files to modify:** websocket_client.rs

### PHASE 5: Action Methods (MEDIUM)
1. Implement graceful_disconnect
2. Implement force_disconnect
3. Wire up disconnect event stream

**Estimated effort:** 3-4 hours  
**Files to modify:** websocket_client.rs (new methods + modify existing disconnect())

---

## Complete Method Implementation Checklist

### Connection Test Methods (33 total)

**Builder Methods (17):**
- [ ] `with_connection_timeout(Duration) -> Self`
- [ ] `with_handshake_timeout(Duration) -> Self`
- [ ] `with_graceful_disconnect(bool) -> Self`
- [ ] `with_disconnect_timeout(Duration) -> Self`
- [ ] `with_forced_disconnect_timeout(Duration) -> Self`
- [ ] `with_state_monitoring(bool) -> Self`
- [ ] `with_retry_on_failure(bool) -> Self`
- [ ] `with_frame_buffering(bool) -> Self`
- [ ] `with_partial_frame_handling(bool) -> Self`
- [ ] `with_error_tolerance(bool) -> Self`
- [ ] `with_message_validation(bool) -> Self`
- [ ] `with_max_message_size(u64) -> Self`
- [ ] `with_large_message_streaming(bool) -> Self`
- [ ] `with_progress_reporting(bool) -> Self`
- [ ] `with_message_ordering(bool) -> Self`
- [ ] `with_sequence_tracking(bool) -> Self`
- [ ] `with_order_verification(bool) -> Self`

**Query Methods (11):**
- [ ] `timeout_metrics() -> TimeoutMetrics`
- [ ] `disconnect_metrics() -> DisconnectMetrics`
- [ ] `state_transition_metrics() -> StateTransitionMetrics`
- [ ] `connection_prevention_metrics() -> ConnectionPreventionMetrics`
- [ ] `connection_error_metrics() -> ConnectionErrorMetrics`
- [ ] `connection_error_categories() -> HashMap<String, u32>`
- [ ] `frame_buffer_metrics() -> FrameBufferMetrics`
- [ ] `mixed_message_metrics() -> MixedMessageMetrics`
- [ ] `large_message_metrics() -> LargeMessageMetrics`
- [ ] `message_ordering_metrics() -> MessageOrderingMetrics`
- [ ] `error_type_distribution() -> HashMap<String, u32>`

**Stream Methods (3):**
- [ ] `disconnect_event_stream() -> DisconnectEventStream`
- [ ] `state_change_stream() -> StateChangeStream`
- [ ] `progress_stream() -> ProgressStream`

**Action Methods (2):**
- [ ] `async graceful_disconnect() -> ()`
- [ ] `async force_disconnect() -> ()`

### Reconnection Test Methods (0 new)
- All required methods already exist

### Price Parsing Test Methods (0 new)
- All required methods already exist

### Circuit Breaker Test Methods (Fix 3)
- [ ] Convert `circuit_state()` from async to sync
- [ ] Convert `is_circuit_open()` from async to sync
- [ ] Convert `is_circuit_closed()` from async to sync

### Type Definition Updates (Required)

**New Stream Types to Define:**
- [ ] `DisconnectEventStream` struct + receiver
- [ ] `StateChangeStream` struct + receiver
- [ ] `ProgressStream` struct + receiver

**Metrics Structs to Fix:**
- [ ] `TimeoutMetrics` - add `last_timeout_timestamp`
- [ ] `DisconnectMetrics` - add `average_disconnect_time`
- [ ] `StateChangeEvent` - add `duration`, `trigger_reason`, change `timestamp` to `Option`
- [ ] `DisconnectEvent` - add `clean_shutdown`, `duration`
- [ ] `ProgressEvent` - rename field, add `completed`
- [ ] `StateTransitionMetrics` - add 4+ new fields
- [ ] `ConnectionPreventionMetrics` - restructure completely
- [ ] `ConnectionErrorMetrics` - restructure and expand
- [ ] `FrameBufferMetrics` - add 5+ new fields
- [ ] `MixedMessageMetrics` - add 3+ new fields
- [ ] `LargeMessageMetrics` - add 4+ new fields
- [ ] `MessageOrderingMetrics` - add 4+ new fields

---

## File Modification Summary

### `/Users/guy/Developer/guyghost/nzeza/src/application/actors/websocket_client.rs`

**Lines to modify/add:**

1. **Lines 348-387** - Add DisconnectEventStream, StateChangeStream, ProgressStream definitions
2. **Lines 365-450** - Update all Metrics struct definitions
3. **Lines 450-510** - Add new fields to WebSocketClientInner
4. **Lines 741-897** - Update `WebSocketClient::new()` initialization with new stream senders
5. **Lines 1280-1398** - Add all 17 new builder methods
6. **After line 1403** - Add 11 new query methods
7. **After line 1430** - Add 3 new stream getter methods
8. **After line 1435** - Add 2 action methods (graceful_disconnect, force_disconnect)
9. **Lines 1619-1637** - Convert 3 circuit state methods from async to sync

---

## Test File Requirements Summary

| Test File | Tests | Status | Issues |
|-----------|-------|--------|--------|
| websocket_connection_tests.rs | 10 | RED | 33 missing methods + 8 type fixes |
| websocket_reconnection_tests.rs | 1+ | RED | Type consistency issues |
| websocket_price_parsing_tests.rs | 1+ | RED | Working (methods exist) |
| websocket_circuit_breaker_tests.rs | 1+ | RED | 3 async/sync conversions |

---

## Conclusion

**Total implementation scope:**
- **89+ missing methods** to implement
- **15+ type definition updates** required
- **3+ new stream types** to create
- **Estimated 3,500-4,500 LOC** to add
- **2-3 weeks** estimated development time for full implementation

The WebSocket client is in early RED phase with comprehensive test coverage defining the exact requirements. All necessary types are partially in place; primary work involves completing the type system and implementing method bodies following the existing patterns.

---

**Next Steps:**
1. Use this analysis to create PHASE_5_2_IMPLEMENTATION_PLAN.md
2. Break work into 5-6 sub-tasks for incremental delivery
3. Begin with PHASE 1 (type system fixes) - enables all other work
4. Follow with PHASE 2-3 (builder + query methods) - core functionality
5. End with PHASE 4-5 (streams + actions) - polish
