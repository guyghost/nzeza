# Phase 5.1 GREEN - Required Changes Reference

## ReconnectionConfig Changes

**Current (WRONG)**:
```rust
pub struct ReconnectionConfig {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
}
```

**Expected (CORRECT)**:
```rust
pub struct ReconnectionConfig {
    pub base_backoff: Duration,
    pub max_backoff: Duration,
    pub max_retries: u32,
    pub backoff_multiplier: f64,
}
```

## ReconnectionEvent Enum Changes

**Current (INCOMPLETE)**:
```rust
pub enum ReconnectionEvent {
    Started { attempt: u32 },
    Succeeded { attempt: u32, duration: Duration },
    Failed { attempt: u32, error: String },
}
```

**Expected (CORRECT)**:
```rust
pub enum ReconnectionEvent {
    AttemptStarted { attempt_number: u32, delay: Duration },
    Connected { attempt_number: u32, duration: Duration },
    Failed { attempt_number: u32, reason: String },
    MaxRetriesExceeded { total_attempts: u32 },
}
```

## CircuitBreakerConfig Changes

**Current (INCOMPLETE)**:
```rust
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout_duration: Duration,
}
```

**Expected (CORRECT)**:
```rust
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub timeout_duration: Duration,
    pub success_threshold: u32,
    pub max_retry_interval: Duration,
}
```

## CircuitBreakerEvent Enum Changes

**Current (INCOMPLETE)**:
```rust
pub enum CircuitBreakerEvent {
    Opened { reason: String },
    Closed,
    HalfOpened,
}
```

**Expected (CORRECT)**:
```rust
pub enum CircuitBreakerEvent {
    StateChanged { 
        from: CircuitState, 
        to: CircuitState, 
        reason: String 
    },
    FailureRecorded { 
        total_failures: u32, 
        reason: String 
    },
    TimeoutStarted { 
        timeout_duration: Duration 
    },
    TimeoutElapsed { 
        new_state: CircuitState 
    },
}
```

## FailureEvent Enhancement

**Tests expect method**:
```rust
impl FailureEvent {
    pub fn is_within_window(&self, duration: Duration) -> bool {
        self.timestamp.elapsed() < duration
    }
}
```

## Client Methods Needed

### Basic State Methods
- `fn is_circuit_open(&self) -> bool`
- `fn is_circuit_closed(&self) -> bool`
- `fn is_circuit_half_open(&self) -> bool`
- `fn circuit_state(&self) -> CircuitState`
- `fn connection_state(&self) -> ConnectionState`

### Stream Methods (async)
- `async fn circuit_breaker_stream(&self) -> CircuitBreakerStream`
- `async fn reconnection_stream(&self) -> ReconnectionStream`
- `async fn connection_attempt_stream(&self) -> ConnectionAttemptStream`
- `async fn price_stream(&self) -> PriceStream`
- `async fn parsing_error_stream(&self) -> ParsingErrorStream`
- `async fn validation_error_stream(&self) -> ValidationErrorStream`
- `async fn type_error_stream(&self) -> TypeErrorStream`

### Metrics Methods (async)
- `async fn circuit_breaker_metrics(&self) -> CircuitBreakerMetrics`
- `async fn failure_history(&self) -> Vec<FailureEvent>`
- `async fn success_history(&self) -> Vec<SuccessEvent>`
- `async fn timeout_history(&self) -> Vec<TimeoutEvent>`
- `async fn backoff_history(&self) -> Vec<BackoffEvent>`
- `async fn circuit_breaker_event_history(&self) -> Vec<CircuitEvent>`

### Control Methods
- `async fn reset_circuit_breaker(&self)`
- `async fn set_failure_mode(mut self, enabled: bool) -> Self`
- `async fn simulate_disconnection(&self)`

## Key Test Patterns

### Connection Test Pattern
```rust
let client = WebSocketClient::new(url)
    .with_bearer_token("valid_bearer_token_abcdef123456")
    .with_price_parsing(true);
    
let result = client.connect().await;
assert!(result.is_ok());
assert!(client.is_connected());
assert_eq!(client.connection_state(), ConnectionState::Connected);
```

