use std::time::{Duration, SystemTime};
use tokio::time::{sleep, timeout};
use tracing::info;
use serde_json::{json, Value};

use super::mock_websocket_server::MockWebSocketServer;

// All tests reference functionality that doesn't exist yet (RED phase)

#[tokio::test]
async fn test_valid_price_message_parsing() {
    info!("Testing parsing of valid price messages");
    
    // Start mock server
    let mut mock_server = MockWebSocketServer::new(9011);
    let server_addr = mock_server.start().await;
    
    // Create client with price parsing enabled
    let client = WebSocketClient::new(&format!("ws://{}", server_addr))
        .with_price_parsing(true)
        .with_strict_validation(true);
    
    client.connect().await.expect("Connection should succeed");
    
    // Set up price message listener
    let price_stream = client.price_stream();
    let mut price_receiver = price_stream.subscribe();
    
    // Test various valid price message formats
    let test_messages = vec![
        // Standard format
        json!({
            "product_id": "BTC-USD",
            "price": "45000.50",
            "timestamp": "2025-10-28T12:00:00Z"
        }),
        // Numeric price
        json!({
            "product_id": "ETH-USD", 
            "price": 3200.75,
            "timestamp": "2025-10-28T12:01:00Z"
        }),
        // High precision price
        json!({
            "product_id": "SOL-USD",
            "price": "95.12345678",
            "timestamp": "2025-10-28T12:02:00Z"
        }),
        // Additional metadata
        json!({
            "product_id": "DOGE-USD",
            "price": "0.08250000",
            "timestamp": "2025-10-28T12:03:00Z",
            "volume": "1000000.0",
            "exchange": "coinbase"
        }),
        // Scientific notation
        json!({
            "product_id": "SHIB-USD",
            "price": "1.23e-5",
            "timestamp": "2025-10-28T12:04:00Z"
        }),
    ];
    
    // Send test messages
    for (i, message) in test_messages.iter().enumerate() {
        let message_str = message.to_string();
        mock_server.send_raw_message(&message_str).await;
        info!("Sent test message {}: {}", i + 1, message_str);
        sleep(Duration::from_millis(50)).await;
    }
    
    // Collect parsed price updates
    let mut parsed_prices = Vec::new();
    for i in 0..test_messages.len() {
        let price_result = timeout(Duration::from_secs(2), price_receiver.recv()).await;
        assert!(price_result.is_ok(), "Should receive parsed price {}", i + 1);
        
        let price_update = price_result.unwrap().unwrap();
        parsed_prices.push(price_update);
        info!("Received parsed price {}: {:?}", i + 1, parsed_prices[i]);
    }
    
    // Verify all messages were parsed correctly
    assert_eq!(parsed_prices.len(), 5, "Should parse all 5 test messages");
    
    // Verify BTC-USD parsing
    let btc_price = &parsed_prices[0];
    assert_eq!(btc_price.product_id, "BTC-USD");
    assert_eq!(btc_price.price, 45000.50);
    assert!(btc_price.timestamp.is_some());
    assert_eq!(btc_price.exchange, None); // No exchange specified
    
    // Verify ETH-USD parsing (numeric price)
    let eth_price = &parsed_prices[1];
    assert_eq!(eth_price.product_id, "ETH-USD");
    assert_eq!(eth_price.price, 3200.75);
    assert!(eth_price.timestamp.is_some());
    
    // Verify SOL-USD parsing (high precision)
    let sol_price = &parsed_prices[2];
    assert_eq!(sol_price.product_id, "SOL-USD");
    assert_eq!(sol_price.price, 95.12345678);
    assert!(sol_price.timestamp.is_some());
    
    // Verify DOGE-USD parsing (with metadata)
    let doge_price = &parsed_prices[3];
    assert_eq!(doge_price.product_id, "DOGE-USD");
    assert_eq!(doge_price.price, 0.08250000);
    assert_eq!(doge_price.volume, Some(1000000.0));
    assert_eq!(doge_price.exchange, Some("coinbase".to_string()));
    
    // Verify SHIB-USD parsing (scientific notation)
    let shib_price = &parsed_prices[4];
    assert_eq!(shib_price.product_id, "SHIB-USD");
    assert!((shib_price.price - 0.0000123).abs() < 1e-10); // Float comparison with tolerance
    
    // Verify timestamp parsing
    for price in &parsed_prices {
        assert!(price.timestamp.is_some(), "All prices should have timestamps");
        let timestamp = price.timestamp.unwrap();
        let now = SystemTime::now();
        let duration_since_epoch = now.duration_since(SystemTime::UNIX_EPOCH).unwrap();
        let timestamp_since_epoch = timestamp.duration_since(SystemTime::UNIX_EPOCH).unwrap();
        
        // Timestamp should be within reasonable range (not too old, not in future)
        assert!(timestamp_since_epoch <= duration_since_epoch, "Timestamp should not be in future");
    }
    
    // Verify parsing metrics
    let parsing_metrics = client.parsing_metrics();
    assert_eq!(parsing_metrics.total_messages_parsed, 5);
    assert_eq!(parsing_metrics.successful_parses, 5);
    assert_eq!(parsing_metrics.parsing_errors, 0);
    assert!(parsing_metrics.average_parse_time > Duration::ZERO);
    assert!(parsing_metrics.max_parse_time > Duration::ZERO);
    
    // Clean up
    client.disconnect().await;
    mock_server.stop().await;
    
    info!("Valid price message parsing test completed");
}

