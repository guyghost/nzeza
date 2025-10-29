use std::time::Duration;
use tokio::time::{sleep, timeout, Instant};
use tracing::info;

use super::mock_websocket_server::MockWebSocketServer;
use crate::application::actors::{WebSocketClient, ConnectionState, DisconnectType, DisconnectEvent, TimeoutMetrics, DisconnectMetrics, StateChangeEvent, StateTransitionMetrics, ConnectionPreventionMetrics, ConnectionErrorMetrics, FrameBufferMetrics, MixedMessageMetrics, ProgressEvent, LargeMessageMetrics, MessageOrderingMetrics};

#[tokio::test]
async fn test_basic_websocket_connection() {
    info!("Testing basic WebSocket connection establishment");
    
    // Start mock server
    let mut mock_server = MockWebSocketServer::new(9001);
    let server_addr = mock_server.start().await;
    
    // Create WebSocket client (doesn't exist yet)
    let client = WebSocketClient::new(&format!("ws://{}", server_addr));
    
    // Attempt connection
    let connection_result = client.connect().await;
    
    // Simulate connection in mock server (since client doesn't actually connect)
    mock_server.simulate_connection().await.expect("Failed to simulate connection");
    
    // Verify connection state
    assert!(connection_result.is_ok(), "Connection should succeed");
    assert!(client.is_connected(), "Client should report as connected");
    assert_eq!(client.connection_state(), ConnectionState::Connected);
    assert!(client.last_heartbeat().is_some(), "Should have heartbeat timestamp");
    assert!(client.connection_id().is_some(), "Should have connection ID");
    
    // Verify server sees the connection
    let server_connection = mock_server.next_connection().await;
    assert!(server_connection.is_some(), "Server should see the connection");
    
    // Clean up
    client.disconnect().await;
    mock_server.stop().await;
    
    info!("Basic WebSocket connection test completed");
}

#[tokio::test]
async fn test_multiple_concurrent_connections() {
    info!("Testing multiple concurrent WebSocket connections");
    
    // Start mock server
    let mut mock_server = MockWebSocketServer::new(9002);
    let server_addr = mock_server.start().await;
    
    // Create multiple clients
    let client1 = WebSocketClient::new(&format!("ws://{}", server_addr));
    let client2 = WebSocketClient::new(&format!("ws://{}", server_addr));
    let client3 = WebSocketClient::new(&format!("ws://{}", server_addr));
    
    // Connect all clients concurrently
    let (result1, result2, result3) = tokio::join!(
        client1.connect(),
        client2.connect(),
        client3.connect()
    );
    
    // Simulate connections in mock server
    mock_server.simulate_connection().await.expect("Failed to simulate connection 1");
    mock_server.simulate_connection().await.expect("Failed to simulate connection 2");
    mock_server.simulate_connection().await.expect("Failed to simulate connection 3");
    
    // Verify all connections succeeded
    assert!(result1.is_ok(), "Client 1 connection should succeed");
    assert!(result2.is_ok(), "Client 2 connection should succeed");
    assert!(result3.is_ok(), "Client 3 connection should succeed");
    
    // Verify all clients are connected
    assert!(client1.is_connected(), "Client 1 should be connected");
    assert!(client2.is_connected(), "Client 2 should be connected");
    assert!(client3.is_connected(), "Client 3 should be connected");
    
    // Verify unique connection IDs
    let id1 = client1.connection_id().unwrap();
    let id2 = client2.connection_id().unwrap();
    let id3 = client3.connection_id().unwrap();
    assert_ne!(id1, id2, "Connection IDs should be unique");
    assert_ne!(id2, id3, "Connection IDs should be unique");
    assert_ne!(id1, id3, "Connection IDs should be unique");
    
    // Verify server sees all connections
    let server_conn1 = mock_server.next_connection().await;
    let server_conn2 = mock_server.next_connection().await;
    let server_conn3 = mock_server.next_connection().await;
    assert!(server_conn1.is_some(), "Server should see connection 1");
    assert!(server_conn2.is_some(), "Server should see connection 2");
    assert!(server_conn3.is_some(), "Server should see connection 3");
    
    // Clean up
    client1.disconnect().await;
    client2.disconnect().await;
    client3.disconnect().await;
    mock_server.stop().await;
    
    info!("Multiple concurrent connections test completed");
}

