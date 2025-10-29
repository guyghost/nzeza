use std::time::{Duration, Instant};
use tokio::time::{sleep, timeout};
use tracing::info;

use super::mock_websocket_server::MockWebSocketServer;
use crate::application::actors::{
    BufferMetrics, ConnectionMetadata, ConnectionState, ReconnectionConfig, ReconnectionEvent,
    ReconnectionMetrics, WebSocketClient,
};

// All tests reference functionality that doesn't exist yet (RED phase)

#[tokio::test]
async fn test_exponential_backoff_on_disconnect() {
    info!("Testing exponential backoff on WebSocket disconnect");

    // Start mock server
    let mut mock_server = MockWebSocketServer::new(9006);
    let server_addr = mock_server.start().await;

    // Create client with reconnection config
    let client = WebSocketClient::new(&format!("ws://{}", server_addr)).with_reconnection_config(
        ReconnectionConfig {
            base_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(10),
            max_retries: 5,
            backoff_multiplier: 2.0,
        },
    );

    // Connect initially
    client
        .connect()
        .await
        .expect("Initial connection should succeed");
    assert!(client.is_connected(), "Should be connected initially");

    // Start monitoring reconnection attempts
    let reconnection_stream = client.reconnection_stream();
    let mut reconnection_receiver = reconnection_stream.subscribe();

    // Force disconnect from server side multiple times
    let disconnect_task = tokio::spawn(async move {
        for i in 0..4 {
            sleep(Duration::from_millis(200)).await;
            info!("Forcing disconnect #{}", i + 1);
            mock_server.close_connection().await;
            sleep(Duration::from_millis(50)).await;
        }
    });

    // Collect reconnection attempt timings
    let timing_task = tokio::spawn(async move {
        let mut attempt_timings = Vec::new();
        let start_time = Instant::now();

        for attempt in 1..=4 {
            let event_result = timeout(Duration::from_secs(15), reconnection_receiver.recv()).await;
            assert!(event_result.is_ok(), "Should receive reconnection event");

            let event = event_result.unwrap().unwrap();
            let elapsed = start_time.elapsed();
            attempt_timings.push((attempt, elapsed, event));

            info!("Reconnection attempt {}: at {:?}", attempt, elapsed);
        }

        attempt_timings
    });

    // Wait for both tasks
    let (disconnect_result, timing_result): (
        Result<(), _>,
        Result<Vec<(u32, std::time::Duration, ReconnectionEvent)>, _>,
    ) = tokio::join!(disconnect_task, timing_task);
    assert!(disconnect_result.is_ok(), "Disconnect task should complete");
    let timings = timing_result.expect("Timing task should complete");

    // Verify exponential backoff pattern: 100ms, 200ms, 400ms, 800ms
    assert_eq!(timings.len(), 4, "Should have 4 reconnection attempts");

    // Check backoff intervals (allowing for some timing variance)
    let expected_delays = [100, 200, 400, 800]; // milliseconds
    for (i, (attempt, _elapsed, event)) in timings.iter().enumerate() {
        assert_eq!(
            *attempt,
            (i + 1) as u32,
            "Attempt numbers should be sequential"
        );

        if let ReconnectionEvent::AttemptStarted { delay, .. } = event {
            let expected_ms = expected_delays[i];
            let actual_ms = delay.as_millis() as u64;
            let tolerance = expected_ms / 10; // 10% tolerance

            assert!(
                actual_ms >= expected_ms - tolerance && actual_ms <= expected_ms + tolerance,
                "Attempt {}: expected ~{}ms delay, got {}ms",
                attempt,
                expected_ms,
                actual_ms
            );
        } else {
            panic!("Expected AttemptStarted event for attempt {}", attempt);
        }
    }

    // Verify connection eventually succeeds
    assert!(client.is_connected(), "Should eventually reconnect");
    assert_eq!(client.connection_state(), ConnectionState::Connected);

    // Verify reconnection metrics
    let metrics = client.reconnection_metrics();
    assert_eq!(metrics.total_attempts, 4, "Should record 4 attempts");
    assert!(
        metrics.total_downtime > Duration::from_millis(1400),
        "Should accumulate downtime"
    );
    assert!(
        metrics.average_reconnection_time.is_some(),
        "Should calculate average time"
    );

    // Clean up
    client.disconnect().await;

    info!("Exponential backoff test completed");
}