#[tokio::test]
async fn test_malformed_json_handling() {
    info!("Testing handling of malformed JSON price messages");
    
    // Start mock server
    let mut mock_server = MockWebSocketServer::new(9012);
    let server_addr = mock_server.start().await;
    
    // Create client with error handling
    let client = WebSocketClient::new(&format!("ws://{}", server_addr))
        .with_price_parsing(true)
        .with_error_recovery(true);
    
    client.connect().await.expect("Connection should succeed");
    
    // Set up listeners
    let price_stream = client.price_stream();
    let mut price_receiver = price_stream.subscribe();
    
    let error_stream = client.parsing_error_stream();
    let mut error_receiver = error_stream.subscribe();
    
    // Send malformed JSON messages
    let malformed_messages = vec![
        r#"{"product_id": "BTC-USD", "price": "45000.50", invalid"#, // Incomplete JSON
        r#"{"product_id": "ETH-USD" "price": "3200.75"}"#,          // Missing comma
        r#"{product_id: "SOL-USD", price: "95.25"}"#,               // Unquoted keys
        r#"{"product_id": "DOGE-USD", "price": 45000.50"#,          // Missing closing brace
        r#"not json at all"#,                                       // Not JSON
        r#""#,                                                      // Empty string
        r#"{"price": "broken", "timestamp": malformed}"#,           // Invalid values
        r#"[{"product_id": "BTC-USD"}]"#,                           // Array instead of object
    ];
    
    // Send malformed messages
    for (i, message) in malformed_messages.iter().enumerate() {
        mock_server.send_raw_message(message).await;
        info!("Sent malformed message {}: {}", i + 1, message);
        sleep(Duration::from_millis(50)).await;
    }
    
    // Send a valid message to ensure parsing still works
    let valid_message = json!({
        "product_id": "BTC-USD",
        "price": "45000.00",
        "timestamp": "2025-10-28T12:00:00Z"
    });
    mock_server.send_raw_message(&valid_message.to_string()).await;
    
    // Collect parsing errors
    let mut parsing_errors = Vec::new();
    for i in 0..malformed_messages.len() {
        let error_result = timeout(Duration::from_secs(2), error_receiver.recv()).await;
        if let Ok(Ok(error)) = error_result {
            parsing_errors.push(error);
            info!("Received parsing error {}: {:?}", i + 1, parsing_errors.last().unwrap());
        }
    }
    
    // Verify errors were captured
    assert!(parsing_errors.len() >= 6, "Should capture most malformed messages as errors");
    
    // Verify error types
    let json_syntax_errors = parsing_errors.iter()
        .filter(|e| matches!(e.error_type, ParsingErrorType::JsonSyntaxError))
        .count();
    assert!(json_syntax_errors >= 4, "Should have multiple JSON syntax errors");
    
    let invalid_structure_errors = parsing_errors.iter()
        .filter(|e| matches!(e.error_type, ParsingErrorType::InvalidStructure))
        .count();
    assert!(invalid_structure_errors >= 1, "Should have structure errors");
    
    // Verify valid message still parses correctly
    let valid_price_result = timeout(Duration::from_secs(2), price_receiver.recv()).await;
    assert!(valid_price_result.is_ok(), "Should still parse valid messages");
    
    let valid_price = valid_price_result.unwrap().unwrap();
    assert_eq!(valid_price.product_id, "BTC-USD");
    assert_eq!(valid_price.price, 45000.00);
    
    // Verify connection remains stable
    assert!(client.is_connected(), "Connection should remain stable after errors");
    assert_eq!(client.connection_state(), ConnectionState::Connected);
    
    // Verify error metrics
    let parsing_metrics = client.parsing_metrics();
    assert!(parsing_metrics.parsing_errors >= 6, "Should count parsing errors");
    assert_eq!(parsing_metrics.successful_parses, 1, "Should count successful parses");
    assert!(parsing_metrics.total_messages_parsed >= 7, "Should count total messages");
    assert!(parsing_metrics.error_recovery_count >= 6, "Should count error recoveries");
    
    // Verify error details are logged
    for error in &parsing_errors {
        assert!(!error.raw_message.is_empty(), "Should preserve raw message");
        assert!(!error.error_message.is_empty(), "Should have error description");
        assert!(error.timestamp.is_some(), "Should have error timestamp");
        assert!(error.line_number.is_some() || error.character_position.is_some(), "Should have position info");
    }
    
    // Clean up
    client.disconnect().await;
    mock_server.stop().await;
    
    info!("Malformed JSON handling test completed");
}

