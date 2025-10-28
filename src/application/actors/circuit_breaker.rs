use std::time::{Duration, Instant};
use crate::application::actors::websocket_client::*;

/// Circuit breaker implementation
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure_time: Option<Instant>,
    last_success_time: Option<Instant>,
    state_changed_time: Instant,
    open_time: Option<Instant>,
    close_time: Option<Instant>,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure_time: None,
            last_success_time: None,
            state_changed_time: Instant::now(),
            open_time: None,
            close_time: None,
        }
    }

    pub fn state(&self) -> CircuitState {
        self.state.clone()
    }

    pub fn is_closed(&self) -> bool {
        matches!(self.state, CircuitState::Closed)
    }

    pub fn is_open(&self) -> bool {
        matches!(self.state, CircuitState::Open)
    }

    pub fn is_half_open(&self) -> bool {
        matches!(self.state, CircuitState::HalfOpen)
    }

    pub fn record_success(&mut self) {
        self.success_count += 1;
        self.last_success_time = Some(Instant::now());

        match self.state {
            CircuitState::Closed => {
                // Reset consecutive failures on success in closed state
                self.failure_count = 0;
            }
            CircuitState::HalfOpen => {
                // Check if we have enough successes to close
                if self.success_count >= self.config.success_threshold {
                    self.transition_to(CircuitState::Closed);
                }
            }
            CircuitState::Open => {
                // Should not happen, but reset if it does
                self.failure_count = 0;
            }
        }
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_time = Some(Instant::now());

        match self.state {
            CircuitState::Closed => {
                // Check if we should open
                if self.failure_count >= self.config.failure_threshold {
                    self.transition_to(CircuitState::Open);
                }
            }
            CircuitState::HalfOpen => {
                // Failure in half-open goes back to open
                self.transition_to(CircuitState::Open);
            }
            CircuitState::Open => {
                // Already open, just increment count
            }
        }
    }

    pub fn should_attempt(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout has elapsed
                if let Some(open_time) = self.open_time {
                    if open_time.elapsed() >= self.config.timeout_duration {
                        self.transition_to(CircuitState::HalfOpen);
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

    pub fn reset(&mut self) {
        self.state = CircuitState::Closed;
        self.failure_count = 0;
        self.success_count = 0;
        self.last_failure_time = None;
        self.last_success_time = None;
        self.state_changed_time = Instant::now();
        self.open_time = None;
        self.close_time = Some(Instant::now());
    }

    fn transition_to(&mut self, new_state: CircuitState) {
        let old_state = self.state.clone();
        self.state = new_state;
        self.state_changed_time = Instant::now();

        match &self.state {
            CircuitState::Open => {
                self.open_time = Some(Instant::now());
            }
            CircuitState::Closed => {
                self.close_time = Some(Instant::now());
                self.failure_count = 0;
                self.success_count = 0;
            }
            CircuitState::HalfOpen => {
                // Half-open doesn't reset counters
            }
        }

        // In a real implementation, emit events here
    }

    pub fn time_in_current_state(&self) -> Duration {
        self.state_changed_time.elapsed()
    }

    pub fn failure_count(&self) -> u32 {
        self.failure_count
    }

    pub fn success_count(&self) -> u32 {
        self.success_count
    }

    pub fn last_failure_time(&self) -> Option<Instant> {
        self.last_failure_time
    }

    pub fn last_success_time(&self) -> Option<Instant> {
        self.last_success_time
    }
}