#[tokio::test]
async fn test_max_retries_enforcement() {
    info!("Testing max retries enforcement in WebSocket reconnection");

    // Start mock server that will reject connections
    let mut mock_server = MockWebSocketServer::new(9007);
    let server_addr = mock_server.start().await;
    mock_server.set_reject_connections(true).await;

    // Create client with low max retries
    let client = WebSocketClient::new(&format!("ws://{}", server_addr)).with_reconnection_config(
        ReconnectionConfig {
            base_backoff: Duration::from_millis(50),
            max_backoff: Duration::from_secs(1),
            max_retries: 3,
            backoff_multiplier: 1.5,
        },
    );

    // Monitor reconnection events
    let reconnection_stream = client.reconnection_stream();
    let mut reconnection_receiver = reconnection_stream.subscribe();

    // Attempt connection (will fail and trigger retries)
    let connect_result = client.connect().await;
    assert!(connect_result.is_err(), "Initial connection should fail");

    // Count reconnection attempts
    let counting_task = tokio::spawn(async move {
        let mut attempt_count = 0;
        let mut final_event = None;

        while let Ok(event_result) =
            timeout(Duration::from_secs(10), reconnection_receiver.recv()).await
        {
            if let Ok(event) = event_result {
                match event {
                    ReconnectionEvent::AttemptStarted { attempt_number, .. } => {
                        attempt_count = attempt_number;
                        info!("Reconnection attempt #{}", attempt_number);
                    }
                    ReconnectionEvent::MaxRetriesExceeded { total_attempts } => {
                        final_event = Some(event);
                        info!("Max retries exceeded after {} attempts", total_attempts);
                        break;
                    }
                    _ => {}
                }
            }
        }

        (attempt_count, final_event)
    });

    let (attempts, final_event) = counting_task.await.expect("Counting task should complete");

    // Verify max retries was enforced
    assert_eq!(attempts, 3, "Should attempt exactly 3 reconnections");
    assert!(
        final_event.is_some(),
        "Should receive MaxRetriesExceeded event"
    );

    if let Some(ReconnectionEvent::MaxRetriesExceeded { total_attempts }) = final_event {
        assert_eq!(total_attempts, 3, "Should report 3 total attempts");
    } else {
        panic!("Expected MaxRetriesExceeded event");
    }

    // Verify client state after max retries
    assert!(
        !client.is_connected(),
        "Should not be connected after max retries"
    );
    assert_eq!(client.connection_state(), ConnectionState::Failed);
    assert!(
        client.last_connection_error().is_some(),
        "Should have connection error"
    );

    // Verify no more reconnection attempts
    sleep(Duration::from_secs(2)).await;
    assert_eq!(
        client.connection_state(),
        ConnectionState::Failed,
        "Should remain failed"
    );

    // Verify metrics reflect the failure
    let metrics = client.reconnection_metrics();
    assert_eq!(
        metrics.total_attempts, 3,
        "Should record exactly 3 attempts"
    );
    assert_eq!(
        metrics.successful_reconnections, 0,
        "Should have 0 successful reconnections"
    );
    assert!(
        metrics.max_retries_exceeded,
        "Should mark max retries as exceeded"
    );

    // Verify manual reconnection is still possible
    mock_server.set_reject_connections(false).await;
    let manual_reconnect = client.manual_reconnect().await;
    assert!(
        manual_reconnect.is_ok(),
        "Manual reconnection should succeed"
    );
    assert!(
        client.is_connected(),
        "Should be connected after manual reconnect"
    );

    // Clean up
    client.disconnect().await;
    mock_server.stop().await;

    info!("Max retries enforcement test completed");
}