#[tokio::test]
async fn test_missing_required_fields() {
    info!("Testing rejection of messages with missing required fields");
    
    // Start mock server
    let mut mock_server = MockWebSocketServer::new(9013);
    let server_addr = mock_server.start().await;
    
    // Create client with strict field validation
    let client = WebSocketClient::new(&format!("ws://{}", server_addr))
        .with_price_parsing(true)
        .with_strict_field_validation(true)
        .with_required_fields(vec!["product_id".to_string(), "price".to_string(), "timestamp".to_string()]);
    
    client.connect().await.expect("Connection should succeed");
    
    // Set up listeners
    let price_stream = client.price_stream();
    let mut price_receiver = price_stream.subscribe();
    
    let validation_error_stream = client.validation_error_stream();
    let mut validation_error_receiver = validation_error_stream.subscribe();
    
    // Test messages with missing required fields
    let incomplete_messages = vec![
        // Missing product_id
        json!({
            "price": "45000.50",
            "timestamp": "2025-10-28T12:00:00Z"
        }),
        // Missing price
        json!({
            "product_id": "ETH-USD",
            "timestamp": "2025-10-28T12:01:00Z"
        }),
        // Missing timestamp
        json!({
            "product_id": "SOL-USD",
            "price": "95.25"
        }),
        // Missing multiple fields
        json!({
            "volume": "1000000.0"
        }),
        // Null required fields
        json!({
            "product_id": null,
            "price": "45000.50",
            "timestamp": "2025-10-28T12:02:00Z"
        }),
        // Empty required fields
        json!({
            "product_id": "",
            "price": "45000.50",
            "timestamp": "2025-10-28T12:03:00Z"
        }),
        // Wrong field types
        json!({
            "product_id": 12345,
            "price": "45000.50",
            "timestamp": "2025-10-28T12:04:00Z"
        }),
    ];
    
    // Send incomplete messages
    for (i, message) in incomplete_messages.iter().enumerate() {
        let message_str = message.to_string();
        mock_server.send_raw_message(&message_str).await;
        info!("Sent incomplete message {}: {}", i + 1, message_str);
        sleep(Duration::from_millis(50)).await;
    }
    
    // Send a complete valid message
    let complete_message = json!({
        "product_id": "BTC-USD",
        "price": "45000.00",
        "timestamp": "2025-10-28T12:05:00Z"
    });
    mock_server.send_raw_message(&complete_message.to_string()).await;
    
    // Collect validation errors
    let mut validation_errors = Vec::new();
    for i in 0..incomplete_messages.len() {
        let error_result = timeout(Duration::from_secs(2), validation_error_receiver.recv()).await;
        if let Ok(Ok(error)) = error_result {
            validation_errors.push(error);
            info!("Received validation error {}: {:?}", i + 1, validation_errors.last().unwrap());
        }
    }
    
    // Verify validation errors
    assert_eq!(validation_errors.len(), 7, "Should reject all incomplete messages");
    
    // Verify specific error types
    let missing_product_id = validation_errors.iter()
        .find(|e| e.missing_fields.contains(&"product_id".to_string()));
    assert!(missing_product_id.is_some(), "Should detect missing product_id");
    
    let missing_price = validation_errors.iter()
        .find(|e| e.missing_fields.contains(&"price".to_string()));
    assert!(missing_price.is_some(), "Should detect missing price");
    
    let missing_timestamp = validation_errors.iter()
        .find(|e| e.missing_fields.contains(&"timestamp".to_string()));
    assert!(missing_timestamp.is_some(), "Should detect missing timestamp");
    
    // Verify null/empty field detection
    let null_field_error = validation_errors.iter()
        .find(|e| e.null_fields.contains(&"product_id".to_string()));
    assert!(null_field_error.is_some(), "Should detect null fields");
    
    let empty_field_error = validation_errors.iter()
        .find(|e| e.empty_fields.contains(&"product_id".to_string()));
    assert!(empty_field_error.is_some(), "Should detect empty fields");
    
    // Verify type mismatch detection
    let type_error = validation_errors.iter()
        .find(|e| !e.type_mismatches.is_empty());
    assert!(type_error.is_some(), "Should detect type mismatches");
    
    // Verify complete message is accepted
    let valid_price_result = timeout(Duration::from_secs(2), price_receiver.recv()).await;
    assert!(valid_price_result.is_ok(), "Should accept complete valid messages");
    
    let valid_price = valid_price_result.unwrap().unwrap();
    assert_eq!(valid_price.product_id, "BTC-USD");
    assert_eq!(valid_price.price, 45000.00);
    assert!(valid_price.timestamp.is_some());
    
    // Verify validation metrics
    let validation_metrics = client.validation_metrics();
    assert_eq!(validation_metrics.total_validations, 8, "Should validate all messages");
    assert_eq!(validation_metrics.validation_failures, 7, "Should count failures");
    assert_eq!(validation_metrics.validation_successes, 1, "Should count successes");
    assert!(validation_metrics.missing_field_errors >= 6, "Should count missing field errors");
    assert!(validation_metrics.type_mismatch_errors >= 1, "Should count type errors");
    
    // Verify error details
    for error in &validation_errors {
        assert!(!error.raw_message.is_empty(), "Should preserve raw message");
        assert!(!error.error_summary.is_empty(), "Should have error summary");
        assert!(error.timestamp.is_some(), "Should have validation timestamp");
        assert!(
            !error.missing_fields.is_empty() || 
            !error.null_fields.is_empty() || 
            !error.empty_fields.is_empty() ||
            !error.type_mismatches.is_empty(),
            "Should specify what validation failed"
        );
    }
    
    // Clean up
    client.disconnect().await;
    mock_server.stop().await;
    
    info!("Missing required fields test completed");
}

