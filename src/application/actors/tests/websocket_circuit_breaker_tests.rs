use std::time::{Duration, Instant};
use tokio::time::{sleep, timeout};
use tracing::info;

use super::mock_websocket_server::MockWebSocketServer;

// All tests reference functionality that doesn't exist yet (RED phase)

#[tokio::test]
async fn test_circuit_opens_after_threshold() {
    info!("Testing circuit breaker opening after failure threshold");
    
    // Start mock server that will fail connections
    let mut mock_server = MockWebSocketServer::new(9016);
    let server_addr = mock_server.start().await;
    mock_server.set_failure_mode(true).await;
    
    // Create client with circuit breaker
    let client = WebSocketClient::new(&format!("ws://{}", server_addr))
        .with_circuit_breaker(CircuitBreakerConfig {
            failure_threshold: 5,
            timeout_duration: Duration::from_secs(10),
            success_threshold: 3,
            max_retry_interval: Duration::from_secs(60),
        });
    
    // Monitor circuit breaker events
    let circuit_stream = client.circuit_breaker_stream();
    let mut circuit_receiver = circuit_stream.subscribe();
    
    // Monitor connection attempts
    let connection_stream = client.connection_attempt_stream();
    let mut connection_receiver = connection_stream.subscribe();
    
    // Attempt initial connection (will fail)
    let initial_result = client.connect().await;
    assert!(initial_result.is_err(), "Initial connection should fail");
    
    // Monitor for failures and circuit opening
    let monitor_task = tokio::spawn(async move {
        let mut failure_count = 0;
        let mut circuit_opened = false;
        let start_time = Instant::now();
        
        while start_time.elapsed() < Duration::from_secs(30) {
            tokio::select! {
                circuit_event = circuit_receiver.recv() => {
                    if let Ok(event) = circuit_event {
                        match event {
                            CircuitBreakerEvent::StateChanged { from, to, .. } => {
                                info!("Circuit state changed: {:?} -> {:?}", from, to);
                                if matches!(to, CircuitState::Open) {
                                    circuit_opened = true;
                                    break;
                                }
                            }
                            CircuitBreakerEvent::FailureRecorded { total_failures, .. } => {
                                failure_count = total_failures;
                                info!("Failure recorded: {} total failures", failure_count);
                            }
                            _ => {}
                        }
                    }
                }
                connection_event = connection_receiver.recv() => {
                    if let Ok(event) = connection_event {
                        info!("Connection event: {:?}", event);
                    }
                }
                _ = sleep(Duration::from_millis(100)) => {
                    // Continue monitoring
                }
            }
        }
        
        (failure_count, circuit_opened)
    });
    
    let (failure_count, circuit_opened) = monitor_task.await.expect("Monitor task should complete");
    
    // Verify circuit opened after threshold
    assert!(circuit_opened, "Circuit should open after failure threshold");
    assert!(failure_count >= 5, "Should record at least 5 failures");
    
    // Verify circuit state
    assert_eq!(client.circuit_state(), CircuitState::Open);
    assert!(client.is_circuit_open(), "Circuit should be in open state");
    assert!(!client.is_circuit_closed(), "Circuit should not be closed");
    
    // Verify connection attempts are now blocked
    let blocked_attempt = client.connect().await;
    assert!(blocked_attempt.is_err(), "Connection attempts should be blocked when circuit is open");
    
    let error_message = blocked_attempt.unwrap_err();
    assert!(error_message.contains("circuit") || error_message.contains("open"), 
           "Error should mention circuit breaker: {}", error_message);
    
    // Verify circuit breaker metrics
    let circuit_metrics = client.circuit_breaker_metrics();
    assert_eq!(circuit_metrics.total_failures, failure_count);
    assert_eq!(circuit_metrics.consecutive_failures, failure_count);
    assert_eq!(circuit_metrics.state_transitions, 1); // Closed -> Open
    assert_eq!(circuit_metrics.current_state, CircuitState::Open);
    assert!(circuit_metrics.time_in_current_state >= Duration::from_millis(100));
    assert!(circuit_metrics.last_failure_time.is_some());
    assert!(circuit_metrics.circuit_open_time.is_some());
    
    // Verify failure tracking
    let failure_history = client.failure_history();
    assert!(failure_history.len() >= 5, "Should track failure history");
    assert!(failure_history.iter().all(|f| f.is_within_window(Duration::from_secs(60))), 
           "All failures should be within time window");
    
    // Clean up
    client.reset_circuit_breaker();
    mock_server.stop().await;
    
    info!("Circuit breaker opening test completed");
}

