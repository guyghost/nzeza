/// Task Runner with Circuit Breaker Pattern
///
/// Provides automatic retry with exponential backoff and failure tracking
/// to prevent silent task failures in production.

use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, warn};

/// Circuit breaker configuration for background tasks
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Maximum number of consecutive failures before panic
    pub max_consecutive_failures: u32,
    /// Initial delay between retries
    pub initial_retry_delay: Duration,
    /// Maximum delay between retries
    pub max_retry_delay: Duration,
}

/// Internal state for circuit breaker
#[derive(Debug)]
struct CircuitBreakerState {
    consecutive_failures: u32,
    current_retry_delay: Duration,
}

impl CircuitBreakerState {
    fn new(initial_delay: Duration) -> Self {
        Self {
            consecutive_failures: 0,
            current_retry_delay: initial_delay,
        }
    }

    fn record_failure(&mut self, max_delay: Duration) {
        self.consecutive_failures += 1;
        // Exponential backoff with cap
        self.current_retry_delay = std::cmp::min(
            self.current_retry_delay * 2,
            max_delay,
        );
    }

    fn reset(&mut self, initial_delay: Duration) {
        self.consecutive_failures = 0;
        self.current_retry_delay = initial_delay;
    }
}

/// Run a background task with circuit breaker protection
///
/// This function wraps a background task with automatic retry logic and
/// exponential backoff. If the task fails too many times consecutively,
/// the function will panic to prevent silent degradation.
///
/// # Arguments
/// * `task_name` - Name of the task for logging purposes
/// * `config` - Circuit breaker configuration
/// * `task_fn` - Async function that executes one iteration of the task
///
/// # Panics
/// Panics after `max_consecutive_failures` consecutive failures to prevent
/// silent degradation of critical background tasks.
pub async fn run_with_circuit_breaker<F, Fut>(
    task_name: &str,
    config: CircuitBreakerConfig,
    mut task_fn: F,
) where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    let mut state = CircuitBreakerState::new(config.initial_retry_delay);

    loop {
        match task_fn().await {
            Ok(()) => {
                // Reset circuit breaker on success
                if state.consecutive_failures > 0 {
                    warn!(
                        "Task '{}' recovered after {} failures",
                        task_name, state.consecutive_failures
                    );
                }
                state.reset(config.initial_retry_delay);
            }
            Err(e) => {
                state.record_failure(config.max_retry_delay);
                error!(
                    "Task '{}' failed (attempt {}/{}): {}",
                    task_name, state.consecutive_failures, config.max_consecutive_failures, e
                );

                if state.consecutive_failures >= config.max_consecutive_failures {
                    panic!(
                        "FATAL: Task '{}' exceeded maximum consecutive failures ({}). \
                         Last error: {}. System cannot continue with failed critical task.",
                        task_name, config.max_consecutive_failures, e
                    );
                }

                // Apply exponential backoff before retry
                warn!(
                    "Task '{}' will retry in {:?}",
                    task_name, state.current_retry_delay
                );
                sleep(state.current_retry_delay).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_circuit_breaker_resets_on_success() {
        let attempt_count = Arc::new(AtomicUsize::new(0));
        let attempt_count_clone = attempt_count.clone();

        let config = CircuitBreakerConfig {
            max_consecutive_failures: 3,
            initial_retry_delay: Duration::from_millis(10),
            max_retry_delay: Duration::from_millis(100),
        };

        let handle = tokio::spawn(async move {
            run_with_circuit_breaker("test_task", config, || {
                let count = attempt_count_clone.fetch_add(1, Ordering::SeqCst);
                async move {
                    if count < 2 {
                        Err("Simulated failure".to_string())
                    } else {
                        Ok(())
                    }
                }
            })
            .await;
        });

        // Let it run for a bit
        tokio::time::sleep(Duration::from_millis(200)).await;
        handle.abort();

        // Should have attempted multiple times and recovered
        assert!(attempt_count.load(Ordering::SeqCst) >= 3);
    }

    #[tokio::test]
    #[should_panic(expected = "exceeded maximum consecutive failures")]
    async fn test_circuit_breaker_panics_on_max_failures() {
        let config = CircuitBreakerConfig {
            max_consecutive_failures: 3,
            initial_retry_delay: Duration::from_millis(1),
            max_retry_delay: Duration::from_millis(10),
        };

        run_with_circuit_breaker("failing_task", config, || async {
            Err("Always fails".to_string())
        })
        .await;
    }
}