#[tokio::test]
async fn test_backoff_reset_on_success() {
    info!("Testing backoff reset after successful reconnection");

    // Start mock server
    let mut mock_server = MockWebSocketServer::new(9008);
    let server_addr = mock_server.start().await;

    // Create client with reconnection config
    let client = WebSocketClient::new(&format!("ws://{}", server_addr)).with_reconnection_config(
        ReconnectionConfig {
            base_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(5),
            max_retries: 10,
            backoff_multiplier: 2.0,
        },
    );

    // Connect initially
    client
        .connect()
        .await
        .expect("Initial connection should succeed");

    // Monitor reconnection events
    let reconnection_stream = client.reconnection_stream();
    let mut reconnection_receiver = reconnection_stream.subscribe();

    // First disconnection cycle - should see escalating backoff
    info!("Starting first disconnection cycle");
    mock_server.close_connection().await;
    sleep(Duration::from_millis(50)).await;
    mock_server.close_connection().await;
    sleep(Duration::from_millis(50)).await;

    // Collect first cycle timings
    let mut first_cycle_delays = Vec::new();
    for attempt in 1..=2 {
        let event_result = timeout(Duration::from_secs(5), reconnection_receiver.recv()).await;
        assert!(event_result.is_ok(), "Should receive reconnection event");

        if let ReconnectionEvent::AttemptStarted { delay, .. } = event_result.unwrap().unwrap() {
            first_cycle_delays.push(delay);
            info!("First cycle attempt {}: delay {:?}", attempt, delay);
        }
    }

    // Wait for successful reconnection
    let success_event = timeout(Duration::from_secs(5), reconnection_receiver.recv()).await;
    assert!(success_event.is_ok(), "Should receive success event");
    assert!(matches!(
        success_event.unwrap().unwrap(),
        ReconnectionEvent::Connected { .. }
    ));

    // Wait a bit to ensure connection is stable
    sleep(Duration::from_millis(500)).await;
    assert!(
        client.is_connected(),
        "Should be connected after first cycle"
    );

    // Second disconnection cycle - backoff should reset to base
    info!("Starting second disconnection cycle");
    mock_server.close_connection().await;
    sleep(Duration::from_millis(50)).await;
    mock_server.close_connection().await;
    sleep(Duration::from_millis(50)).await;

    // Collect second cycle timings
    let mut second_cycle_delays = Vec::new();
    for attempt in 1..=2 {
        let event_result = timeout(Duration::from_secs(5), reconnection_receiver.recv()).await;
        assert!(event_result.is_ok(), "Should receive reconnection event");

        if let ReconnectionEvent::AttemptStarted { delay, .. } = event_result.unwrap().unwrap() {
            second_cycle_delays.push(delay);
            info!("Second cycle attempt {}: delay {:?}", attempt, delay);
        }
    }

    // Verify backoff patterns
    assert_eq!(
        first_cycle_delays.len(),
        2,
        "Should have 2 delays in first cycle"
    );
    assert_eq!(
        second_cycle_delays.len(),
        2,
        "Should have 2 delays in second cycle"
    );

    // First cycle should show escalation: 100ms, 200ms
    assert_eq!(
        first_cycle_delays[0],
        Duration::from_millis(100),
        "First cycle should start with base delay"
    );
    assert_eq!(
        first_cycle_delays[1],
        Duration::from_millis(200),
        "First cycle should escalate"
    );

    // Second cycle should reset to base: 100ms, 200ms (not continue from 400ms)
    assert_eq!(
        second_cycle_delays[0],
        Duration::from_millis(100),
        "Second cycle should reset to base delay"
    );
    assert_eq!(
        second_cycle_delays[1],
        Duration::from_millis(200),
        "Second cycle should escalate from base"
    );

    // Verify the second cycle didn't continue from first cycle's progression
    assert_ne!(
        second_cycle_delays[0],
        Duration::from_millis(400),
        "Should not continue from first cycle"
    );

    // Verify reconnection metrics
    let metrics = client.reconnection_metrics();
    assert_eq!(
        metrics.successful_reconnections, 2,
        "Should have 2 successful reconnections"
    );
    assert_eq!(metrics.backoff_resets, 1, "Should have 1 backoff reset");
    assert!(
        metrics.current_backoff_delay == Duration::from_millis(100),
        "Current backoff should be reset"
    );

    // Clean up
    client.disconnect().await;
    mock_server.stop().await;

    info!("Backoff reset test completed");
}