#[tokio::test]
async fn test_circuit_half_open_after_timeout() {
    info!("Testing circuit breaker half-open state after timeout");
    
    // Start mock server
    let mut mock_server = MockWebSocketServer::new(9017);
    let server_addr = mock_server.start().await;
    
    // Create client with short timeout for testing
    let client = WebSocketClient::new(&format!("ws://{}", server_addr))
        .with_circuit_breaker(CircuitBreakerConfig {
            failure_threshold: 3,
            timeout_duration: Duration::from_millis(500), // Short timeout for testing
            success_threshold: 2,
            max_retry_interval: Duration::from_secs(5),
        });
    
    // Monitor circuit breaker events
    let circuit_stream = client.circuit_breaker_stream();
    let mut circuit_receiver = circuit_stream.subscribe();
    
    // Force circuit to open by causing failures
    mock_server.set_failure_mode(true).await;
    
    for i in 1..=3 {
        let result = client.connect().await;
        assert!(result.is_err(), "Connection {} should fail", i);
        sleep(Duration::from_millis(50)).await;
    }
    
    // Wait for circuit to open
    let open_event = timeout(Duration::from_secs(5), async {
        while let Ok(event) = circuit_receiver.recv().await {
            if let CircuitBreakerEvent::StateChanged { to: CircuitState::Open, .. } = event {
                return true;
            }
        }
        false
    }).await;
    
    assert!(open_event.unwrap_or(false), "Circuit should open");
    assert_eq!(client.circuit_state(), CircuitState::Open);
    
    // Record the time when circuit opened
    let circuit_open_time = Instant::now();
    
    // Wait for timeout to pass and circuit to become half-open
    let half_open_event = timeout(Duration::from_secs(2), async {
        while let Ok(event) = circuit_receiver.recv().await {
            match event {
                CircuitBreakerEvent::StateChanged { to: CircuitState::HalfOpen, .. } => {
                    return true;
                }
                CircuitBreakerEvent::TimeoutElapsed { next_state } => {
                    info!("Timeout elapsed, next state: {:?}", next_state);
                }
                _ => {}
            }
        }
        false
    }).await;
    
    assert!(half_open_event.unwrap_or(false), "Circuit should transition to half-open");
    assert_eq!(client.circuit_state(), CircuitState::HalfOpen);
    
    // Verify timing
    let elapsed = circuit_open_time.elapsed();
    assert!(elapsed >= Duration::from_millis(450), "Should wait at least the timeout duration");
    assert!(elapsed <= Duration::from_millis(1000), "Should not wait too much longer");
    
    // Verify half-open state behavior
    assert!(client.is_circuit_half_open(), "Should be in half-open state");
    assert!(!client.is_circuit_open(), "Should not be in open state");
    assert!(!client.is_circuit_closed(), "Should not be in closed state");
    
    // In half-open state, limited connection attempts should be allowed
    mock_server.set_failure_mode(false).await;
    let half_open_attempt = client.connect().await;
    
    // The behavior depends on implementation - could succeed or fail
    info!("Half-open connection attempt result: {:?}", half_open_attempt);
    
    // Verify circuit breaker metrics for half-open state
    let circuit_metrics = client.circuit_breaker_metrics();
    assert_eq!(circuit_metrics.current_state, CircuitState::HalfOpen);
    assert!(circuit_metrics.state_transitions >= 2); // Closed -> Open -> HalfOpen
    assert!(circuit_metrics.time_in_open_state >= Duration::from_millis(400));
    assert!(circuit_metrics.timeout_events >= 1);
    assert!(circuit_metrics.half_open_attempts >= 0);
    
    // Verify timeout tracking
    let timeout_history = client.timeout_history();
    assert!(timeout_history.len() >= 1, "Should track timeout events");
    assert!(timeout_history[0].duration >= Duration::from_millis(450));
    assert_eq!(timeout_history[0].from_state, CircuitState::Open);
    assert_eq!(timeout_history[0].to_state, CircuitState::HalfOpen);
    
    // Clean up
    client.reset_circuit_breaker();
    mock_server.stop().await;
    
    info!("Circuit breaker half-open test completed");
}

