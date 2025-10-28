// Test modules for WebSocket integration tests
// All tests are in RED phase (failing) - they reference functionality that doesn't exist yet

pub mod mock_websocket_server;
pub mod websocket_connection_tests;
pub mod websocket_reconnection_tests;
pub mod websocket_price_parsing_tests;
pub mod websocket_circuit_breaker_tests;

// Re-export the mock server for easy access in tests
pub use mock_websocket_server::MockWebSocketServer;