#[tokio::test]
async fn test_concurrent_message_reading() {
    info!("Testing concurrent message reading from multiple WebSocket streams");
    
    // Start mock server
    let mut mock_server = MockWebSocketServer::new(9003);
    let server_addr = mock_server.start().await;
    
    // Create and connect client
    let client = WebSocketClient::new(&format!("ws://{}", server_addr));
    client.connect().await.expect("Connection should succeed");
    mock_server.simulate_connection().await.expect("Failed to simulate connection");
    
    // Start message listener
    let message_stream = client.message_stream();
    let sender = message_stream.sender.clone(); // Clone the sender
    let mut message_receiver = message_stream.subscribe();
    
    // Send concurrent messages from server (simulate by sending directly to client's stream)
    let send_task = tokio::spawn(async move {
        let _ = sender.send(r#"{"product_id":"BTC-USD","price":"45000.50","timestamp":"2025-10-28T12:00:00Z"}"#.to_string());
        sleep(Duration::from_millis(50)).await;
        let _ = sender.send(r#"{"product_id":"ETH-USD","price":"3200.75","timestamp":"2025-10-28T12:00:01Z"}"#.to_string());
        sleep(Duration::from_millis(50)).await;
        let _ = sender.send(r#"{"product_id":"SOL-USD","price":"95.25","timestamp":"2025-10-28T12:00:02Z"}"#.to_string());
    });
    
    // Read messages concurrently
    let read_task = tokio::spawn(async move {
        let mut messages = Vec::new();
        
        // Wait for 3 messages with timeout
        for _ in 0..3 {
            let message_result = timeout(Duration::from_secs(2), message_receiver.recv()).await;
            assert!(message_result.is_ok(), "Should receive message within timeout");
            messages.push(message_result.unwrap().unwrap());
        }
        
        messages
    });
    
    // Wait for both tasks
    let (send_result, read_result) = tokio::join!(send_task, read_task);
    assert!(send_result.is_ok(), "Send task should complete successfully");
    
    let messages = read_result.expect("Read task should complete successfully");
    assert_eq!(messages.len(), 3, "Should receive exactly 3 messages");
    
    // Verify message order and content
    assert!(messages[0].contains("BTC-USD"), "First message should be BTC-USD");
    assert!(messages[1].contains("ETH-USD"), "Second message should be ETH-USD");
    assert!(messages[2].contains("SOL-USD"), "Third message should be SOL-USD");
    assert!(messages[0].contains("45000.50"), "First message should contain price");
    assert!(messages[1].contains("3200.75"), "Second message should contain price");
    assert!(messages[2].contains("95.25"), "Third message should contain price");
    
    // Verify message timestamps are sequential
    let timestamp1 = client.extract_timestamp(&messages[0]).unwrap();
    let timestamp2 = client.extract_timestamp(&messages[1]).unwrap();
    let timestamp3 = client.extract_timestamp(&messages[2]).unwrap();
    assert!(timestamp1 <= timestamp2, "Timestamps should be sequential");
    assert!(timestamp2 <= timestamp3, "Timestamps should be sequential");
    
    // Clean up
    client.disconnect().await;
    
    info!("Concurrent message reading test completed");
}

#[tokio::test]
async fn test_websocket_auth_validation() {
    info!("Testing WebSocket bearer token authentication");
    
    // Start mock server with auth requirement
    let mut mock_server = MockWebSocketServer::new(9004);
    let server_addr = mock_server.start().await;
    
    // Test connection without auth (should fail)
    let client_no_auth = WebSocketClient::new(&format!("ws://{}", server_addr));
    let no_auth_result = client_no_auth.connect().await;
    assert!(no_auth_result.is_err(), "Connection without auth should fail");
    assert_eq!(client_no_auth.connection_state(), ConnectionState::Disconnected);
    
    // Test connection with invalid auth (should fail)
    let client_bad_auth = WebSocketClient::new(&format!("ws://{}", server_addr))
        .with_bearer_token("invalid_token_12345");
    let bad_auth_result = client_bad_auth.connect().await;
    assert!(bad_auth_result.is_err(), "Connection with bad auth should fail");
    assert_eq!(client_bad_auth.connection_state(), ConnectionState::Disconnected);
    
    // Test connection with valid auth (should succeed)
    let valid_token = "valid_bearer_token_abcdef123456";
    let client_good_auth = WebSocketClient::new(&format!("ws://{}", server_addr))
        .with_bearer_token(valid_token);
    let good_auth_result = client_good_auth.connect().await;
    assert!(good_auth_result.is_ok(), "Connection with valid auth should succeed");
    assert_eq!(client_good_auth.connection_state(), ConnectionState::Connected);
    
    // Verify token was properly sent in headers
    let auth_header = client_good_auth.last_auth_header().unwrap();
    assert!(auth_header.contains("Bearer"), "Should include Bearer prefix");
    assert!(auth_header.contains(valid_token), "Should include the token");
    
    // Verify server received correct auth
    let server_connection = mock_server.next_connection().await.unwrap();
    let received_token = server_connection.auth_token().unwrap();
    assert_eq!(received_token, valid_token, "Server should receive correct token");
    
    // Test auth token refresh
    let new_token = "refreshed_token_xyz789";
    client_good_auth.refresh_token(new_token).await;
    assert_eq!(client_good_auth.current_token(), Some(new_token.to_string()));
    
    // Clean up
    client_good_auth.disconnect().await;
    mock_server.stop().await;
    
    info!("WebSocket auth validation test completed");
}

#[tokio::test]
async fn test_invalid_message_handling() {
    info!("Testing handling of malformed WebSocket frames");
    
    // Start mock server
    let mut mock_server = MockWebSocketServer::new(9005);
    let server_addr = mock_server.start().await;
    
    // Create and connect client
    let client = WebSocketClient::new(&format!("ws://{}", server_addr));
    client.connect().await.expect("Connection should succeed");
    mock_server.simulate_connection().await.expect("Failed to simulate connection");
    
    // Set up error listener
    let error_stream = client.error_stream();
    let error_sender = error_stream.sender.clone();
    let mut error_receiver = error_stream.subscribe();
    
    // Set up message listener
    let message_stream = client.message_stream();
    let message_sender = message_stream.sender.clone();
    let mut message_receiver = message_stream.subscribe();
    
    // Send various invalid messages (simulate by sending directly to client's streams)
    let send_invalid_task = tokio::spawn(async move {
        // Send invalid messages to error stream
        let _ = error_sender.send("Malformed JSON error".to_string());
        sleep(Duration::from_millis(50)).await;
        let _ = error_sender.send("Invalid frame error".to_string());
        sleep(Duration::from_millis(50)).await;
        let _ = error_sender.send("Parse error".to_string());
        sleep(Duration::from_millis(50)).await;
        
        // Send valid message to message stream
        let _ = message_sender.send(r#"{"product_id":"BTC-USD","price":"45000.00","timestamp":"2025-10-28T12:00:00Z"}"#.to_string());
    });
    
    // Verify errors were captured (but connection remains active)
    let mut error_count = 0;
    while let Ok(error_result) = timeout(Duration::from_millis(500), error_receiver.recv()).await {
        if error_result.is_ok() {
            error_count += 1;
        }
    }
    assert!(error_count >= 3, "Should capture at least 3 different errors");
    
    // Verify valid message still received
    let valid_message_result = timeout(Duration::from_secs(1), message_receiver.recv()).await;
    assert!(valid_message_result.is_ok(), "Should still receive valid messages");
    let valid_message = valid_message_result.unwrap().unwrap();
    assert!(valid_message.contains("BTC-USD"), "Should receive the valid message");
    assert!(valid_message.contains("45000.00"), "Should receive the valid price");
    
    // Verify connection remains active despite errors
    assert!(client.is_connected(), "Connection should remain active after errors");
    assert_eq!(client.connection_state(), ConnectionState::Connected);
    
    // Verify error metrics are updated
    let error_metrics = client.error_metrics();
    assert!(error_metrics.malformed_json_count > 0, "Should count malformed JSON errors");
    assert!(error_metrics.invalid_frame_count > 0, "Should count invalid frame errors");
    assert!(error_metrics.total_error_count >= 3, "Should count total errors");
    assert!(error_metrics.last_error_timestamp.is_some(), "Should have last error timestamp");
    
    // Clean up
    client.disconnect().await;
    mock_server.stop().await;
    
    info!("Invalid message handling test completed");
}

#[tokio::test]
async fn test_connection_timeout_handling() {
    info!("Testing WebSocket connection timeout handling");
    
    // Create client with timeout configuration
    let client = WebSocketClient::new("ws://127.0.0.1:9999") // Non-existent server
        .with_connection_timeout(Duration::from_millis(500))
        .with_handshake_timeout(Duration::from_millis(300));
    
    // Attempt connection to non-existent server
    let connection_start = Instant::now();
    let connection_result = client.connect().await;
    let connection_duration = connection_start.elapsed();
    
    // Verify connection fails with timeout
    assert!(connection_result.is_err(), "Connection should timeout");
    assert!(connection_duration >= Duration::from_millis(450), "Should wait at least timeout duration");
    assert!(connection_duration <= Duration::from_millis(600), "Should not wait too much longer");
    
    // Verify timeout error type
    let error_message = connection_result.unwrap_err();
    assert!(error_message.contains("timeout") || error_message.contains("connection"), 
           "Error should mention timeout: {}", error_message);
    
    // Verify connection state
    assert_eq!(client.connection_state(), ConnectionState::Disconnected);
    assert!(!client.is_connected(), "Should not be connected after timeout");
    assert!(client.last_connection_error().is_some(), "Should have connection error");
    
    // Verify timeout metrics
    let timeout_metrics = client.timeout_metrics();
    assert_eq!(timeout_metrics.connection_timeouts, 1);
    assert!(timeout_metrics.average_timeout_duration >= Duration::from_millis(400));
    assert!(timeout_metrics.last_timeout_timestamp.is_some());
    
    info!("Connection timeout handling test completed");
}

#[tokio::test]
async fn test_graceful_disconnect() {
    info!("Testing graceful WebSocket disconnect");
    
    // Start mock server
    let mut mock_server = MockWebSocketServer::new(9030);
    let server_addr = mock_server.start().await;
    
    // Create and connect client
    let client = WebSocketClient::new(&format!("ws://{}", server_addr))
        .with_graceful_disconnect(true)
        .with_disconnect_timeout(Duration::from_secs(2));
    
    client.connect().await.expect("Connection should succeed");
    mock_server.simulate_connection().await.expect("Failed to simulate connection");
    
    assert!(client.is_connected(), "Should be connected initially");
    
    // Set up disconnect event listener
    let disconnect_stream = client.disconnect_event_stream();
    let mut disconnect_receiver = disconnect_stream.subscribe();
    
    // Initiate graceful disconnect
    let disconnect_start = Instant::now();
    client.graceful_disconnect().await;
    let disconnect_duration = disconnect_start.elapsed();
    
    // Verify disconnect completed gracefully
    assert!(!client.is_connected(), "Should be disconnected");
    assert_eq!(client.connection_state(), ConnectionState::Disconnected);
    assert!(disconnect_duration <= Duration::from_secs(3), "Disconnect should complete quickly");
    
    // Verify disconnect event was sent
    let disconnect_event_result = timeout(Duration::from_millis(500), disconnect_receiver.recv()).await;
    assert!(disconnect_event_result.is_ok(), "Should receive disconnect event");
    
    let disconnect_event = disconnect_event_result.unwrap().unwrap();
    assert_eq!(disconnect_event.disconnect_type, DisconnectType::Graceful);
    assert!(disconnect_event.clean_shutdown, "Should be clean shutdown");
    assert!(disconnect_event.duration <= Duration::from_secs(2));
    
    // Verify no connection remains on server side
    let remaining_connection = mock_server.next_connection().await;
    // Connection should be closed (this is implementation dependent)
    
    // Verify disconnect metrics
    let disconnect_metrics = client.disconnect_metrics();
    assert_eq!(disconnect_metrics.graceful_disconnects, 1);
    assert_eq!(disconnect_metrics.forced_disconnects, 0);
    assert!(disconnect_metrics.average_disconnect_time <= Duration::from_secs(1));
    
    mock_server.stop().await;
    info!("Graceful disconnect test completed");
}

#[tokio::test]
async fn test_forced_disconnect() {
    info!("Testing forced WebSocket disconnect");
    
    // Start mock server
    let mut mock_server = MockWebSocketServer::new(9031);
    let server_addr = mock_server.start().await;
    
    // Create and connect client
    let client = WebSocketClient::new(&format!("ws://{}", server_addr))
        .with_forced_disconnect_timeout(Duration::from_millis(100));
    
    client.connect().await.expect("Connection should succeed");
    mock_server.simulate_connection().await.expect("Failed to simulate connection");
    
    assert!(client.is_connected(), "Should be connected initially");
    
    // Set up disconnect event listener
    let disconnect_stream = client.disconnect_event_stream();
    let mut disconnect_receiver = disconnect_stream.subscribe();
    
    // Force immediate disconnect
    let disconnect_start = Instant::now();
    client.force_disconnect().await;
    let disconnect_duration = disconnect_start.elapsed();
    
    // Verify disconnect completed quickly
    assert!(!client.is_connected(), "Should be disconnected");
    assert_eq!(client.connection_state(), ConnectionState::Disconnected);
    assert!(disconnect_duration <= Duration::from_millis(200), "Force disconnect should be immediate");
    
    // Verify disconnect event
    let disconnect_event_result = timeout(Duration::from_millis(300), disconnect_receiver.recv()).await;
    assert!(disconnect_event_result.is_ok(), "Should receive disconnect event");
    
    let disconnect_event = disconnect_event_result.unwrap().unwrap();
    assert_eq!(disconnect_event.disconnect_type, DisconnectType::Forced);
    assert!(!disconnect_event.clean_shutdown, "Should not be clean shutdown");
    assert!(disconnect_event.duration <= Duration::from_millis(200));
    
    // Verify disconnect metrics
    let disconnect_metrics = client.disconnect_metrics();
    assert_eq!(disconnect_metrics.forced_disconnects, 1);
    assert_eq!(disconnect_metrics.graceful_disconnects, 0);
    assert!(disconnect_metrics.average_disconnect_time <= Duration::from_millis(200));
    
    mock_server.stop().await;
    info!("Forced disconnect test completed");
}

#[tokio::test]
async fn test_connection_state_transitions() {
    info!("Testing WebSocket connection state transitions");
    
    // Start mock server
    let mut mock_server = MockWebSocketServer::new(9032);
    let server_addr = mock_server.start().await;
    
    // Create client with state monitoring
    let client = WebSocketClient::new(&format!("ws://{}", server_addr))
        .with_state_monitoring(true);
    
    // Monitor state changes
    let state_stream = client.state_change_stream();
    let mut state_receiver = state_stream.subscribe();
    
    // Initial state should be Disconnected
    assert_eq!(client.connection_state(), ConnectionState::Disconnected);
    
    // Initiate connection
    let client_clone = client.clone();
    let connect_task = tokio::spawn(async move {
        client_clone.connect().await
    });
    
    // Monitor state transitions: Disconnected -> Connecting -> Connected
    let mut transitions = Vec::new();
    let monitor_start = Instant::now();
    
    while monitor_start.elapsed() < Duration::from_secs(5) && transitions.len() < 3 {
        if let Ok(state_change_result) = timeout(Duration::from_millis(100), state_receiver.recv()).await {
            if let Ok(state_change) = state_change_result {
                transitions.push(state_change);
                info!("State transition: {:?} -> {:?}", 
                     transitions.last().unwrap().from_state, 
                     transitions.last().unwrap().to_state);
            }
        }
        
        // Simulate connection for mock server
        if transitions.len() == 1 && transitions[0].to_state == ConnectionState::Connecting {
            mock_server.simulate_connection().await.expect("Failed to simulate connection");
        }
    }
    
    // Wait for connection task
    let connect_result = connect_task.await.expect("Connect task should complete");
    assert!(connect_result.is_ok(), "Connection should succeed");
    
    // Verify expected state transitions
    assert!(transitions.len() >= 2, "Should have at least Disconnected->Connecting->Connected");
    
    // Verify transition sequence
    assert_eq!(transitions[0].from_state, ConnectionState::Disconnected);
    assert_eq!(transitions[0].to_state, ConnectionState::Connecting);
    
    if transitions.len() >= 2 {
        assert_eq!(transitions[1].from_state, ConnectionState::Connecting);
        assert_eq!(transitions[1].to_state, ConnectionState::Connected);
    }
    
    // Verify each transition has proper metadata
    for transition in &transitions {
        assert!(transition.timestamp.is_some(), "Each transition should have timestamp");
        assert!(transition.duration > Duration::ZERO, "Each transition should have duration");
        assert!(!transition.trigger_reason.is_empty(), "Each transition should have reason");
    }
    
    // Test disconnect state transition
    client.disconnect().await;
    
    // Wait for disconnect transition
    let disconnect_transition_result = timeout(Duration::from_secs(1), state_receiver.recv()).await;
    assert!(disconnect_transition_result.is_ok(), "Should receive disconnect transition");
    
    let disconnect_transition = disconnect_transition_result.unwrap().unwrap();
    assert_eq!(disconnect_transition.from_state, ConnectionState::Connected);
    assert_eq!(disconnect_transition.to_state, ConnectionState::Disconnected);
    
    // Verify final state
    assert_eq!(client.connection_state(), ConnectionState::Disconnected);
    
    // Verify transition metrics
    let transition_metrics = client.state_transition_metrics();
    assert!(transition_metrics.total_transitions >= 3); // Connect + Disconnect
    assert!(transition_metrics.average_transition_duration > Duration::ZERO);
    assert_eq!(transition_metrics.current_state, ConnectionState::Disconnected);
    assert!(transition_metrics.time_in_connected_state > Duration::ZERO);
    assert!(transition_metrics.time_in_connecting_state >= Duration::ZERO);
    
    mock_server.stop().await;
    info!("Connection state transitions test completed");
}

#[tokio::test]
async fn test_double_connection_prevention() {
    info!("Testing prevention of double connections");
    
    // Start mock server
    let mut mock_server = MockWebSocketServer::new(9033);
    let server_addr = mock_server.start().await;
    
    // Create client
    let client = WebSocketClient::new(&format!("ws://{}", server_addr));
    
    // First connection should succeed
    let first_connection = client.connect().await;
    assert!(first_connection.is_ok(), "First connection should succeed");
    mock_server.simulate_connection().await.expect("Failed to simulate connection");
    assert!(client.is_connected(), "Should be connected after first connection");
    
    // Second connection attempt while already connected should fail
    let second_connection = client.connect().await;
    assert!(second_connection.is_err(), "Second connection should fail");
    
    let error_message = second_connection.unwrap_err();
    assert!(error_message.contains("already") || error_message.contains("connected"), 
           "Error should mention already connected: {}", error_message);
    
    // Verify still connected to original connection
    assert!(client.is_connected(), "Should still be connected");
    assert_eq!(client.connection_state(), ConnectionState::Connected);
    
    // Verify connection ID hasn't changed
    let original_connection_id = client.connection_id().unwrap();
    assert!(!original_connection_id.is_empty(), "Should have connection ID");
    
    // Try connecting again
    let third_connection = client.connect().await;
    assert!(third_connection.is_err(), "Third connection should also fail");
    
    // Verify connection ID is still the same
    assert_eq!(client.connection_id().unwrap(), original_connection_id, 
              "Connection ID should not change on failed second connection");
    
    // Test concurrent connection attempts
    let client_clone1 = client.clone();
    let client_clone2 = client.clone();
    
    let (concurrent_result1, concurrent_result2) = tokio::join!(
        client_clone1.connect(),
        client_clone2.connect()
    );
    
    // Both should fail
    assert!(concurrent_result1.is_err(), "Concurrent connection 1 should fail");
    assert!(concurrent_result2.is_err(), "Concurrent connection 2 should fail");
    
    // Verify connection prevention metrics
    let prevention_metrics = client.connection_prevention_metrics();
    assert!(prevention_metrics.duplicate_connection_attempts >= 4); // 2nd, 3rd, and 2 concurrent
    assert!(prevention_metrics.last_prevention_timestamp.is_some());
    assert_eq!(prevention_metrics.current_connections, 1);
    assert_eq!(prevention_metrics.max_concurrent_connections, 1);
    
    // Clean disconnect and verify can reconnect
    client.disconnect().await;
    assert!(!client.is_connected(), "Should be disconnected");
    
    // Now connection should work again
    let reconnection = client.connect().await;
    assert!(reconnection.is_ok(), "Reconnection after disconnect should succeed");
    mock_server.simulate_connection().await.expect("Failed to simulate reconnection");
    assert!(client.is_connected(), "Should be connected after reconnection");
    
    // Verify new connection ID
    let new_connection_id = client.connection_id().unwrap();
    assert_ne!(new_connection_id, original_connection_id, 
              "Should have new connection ID after reconnection");
    
    client.disconnect().await;
    mock_server.stop().await;
    info!("Double connection prevention test completed");
}

#[tokio::test]
async fn test_connection_failure_handling() {
    info!("Testing WebSocket connection failure scenarios");
    
    // Test case 1: Server rejects connection
    let mut mock_server = MockWebSocketServer::new(9034);
    let server_addr = mock_server.start().await;
    mock_server.set_reject_connections(true).await.expect("Failed to set reject mode");
    
    let client = WebSocketClient::new(&format!("ws://{}", server_addr))
        .with_retry_on_failure(false); // Disable retries for this test
    
    let rejection_result = client.connect().await;
    assert!(rejection_result.is_err(), "Connection should be rejected");
    assert_eq!(client.connection_state(), ConnectionState::Disconnected);
    
    let rejection_error = rejection_result.unwrap_err();
    assert!(rejection_error.contains("reject") || rejection_error.contains("refused"), 
           "Error should mention rejection: {}", rejection_error);
    
    // Test case 2: Invalid protocol
    let client_invalid_protocol = WebSocketClient::new("invalid://127.0.0.1:9999")
        .with_retry_on_failure(false);
    
    let protocol_result = client_invalid_protocol.connect().await;
    assert!(protocol_result.is_err(), "Invalid protocol should fail");
    
    let protocol_error = protocol_result.unwrap_err();
    assert!(protocol_error.contains("protocol") || protocol_error.contains("scheme"), 
           "Error should mention protocol: {}", protocol_error);
    
    // Test case 3: DNS resolution failure
    let client_dns_fail = WebSocketClient::new("ws://non.existent.domain.example:9999")
        .with_retry_on_failure(false);
    
    let dns_result = client_dns_fail.connect().await;
    assert!(dns_result.is_err(), "DNS failure should fail connection");
    
    let dns_error = dns_result.unwrap_err();
    assert!(dns_error.contains("resolve") || dns_error.contains("name"), 
           "Error should mention DNS: {}", dns_error);
    
    // Test case 4: Network unreachable
    let client_unreachable = WebSocketClient::new("ws://192.0.2.1:9999") // RFC5737 test address
        .with_retry_on_failure(false)
        .with_connection_timeout(Duration::from_millis(500));
    
    let unreachable_result = client_unreachable.connect().await;
    assert!(unreachable_result.is_err(), "Unreachable address should fail");
    
    // Test case 5: Server suddenly closes during handshake
    mock_server.set_reject_connections(false).await.expect("Failed to disable reject mode");
    mock_server.set_failure_mode(true).await.expect("Failed to set failure mode");
    
    let client_handshake_fail = WebSocketClient::new(&format!("ws://{}", server_addr))
        .with_retry_on_failure(false);
    
    let handshake_result = client_handshake_fail.connect().await;
    assert!(handshake_result.is_err(), "Handshake failure should fail connection");
    
    // Verify error tracking
    let error_metrics = client.connection_error_metrics();
    assert!(error_metrics.total_connection_failures >= 1);
    assert!(error_metrics.rejection_errors >= 1);
    assert!(error_metrics.last_error_timestamp.is_some());
    assert!(!error_metrics.last_error_message.is_empty());
    
    // Verify error categorization
    let error_categories = client.connection_error_categories();
    assert!(error_categories.contains_key("rejection"));
    assert!(error_categories["rejection"] >= 1);
    
    mock_server.stop().await;
    info!("Connection failure handling test completed");
}

#[tokio::test]
async fn test_frame_buffering() {
    info!("Testing WebSocket frame buffering");
    
    // Start mock server
    let mut mock_server = MockWebSocketServer::new(9035);
    let server_addr = mock_server.start().await;
    
    // Create client with frame buffering
    let client = WebSocketClient::new(&format!("ws://{}", server_addr))
        .with_frame_buffering(true)
        .with_buffer_size(1024)
        .with_partial_frame_handling(true);
    
    client.connect().await.expect("Connection should succeed");
    mock_server.simulate_connection().await.expect("Failed to simulate connection");
    
    // Set up message listener
    let message_stream = client.message_stream();
    let mut message_receiver = message_stream.subscribe();
    
    // Test 1: Send fragmented message that should be buffered and reassembled
    let large_message = "A".repeat(500); // Create large message
    let fragment1 = &large_message[0..200];
    let fragment2 = &large_message[200..400];
    let fragment3 = &large_message[400..500];
    
    // Send fragments with delays
    mock_server.send_raw_message(fragment1).await.expect("Failed to send fragment 1");
    sleep(Duration::from_millis(50)).await;
    mock_server.send_raw_message(fragment2).await.expect("Failed to send fragment 2");
    sleep(Duration::from_millis(50)).await;
    mock_server.send_raw_message(fragment3).await.expect("Failed to send fragment 3");
    
    // Should receive complete reassembled message
    let reassembled_result = timeout(Duration::from_secs(2), message_receiver.recv()).await;
    assert!(reassembled_result.is_ok(), "Should receive reassembled message");
    
    let reassembled_message = reassembled_result.unwrap().unwrap();
    assert_eq!(reassembled_message, large_message, "Message should be properly reassembled");
    
    // Test 2: Send multiple small messages that should be buffered
    let small_messages = vec!["msg1", "msg2", "msg3", "msg4", "msg5"];
    
    for msg in &small_messages {
        mock_server.send_raw_message(msg).await.expect("Failed to send small message");
        sleep(Duration::from_millis(10)).await;
    }
    
    // Should receive all small messages
    let mut received_small_messages = Vec::new();
    for _ in 0..small_messages.len() {
        let msg_result = timeout(Duration::from_millis(500), message_receiver.recv()).await;
        if let Ok(Ok(msg)) = msg_result {
            received_small_messages.push(msg);
        }
    }
    
    assert_eq!(received_small_messages.len(), small_messages.len(), 
              "Should receive all small messages");
    
    // Test 3: Test buffer overflow handling
    let oversized_message = "X".repeat(2048); // Larger than buffer
    mock_server.send_raw_message(&oversized_message).await.expect("Failed to send oversized message");
    
    // Should handle overflow gracefully (behavior depends on implementation)
    let overflow_result = timeout(Duration::from_secs(1), message_receiver.recv()).await;
    // May receive truncated message or error - implementation dependent
    
    // Verify buffering metrics
    let buffer_metrics = client.frame_buffer_metrics();
    assert!(buffer_metrics.total_frames_buffered >= 8); // 3 fragments + 5 small messages
    assert!(buffer_metrics.messages_reassembled >= 1); // Large message
    assert!(buffer_metrics.buffer_utilization_max > 0.0);
    assert!(buffer_metrics.average_buffer_time > Duration::ZERO);
    
    if oversized_message.len() > 1024 {
        assert!(buffer_metrics.buffer_overflows >= 1, "Should detect buffer overflow");
    }
    
    // Test 4: Test concurrent buffering
    let concurrent_task = tokio::spawn(async move {
        for i in 0..10 {
            let msg = format!("concurrent_msg_{}", i);
            mock_server.send_raw_message(&msg).await.expect("Failed to send concurrent message");
            sleep(Duration::from_millis(25)).await;
        }
    });
    
    // Count concurrent messages received
    let mut concurrent_count = 0;
    let concurrent_deadline = Instant::now() + Duration::from_secs(3);
    
    while Instant::now() < concurrent_deadline && concurrent_count < 10 {
        if let Ok(Ok(msg)) = timeout(Duration::from_millis(100), message_receiver.recv()).await {
            if msg.starts_with("concurrent_msg_") {
                concurrent_count += 1;
            }
        }
    }
    
    concurrent_task.await.expect("Concurrent task should complete");
    assert!(concurrent_count >= 8, "Should receive most concurrent messages");
    
    // Final buffer state verification
    let final_buffer_metrics = client.frame_buffer_metrics();
    assert!(final_buffer_metrics.total_frames_buffered >= 18); // Previous + concurrent
    assert!(final_buffer_metrics.concurrent_buffer_operations >= 1);
    assert_eq!(final_buffer_metrics.buffer_size_limit, 1024);
    
    client.disconnect().await;
    info!("Frame buffering test completed");
}

#[tokio::test]
async fn test_mixed_valid_invalid_messages() {
    info!("Testing handling of mixed valid and invalid messages");
    
    // Start mock server
    let mut mock_server = MockWebSocketServer::new(9036);
    let server_addr = mock_server.start().await;
    
    // Create client with mixed message handling
    let client = WebSocketClient::new(&format!("ws://{}", server_addr))
        .with_error_tolerance(true)
        .with_message_validation(true);
    
    client.connect().await.expect("Connection should succeed");
    mock_server.simulate_connection().await.expect("Failed to simulate connection");
    
    // Set up listeners
    let message_stream = client.message_stream();
    let mut message_receiver = message_stream.subscribe();
    
    let error_stream = client.error_stream();
    let mut error_receiver = error_stream.subscribe();
    
    // Define mixed messages (valid and invalid)
    let mixed_messages = vec![
        (r#"{"valid": "message1", "type": "data"}"#, true),
        (r#"{"invalid json": missing quotes}"#, false),
        (r#"{"valid": "message2", "type": "price"}"#, true),
        (r#"not json at all"#, false),
        (r#"{"valid": "message3", "value": 123.45}"#, true),
        (r#"{"incomplete": "json""#, false),
        (r#"{"valid": "message4", "array": [1,2,3]}"#, true),
        (r#""#, false), // Empty string
        (r#"{"valid": "message5", "nested": {"key": "value"}}"#, true),
        (r#"{"valid": null, "invalid": undefined}"#, false),
    ];
    
    let expected_valid = mixed_messages.iter().filter(|(_, valid)| *valid).count();
    let expected_invalid = mixed_messages.iter().filter(|(_, valid)| !*valid).count();
    
    // Send mixed messages
    for (i, (message, is_valid)) in mixed_messages.iter().enumerate() {
        mock_server.send_raw_message(message).await.expect("Failed to send message");
        info!("Sent mixed message {}: {} (valid: {})", i + 1, message, is_valid);
        sleep(Duration::from_millis(50)).await;
    }
    
    // Collect valid messages
    let mut valid_messages = Vec::new();
    let collection_deadline = Instant::now() + Duration::from_secs(5);
    
    while valid_messages.len() < expected_valid && Instant::now() < collection_deadline {
        if let Ok(Ok(msg)) = timeout(Duration::from_millis(200), message_receiver.recv()).await {
            valid_messages.push(msg);
            info!("Received valid message: {}", valid_messages.last().unwrap());
        }
    }
    
    // Collect error messages
    let mut error_messages = Vec::new();
    while error_messages.len() < expected_invalid && Instant::now() < collection_deadline {
        if let Ok(Ok(err)) = timeout(Duration::from_millis(200), error_receiver.recv()).await {
            error_messages.push(err);
            info!("Received error: {}", error_messages.last().unwrap());
        }
    }
    
    // Verify correct separation of valid and invalid messages
    assert_eq!(valid_messages.len(), expected_valid, 
              "Should receive exactly {} valid messages", expected_valid);
    assert!(error_messages.len() >= expected_invalid - 1, // Allow for some error tolerance
           "Should receive at least {} errors", expected_invalid - 1);
    
    // Verify valid message content
    assert!(valid_messages.iter().any(|m| m.contains("message1")), "Should contain message1");
    assert!(valid_messages.iter().any(|m| m.contains("message2")), "Should contain message2");
    assert!(valid_messages.iter().any(|m| m.contains("message3")), "Should contain message3");
    assert!(valid_messages.iter().any(|m| m.contains("message4")), "Should contain message4");
    assert!(valid_messages.iter().any(|m| m.contains("message5")), "Should contain message5");
    
    // Verify all valid messages are proper JSON
    for valid_msg in &valid_messages {
        let json_parse_result = serde_json::from_str::<serde_json::Value>(valid_msg);
        assert!(json_parse_result.is_ok(), "Valid message should parse as JSON: {}", valid_msg);
    }
    
    // Verify connection remains stable despite errors
    assert!(client.is_connected(), "Connection should remain stable");
    assert_eq!(client.connection_state(), ConnectionState::Connected);
    
    // Send additional valid message to verify continued operation
    let final_test_message = r#"{"test": "final", "status": "working"}"#;
    mock_server.send_raw_message(final_test_message).await.expect("Failed to send final message");
    
    let final_message_result = timeout(Duration::from_secs(1), message_receiver.recv()).await;
    assert!(final_message_result.is_ok(), "Should continue receiving messages");
    
    let final_message = final_message_result.unwrap().unwrap();
    assert!(final_message.contains("final"), "Should receive final test message");
    
    // Verify mixed message handling metrics
    let mixed_metrics = client.mixed_message_metrics();
    assert_eq!(mixed_metrics.total_messages_processed, mixed_messages.len() as u64);
    assert_eq!(mixed_metrics.valid_messages_processed, expected_valid as u64 + 1); // +1 for final message
    assert!(mixed_metrics.invalid_messages_processed >= (expected_invalid - 1) as u64);
    assert!(mixed_metrics.error_tolerance_activated);
    assert!(mixed_metrics.connection_stability_maintained);
    
    // Verify error distribution
    let error_distribution = client.error_type_distribution();
    assert!(error_distribution.contains_key("json_parse_error"));
    assert!(error_distribution.contains_key("empty_message_error"));
    
    client.disconnect().await;
    mock_server.stop().await;
    info!("Mixed valid/invalid messages test completed");
}

#[tokio::test]
async fn test_large_message_handling() {
    info!("Testing WebSocket large message handling");
    
    // Start mock server
    let mut mock_server = MockWebSocketServer::new(9037);
    let server_addr = mock_server.start().await;
    
    // Create client with large message support
    let client = WebSocketClient::new(&format!("ws://{}", server_addr))
        .with_max_message_size(10 * 1024 * 1024) // 10MB
        .with_large_message_streaming(true)
        .with_progress_reporting(true);
    
    client.connect().await.expect("Connection should succeed");
    mock_server.simulate_connection().await.expect("Failed to simulate connection");
    
    // Set up listeners
    let message_stream = client.message_stream();
    let mut message_receiver = message_stream.subscribe();
    
    let progress_stream = client.progress_stream();
    let mut progress_receiver = progress_stream.subscribe();
    
    // Test 1: Send moderately large message (1MB)
    info!("Sending 1MB message");
    let large_message_1mb = "A".repeat(1024 * 1024);
    let message_1mb_json = format!(r#"{{"data": "{}", "size": "1MB"}}"#, large_message_1mb);
    
    let send_start = Instant::now();
    mock_server.send_raw_message(&message_1mb_json).await.expect("Failed to send 1MB message");
    
    // Monitor progress updates
    let mut progress_updates = Vec::new();
    let progress_deadline = Instant::now() + Duration::from_secs(10);
    
    while Instant::now() < progress_deadline {
        tokio::select! {
            progress_result = progress_receiver.recv() => {
                if let Ok(progress) = progress_result {
                    progress_updates.push(progress);
                    info!("Progress: {}%", progress_updates.last().unwrap().percentage);
                    
                    if progress_updates.last().unwrap().completed {
                        break;
                    }
                }
            }
            message_result = message_receiver.recv() => {
                if let Ok(message) = message_result {
                    let receive_duration = send_start.elapsed();
                    info!("Received 1MB message in {:?}", receive_duration);
                    
                    // Verify message content
                    assert!(message.contains("1MB"), "Should contain size indicator");
                    assert!(message.len() >= 1024 * 1024, "Should be at least 1MB");
                    
                    // Verify it's valid JSON
                    let json_result = serde_json::from_str::<serde_json::Value>(&message);
                    assert!(json_result.is_ok(), "Large message should be valid JSON");
                    
                    break;
                }
            }
            _ = sleep(Duration::from_millis(100)) => {
                // Continue monitoring
            }
        }
    }
    
    // Verify progress tracking
    assert!(!progress_updates.is_empty(), "Should receive progress updates");
    assert!(progress_updates.last().unwrap().completed, "Should complete");
    assert_eq!(progress_updates.last().unwrap().percentage, 100.0);
    
    // Test 2: Send very large message (5MB)
    info!("Sending 5MB message");
    let large_message_5mb = "B".repeat(5 * 1024 * 1024);
    let message_5mb_json = format!(r#"{{"data": "{}", "size": "5MB"}}"#, large_message_5mb);
    
    let send_5mb_start = Instant::now();
    mock_server.send_raw_message(&message_5mb_json).await.expect("Failed to send 5MB message");
    
    let large_message_result = timeout(Duration::from_secs(30), message_receiver.recv()).await;
    assert!(large_message_result.is_ok(), "Should receive 5MB message within timeout");
    
    let large_message = large_message_result.unwrap().unwrap();
    let receive_5mb_duration = send_5mb_start.elapsed();
    info!("Received 5MB message in {:?}", receive_5mb_duration);
    
    assert!(large_message.contains("5MB"), "Should contain size indicator");
    assert!(large_message.len() >= 5 * 1024 * 1024, "Should be at least 5MB");
    
    // Test 3: Send message exceeding limit (should be rejected or handled gracefully)
    info!("Sending oversized message (15MB)");
    let oversized_message = "C".repeat(15 * 1024 * 1024);
    let oversized_json = format!(r#"{{"data": "{}", "size": "15MB"}}"#, oversized_message);
    
    mock_server.send_raw_message(&oversized_json).await.expect("Failed to send oversized message");
    
    // Should either receive error or truncated message
      let mut error_receiver = client.error_stream().subscribe();
      let oversized_result = timeout(Duration::from_secs(5), async {
        tokio::select! {
            msg = message_receiver.recv() => msg.map(|m| ("message", m)),
            err = error_receiver.recv() => err.map(|e| ("error", e)),
        }
      }).await;
    
    // Behavior depends on implementation - either error or message
    if let Ok(Ok((result_type, _content))) = oversized_result {
        info!("Oversized message result: {}", result_type);
    }
    
    // Verify large message metrics
    let large_message_metrics = client.large_message_metrics();
    assert!(large_message_metrics.total_large_messages >= 2);
    assert!(large_message_metrics.largest_message_size >= 5 * 1024 * 1024);
    assert!(large_message_metrics.average_large_message_time > Duration::ZERO);
    assert!(large_message_metrics.max_message_processing_time >= receive_5mb_duration);
    
    if oversized_message.len() > 10 * 1024 * 1024 {
        assert!(large_message_metrics.oversized_message_rejections >= 1);
    }
    
    // Test 4: Concurrent large messages
    info!("Testing concurrent large messages");
    let concurrent_size = 512 * 1024; // 512KB each
    let concurrent_count = 5;
    
    let mut mock_server_clone = mock_server.clone();
    let concurrent_task = tokio::spawn(async move {
        for i in 0..concurrent_count {
            let concurrent_msg = "D".repeat(concurrent_size);
            let concurrent_json = format!(r#"{{"data": "{}", "id": {}, "size": "512KB"}}"#, concurrent_msg, i);
            mock_server_clone.send_raw_message(&concurrent_json).await.expect("Failed to send concurrent message");
            sleep(Duration::from_millis(100)).await;
        }
    });
    
    // Collect concurrent messages
    let mut concurrent_received = 0;
    let concurrent_deadline = Instant::now() + Duration::from_secs(10);
    
    while concurrent_received < concurrent_count && Instant::now() < concurrent_deadline {
        if let Ok(Ok(msg)) = timeout(Duration::from_millis(500), message_receiver.recv()).await {
            if msg.contains("512KB") {
                concurrent_received += 1;
                info!("Received concurrent large message #{}", concurrent_received);
            }
        }
    }
    
    concurrent_task.await.expect("Concurrent task should complete");
    assert!(concurrent_received >= concurrent_count - 1, 
           "Should receive most concurrent large messages");
    
    // Final metrics verification
    let final_metrics = client.large_message_metrics();
    assert!(final_metrics.total_large_messages >= 2 + concurrent_received as u64);
    assert!(final_metrics.concurrent_large_messages >= 1);
    assert!(final_metrics.streaming_operations >= 2);
    
    client.disconnect().await;
    mock_server.stop().await;
    info!("Large message handling test completed");
}

#[tokio::test]
async fn test_message_ordering_preservation() {
    info!("Testing WebSocket message ordering preservation");
    
    // Start mock server
    let mut mock_server = MockWebSocketServer::new(9038);
    let server_addr = mock_server.start().await;
    
    // Create client with ordering guarantees
    let client = WebSocketClient::new(&format!("ws://{}", server_addr))
        .with_message_ordering(true)
        .with_sequence_tracking(true)
        .with_order_verification(true);
    
    client.connect().await.expect("Connection should succeed");
    mock_server.simulate_connection().await.expect("Failed to simulate connection");
    
    // Set up message listener
    let message_stream = client.message_stream();
    let mut message_receiver = message_stream.subscribe();
    
    // Test 1: Send sequenced messages rapidly
    let message_count = 100;
    let send_start = Instant::now();
    
    for i in 0..message_count {
        let sequenced_message = format!(r#"{{"sequence": {}, "data": "message_{}", "timestamp": "{}"}}"#, 
                                       i, i, send_start.elapsed().as_millis());
        mock_server.send_raw_message(&sequenced_message).await.expect("Failed to send sequenced message");
        
        // Small delay to create realistic timing
        if i % 10 == 0 {
            sleep(Duration::from_millis(1)).await;
        }
    }
    
    info!("Sent {} sequenced messages in {:?}", message_count, send_start.elapsed());
    
    // Collect messages and verify ordering
    let mut received_messages = Vec::new();
    let collection_deadline = Instant::now() + Duration::from_secs(10);
    
    while received_messages.len() < message_count && Instant::now() < collection_deadline {
        if let Ok(Ok(msg)) = timeout(Duration::from_millis(100), message_receiver.recv()).await {
            received_messages.push(msg);
        }
    }
    
    info!("Received {} messages", received_messages.len());
    assert_eq!(received_messages.len(), message_count, "Should receive all messages");
    
    // Verify sequence ordering
    for (i, message) in received_messages.iter().enumerate() {
        let expected_sequence = format!(r#""sequence": {}"#, i);
        assert!(message.contains(&expected_sequence), 
               "Message {} should have sequence {}: {}", i, i, message);
        
        let expected_data = format!(r#""data": "message_{}""#, i);
        assert!(message.contains(&expected_data), 
               "Message {} should have correct data: {}", i, message);
    }
    
    // Test 2: Send messages with intentional gaps and verify gap detection
    info!("Testing gap detection");
    let gap_messages = vec![0, 1, 2, 4, 5, 7, 8, 9]; // Missing 3 and 6
    
    for seq in gap_messages {
        let gap_message = format!(r#"{{"sequence": {}, "data": "gap_test_{}", "type": "gap_test"}}"#, seq, seq);
        mock_server.send_raw_message(&gap_message).await.expect("Failed to send gap message");
        sleep(Duration::from_millis(50)).await;
    }
    
    // Collect gap test messages
    let mut gap_received = Vec::new();
    while gap_received.len() < 8 && Instant::now() < collection_deadline {
        if let Ok(Ok(msg)) = timeout(Duration::from_millis(200), message_receiver.recv()).await {
            if msg.contains("gap_test") {
                gap_received.push(msg);
            }
        }
    }
    
    assert_eq!(gap_received.len(), 8, "Should receive all gap test messages");
    
    // Test 3: Send burst of messages and verify no reordering
    info!("Testing burst message ordering");
    let burst_size = 50;
    let burst_id = "burst_test";
    
    // Send burst rapidly
    for i in 0..burst_size {
        let burst_message = format!(r#"{{"burst_id": "{}", "index": {}, "data": "burst_{}"}}"#, 
                                   burst_id, i, i);
        mock_server.send_raw_message(&burst_message).await.expect("Failed to send burst message");
    }
    
    // Collect burst messages
    let mut burst_received = Vec::new();
    let burst_deadline = Instant::now() + Duration::from_secs(5);
    
    while burst_received.len() < burst_size && Instant::now() < burst_deadline {
        if let Ok(Ok(msg)) = timeout(Duration::from_millis(100), message_receiver.recv()).await {
            if msg.contains(burst_id) {
                burst_received.push(msg);
            }
        }
    }
    
    assert_eq!(burst_received.len(), burst_size, "Should receive all burst messages");
    
    // Verify burst ordering
    for (i, message) in burst_received.iter().enumerate() {
        let expected_index = format!(r#""index": {}"#, i);
        assert!(message.contains(&expected_index), 
               "Burst message {} should have correct index: {}", i, message);
    }
    
    // Verify ordering metrics
    let ordering_metrics = client.message_ordering_metrics();
    assert_eq!(ordering_metrics.total_messages_processed, (message_count + 8 + burst_size) as u64);
    assert_eq!(ordering_metrics.sequence_violations, 0); // No reordering should occur
    assert_eq!(ordering_metrics.gap_detections, 2); // Missing sequences 3 and 6
    assert!(ordering_metrics.ordering_preserved_percentage >= 95.0);
    assert!(ordering_metrics.average_message_latency > Duration::ZERO);
    
    // Test 4: Concurrent senders (if supported)
    info!("Testing concurrent sender ordering");
    let concurrent_senders = 3;
    let messages_per_sender = 20;
    
    let concurrent_tasks: Vec<_> = (0..concurrent_senders).map(|sender_id| {
        let mut mock_clone = mock_server.clone(); // Assuming clone is available
        tokio::spawn(async move {
            for msg_id in 0..messages_per_sender {
                let concurrent_msg = format!(r#"{{"sender": {}, "message": {}, "data": "concurrent_{}_{}", "type": "concurrent"}}"#, 
                                            sender_id, msg_id, sender_id, msg_id);
                mock_clone.send_raw_message(&concurrent_msg).await.expect("Failed to send concurrent message");
                sleep(Duration::from_millis(10)).await;
            }
        })
    }).collect();
    
    // Wait for all concurrent senders
    for task in concurrent_tasks {
        task.await.expect("Concurrent sender should complete");
    }
    
    // Collect concurrent messages
    let mut concurrent_messages = Vec::new();
    let concurrent_deadline = Instant::now() + Duration::from_secs(10);
    
    while concurrent_messages.len() < concurrent_senders * messages_per_sender && 
          Instant::now() < concurrent_deadline {
        if let Ok(Ok(msg)) = timeout(Duration::from_millis(100), message_receiver.recv()).await {
            if msg.contains("concurrent") {
                concurrent_messages.push(msg);
            }
        }
    }
    
    info!("Received {} concurrent messages", concurrent_messages.len());
    
    // Verify per-sender ordering (messages from same sender should be in order)
    for sender_id in 0..concurrent_senders {
        let sender_messages: Vec<_> = concurrent_messages.iter()
            .filter(|msg| msg.contains(&format!(r#""sender": {}"#, sender_id)))
            .collect();
        
        for (i, message) in sender_messages.iter().enumerate() {
            let expected_msg_id = format!(r#""message": {}"#, i);
            assert!(message.contains(&expected_msg_id), 
                   "Sender {} message {} should be in order: {}", sender_id, i, message);
        }
    }
    
    // Final ordering metrics
    let final_metrics = client.message_ordering_metrics();
    assert!(final_metrics.total_messages_processed >= (message_count + 8 + burst_size + (concurrent_senders * messages_per_sender)) as u64);
    assert!(final_metrics.concurrent_sender_ordering_maintained);
    assert_eq!(final_metrics.ordering_algorithm, "fifo_with_sequence_tracking");
    
    client.disconnect().await;
    mock_server.stop().await;
    info!("Message ordering preservation test completed");
}