#[tokio::test]
async fn test_circuit_closes_on_success() {
    info!("Testing circuit breaker closing after successful connections in half-open state");
    
    // Start mock server
    let mut mock_server = MockWebSocketServer::new(9018);
    let server_addr = mock_server.start().await;
    
    // Create client with circuit breaker
    let client = WebSocketClient::new(&format!("ws://{}", server_addr))
        .with_circuit_breaker(CircuitBreakerConfig {
            failure_threshold: 2,
            timeout_duration: Duration::from_millis(200),
            success_threshold: 3, // Need 3 successes to close
            max_retry_interval: Duration::from_secs(5),
        });
    
    // Monitor circuit breaker events
    let circuit_stream = client.circuit_breaker_stream();
    let mut circuit_receiver = circuit_stream.subscribe();
    
    // Force circuit to open
    mock_server.set_failure_mode(true).await;
    
    for i in 1..=2 {
        let result = client.connect().await;
        assert!(result.is_err(), "Connection {} should fail", i);
        sleep(Duration::from_millis(50)).await;
    }
    
    // Wait for circuit to open and then half-open
    let mut circuit_opened = false;
    let mut circuit_half_opened = false;
    
    let state_transition_task = timeout(Duration::from_secs(5), async {
        while let Ok(event) = circuit_receiver.recv().await {
            match event {
                CircuitBreakerEvent::StateChanged { from, to, .. } => {
                    info!("Circuit state: {:?} -> {:?}", from, to);
                    if matches!(to, CircuitState::Open) {
                        circuit_opened = true;
                    }
                    if matches!(to, CircuitState::HalfOpen) {
                        circuit_half_opened = true;
                        break;
                    }
                }
                _ => {}
            }
        }
        (circuit_opened, circuit_half_opened)
    }).await;
    
    let (opened, half_opened) = state_transition_task.expect("Should receive state transitions");
    assert!(opened, "Circuit should have opened");
    assert!(half_opened, "Circuit should have moved to half-open");
    
    // Now enable successful connections
    mock_server.set_failure_mode(false).await;
    assert_eq!(client.circuit_state(), CircuitState::HalfOpen);
    
    // Make successful connections to trigger circuit closing
    let mut success_count = 0;
    let mut circuit_closed = false;
    
    for attempt in 1..=5 {
        info!("Making success attempt {}", attempt);
        
        let connection_result = client.connect().await;
        if connection_result.is_ok() {
            success_count += 1;
            info!("Successful connection #{}", success_count);
            
            // Disconnect to make next connection attempt
            client.disconnect().await;
            sleep(Duration::from_millis(100)).await;
        }
        
        // Check if circuit closed
        if client.circuit_state() == CircuitState::Closed {
            circuit_closed = true;
            info!("Circuit closed after {} successes", success_count);
            break;
        }
        
        // Check for circuit state change events
        while let Ok(event) = timeout(Duration::from_millis(50), circuit_receiver.recv()).await {
            if let Ok(CircuitBreakerEvent::StateChanged { to: CircuitState::Closed, .. }) = event {
                circuit_closed = true;
                break;
            }
        }
        
        if circuit_closed {
            break;
        }
        
        sleep(Duration::from_millis(100)).await;
    }
    
    // Verify circuit closed after enough successes
    assert!(circuit_closed, "Circuit should close after successful connections");
    assert_eq!(client.circuit_state(), CircuitState::Closed);
    assert!(success_count >= 3, "Should require at least 3 successes to close");
    
    // Verify circuit state properties
    assert!(client.is_circuit_closed(), "Should be in closed state");
    assert!(!client.is_circuit_open(), "Should not be in open state");
    assert!(!client.is_circuit_half_open(), "Should not be in half-open state");
    
    // Verify normal operation resumes
    let normal_connection = client.connect().await;
    assert!(normal_connection.is_ok(), "Normal connections should work when circuit is closed");
    
    // Verify circuit breaker metrics
    let circuit_metrics = client.circuit_breaker_metrics();
    assert_eq!(circuit_metrics.current_state, CircuitState::Closed);
    assert!(circuit_metrics.state_transitions >= 3); // Closed -> Open -> HalfOpen -> Closed
    assert!(circuit_metrics.successful_connections >= 3);
    assert_eq!(circuit_metrics.consecutive_successes, success_count);
    assert!(circuit_metrics.time_in_half_open_state > Duration::ZERO);
    assert!(circuit_metrics.last_success_time.is_some());
    assert!(circuit_metrics.circuit_close_time.is_some());
    
    // Verify success tracking
    let success_history = client.success_history();
    assert!(success_history.len() >= 3, "Should track success history");
    assert!(success_history.iter().all(|s| s.occurred_recently(Duration::from_secs(10))), 
           "All successes should be recent");
    
    // Verify reset behavior
    let reset_metrics = client.circuit_breaker_metrics();
    assert_eq!(reset_metrics.consecutive_failures, 0, "Failures should be reset after closing");
    // Note: failure_rate_window will be implemented in GREEN phase
    // assert!(reset_metrics.failure_rate_window.is_empty() || 
    //         reset_metrics.failure_rate_window.iter().all(|f| !f.is_recent(Duration::from_secs(10))),
    //        "Recent failure window should be cleared");
    
    // Clean up
    client.disconnect().await;
    mock_server.stop().await;
    
    info!("Circuit breaker closing test completed");
}