#[tokio::test]
async fn test_price_type_validation() {
    info!("Testing price field type validation");
    
    // Start mock server
    let mut mock_server = MockWebSocketServer::new(9014);
    let server_addr = mock_server.start().await;
    
    // Create client with price type validation
    let client = WebSocketClient::new(&format!("ws://{}", server_addr))
        .with_price_parsing(true)
        .with_price_type_validation(true)
        .with_numeric_validation(true);
    
    client.connect().await.expect("Connection should succeed");
    
    // Set up listeners
    let price_stream = client.price_stream();
    let mut price_receiver = price_stream.subscribe();
    
    let type_error_stream = client.type_error_stream();
    let mut type_error_receiver = type_error_stream.subscribe();
    
    // Test various price types
    let price_type_tests = vec![
        // Valid numeric string
        (json!({
            "product_id": "BTC-USD",
            "price": "45000.50",
            "timestamp": "2025-10-28T12:00:00Z"
        }), true, "valid numeric string"),
        
        // Valid number
        (json!({
            "product_id": "ETH-USD",
            "price": 3200.75,
            "timestamp": "2025-10-28T12:01:00Z"
        }), true, "valid number"),
        
        // Invalid string price
        (json!({
            "product_id": "SOL-USD",
            "price": "ABC123",
            "timestamp": "2025-10-28T12:02:00Z"
        }), false, "non-numeric string"),
        
        // Boolean price
        (json!({
            "product_id": "DOGE-USD",
            "price": true,
            "timestamp": "2025-10-28T12:03:00Z"
        }), false, "boolean value"),
        
        // Array price
        (json!({
            "product_id": "ADA-USD",
            "price": [45000.50],
            "timestamp": "2025-10-28T12:04:00Z"
        }), false, "array value"),
        
        // Object price
        (json!({
            "product_id": "DOT-USD",
            "price": {"value": 45000.50},
            "timestamp": "2025-10-28T12:05:00Z"
        }), false, "object value"),
        
        // Negative price
        (json!({
            "product_id": "LINK-USD",
            "price": "-15.50",
            "timestamp": "2025-10-28T12:06:00Z"
        }), false, "negative price"),
        
        // Zero price
        (json!({
            "product_id": "UNI-USD",
            "price": "0.00",
            "timestamp": "2025-10-28T12:07:00Z"
        }), false, "zero price"),
        
        // Infinity
        (json!({
            "product_id": "AAVE-USD",
            "price": "Infinity",
            "timestamp": "2025-10-28T12:08:00Z"
        }), false, "infinity value"),
        
        // Scientific notation (valid)
        (json!({
            "product_id": "SHIB-USD",
            "price": "1.23e-5",
            "timestamp": "2025-10-28T12:09:00Z"
        }), true, "scientific notation"),
    ];
    
    let mut valid_count = 0;
    let mut invalid_count = 0;
    
    // Send test messages
    for (i, (message, should_be_valid, description)) in price_type_tests.iter().enumerate() {
        let message_str = message.to_string();
        mock_server.send_raw_message(&message_str).await;
        info!("Sent test {}: {} - {}", i + 1, description, message_str);
        
        if *should_be_valid {
            valid_count += 1;
        } else {
            invalid_count += 1;
        }
        
        sleep(Duration::from_millis(100)).await;
    }
    
    // Collect valid price updates
    let mut valid_prices = Vec::new();
    while valid_prices.len() < valid_count {
        let price_result = timeout(Duration::from_secs(1), price_receiver.recv()).await;
        if let Ok(Ok(price)) = price_result {
            valid_prices.push(price);
            info!("Received valid price: {:?}", valid_prices.last().unwrap());
        } else {
            break;
        }
    }
    
    // Collect type errors
    let mut type_errors = Vec::new();
    while type_errors.len() < invalid_count {
        let error_result = timeout(Duration::from_secs(1), type_error_receiver.recv()).await;
        if let Ok(Ok(error)) = error_result {
            type_errors.push(error);
            info!("Received type error: {:?}", type_errors.last().unwrap());
        } else {
            break;
        }
    }
    
    // Verify correct parsing/rejection
    assert_eq!(valid_prices.len(), 3, "Should accept exactly 3 valid prices");
    assert!(type_errors.len() >= 6, "Should reject at least 6 invalid prices");
    
    // Verify valid prices
    let btc_price = valid_prices.iter().find(|p| p.product_id == "BTC-USD").unwrap();
    assert_eq!(btc_price.price, 45000.50);
    
    let eth_price = valid_prices.iter().find(|p| p.product_id == "ETH-USD").unwrap();
    assert_eq!(eth_price.price, 3200.75);
    
    let shib_price = valid_prices.iter().find(|p| p.product_id == "SHIB-USD").unwrap();
    assert!((shib_price.price - 0.0000123).abs() < 1e-10);
    
    // Verify type error details
    let non_numeric_error = type_errors.iter()
        .find(|e| e.field_name == "price" && e.actual_type == "string" && e.reason.contains("non-numeric"));
    assert!(non_numeric_error.is_some(), "Should detect non-numeric string");
    
    let boolean_error = type_errors.iter()
        .find(|e| e.field_name == "price" && e.actual_type == "boolean");
    assert!(boolean_error.is_some(), "Should detect boolean type");
    
    let negative_error = type_errors.iter()
        .find(|e| e.validation_rule == "positive_price" && e.reason.contains("negative"));
    assert!(negative_error.is_some(), "Should reject negative prices");
    
    let zero_error = type_errors.iter()
        .find(|e| e.validation_rule == "positive_price" && e.reason.contains("zero"));
    assert!(zero_error.is_some(), "Should reject zero prices");
    
    // Verify type validation metrics
    let type_metrics = client.type_validation_metrics();
    assert_eq!(type_metrics.total_type_checks, 10, "Should check all message types");
    assert_eq!(type_metrics.type_validation_successes, 3, "Should count successes");
    assert!(type_metrics.type_validation_failures >= 6, "Should count failures");
    assert!(type_metrics.non_numeric_price_errors >= 3, "Should count non-numeric errors");
    assert!(type_metrics.negative_price_errors >= 1, "Should count negative price errors");
    assert!(type_metrics.zero_price_errors >= 1, "Should count zero price errors");
    
    // Clean up
    client.disconnect().await;
    mock_server.stop().await;
    
    info!("Price type validation test completed");
}