### Reconnection Test Pattern
```rust
let client = WebSocketClient::new(url)
    .with_reconnection_config(ReconnectionConfig {
        base_backoff: Duration::from_millis(100),
        max_backoff: Duration::from_secs(10),
        max_retries: 5,
        backoff_multiplier: 2.0,
    });

let stream = client.reconnection_stream();
let mut receiver = stream.subscribe();

// Expect ReconnectionEvent::AttemptStarted { attempt_number, delay }
// Expect ReconnectionEvent::Connected { attempt_number, duration }
// Expect ReconnectionEvent::Failed { attempt_number, reason }
// Expect ReconnectionEvent::MaxRetriesExceeded { total_attempts }
```

### Price Parsing Test Pattern
```rust
let client = WebSocketClient::new(url)
    .with_bearer_token("valid_bearer_token_abcdef123456")
    .with_price_parsing(true)
    .with_strict_validation(true);

client.connect().await.expect("Connection should succeed");

let price_stream = client.price_stream().await;
let mut price_receiver = price_stream.subscribe();

let message = r#"{"product_id":"BTC-USD","price":"45000.50"}"#;
client.queue_outbound_message(message).await;

let price = price_receiver.recv().await.expect("Should receive price");
assert_eq!(price.product_id, "BTC-USD");
assert_eq!(price.price, 45000.50);
assert_eq!(price.decimal_places, 2);
```

### Circuit Breaker Test Pattern
```rust
let client = WebSocketClient::new(url)
    .with_circuit_breaker(CircuitBreakerConfig {
        failure_threshold: 5,
        timeout_duration: Duration::from_secs(10),
        success_threshold: 3,
        max_retry_interval: Duration::from_secs(60),
    });

let circuit_stream = client.circuit_breaker_stream();
let mut circuit_receiver = circuit_stream.subscribe();

// After 5 failures, expect:
// CircuitBreakerEvent::StateChanged { from: Closed, to: Open, .. }
// CircuitBreakerEvent::FailureRecorded { total_failures: 5, .. }

// After timeout, expect:
// CircuitBreakerEvent::TimeoutStarted { .. }
// CircuitBreakerEvent::TimeoutElapsed { new_state: HalfOpen }

// On success in HalfOpen, expect:
// CircuitBreakerEvent::StateChanged { from: HalfOpen, to: Closed, .. }
```

## Exponential Backoff Formula

For reconnection attempts, calculate delay as:
```
delay = base_backoff * (backoff_multiplier ^ (attempt_number - 1))
delay = min(delay, max_backoff)
```

Example with base=100ms, multiplier=2.0:
- Attempt 1: 100ms * 2^0 = 100ms
- Attempt 2: 100ms * 2^1 = 200ms
- Attempt 3: 100ms * 2^2 = 400ms
- Attempt 4: 100ms * 2^3 = 800ms
- Attempt 5: 100ms * 2^4 = 1600ms
- etc. (capped at max_backoff)

## Bearer Token Validation

The exact token expected by tests: `"valid_bearer_token_abcdef123456"`

When connecting without a token or with an invalid token, the client should:
1. Return an error from `connect()`
2. Set connection state to `ConnectionState::Disconnected`
3. Send failure event through connection_attempt_stream

## Decimal Precision Handling

For price parsing, decimal places should be calculated from the original string representation:
- "45000.5" → 1 decimal place
- "45000.50" → 2 decimal places
- "0.000000001" → 9 decimal places
- "1.23e-5" → should be parsed and decimal places calculated appropriately

Store `original_price_string` to preserve precision information.

## Message Ordering Guarantee

Messages processed through `queue_outbound_message()` should maintain FIFO ordering when sent through price_stream.

## State Preservation Across Reconnects

When reconnecting:
1. Maintain session_id across disconnects
2. Preserve configuration (reconnection config, circuit breaker config, etc.)
3. Keep metrics accumulated (don't reset on reconnect)
4. Preserve original_connect_time

## Concurrent Connection Handling

When multiple clients connect:
- Each must get unique connection_id
- Connection IDs should follow pattern: "conn_{uuid or counter}"
- Each client maintains independent state

---

**This document will guide the implementer phase to completion.**