#[tokio::test]
async fn test_exponential_backoff_during_open() {
    info!("Testing exponential backoff for timeout during open state");
    
    // Start mock server
    let mut mock_server = MockWebSocketServer::new(9019);
    let server_addr = mock_server.start().await;
    
    // Create client with exponential backoff config
    let client = WebSocketClient::new(&format!("ws://{}", server_addr))
        .with_circuit_breaker(CircuitBreakerConfig {
            failure_threshold: 2,
            timeout_duration: Duration::from_millis(100), // Base timeout
            success_threshold: 2,
            max_retry_interval: Duration::from_secs(10),
        })
        .with_exponential_backoff(ExponentialBackoffConfig {
            multiplier: 2.0,
            max_timeout: Duration::from_secs(5),
            jitter: false, // Disable jitter for predictable testing
        });
    
    // Monitor circuit breaker events
    let circuit_stream = client.circuit_breaker_stream();
    let mut circuit_receiver = circuit_stream.subscribe();
    
    // Force circuit to open
    mock_server.set_failure_mode(true).await;
    
    for i in 1..=2 {
        let result = client.connect().await;
        assert!(result.is_err(), "Connection {} should fail", i);
        sleep(Duration::from_millis(50)).await;
    }
    
    // Wait for circuit to open
    let open_time = Instant::now();
    let mut circuit_opened = false;
    
    while !circuit_opened && open_time.elapsed() < Duration::from_secs(5) {
        if let Ok(event) = timeout(Duration::from_millis(100), circuit_receiver.recv()).await {
            if let Ok(CircuitBreakerEvent::StateChanged { to: CircuitState::Open, .. }) = event {
                circuit_opened = true;
                break;
            }
        }
    }
    
    assert!(circuit_opened, "Circuit should open");
    
    // Monitor timeout exponential backoff
    let backoff_monitor_task = tokio::spawn(async move {
        let mut timeout_durations = Vec::new();
        let start_monitor = Instant::now();
        
        while start_monitor.elapsed() < Duration::from_secs(15) {
            if let Ok(event) = timeout(Duration::from_millis(100), circuit_receiver.recv()).await {
                if let Ok(event) = event {
                    match event {
                        CircuitBreakerEvent::TimeoutStarted { duration, attempt } => {
                            timeout_durations.push((attempt, duration));
                            info!("Timeout attempt {}: {:?}", attempt, duration);
                        }
                        CircuitBreakerEvent::StateChanged { to: CircuitState::HalfOpen, .. } => {
                            info!("Transitioned to half-open");
                        }
                        _ => {}
                    }
                    
                    // Stop after several timeout attempts
                    if timeout_durations.len() >= 4 {
                        break;
                    }
                }
            }
        }
        
        timeout_durations
    });
    
    let timeout_durations = backoff_monitor_task.await.expect("Backoff monitor should complete");
    
    // Verify exponential backoff pattern
    assert!(timeout_durations.len() >= 3, "Should have multiple timeout attempts");
    
    // Expected pattern: 100ms, 200ms, 400ms, 800ms, ...
    let expected_durations = [
        Duration::from_millis(100),
        Duration::from_millis(200),
        Duration::from_millis(400),
        Duration::from_millis(800),
    ];
    
    for (i, (attempt, duration)) in timeout_durations.iter().enumerate() {
        assert_eq!(*attempt, (i + 1) as u32, "Attempt numbers should be sequential");
        
        if i < expected_durations.len() {
            let expected = expected_durations[i];
            let tolerance = expected.as_millis() / 10; // 10% tolerance
            let actual_ms = duration.as_millis();
            let expected_ms = expected.as_millis();
            
            assert!(
                actual_ms >= expected_ms - tolerance && actual_ms <= expected_ms + tolerance,
                "Timeout {}: expected ~{:?}, got {:?}", 
                attempt, expected, duration
            );
        }
    }
    
    // Verify backoff doesn't exceed maximum
    let max_timeout_reached = timeout_durations.iter()
        .any(|(_, duration)| *duration >= Duration::from_secs(5));
    
    if timeout_durations.len() > 6 {
        assert!(max_timeout_reached, "Should reach maximum timeout for long sequences");
    }
    
    // Verify circuit breaker metrics include backoff info
    let circuit_metrics = client.circuit_breaker_metrics();
    assert!(circuit_metrics.total_timeout_attempts >= 3);
    assert!(circuit_metrics.exponential_backoff_enabled);
    assert_eq!(circuit_metrics.backoff_multiplier, 2.0);
    assert_eq!(circuit_metrics.max_backoff_duration, Duration::from_secs(5));
    assert!(circuit_metrics.average_timeout_duration >= Duration::from_millis(200));
    
    // Verify backoff tracking
    let backoff_history = client.backoff_history();
    assert!(backoff_history.len() >= 3, "Should track backoff history");
    
    for (i, backoff_event) in backoff_history.iter().enumerate() {
        assert_eq!(backoff_event.attempt_number, (i + 1) as u32);
        assert!(backoff_event.timeout_duration >= Duration::from_millis(100));
        assert!(backoff_event.calculated_at.is_some());
        
        if i > 0 {
            let prev_duration = backoff_history[i-1].timeout_duration;
            let current_duration = backoff_event.timeout_duration;
            
            // Should be approximately double (with some tolerance for jitter/rounding)
            let ratio = current_duration.as_millis() as f64 / prev_duration.as_millis() as f64;
            assert!(ratio >= 1.8 && ratio <= 2.2, 
                   "Backoff should roughly double: prev={:?}, current={:?}, ratio={}", 
                   prev_duration, current_duration, ratio);
        }
    }
    
    // Clean up
    client.reset_circuit_breaker();
    mock_server.stop().await;
    
    info!("Exponential backoff test completed");
}

