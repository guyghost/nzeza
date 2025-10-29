use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::debug;

static CONNECTION_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Connected,
    Disconnected,
    Connecting,
    Failed,
}

#[derive(Debug, Clone)]
pub struct PriceUpdate {
    pub product_id: String,
    pub price: String,
    pub timestamp: String,
}

pub struct WebSocketClient {
    url: String,
    connection_state: Arc<RwLock<ConnectionState>>,
    connection_id: Arc<RwLock<Option<String>>>,
    last_heartbeat: Arc<RwLock<Option<Instant>>>,
    retry_count: Arc<RwLock<u32>>,
    backoff_ms: Arc<RwLock<u64>>,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl WebSocketClient {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            connection_state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            connection_id: Arc::new(RwLock::new(None)),
            last_heartbeat: Arc::new(RwLock::new(None)),
            retry_count: Arc::new(RwLock::new(0)),
            backoff_ms: Arc::new(RwLock::new(100)),
            circuit_breaker: Arc::new(CircuitBreaker::new(5, 3)),
        }
    }

    pub async fn connect(&self) -> Result<(), String> {
        *self.connection_state.write().await = ConnectionState::Connecting;

        // Simulate connection
        let id = CONNECTION_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
        let connection_id = format!("conn_{}", id);
        *self.connection_id.write().await = Some(connection_id);
        *self.last_heartbeat.write().await = Some(Instant::now());
        *self.connection_state.write().await = ConnectionState::Connected;

        Ok(())
    }

    pub async fn disconnect(&self) {
        *self.connection_state.write().await = ConnectionState::Disconnected;
        *self.connection_id.write().await = None;
    }

    pub fn is_connected(&self) -> bool {
        matches!(
            *self.connection_state.blocking_read(),
            ConnectionState::Connected
        )
    }

    pub fn connection_state(&self) -> ConnectionState {
        *self.connection_state.blocking_read()
    }

    pub fn last_heartbeat(&self) -> Option<Instant> {
        *self.last_heartbeat.blocking_read()
    }

    pub fn connection_id(&self) -> Option<String> {
        self.connection_id.blocking_read().clone()
    }

    pub async fn reconnect(&self) -> Result<(), String> {
        let current_retry = {
            let mut retry_count = self.retry_count.write().await;
            let max_retries = 5;
            if *retry_count >= max_retries {
                return Err("Max retries exceeded".to_string());
            }
            *retry_count += 1;
            *retry_count
        };

        // Exponential backoff
        let backoff = {
            let mut backoff_ms = self.backoff_ms.write().await;
            let delay = *backoff_ms;
            *backoff_ms = (*backoff_ms * 2).min(1600); // Cap at 1600ms
            delay
        };

        debug!(
            "Reconnecting attempt {} with backoff {}ms",
            current_retry, backoff
        );
        sleep(Duration::from_millis(backoff)).await;
        self.connect().await
    }

    pub async fn reset_backoff(&self) {
        *self.backoff_ms.write().await = 100;
        *self.retry_count.write().await = 0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitBreakerState>>,
    failure_count: Arc<RwLock<u32>>,
    success_count: Arc<RwLock<u32>>,
    last_state_change: Arc<RwLock<Instant>>,
    failure_threshold: u32,
    success_threshold: u32,
    timeout_seconds: u64,
    metrics: Arc<CircuitBreakerMetrics>,
}

pub struct CircuitBreakerMetrics {
    total_failures: Arc<AtomicU32>,
    total_successes: Arc<AtomicU32>,
    state_changes: Arc<AtomicU32>,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, success_threshold: u32) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitBreakerState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            success_count: Arc::new(RwLock::new(0)),
            last_state_change: Arc::new(RwLock::new(Instant::now())),
            failure_threshold,
            success_threshold,
            timeout_seconds: 10,
            metrics: Arc::new(CircuitBreakerMetrics {
                total_failures: Arc::new(AtomicU32::new(0)),
                total_successes: Arc::new(AtomicU32::new(0)),
                state_changes: Arc::new(AtomicU32::new(0)),
            }),
        }
    }

    pub async fn record_failure(&self) {
        let mut failure_count = self.failure_count.write().await;
        *failure_count += 1;

        self.metrics.total_failures.fetch_add(1, Ordering::SeqCst);

        if *failure_count >= self.failure_threshold {
            let mut state = self.state.write().await;
            *state = CircuitBreakerState::Open;
            *self.last_state_change.write().await = Instant::now();

            self.metrics.state_changes.fetch_add(1, Ordering::SeqCst);
        }
    }

    pub async fn record_success(&self) {
        let mut state = self.state.write().await;

        match *state {
            CircuitBreakerState::Closed => {
                let mut success_count = self.success_count.write().await;
                *success_count += 1;

                self.metrics.total_successes.fetch_add(1, Ordering::SeqCst);
            }
            CircuitBreakerState::HalfOpen => {
                let mut success_count = self.success_count.write().await;
                *success_count += 1;

                self.metrics.total_successes.fetch_add(1, Ordering::SeqCst);

                if *success_count >= self.success_threshold {
                    *state = CircuitBreakerState::Closed;
                    *self.failure_count.write().await = 0;
                    *self.success_count.write().await = 0;
                    *self.last_state_change.write().await = Instant::now();

                    self.metrics.state_changes.fetch_add(1, Ordering::SeqCst);
                }
            }
            _ => {}
        }
    }

    pub async fn check_state(&self) {
        let mut state = self.state.write().await;

        if *state == CircuitBreakerState::Open {
            let elapsed = self.last_state_change.read().await.elapsed();
            if elapsed.as_secs() >= self.timeout_seconds {
                *state = CircuitBreakerState::HalfOpen;
                *self.success_count.write().await = 0;
                *self.failure_count.write().await = 0;
                *self.last_state_change.write().await = Instant::now();

                self.metrics.state_changes.fetch_add(1, Ordering::SeqCst);
            }
        }
    }

    pub fn state(&self) -> CircuitBreakerState {
        *self.state.blocking_read()
    }

    pub fn metrics(&self) -> (u32, u32, u32) {
        let failures = self.metrics.total_failures.load(Ordering::SeqCst);
        let successes = self.metrics.total_successes.load(Ordering::SeqCst);
        let changes = self.metrics.state_changes.load(Ordering::SeqCst);
        (failures, successes, changes)
    }
}