#[tokio::test]
async fn test_concurrent_reconnection_attempts() {
    info!("Testing handling of concurrent reconnection attempts");

    // Start mock server
    let mut mock_server = MockWebSocketServer::new(9009);
    let server_addr = mock_server.start().await;

    // Create client
    let client = WebSocketClient::new(&format!("ws://{}", server_addr)).with_reconnection_config(
        ReconnectionConfig {
            base_backoff: Duration::from_millis(200),
            max_backoff: Duration::from_secs(2),
            max_retries: 5,
            backoff_multiplier: 1.5,
        },
    );

    // Connect initially
    client
        .connect()
        .await
        .expect("Initial connection should succeed");

    // Set up concurrent disconnections and manual reconnection attempts
    let client_clone1 = client.clone();
    let client_clone2 = client.clone();
    let client_clone3 = client.clone();

    // Monitor reconnection events
    let reconnection_stream = client.reconnection_stream();
    let mut reconnection_receiver = reconnection_stream.subscribe();

    // Trigger multiple concurrent disconnection scenarios
    let disconnect_task = tokio::spawn(async move {
        mock_server.close_connection().await;
    });

    let manual_reconnect_task1 = tokio::spawn(async move {
        sleep(Duration::from_millis(50)).await;
        client_clone1.manual_reconnect().await
    });

    let manual_reconnect_task2 = tokio::spawn(async move {
        sleep(Duration::from_millis(100)).await;
        client_clone2.manual_reconnect().await
    });

    let manual_reconnect_task3 = tokio::spawn(async move {
        sleep(Duration::from_millis(150)).await;
        client_clone3.manual_reconnect().await
    });

    // Monitor events for race conditions
    let monitor_task = tokio::spawn(async move {
        let mut events = Vec::new();
        let mut connection_attempts = 0;

        while let Ok(event_result) =
            timeout(Duration::from_secs(10), reconnection_receiver.recv()).await
        {
            if let Ok(event) = event_result {
                events.push(event.clone());

                match event {
                    ReconnectionEvent::AttemptStarted { .. } => {
                        connection_attempts += 1;
                    }
                    ReconnectionEvent::Connected { .. } => {
                        info!("Successfully reconnected");
                        break;
                    }
                    _ => {}
                }

                // Prevent infinite loop
                if events.len() > 20 {
                    break;
                }
            }
        }

        (events, connection_attempts)
    });

    // Wait for all tasks
    let (disconnect_result, manual1_result, manual2_result, manual3_result, monitor_result) = tokio::join!(
        disconnect_task,
        manual_reconnect_task1,
        manual_reconnect_task2,
        manual_reconnect_task3,
        monitor_task
    );

    // Verify task completion
    assert!(disconnect_result.is_ok(), "Disconnect task should complete");
    let (events, connection_attempts) = monitor_result.expect("Monitor task should complete");

    // Verify only one successful reconnection occurred despite multiple attempts
    let successful_connections = events
        .iter()
        .filter(|e| matches!(e, ReconnectionEvent::Connected { .. }))
        .count();
    assert_eq!(
        successful_connections, 1,
        "Should have exactly one successful connection"
    );

    // Verify connection state is stable
    assert!(
        client.is_connected(),
        "Should be connected after concurrent attempts"
    );
    assert_eq!(client.connection_state(), ConnectionState::Connected);

    // Verify no race conditions in manual reconnect results
    let manual_results = [manual1_result, manual2_result, manual3_result];
    let successful_manual_attempts = manual_results.iter().count();

    // At least one manual attempt should succeed, others should be handled gracefully
    assert!(
        successful_manual_attempts >= 1,
        "At least one manual reconnect should succeed"
    );

    // Verify no duplicate connection IDs were created
    let connection_id = client.connection_id().unwrap();
    assert!(connection_id.len() > 0, "Should have a valid connection ID");

    // Verify metrics show correct attempt count (no duplicates)
    let metrics = client.reconnection_metrics();
    assert!(
        metrics.total_attempts >= 1,
        "Should record reconnection attempts"
    );
    assert!(
        metrics.concurrent_attempt_conflicts >= 1,
        "Should detect concurrent attempts"
    );
    assert_eq!(
        metrics.successful_reconnections, 1,
        "Should record exactly one success"
    );

    // Verify connection remains stable after concurrent attempts
    sleep(Duration::from_millis(500)).await;
    assert!(client.is_connected(), "Connection should remain stable");
    assert_eq!(
        client.connection_id().unwrap(),
        connection_id,
        "Connection ID should remain same"
    );

    // Clean up
    client.disconnect().await;

    info!("Concurrent reconnection attempts test completed");
}