#[tokio::test]
async fn test_circuit_metrics_collection() {
    info!("Testing comprehensive circuit breaker metrics collection");
    
    // Start mock server
    let mut mock_server = MockWebSocketServer::new(9020);
    let server_addr = mock_server.start().await;
    
    // Create client with metrics collection enabled
    let client = WebSocketClient::new(&format!("ws://{}", server_addr))
        .with_circuit_breaker(CircuitBreakerConfig {
            failure_threshold: 3,
            timeout_duration: Duration::from_millis(200),
            success_threshold: 2,
            max_retry_interval: Duration::from_secs(2),
        })
        .with_metrics_collection(true)
        .with_detailed_metrics(true);
    
    // Monitor circuit breaker events
    let circuit_stream = client.circuit_breaker_stream();
    let mut circuit_receiver = circuit_stream.subscribe();
    
    // Test scenario: Failures -> Open -> HalfOpen -> Successes -> Closed
    let test_start_time = Instant::now();
    
    // Phase 1: Cause failures to open circuit
    mock_server.set_failure_mode(true).await;
    info!("Phase 1: Causing failures");
    
    for i in 1..=3 {
        let result = client.connect().await;
        assert!(result.is_err(), "Failure {} should occur", i);
        sleep(Duration::from_millis(50)).await;
    }
    
    // Wait for circuit to open
    let mut phase1_completed = false;
    while !phase1_completed && test_start_time.elapsed() < Duration::from_secs(10) {
        if let Ok(event) = timeout(Duration::from_millis(100), circuit_receiver.recv()).await {
            if let Ok(CircuitBreakerEvent::StateChanged { to: CircuitState::Open, .. }) = event {
                phase1_completed = true;
            }
        }
    }
    assert!(phase1_completed, "Phase 1: Circuit should open");
    
    let phase1_metrics = client.circuit_breaker_metrics();
    assert_eq!(phase1_metrics.current_state, CircuitState::Open);
    assert_eq!(phase1_metrics.total_failures, 3);
    assert_eq!(phase1_metrics.consecutive_failures, 3);
    assert_eq!(phase1_metrics.state_transitions, 1); // Closed -> Open
    
    // Phase 2: Wait for half-open
    info!("Phase 2: Waiting for half-open");
    let phase2_start = Instant::now();
    let mut phase2_completed = false;
    
    while !phase2_completed && phase2_start.elapsed() < Duration::from_secs(5) {
        if let Ok(event) = timeout(Duration::from_millis(100), circuit_receiver.recv()).await {
            if let Ok(CircuitBreakerEvent::StateChanged { to: CircuitState::HalfOpen, .. }) = event {
                phase2_completed = true;
            }
        }
    }
    assert!(phase2_completed, "Phase 2: Circuit should become half-open");
    
    let phase2_metrics = client.circuit_breaker_metrics();
    assert_eq!(phase2_metrics.current_state, CircuitState::HalfOpen);
    assert!(phase2_metrics.time_in_open_state >= Duration::from_millis(150));
    assert_eq!(phase2_metrics.state_transitions, 2); // Closed -> Open -> HalfOpen
    assert!(phase2_metrics.timeout_events >= 1);
    
    // Phase 3: Make successful connections to close circuit
    info!("Phase 3: Making successful connections");
    mock_server.set_failure_mode(false).await;
    
    for i in 1..=2 {
        let result = client.connect().await;
        assert!(result.is_ok(), "Success {} should occur", i);
        client.disconnect().await;
        sleep(Duration::from_millis(100)).await;
    }
    
    // Wait for circuit to close
    let mut phase3_completed = false;
    let phase3_start = Instant::now();
    
    while !phase3_completed && phase3_start.elapsed() < Duration::from_secs(3) {
        if client.circuit_state() == CircuitState::Closed {
            phase3_completed = true;
            break;
        }
        
        if let Ok(event) = timeout(Duration::from_millis(100), circuit_receiver.recv()).await {
            if let Ok(CircuitBreakerEvent::StateChanged { to: CircuitState::Closed, .. }) = event {
                phase3_completed = true;
            }
        }
    }
    assert!(phase3_completed, "Phase 3: Circuit should close");
    
    // Comprehensive metrics verification
    let final_metrics = client.circuit_breaker_metrics();
    
    // State metrics
    assert_eq!(final_metrics.current_state, CircuitState::Closed);
    assert_eq!(final_metrics.state_transitions, 3); // Closed -> Open -> HalfOpen -> Closed
    assert!(final_metrics.total_uptime >= Duration::from_millis(500));
    
    // Failure metrics
    assert_eq!(final_metrics.total_failures, 3);
    assert_eq!(final_metrics.consecutive_failures, 0); // Reset after closing
    assert!(final_metrics.failure_rate_percent >= 0.0);
    assert!(final_metrics.last_failure_time.is_some());
    
    // Success metrics
    assert!(final_metrics.total_successes >= 2);
    assert!(final_metrics.consecutive_successes >= 2);
    assert!(final_metrics.success_rate_percent >= 0.0);
    assert!(final_metrics.last_success_time.is_some());
    
    // Timing metrics
    assert!(final_metrics.time_in_closed_state >= Duration::ZERO);
    assert!(final_metrics.time_in_open_state >= Duration::from_millis(150));
    assert!(final_metrics.time_in_half_open_state >= Duration::from_millis(100));
    assert!(final_metrics.average_state_duration > Duration::ZERO);
    
    // Event metrics
    assert!(final_metrics.timeout_events >= 1);
    assert!(final_metrics.half_open_attempts >= 2);
    assert!(final_metrics.state_change_events >= 3);
    
    // Performance metrics
    assert!(final_metrics.average_connection_time.is_some());
    assert!(final_metrics.fastest_connection_time.is_some());
    assert!(final_metrics.slowest_connection_time.is_some());
    
    // Verify detailed event history
    let event_history = client.circuit_breaker_event_history();
    assert!(event_history.len() >= 6, "Should track multiple events");
    
    // Verify event types
    let state_changes = event_history.iter()
        .filter(|e| matches!(e.event_type, CircuitEventType::StateChange))
        .count();
    assert_eq!(state_changes, 3, "Should have 3 state change events");
    
    let failures = event_history.iter()
        .filter(|e| matches!(e.event_type, CircuitEventType::Failure))
        .count();
    assert_eq!(failures, 3, "Should have 3 failure events");
    
    let successes = event_history.iter()
        .filter(|e| matches!(e.event_type, CircuitEventType::Success))
        .count();
    assert!(successes >= 2, "Should have at least 2 success events");
    
    // Verify metrics export capability
    let metrics_json = client.export_metrics_as_json();
    assert!(!metrics_json.is_empty(), "Should export metrics as JSON");
    assert!(metrics_json.contains("current_state"), "Should include state info");
    assert!(metrics_json.contains("total_failures"), "Should include failure count");
    assert!(metrics_json.contains("total_successes"), "Should include success count");
    
    let metrics_prometheus = client.export_metrics_as_prometheus();
    assert!(!metrics_prometheus.is_empty(), "Should export metrics in Prometheus format");
    assert!(metrics_prometheus.contains("circuit_breaker_state"), "Should include Prometheus metrics");
    
    // Verify metrics reset functionality
    let reset_time = Instant::now();
    client.reset_circuit_breaker_metrics();
    
    let reset_metrics = client.circuit_breaker_metrics();
    assert_eq!(reset_metrics.total_failures, 0, "Failures should be reset");
    assert_eq!(reset_metrics.total_successes, 0, "Successes should be reset");
    assert_eq!(reset_metrics.state_transitions, 0, "Transitions should be reset");
    assert!(reset_metrics.metrics_reset_time.unwrap() >= reset_time, "Should record reset time");
    
    // Clean up
    client.disconnect().await;
    mock_server.stop().await;
    
    info!("Circuit breaker metrics collection test completed");
}