#[tokio::test]
async fn test_decimal_precision_preservation() {
    info!("Testing preservation of decimal precision in price parsing");
    
    // Start mock server
    let mut mock_server = MockWebSocketServer::new(9015);
    let server_addr = mock_server.start().await;
    
    // Create client with high precision parsing
    let client = WebSocketClient::new(&format!("ws://{}", server_addr))
        .with_price_parsing(true)
        .with_high_precision_mode(true)
        .with_decimal_places(18); // Support up to 18 decimal places
    
    client.connect().await.expect("Connection should succeed");
    
    // Set up price listener
    let price_stream = client.price_stream();
    let mut price_receiver = price_stream.subscribe();
    
    // Test high precision prices
    let precision_tests = vec![
        // Standard 2 decimal places
        ("BTC-USD", "45000.50", 45000.50),
        
        // 4 decimal places
        ("ETH-USD", "3200.7500", 3200.7500),
        
        // 8 decimal places (crypto precision)
        ("SOL-USD", "95.12345678", 95.12345678),
        
        // 10 decimal places
        ("DOGE-USD", "0.0825000000", 0.0825000000),
        
        // 12 decimal places
        ("SHIB-USD", "0.000012345678", 0.000012345678),
        
        // 18 decimal places (maximum)
        ("WEI-ETH", "0.000000000000000001", 0.000000000000000001),
        
        // Large number with precision
        ("MEGA-USD", "999999999.123456789", 999999999.123456789),
        
        // Trailing zeros preservation
        ("TRAIL-USD", "100.100000000", 100.100000000),
        
        // Scientific notation with precision
        ("SCI-USD", "1.23456789e-8", 0.0000000123456789),
        
        // Very small number
        ("TINY-USD", "0.000000000123456789", 0.000000000123456789),
    ];
    
    // Send precision test messages
    for (product_id, price_str, expected_price) in &precision_tests {
        let message = json!({
            "product_id": product_id,
            "price": price_str,
            "timestamp": "2025-10-28T12:00:00Z"
        });
        
        mock_server.send_raw_message(&message.to_string()).await;
        info!("Sent precision test: {} = {}", product_id, price_str);
        sleep(Duration::from_millis(50)).await;
    }
    
    // Collect parsed prices
    let mut parsed_prices = Vec::new();
    for i in 0..precision_tests.len() {
        let price_result = timeout(Duration::from_secs(2), price_receiver.recv()).await;
        assert!(price_result.is_ok(), "Should receive price update {}", i + 1);
        
        let price_update = price_result.unwrap().unwrap();
        parsed_prices.push(price_update);
        info!("Received parsed price: {} = {}", 
              parsed_prices[i].product_id, parsed_prices[i].price);
    }
    
    // Verify precision preservation
    assert_eq!(parsed_prices.len(), 10, "Should parse all precision tests");
    
    for (i, (expected_product_id, original_price_str, expected_price)) in precision_tests.iter().enumerate() {
        let parsed_price = &parsed_prices[i];
        
        // Verify product ID
        assert_eq!(parsed_price.product_id, *expected_product_id);
        
        // Verify price precision with appropriate tolerance
        let price_diff = (parsed_price.price - expected_price).abs();
        let tolerance = if *expected_price < 1e-10 { 1e-20 } else { 1e-15 };
        
        assert!(
            price_diff < tolerance,
            "Price precision lost for {}: expected {}, got {}, diff {}",
            expected_product_id, expected_price, parsed_price.price, price_diff
        );
        
        // Verify original string preservation (if available)
        if let Some(original_str) = &parsed_price.original_price_string {
            assert_eq!(original_str, original_price_str, 
                      "Should preserve original price string for {}", expected_product_id);
        }
        
        // Verify decimal places count
        let decimal_places = if original_price_str.contains('.') {
            let parts: Vec<&str> = original_price_str.split('.').collect();
            if parts.len() == 2 {
                parts[1].trim_end_matches('0').len()
            } else { 0 }
        } else { 0 };
        
        assert!(parsed_price.decimal_places >= decimal_places as u8,
               "Should preserve at least {} decimal places for {}", 
               decimal_places, expected_product_id);
    }
    
    // Test precision arithmetic
    let btc_price = parsed_prices.iter().find(|p| p.product_id == "BTC-USD").unwrap();
    let eth_price = parsed_prices.iter().find(|p| p.product_id == "ETH-USD").unwrap();
    
    // Perform arithmetic and verify precision is maintained
    let price_ratio = btc_price.price / eth_price.price;
    let expected_ratio = 45000.50 / 3200.7500;
    let ratio_diff = (price_ratio - expected_ratio).abs();
    assert!(ratio_diff < 1e-10, "Arithmetic should preserve precision");
    
    // Test serialization round-trip
    for price in &parsed_prices {
        let serialized = client.serialize_price(price);
        let deserialized = client.deserialize_price(&serialized).unwrap();
        
        let precision_diff = (price.price - deserialized.price).abs();
        assert!(precision_diff < 1e-18, 
               "Serialization round-trip should preserve precision for {}", 
               price.product_id);
    }
    
    // Verify precision metrics
    let precision_metrics = client.precision_metrics();
    assert_eq!(precision_metrics.total_precision_tests, 10);
    assert_eq!(precision_metrics.precision_preserved_count, 10);
    assert_eq!(precision_metrics.precision_loss_count, 0);
    assert!(precision_metrics.max_decimal_places_handled >= 18);
    assert!(precision_metrics.min_value_handled <= 1e-18);
    assert!(precision_metrics.max_value_handled >= 999999999.0);
    
    // Clean up
    client.disconnect().await;
    mock_server.stop().await;
    
    info!("Decimal precision preservation test completed");
}