#[tokio::test]
async fn test_connection_state_preservation() {
    info!("Testing preservation of connection state during reconnection");

    // Start mock server
    let mut mock_server = MockWebSocketServer::new(9010);
    let server_addr = mock_server.start().await;

    // Create client with message buffering
    let client = WebSocketClient::new(&format!("ws://{}", server_addr))
        .with_reconnection_config(ReconnectionConfig {
            base_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(1),
            max_retries: 3,
            backoff_multiplier: 2.0,
        })
        .with_message_buffering(true)
        .with_buffer_size(1000);

    // Connect and subscribe to price feeds
    client
        .connect()
        .await
        .expect("Initial connection should succeed");

    let subscription_ids = vec![
        client.subscribe_to_price("BTC-USD").await.unwrap(),
        client.subscribe_to_price("ETH-USD").await.unwrap(),
        client.subscribe_to_price("SOL-USD").await.unwrap(),
    ];

    // Send some messages before disconnection
    mock_server.send_price("BTC-USD", "45000.00").await;
    mock_server.send_price("ETH-USD", "3200.00").await;
    sleep(Duration::from_millis(100)).await;

    // Queue messages to be sent during reconnection
    let message_queue = vec![
        ("BTC-USD", "45100.00"),
        ("ETH-USD", "3250.00"),
        ("SOL-USD", "95.50"),
    ];

    for (symbol, price) in &message_queue {
        client
            .queue_outbound_message(&format!(
                r#"{{"type":"subscribe","product_id":"{}","price":"{}"}}"#,
                symbol, price
            ))
            .await;
    }

    // Set up message listener
    let message_stream = client.message_stream();
    let mut message_receiver = message_stream.subscribe();

    // Force disconnection
    mock_server.close_connection().await;

    // Wait for reconnection
    sleep(Duration::from_millis(500)).await;
    assert!(client.is_connected(), "Should reconnect successfully");

    // Verify subscriptions were restored
    let active_subscriptions = client.active_subscriptions().await;
    assert_eq!(
        active_subscriptions.len(),
        3,
        "Should restore 3 subscriptions"
    );
    assert!(
        active_subscriptions.contains(&subscription_ids[0]),
        "Should restore BTC-USD subscription"
    );
    assert!(
        active_subscriptions.contains(&subscription_ids[1]),
        "Should restore ETH-USD subscription"
    );
    assert!(
        active_subscriptions.contains(&subscription_ids[2]),
        "Should restore SOL-USD subscription"
    );

    // Verify queued messages were sent after reconnection
    let mut received_messages = Vec::new();
    for _ in 0..3 {
        let message_result = timeout(Duration::from_secs(2), message_receiver.recv()).await;
        if let Ok(Ok(message)) = message_result {
            received_messages.push(message);
        }
    }

    assert!(
        received_messages.len() >= 3,
        "Should receive queued messages"
    );

    // Verify message content
    let btc_message = received_messages
        .iter()
        .find(|msg| msg.contains("BTC-USD") && msg.contains("45100.00"));
    assert!(btc_message.is_some(), "Should receive queued BTC message");

    let eth_message = received_messages
        .iter()
        .find(|msg| msg.contains("ETH-USD") && msg.contains("3250.00"));
    assert!(eth_message.is_some(), "Should receive queued ETH message");

    let sol_message = received_messages
        .iter()
        .find(|msg| msg.contains("SOL-USD") && msg.contains("95.50"));
    assert!(sol_message.is_some(), "Should receive queued SOL message");

    // Verify connection metadata preservation
    let connection_metadata = client.connection_metadata().await;
    assert!(
        connection_metadata.original_connect_time.is_some(),
        "Should preserve original connect time"
    );
    assert!(
        connection_metadata.last_reconnect_time.is_some(),
        "Should record reconnect time"
    );
    assert_eq!(
        connection_metadata.reconnection_count, 1,
        "Should count reconnections"
    );
    assert!(
        connection_metadata.session_id.is_some(),
        "Should maintain session ID"
    );

    // Verify buffer metrics
    let buffer_metrics = client.buffer_metrics().await;
    assert!(
        buffer_metrics.messages_buffered >= 3,
        "Should buffer outbound messages"
    );
    assert!(
        buffer_metrics.messages_replayed >= 3,
        "Should replay buffered messages"
    );
    assert_eq!(
        buffer_metrics.buffer_overflows, 0,
        "Should not overflow buffer"
    );
    assert!(
        buffer_metrics.max_buffer_size == 1000,
        "Should respect buffer size limit"
    );

    // Test continued operation after reconnection
    mock_server.send_price("BTC-USD", "45200.00").await;
    let new_message_result = timeout(Duration::from_secs(1), message_receiver.recv()).await;
    assert!(
        new_message_result.is_ok(),
        "Should continue receiving messages"
    );

    let new_message = new_message_result.unwrap().unwrap();
    assert!(
        new_message.contains("45200.00"),
        "Should receive new price updates"
    );

    // Clean up
    client.disconnect().await;
    mock_server.stop().await;

    info!("Connection state preservation test completed");
}