// Placeholder structs and enums that will be implemented in GREEN phase
struct WebSocketClient {
    url: String,
    circuit_config: Option<CircuitBreakerConfig>,
}

#[derive(Clone)]
struct CircuitBreakerConfig {
    failure_threshold: u32,
    timeout_duration: Duration,
    success_threshold: u32,
    max_retry_interval: Duration,
}

#[derive(Clone)]
struct ExponentialBackoffConfig {
    multiplier: f64,
    max_timeout: Duration,
    jitter: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, Clone)]
enum CircuitBreakerEvent {
    StateChanged {
        from: CircuitState,
        to: CircuitState,
        reason: String,
    },
    FailureRecorded {
        total_failures: u32,
        consecutive_failures: u32,
    },
    SuccessRecorded {
        total_successes: u32,
        consecutive_successes: u32,
    },
    TimeoutStarted {
        duration: Duration,
        attempt: u32,
    },
    TimeoutElapsed {
        next_state: CircuitState,
    },
}

#[derive(Debug, Clone)]
enum ConnectionAttemptEvent {
    Started { attempt_id: String },
    Succeeded { attempt_id: String, duration: Duration },
    Failed { attempt_id: String, error: String },
    Blocked { reason: String },
}

#[derive(Debug, Clone)]
enum CircuitEventType {
    StateChange,
    Failure,
    Success,
    Timeout,
    Attempt,
}