// Placeholder structs and enums that will be implemented in GREEN phase
struct WebSocketClient {
    url: String,
}

#[derive(Debug, PartialEq)]
enum ConnectionState {
    Connected,
    Disconnected,
}

#[derive(Clone, Debug)]
enum ParsingErrorType {
    JsonSyntaxError,
    InvalidStructure,
}

impl WebSocketClient {
    fn new(url: &str) -> Self {
        unimplemented!("WebSocketClient::new() - to be implemented in GREEN phase")
    }
    
    fn with_price_parsing(self, enabled: bool) -> Self {
        unimplemented!("WebSocketClient::with_price_parsing() - to be implemented in GREEN phase")
    }
    
    fn with_high_precision_mode(self, enabled: bool) -> Self {
        unimplemented!("WebSocketClient::with_high_precision_mode() - to be implemented in GREEN phase")
    }
    
    fn with_decimal_places(self, places: u8) -> Self {
        unimplemented!("WebSocketClient::with_decimal_places() - to be implemented in GREEN phase")
    }
    
    fn with_strict_validation(self, enabled: bool) -> Self {
        unimplemented!("WebSocketClient::with_strict_validation() - to be implemented in GREEN phase")
    }
    
    fn with_error_recovery(self, enabled: bool) -> Self {
        unimplemented!("WebSocketClient::with_error_recovery() - to be implemented in GREEN phase")
    }
    
    fn with_strict_field_validation(self, enabled: bool) -> Self {
        unimplemented!("WebSocketClient::with_strict_field_validation() - to be implemented in GREEN phase")
    }
    
    fn with_price_type_validation(self, enabled: bool) -> Self {
        unimplemented!("WebSocketClient::with_price_type_validation() - to be implemented in GREEN phase")
    }
    
    async fn connect(&self) -> Result<(), String> {
        unimplemented!("WebSocketClient::connect() - to be implemented in GREEN phase")
    }
    
    async fn disconnect(&self) {
        unimplemented!("WebSocketClient::disconnect() - to be implemented in GREEN phase")
    }
    
    fn connection_state(&self) -> ConnectionState {
        ConnectionState::Connected
    }
    
    fn is_connected(&self) -> bool {
        unimplemented!("WebSocketClient::is_connected() - to be implemented in GREEN phase")
    }
    
    fn price_stream(&self) -> PriceStream {
        unimplemented!("WebSocketClient::price_stream() - to be implemented in GREEN phase")
    }
    
    fn serialize_price(&self, _price: &PriceUpdate) -> String {
        unimplemented!("WebSocketClient::serialize_price() - to be implemented in GREEN phase")
    }
    
    fn deserialize_price(&self, _serialized: &str) -> Result<PriceUpdate, String> {
        unimplemented!("WebSocketClient::deserialize_price() - to be implemented in GREEN phase")
    }
    
    fn with_required_fields(self, fields: Vec<String>) -> Self {
        unimplemented!("WebSocketClient::with_required_fields() - to be implemented in GREEN phase")
    }
    
    fn with_numeric_validation(self, enabled: bool) -> Self {
        unimplemented!("WebSocketClient::with_numeric_validation() - to be implemented in GREEN phase")
    }
    
    fn parsing_metrics(&self) -> ParsingMetrics {
        unimplemented!("WebSocketClient::parsing_metrics() - to be implemented in GREEN phase")
    }
    
    fn parsing_error_stream(&self) -> ParsingErrorStream {
        unimplemented!("WebSocketClient::parsing_error_stream() - to be implemented in GREEN phase")
    }
    
    fn validation_metrics(&self) -> ValidationMetrics {
        unimplemented!("WebSocketClient::validation_metrics() - to be implemented in GREEN phase")
    }
    
    fn validation_error_stream(&self) -> ValidationErrorStream {
        unimplemented!("WebSocketClient::validation_error_stream() - to be implemented in GREEN phase")
    }
    
    fn type_error_stream(&self) -> TypeErrorStream {
        unimplemented!("WebSocketClient::type_error_stream() - to be implemented in GREEN phase")
    }
    
    fn type_validation_metrics(&self) -> TypeValidationMetrics {
        unimplemented!()
    }
    
    fn reconnection_stream(&self) -> ReconnectionStream {
        unimplemented!()
    }
    
    fn reconnection_metrics(&self) -> ReconnectionMetrics {
        unimplemented!()
    }
    
    fn last_connection_error(&self) -> Option<String> {
        unimplemented!()
    }
    
    async fn manual_reconnect(&self) -> Result<(), String> {
        unimplemented!()
    }
    
    fn clone(&self) -> Self {
        unimplemented!()
    }
    
    async fn subscribe_to_price(&self, _symbol: &str) -> Result<String, String> {
        unimplemented!()
    }
    
    async fn queue_outbound_message(&self, _message: &str) {
        unimplemented!()
    }
    
    fn active_subscriptions(&self) -> Vec<String> {
        unimplemented!()
    }
    
    fn connection_metadata(&self) -> ConnectionMetadata {
        unimplemented!()
    }
    
    fn buffer_metrics(&self) -> BufferMetrics {
        unimplemented!()
    }
    
    fn circuit_breaker_stream(&self) -> CircuitBreakerStream {
        unimplemented!()
    }
    
    fn connection_attempt_stream(&self) -> ConnectionAttemptStream {
        unimplemented!()
    }
    
    fn circuit_state(&self) -> CircuitState {
        unimplemented!()
    }
    
