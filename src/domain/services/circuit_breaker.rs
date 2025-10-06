use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Circuit breaker states following the standard pattern
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    /// Circuit is closed, requests flow normally
    Closed,
    /// Circuit is open, all requests fail immediately
    Open,
    /// Circuit is half-open, testing if service recovered
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening circuit
    pub failure_threshold: u32,
    /// Number of successes needed to close circuit from half-open
    pub success_threshold: u32,
    /// Duration to wait before moving from open to half-open
    pub timeout: Duration,
    /// Window duration for counting failures
    pub window_duration: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(60),
            window_duration: Duration::from_secs(60),
        }
    }
}

/// Internal state of the circuit breaker
#[derive(Debug)]
struct CircuitBreakerState {
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure_time: Option<Instant>,
    last_state_change: Instant,
    window_start: Instant,
}

impl CircuitBreakerState {
    fn new() -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure_time: None,
            last_state_change: Instant::now(),
            window_start: Instant::now(),
        }
    }

    /// Reset failure window if expired
    fn reset_window_if_expired(&mut self, window_duration: Duration) {
        if self.window_start.elapsed() > window_duration {
            self.failure_count = 0;
            self.window_start = Instant::now();
        }
    }
}

/// Circuit breaker to prevent cascading failures
///
/// Implements the circuit breaker pattern:
/// - **Closed**: Normal operation, requests pass through
/// - **Open**: Service is failing, requests fail immediately
/// - **Half-Open**: Testing if service recovered, limited requests allowed
///
/// # Example
/// ```rust,no_run
/// use nzeza::domain::services::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
///
/// let breaker = CircuitBreaker::new(CircuitBreakerConfig::default());
///
/// // Try to execute a request
/// match breaker.call(|| async {
///     // Your async operation here
///     Ok::<_, String>("success")
/// }).await {
///     Ok(result) => println!("Success: {:?}", result),
///     Err(e) => println!("Circuit breaker rejected or operation failed: {}", e),
/// }
/// ```
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Arc<Mutex<CircuitBreakerState>>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with given configuration
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(CircuitBreakerState::new())),
        }
    }

    /// Get current circuit state
    pub async fn state(&self) -> CircuitState {
        let state = self.state.lock().await;
        state.state.clone()
    }

    /// Check if circuit allows requests
    pub async fn is_call_permitted(&self) -> bool {
        let mut state = self.state.lock().await;

        // Reset failure window if expired
        state.reset_window_if_expired(self.config.window_duration);

        match state.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout expired, move to half-open
                if let Some(last_failure) = state.last_failure_time {
                    if last_failure.elapsed() >= self.config.timeout {
                        state.state = CircuitState::HalfOpen;
                        state.success_count = 0;
                        state.last_state_change = Instant::now();
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Record a successful operation
    pub async fn on_success(&self) {
        let mut state = self.state.lock().await;

        match state.state {
            CircuitState::HalfOpen => {
                state.success_count += 1;
                if state.success_count >= self.config.success_threshold {
                    // Service recovered, close circuit
                    state.state = CircuitState::Closed;
                    state.failure_count = 0;
                    state.success_count = 0;
                    state.last_state_change = Instant::now();
                    state.window_start = Instant::now();
                }
            }
            CircuitState::Closed => {
                // Reset failure count on success in closed state
                state.failure_count = 0;
                state.window_start = Instant::now();
            }
            CircuitState::Open => {
                // Ignore success in open state (shouldn't happen)
            }
        }
    }

    /// Record a failed operation
    pub async fn on_failure(&self) {
        let mut state = self.state.lock().await;
        state.reset_window_if_expired(self.config.window_duration);

        match state.state {
            CircuitState::Closed => {
                state.failure_count += 1;
                state.last_failure_time = Some(Instant::now());

                if state.failure_count >= self.config.failure_threshold {
                    // Too many failures, open circuit
                    state.state = CircuitState::Open;
                    state.last_state_change = Instant::now();
                }
            }
            CircuitState::HalfOpen => {
                // Failure in half-open means service still failing
                state.state = CircuitState::Open;
                state.success_count = 0;
                state.failure_count = 1;
                state.last_failure_time = Some(Instant::now());
                state.last_state_change = Instant::now();
            }
            CircuitState::Open => {
                // Update last failure time
                state.last_failure_time = Some(Instant::now());
            }
        }
    }

    /// Execute a function with circuit breaker protection
    ///
    /// Returns Err if circuit is open or if the function fails
    pub async fn call<F, Fut, T, E>(&self, f: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
    {
        if !self.is_call_permitted().await {
            return Err(CircuitBreakerError::CircuitOpen);
        }

        match f().await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(e) => {
                self.on_failure().await;
                Err(CircuitBreakerError::OperationFailed(e))
            }
        }
    }

    /// Get statistics about circuit breaker
    pub async fn stats(&self) -> CircuitBreakerStats {
        let state = self.state.lock().await;
        CircuitBreakerStats {
            state: state.state.clone(),
            failure_count: state.failure_count,
            success_count: state.success_count,
            time_in_current_state: state.last_state_change.elapsed(),
            time_since_last_failure: state.last_failure_time.map(|t| t.elapsed()),
        }
    }

    /// Manually reset the circuit breaker to closed state
    pub async fn reset(&self) {
        let mut state = self.state.lock().await;
        state.state = CircuitState::Closed;
        state.failure_count = 0;
        state.success_count = 0;
        state.last_failure_time = None;
        state.last_state_change = Instant::now();
        state.window_start = Instant::now();
    }
}

/// Statistics about circuit breaker state
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
    pub time_in_current_state: Duration,
    pub time_since_last_failure: Option<Duration>,
}

/// Error type for circuit breaker operations
#[derive(Debug)]
pub enum CircuitBreakerError<E> {
    /// Circuit is open, request rejected
    CircuitOpen,
    /// Operation failed
    OperationFailed(E),
}

impl<E: std::fmt::Display> std::fmt::Display for CircuitBreakerError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitBreakerError::CircuitOpen => write!(f, "Circuit breaker is open"),
            CircuitBreakerError::OperationFailed(e) => write!(f, "Operation failed: {}", e),
        }
    }
}