struct CircuitBreakerStream;
struct ConnectionAttemptStream;
struct MockCircuitBreakerReceiver;
struct MockConnectionAttemptReceiver;

#[derive(Clone)]
struct CircuitBreakerMetrics {
    // State metrics
    current_state: CircuitState,
    state_transitions: u32,
    total_uptime: std::time::Duration,
    
    // Failure metrics
    total_failures: u32,
    consecutive_failures: u32,
    failure_rate_percent: f64,
    last_failure_time: Option<std::time::Instant>,
    
    // Success metrics
    total_successes: u32,
    consecutive_successes: u32,
    success_rate_percent: f64,
    last_success_time: Option<std::time::Instant>,
    
    // Timing metrics
    time_in_current_state: std::time::Duration,
    time_in_closed_state: std::time::Duration,
    time_in_open_state: std::time::Duration,
    time_in_half_open_state: std::time::Duration,
    average_state_duration: std::time::Duration,
    
    // Event metrics
    timeout_events: u32,
    half_open_attempts: u32,
    state_change_events: u32,
    
    // Performance metrics
    average_connection_time: Option<std::time::Duration>,
    fastest_connection_time: Option<std::time::Duration>,
    slowest_connection_time: Option<std::time::Duration>,
    
    // Configuration metrics
    failure_threshold: u32,
    success_threshold: u32,
    timeout_duration: std::time::Duration,
    
    // Advanced metrics
    circuit_open_time: Option<std::time::Instant>,
    circuit_close_time: Option<std::time::Instant>,
    successful_connections: u32,
    
    // Backoff metrics
    total_timeout_attempts: u32,
    exponential_backoff_enabled: bool,
    backoff_multiplier: f64,
    max_backoff_duration: std::time::Duration,
    average_timeout_duration: std::time::Duration,
    
    // Reset tracking
    metrics_reset_time: Option<std::time::Instant>,
}

#[derive(Clone)]
struct FailureEvent {
    timestamp: std::time::Instant,
    error: String,
    connection_attempt_id: String,
}

#[derive(Clone)]
struct SuccessEvent {
    timestamp: std::time::Instant,
    duration: std::time::Duration,
    connection_attempt_id: String,
}

#[derive(Clone)]
struct TimeoutEvent {
    duration: std::time::Duration,
    from_state: CircuitState,
    to_state: CircuitState,
    timestamp: std::time::Instant,
}

#[derive(Clone)]
struct BackoffEvent {
    attempt_number: u32,
    timeout_duration: std::time::Duration,
    calculated_at: Option<std::time::Instant>,
}

#[derive(Clone)]
struct CircuitEvent {
    event_type: CircuitEventType,
    timestamp: std::time::Instant,
    state_before: CircuitState,
    state_after: CircuitState,
    details: String,
}

struct MockFailureEvent {
    timestamp: Instant,
    error: String,
    connection_attempt_id: String,
}

struct MockSuccessEvent {
    timestamp: Instant,
    duration: Duration,
    connection_attempt_id: String,
}

struct MockTimeoutEvent {
    duration: Duration,
    from_state: CircuitState,
    to_state: CircuitState,
    timestamp: Instant,
}

struct MockBackoffEvent {
    attempt_number: u32,
    timeout_duration: Duration,
    calculated_at: Option<Instant>,
}

impl WebSocketClient {
    fn new(url: &str) -> Self {
        unimplemented!("WebSocketClient::new() - to be implemented in GREEN phase")
    }
    
    fn with_circuit_breaker(mut self, config: CircuitBreakerConfig) -> Self {
        unimplemented!("WebSocketClient::with_circuit_breaker() - to be implemented in GREEN phase")
    }
    
    fn with_exponential_backoff(mut self, config: ExponentialBackoffConfig) -> Self {
        unimplemented!("WebSocketClient::with_exponential_backoff() - to be implemented in GREEN phase")
    }
    
    fn with_metrics_collection(mut self, enabled: bool) -> Self {
        unimplemented!("WebSocketClient::with_metrics_collection() - to be implemented in GREEN phase")
    }
    
    fn with_detailed_metrics(mut self, enabled: bool) -> Self {
        unimplemented!("WebSocketClient::with_detailed_metrics() - to be implemented in GREEN phase")
    }
    
    async fn connect(&self) -> Result<(), String> {
        unimplemented!("WebSocketClient::connect() - to be implemented in GREEN phase")
    }
    
    async fn disconnect(&self) {
        unimplemented!("WebSocketClient::disconnect() - to be implemented in GREEN phase")
    }
    