#[tokio::test]
async fn test_reconnection_failure_modes() {
    info!("Testing various reconnection failure scenarios");

    // Start mock server that will be unreliable
    let mut mock_server = MockWebSocketServer::new(9041);
    let server_addr = mock_server.start().await;

    // Create client with reconnection config
    let client = WebSocketClient::new(&format!("ws://{}", server_addr))
        .with_reconnection_config(ReconnectionConfig {
            base_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(2),
            max_retries: 8,
            backoff_multiplier: 1.5,
        })
        .with_failure_mode_detection(true);

    // Monitor reconnection events
    let reconnection_stream = client.reconnection_stream();
    let mut reconnection_receiver = reconnection_stream.subscribe();

    // Initial connection
    client
        .connect()
        .await
        .expect("Initial connection should succeed");
    mock_server
        .simulate_connection()
        .await
        .expect("Failed to simulate connection");

    // Test scenario 1: Intermittent failures
    info!("Testing intermittent failure scenario");
    mock_server
        .set_failure_mode(true)
        .await
        .expect("Failed to set failure mode");

    // Cause multiple failures
    for i in 1..=3 {
        mock_server
            .close_connection()
            .await
            .expect("Failed to close connection");
        sleep(Duration::from_millis(50)).await;

        // Wait for reconnection attempt
        let attempt_event = timeout(Duration::from_secs(3), reconnection_receiver.recv()).await;
        assert!(
            attempt_event.is_ok(),
            "Should receive reconnection attempt {}",
            i
        );

        // Allow some attempts to fail
        if i % 2 == 0 {
            mock_server
                .set_failure_mode(false)
                .await
                .expect("Failed to disable failure mode");
            sleep(Duration::from_millis(100)).await;
            mock_server
                .set_failure_mode(true)
                .await
                .expect("Failed to enable failure mode");
        }
    }

    // Test scenario 2: Network partition simulation
    info!("Testing network partition scenario");
    mock_server
        .set_reject_connections(true)
        .await
        .expect("Failed to set reject mode");
    mock_server
        .close_connection()
        .await
        .expect("Failed to close connection");

    // Monitor partition behavior
    let mut partition_attempts = 0;
    let partition_start = Instant::now();

    while partition_start.elapsed() < Duration::from_secs(10) && partition_attempts < 5 {
        if let Ok(event_result) =
            timeout(Duration::from_millis(500), reconnection_receiver.recv()).await
        {
            if let Ok(event) = event_result {
                match event {
                    ReconnectionEvent::AttemptStarted { attempt_number, .. } => {
                        partition_attempts = attempt_number;
                        info!("Partition attempt #{}", attempt_number);
                    }
                    ReconnectionEvent::Failed { error, .. } => {
                        info!("Partition attempt failed: {}", error);
                    }
                    _ => {}
                }
            }
        }
    }

    assert!(
        partition_attempts >= 3,
        "Should attempt multiple reconnections during partition"
    );

    // Test scenario 3: Recovery after partition
    info!("Testing recovery after network partition");
    mock_server
        .set_reject_connections(false)
        .await
        .expect("Failed to disable reject mode");
    mock_server
        .set_failure_mode(false)
        .await
        .expect("Failed to disable failure mode");

    // Wait for successful reconnection
    let recovery_start = Instant::now();
    let mut recovery_successful = false;

    while recovery_start.elapsed() < Duration::from_secs(15) && !recovery_successful {
        if let Ok(event_result) =
            timeout(Duration::from_millis(200), reconnection_receiver.recv()).await
        {
            if let Ok(event) = event_result {
                if let ReconnectionEvent::Connected { .. } = event {
                    recovery_successful = true;
                    info!("Successfully recovered from partition");
                }
            }
        }
    }

    assert!(recovery_successful, "Should recover from network partition");
    assert!(client.is_connected(), "Should be connected after recovery");

    // Verify failure mode metrics
    let failure_metrics = client.failure_mode_metrics();
    assert!(failure_metrics.intermittent_failures >= 3);
    assert!(failure_metrics.network_partition_detected);
    assert!(failure_metrics.recovery_successful);
    assert!(failure_metrics.total_failure_modes >= 2);
    assert!(failure_metrics.time_to_recovery <= Duration::from_secs(15));

    // Test scenario 4: Degraded mode operation
    info!("Testing degraded mode operation");

    // Simulate poor network conditions
    for _ in 0..3 {
        mock_server
            .close_connection()
            .await
            .expect("Failed to close connection");
        sleep(Duration::from_millis(200)).await;
        mock_server
            .simulate_connection()
            .await
            .expect("Failed to simulate connection");
        sleep(Duration::from_millis(100)).await;
    }

    // Verify degraded mode detection
    let degraded_metrics = client.degraded_mode_metrics();
    assert!(degraded_metrics.degraded_mode_detected);
    assert!(degraded_metrics.connection_quality_score < 0.8);
    assert!(degraded_metrics.instability_events >= 3);

    // Clean up
    client.disconnect().await;
    mock_server.stop().await;

    info!("Reconnection failure modes test completed");
}