    fn is_circuit_open(&self) -> bool {
        unimplemented!()
    }
    
    fn is_circuit_closed(&self) -> bool {
        unimplemented!()
    }
    
    fn is_circuit_half_open(&self) -> bool {
        unimplemented!()
    }
    
    fn circuit_breaker_metrics(&self) -> CircuitBreakerMetrics {
        unimplemented!()
    }
    
    fn failure_history(&self) -> Vec<MockFailureEvent> {
        unimplemented!()
    }
    
    fn success_history(&self) -> Vec<MockSuccessEvent> {
        unimplemented!()
    }
    
    fn timeout_history(&self) -> Vec<MockTimeoutEvent> {
        unimplemented!()
    }
    
    fn backoff_history(&self) -> Vec<MockBackoffEvent> {
        unimplemented!()
    }
    
    fn circuit_breaker_event_history(&self) -> Vec<CircuitEvent> {
        unimplemented!()
    }
    
    fn export_metrics_as_json(&self) -> String {
        unimplemented!()
    }
    
    fn export_metrics_as_prometheus(&self) -> String {
        unimplemented!()
    }
    
    fn reset_circuit_breaker(&self) {
        unimplemented!()
    }
    
    fn reset_circuit_breaker_metrics(&self) {
        unimplemented!()
    }
    
    fn with_circuit_breaker(self, _config: CircuitBreakerConfig) -> Self {
        unimplemented!()
    }
    
    fn with_exponential_backoff(self, _config: ExponentialBackoffConfig) -> Self {
        unimplemented!()
    }
    
    fn with_metrics_collection(self, _enabled: bool) -> Self {
        unimplemented!()
    }
    
    fn with_detailed_metrics(self, _enabled: bool) -> Self {
        unimplemented!()
    }
    
    fn last_heartbeat(&self) -> Option<std::time::Instant> {
        unimplemented!()
    }
    
    fn error_stream(&self) -> ErrorStream {
        unimplemented!()
    }
    
    fn extract_timestamp(&self, _message: &str) -> Option<std::time::SystemTime> {
        unimplemented!()
    }
    
    fn last_auth_header(&self) -> Option<String> {
        unimplemented!()
    }
    
    async fn refresh_token(&self, _token: &str) {
        unimplemented!()
    }
    
    fn current_token(&self) -> Option<&str> {
        unimplemented!()
    }
    
    fn error_metrics(&self) -> ErrorMetrics {
        unimplemented!()
    }
     
     fn with_bearer_token(self, _token: &str) -> Self {
         unimplemented!()
     }
     
     fn precision_metrics(&self) -> PrecisionMetrics {
         unimplemented!()
     }
 }

struct PriceStream;
struct PriceReceiver;

impl PriceStream {
    fn subscribe(&self) -> PriceReceiver {
        unimplemented!("PriceStream::subscribe() - to be implemented in GREEN phase")
    }
}

impl PriceReceiver {
    async fn recv(&mut self) -> Result<PriceUpdate, String> {
        unimplemented!("PriceReceiver::recv() - to be implemented in GREEN phase")
    }
}

#[derive(Clone, Debug)]
struct PriceUpdate {
    product_id: String,
    price: f64,
    timestamp: Option<SystemTime>,
    volume: Option<f64>,
    exchange: Option<String>,
    original_price_string: Option<String>,
    decimal_places: u8,
}

#[derive(Clone)]
struct PrecisionMetrics {
    total_precision_tests: u32,
    precision_preserved_count: u32,
    precision_loss_count: u32,
    max_decimal_places_handled: u8,
    min_value_handled: f64,
    max_value_handled: f64,
}

#[derive(Clone)]
struct ParsingMetrics {
    total_messages_parsed: u32,
    successful_parses: u32,
    parsing_errors: u32,
    average_parse_time: Duration,
    max_parse_time: Duration,
    error_recovery_count: u32,
}

#[derive(Clone)]
struct ValidationMetrics {
    total_validations: u32,
    validation_failures: u32,
    validation_successes: u32,
    missing_field_errors: u32,
    type_mismatch_errors: u32,
}

#[derive(Clone)]
struct TypeValidationMetrics {
    total_type_checks: u32,
    type_validation_successes: u32,
    type_validation_failures: u32,
    non_numeric_price_errors: u32,
    negative_price_errors: u32,
    zero_price_errors: u32,
}

struct ParsingErrorStream;
struct ValidationErrorStream;
struct TypeErrorStream;

impl ParsingErrorStream {
    fn subscribe(&self) -> ParsingErrorReceiver {
        unimplemented!("ParsingErrorStream::subscribe() - to be implemented in GREEN phase")
    }
}

impl ValidationErrorStream {
    fn subscribe(&self) -> ValidationErrorReceiver {
        unimplemented!("ValidationErrorStream::subscribe() - to be implemented in GREEN phase")
    }
}

impl TypeErrorStream {
    fn subscribe(&self) -> TypeErrorReceiver {
        unimplemented!("TypeErrorStream::subscribe() - to be implemented in GREEN phase")
    }
}

struct ParsingErrorReceiver;
struct ValidationErrorReceiver;
struct TypeErrorReceiver;

impl ParsingErrorReceiver {
    async fn recv(&mut self) -> Result<ParsingError, String> {
        unimplemented!("ParsingErrorReceiver::recv() - to be implemented in GREEN phase")
    }
}

impl ValidationErrorReceiver {
    async fn recv(&mut self) -> Result<ValidationError, String> {
        unimplemented!("ValidationErrorReceiver::recv() - to be implemented in GREEN phase")
    }
}