    fn circuit_state(&self) -> CircuitState {
        unimplemented!("WebSocketClient::circuit_state() - to be implemented in GREEN phase")
    }
    
    fn is_circuit_open(&self) -> bool {
        unimplemented!("WebSocketClient::is_circuit_open() - to be implemented in GREEN phase")
    }
    
    fn is_circuit_closed(&self) -> bool {
        unimplemented!("WebSocketClient::is_circuit_closed() - to be implemented in GREEN phase")
    }
    
    fn is_circuit_half_open(&self) -> bool {
        unimplemented!("WebSocketClient::is_circuit_half_open() - to be implemented in GREEN phase")
    }
    
    fn circuit_breaker_stream(&self) -> CircuitBreakerStream {
        unimplemented!("WebSocketClient::circuit_breaker_stream() - to be implemented in GREEN phase")
    }
    
    fn connection_attempt_stream(&self) -> ConnectionAttemptStream {
        unimplemented!("WebSocketClient::connection_attempt_stream() - to be implemented in GREEN phase")
    }
    
    fn circuit_breaker_metrics(&self) -> CircuitBreakerMetrics {
        unimplemented!("WebSocketClient::circuit_breaker_metrics() - to be implemented in GREEN phase")
    }
    
    fn failure_history(&self) -> Vec<MockFailureEvent> {
        unimplemented!("WebSocketClient::failure_history() - to be implemented in GREEN phase")
    }
    
    fn success_history(&self) -> Vec<MockSuccessEvent> {
        unimplemented!("WebSocketClient::success_history() - to be implemented in GREEN phase")
    }
    
    fn timeout_history(&self) -> Vec<MockTimeoutEvent> {
        unimplemented!("WebSocketClient::timeout_history() - to be implemented in GREEN phase")
    }
    
    fn backoff_history(&self) -> Vec<MockBackoffEvent> {
        unimplemented!("WebSocketClient::backoff_history() - to be implemented in GREEN phase")
    }
    
    fn circuit_breaker_event_history(&self) -> Vec<CircuitEvent> {
        unimplemented!("WebSocketClient::circuit_breaker_event_history() - to be implemented in GREEN phase")
    }
    
    fn export_metrics_as_json(&self) -> String {
        unimplemented!("WebSocketClient::export_metrics_as_json() - to be implemented in GREEN phase")
    }
    
    fn export_metrics_as_prometheus(&self) -> String {
        unimplemented!("WebSocketClient::export_metrics_as_prometheus() - to be implemented in GREEN phase")
    }
    
    fn reset_circuit_breaker(&self) {
        unimplemented!("WebSocketClient::reset_circuit_breaker() - to be implemented in GREEN phase")
    }
    
    fn reset_circuit_breaker_metrics(&self) {
        unimplemented!("WebSocketClient::reset_circuit_breaker_metrics() - to be implemented in GREEN phase")
    }
}

impl CircuitBreakerStream {
    fn subscribe(&self) -> MockCircuitBreakerReceiver {
        unimplemented!("CircuitBreakerStream::subscribe() - to be implemented in GREEN phase")
    }
}

impl ConnectionAttemptStream {
    fn subscribe(&self) -> MockConnectionAttemptReceiver {
        unimplemented!("ConnectionAttemptStream::subscribe() - to be implemented in GREEN phase")
    }
}

impl MockCircuitBreakerReceiver {
    async fn recv(&mut self) -> Result<CircuitBreakerEvent, String> {
        unimplemented!("MockCircuitBreakerReceiver::recv() - to be implemented in GREEN phase")
    }
}

impl MockConnectionAttemptReceiver {
    async fn recv(&mut self) -> Result<ConnectionAttemptEvent, String> {
        unimplemented!("MockConnectionAttemptReceiver::recv() - to be implemented in GREEN phase")
    }
}

impl MockFailureEvent {
    fn is_within_window(&self, duration: Duration) -> bool {
        unimplemented!("MockFailureEvent::is_within_window() - to be implemented in GREEN phase")
    }
}

impl MockSuccessEvent {
    fn occurred_recently(&self, _duration: Duration) -> bool {
        unimplemented!("MockSuccessEvent::occurred_recently() - to be implemented in GREEN phase")
    }
}

impl MockFailureEvent {
    fn is_recent(&self, _duration: Duration) -> bool {
        unimplemented!("MockFailureEvent::is_recent() - to be implemented in GREEN phase")
    }
}

// Note: MockWebSocketServer methods are defined in mock_websocket_server.rs

// Additional placeholder structs for circuit breaker extension
impl MockWebSocketServer {
    // Clone method for concurrent testing
    fn clone(&self) -> Self {
        unimplemented!("MockWebSocketServer::clone() - to be implemented in GREEN phase")
    }
}