impl<E: std::error::Error> std::error::Error for CircuitBreakerError<E> {}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_circuit_breaker_closed_state() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        assert_eq!(breaker.state().await, CircuitState::Closed);
        assert!(breaker.is_call_permitted().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_after_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // Record 3 failures
        for _ in 0..3 {
            breaker.on_failure().await;
        }

        assert_eq!(breaker.state().await, CircuitState::Open);
        assert!(!breaker.is_call_permitted().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_after_timeout() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            timeout: Duration::from_millis(100),
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // Open the circuit
        breaker.on_failure().await;
        breaker.on_failure().await;
        assert_eq!(breaker.state().await, CircuitState::Open);

        // Wait for timeout
        sleep(Duration::from_millis(150)).await;

        // Should allow one request (half-open)
        assert!(breaker.is_call_permitted().await);
        assert_eq!(breaker.state().await, CircuitState::HalfOpen);
    }

    #[tokio::test]
    async fn test_circuit_breaker_closes_after_successes() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout: Duration::from_millis(50),
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // Open the circuit
        breaker.on_failure().await;
        breaker.on_failure().await;

        // Wait for timeout and move to half-open
        sleep(Duration::from_millis(100)).await;
        let _ = breaker.is_call_permitted().await;

        // Record successes to close circuit
        breaker.on_success().await;
        breaker.on_success().await;

        assert_eq!(breaker.state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_call() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // Successful call
        let result = breaker.call(|| async { Ok::<_, String>("success") }).await;
        assert!(result.is_ok());

        // Failed calls
        for _ in 0..2 {
            let _ = breaker.call(|| async { Err::<String, _>("error") }).await;
        }

        // Circuit should be open now
        let result = breaker.call(|| async { Ok::<_, String>("success") }).await;
        assert!(matches!(result, Err(CircuitBreakerError::CircuitOpen)));
    }

    #[tokio::test]
    async fn test_circuit_breaker_reset() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // Open the circuit
        breaker.on_failure().await;
        breaker.on_failure().await;
        assert_eq!(breaker.state().await, CircuitState::Open);

        // Reset
        breaker.reset().await;
        assert_eq!(breaker.state().await, CircuitState::Closed);
        assert!(breaker.is_call_permitted().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_stats() {
        let breaker = CircuitBreaker::new(CircuitBreakerConfig::default());

        breaker.on_failure().await;
        let stats = breaker.stats().await;

        assert_eq!(stats.state, CircuitState::Closed);
        assert_eq!(stats.failure_count, 1);
        assert!(stats.time_since_last_failure.is_some());
    }
}