impl TypeErrorReceiver {
    async fn recv(&mut self) -> Result<TypeError, String> {
        unimplemented!("TypeErrorReceiver::recv() - to be implemented in GREEN phase")
    }
}

#[derive(Clone, Debug)]
struct ParsingError {
    raw_message: String,
    error_message: String,
    timestamp: Option<SystemTime>,
    line_number: Option<u32>,
    character_position: Option<u32>,
    error_type: ParsingErrorType,
}

#[derive(Clone, Debug)]
struct ValidationError {
    raw_message: String,
    error_summary: String,
    timestamp: Option<SystemTime>,
    missing_fields: Vec<String>,
    null_fields: Vec<String>,
    empty_fields: Vec<String>,
    type_mismatches: Vec<String>,
}

#[derive(Clone, Debug)]
struct TypeError {
    field_name: String,
    actual_type: String,
    expected_type: String,
    validation_rule: String,
    reason: String,
    raw_value: String,
    timestamp: Option<SystemTime>,
}

// Additional structs for reconnection tests
struct ReconnectionStream;
struct MockReconnectionReceiver;
#[derive(Clone)]
struct ReconnectionMetrics {
    total_attempts: u32,
    successful_reconnections: u32,
    max_retries_exceeded: bool,
    total_downtime: std::time::Duration,
    average_reconnection_time: Option<std::time::Duration>,
    backoff_resets: u32,
    current_backoff_delay: std::time::Duration,
    concurrent_attempt_conflicts: u32,
}

#[derive(Clone)]
struct ConnectionMetadata {
    original_connect_time: Option<std::time::Instant>,
    last_reconnect_time: Option<std::time::Instant>,
    reconnection_count: u32,
    session_id: Option<String>,
}

#[derive(Clone)]
struct BufferMetrics {
    messages_buffered: u64,
    messages_replayed: u64,
    buffer_overflows: u64,
    max_buffer_size: usize,
}

struct MessageStream;
struct MockMessageReceiver;

#[derive(Debug, Clone, PartialEq)]
enum ReconnectionEvent {
    AttemptStarted { 
        attempt_number: u32, 
        delay: Duration 
    },
    Connected { 
        attempt_number: u32, 
        total_downtime: Duration 
    },
    MaxRetriesExceeded { 
        total_attempts: u32 
    },
    Failed { 
        error: String, 
        attempt_number: u32 
    },
}

// Additional structs for circuit breaker tests
struct CircuitBreakerStream;
struct ConnectionAttemptStream;
struct MockCircuitBreakerReceiver;
struct MockConnectionAttemptReceiver;

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
    duration: Duration,
    from_state: CircuitState,
    to_state: CircuitState,
    timestamp: std::time::Instant,
}

#[derive(Clone)]
struct BackoffEvent {
    attempt_number: u32,
    timeout_duration: Duration,
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
    timestamp: std::time::Instant,
    error: String,
    connection_attempt_id: String,
}

struct MockSuccessEvent {
    timestamp: std::time::Instant,
    duration: Duration,
    connection_attempt_id: String,
}

struct MockTimeoutEvent {
    duration: Duration,
    from_state: CircuitState,
    to_state: CircuitState,
    timestamp: std::time::Instant,
}

struct MockBackoffEvent {
    attempt_number: u32,
    timeout_duration: Duration,
    calculated_at: Option<std::time::Instant>,
}

// Additional structs for connection tests
struct ErrorStream;
struct MockErrorReceiver;

#[derive(Clone)]
struct ErrorMetrics {
    malformed_json_count: u64,
    invalid_frame_count: u64,
    total_error_count: u64,
    last_error_timestamp: Option<std::time::SystemTime>,
}

// Implementations for new streams
impl ReconnectionStream {
    fn subscribe(&self) -> MockReconnectionReceiver {
        unimplemented!()
    }
}

impl MockReconnectionReceiver {
    async fn recv(&mut self) -> Result<ReconnectionEvent, String> {
        unimplemented!()
    }
}

impl MessageStream {
    fn subscribe(&self) -> MockMessageReceiver {
        unimplemented!()
    }
}

impl MockMessageReceiver {
    async fn recv(&mut self) -> Result<String, String> {
        unimplemented!()
    }
}

impl CircuitBreakerStream {
    fn subscribe(&self) -> MockCircuitBreakerReceiver {
        unimplemented!()
    }
}

impl ConnectionAttemptStream {
    fn subscribe(&self) -> MockConnectionAttemptReceiver {
        unimplemented!()
    }
}

impl MockCircuitBreakerReceiver {
    async fn recv(&mut self) -> Result<CircuitBreakerEvent, String> {
        unimplemented!()
    }
}

impl MockConnectionAttemptReceiver {
    async fn recv(&mut self) -> Result<ConnectionAttemptEvent, String> {
        unimplemented!()
    }
}

impl ErrorStream {
    fn subscribe(&self) -> MockErrorReceiver {
        unimplemented!()
    }
}

impl MockErrorReceiver {
    async fn recv(&mut self) -> Result<String, String> {
        unimplemented!()
    }
}

impl MockFailureEvent {
    fn is_within_window(&self, _duration: Duration) -> bool {
        unimplemented!()
    }
}

impl MockSuccessEvent {
    fn occurred_recently(&self, _duration: Duration) -> bool {
        unimplemented!()
    }
}

impl MockFailureEvent {
    fn is_recent(&self, _duration: Duration) -> bool {
        unimplemented!()
    }
}