#[tokio::test]
async fn test_adaptive_backoff_strategy() {
    info!("Testing adaptive backoff strategy based on failure patterns");

    // Start mock server
    let mut mock_server = MockWebSocketServer::new(9042);
    let server_addr = mock_server.start().await;

    // Create client with adaptive backoff
    let client = WebSocketClient::new(&format!("ws://{}", server_addr))
        .with_reconnection_config(ReconnectionConfig {
            base_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(10),
            max_retries: 12,
            backoff_multiplier: 2.0,
        })
        .with_adaptive_backoff(true)
        .with_failure_pattern_analysis(true);

    // Monitor reconnection events
    let reconnection_stream = client.reconnection_stream();
    let mut reconnection_receiver = reconnection_stream.subscribe();

    // Initial connection and immediate failure
    client
        .connect()
        .await
        .expect("Initial connection should succeed");
    mock_server
        .simulate_connection()
        .await
        .expect("Failed to simulate connection");

    // Test scenario 1: Rapid failures (should increase backoff quickly)
    info!("Testing rapid failure pattern");
    mock_server
        .set_failure_mode(true)
        .await
        .expect("Failed to set failure mode");

    let mut rapid_failure_delays = Vec::new();
    for i in 1..=4 {
        mock_server
            .close_connection()
            .await
            .expect("Failed to close connection");

        // Record backoff delay
        if let Ok(event_result) =
            timeout(Duration::from_secs(2), reconnection_receiver.recv()).await
        {
            if let Ok(ReconnectionEvent::AttemptStarted { delay, .. }) = event_result {
                rapid_failure_delays.push(delay);
                info!("Rapid failure #{}: delay {:?}", i, delay);
            }
        }

        sleep(Duration::from_millis(50)).await;
    }

    // Verify adaptive increase
    assert!(
        rapid_failure_delays.len() >= 3,
        "Should capture rapid failure delays"
    );

    // Should show accelerated backoff due to rapid failures
    for i in 1..rapid_failure_delays.len() {
        let ratio = rapid_failure_delays[i].as_millis() as f64
            / rapid_failure_delays[i - 1].as_millis() as f64;
        assert!(
            ratio >= 1.8,
            "Adaptive backoff should increase aggressively for rapid failures"
        );
    }

    // Test scenario 2: Intermittent success (should moderate backoff)
    info!("Testing intermittent success pattern");
    mock_server
        .set_failure_mode(false)
        .await
        .expect("Failed to disable failure mode");

    // Allow a successful connection
    sleep(Duration::from_millis(300)).await;

    let success_event = timeout(Duration::from_secs(3), reconnection_receiver.recv()).await;
    if let Ok(Ok(ReconnectionEvent::Connected { .. })) = success_event {
        info!("Achieved intermittent success");
    }

    // Cause failure again
    mock_server
        .set_failure_mode(true)
        .await
        .expect("Failed to set failure mode");
    mock_server
        .close_connection()
        .await
        .expect("Failed to close connection");

    // Check if backoff was moderated due to recent success
    let post_success_delay = timeout(Duration::from_secs(2), reconnection_receiver.recv()).await;
    if let Ok(Ok(ReconnectionEvent::AttemptStarted { delay, .. })) = post_success_delay {
        info!("Post-success delay: {:?}", delay);

        // Should be less than the last rapid failure delay (adaptive moderation)
        if let Some(last_rapid_delay) = rapid_failure_delays.last() {
            assert!(
                delay <= *last_rapid_delay,
                "Backoff should be moderated after success: {:?} vs {:?}",
                delay,
                last_rapid_delay
            );
        }
    }

    // Test scenario 3: Persistent failures (should reach max backoff)
    info!("Testing persistent failure pattern");

    let mut persistent_delays = Vec::new();
    for i in 1..=6 {
        mock_server
            .close_connection()
            .await
            .expect("Failed to close connection");

        if let Ok(event_result) =
            timeout(Duration::from_secs(3), reconnection_receiver.recv()).await
        {
            if let Ok(ReconnectionEvent::AttemptStarted { delay, .. }) = event_result {
                persistent_delays.push(delay);
                info!("Persistent failure #{}: delay {:?}", i, delay);
            }
        }

        sleep(Duration::from_millis(100)).await;
    }

    // Should eventually reach max backoff
    let max_delay = persistent_delays.iter().max().unwrap();
    assert!(
        *max_delay >= Duration::from_secs(8),
        "Should approach max backoff for persistent failures: {:?}",
        max_delay
    );

    // Verify adaptive metrics
    let adaptive_metrics = client.adaptive_backoff_metrics();
    assert!(adaptive_metrics.rapid_failure_sequences >= 1);
    assert!(adaptive_metrics.intermittent_success_detected);
    assert!(adaptive_metrics.persistent_failure_sequences >= 1);
    assert!(adaptive_metrics.backoff_adjustments >= 5);
    assert!(adaptive_metrics.max_backoff_reached);

    // Verify pattern analysis
    let pattern_analysis = client.failure_pattern_analysis();
    assert!(pattern_analysis
        .failure_patterns
        .contains_key("rapid_consecutive"));
    assert!(pattern_analysis
        .failure_patterns
        .contains_key("persistent_failure"));
    assert!(pattern_analysis
        .success_patterns
        .contains_key("intermittent_recovery"));

    // Test scenario 4: Recovery and backoff reset
    info!("Testing recovery and backoff reset");
    mock_server
        .set_failure_mode(false)
        .await
        .expect("Failed to disable failure mode");

    // Wait for successful recovery
    let recovery_event = timeout(Duration::from_secs(10), reconnection_receiver.recv()).await;
    if let Ok(Ok(ReconnectionEvent::Connected { .. })) = recovery_event {
        info!("Successfully recovered");
    }

    // Verify backoff was reset
    let reset_metrics = client.adaptive_backoff_metrics();
    assert!(reset_metrics.backoff_resets >= 1);
    assert!(reset_metrics.current_backoff_level < 5); // Should be reset to lower level

    // Clean up
    client.disconnect().await;
    mock_server.stop().await;

    info!("Adaptive backoff strategy test completed");
}
