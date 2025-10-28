pub mod trader_actor;
pub mod websocket_handler;
pub mod websocket_client;
pub mod circuit_breaker;

#[cfg(test)]
pub mod tests;

// Re-exports for convenience
// All WebSocketClient and related types come from websocket_client module
pub use websocket_client::*;
pub use circuit_breaker::*;

// Re-export only specialized types from websocket_handler
pub use websocket_handler::{CircuitBreaker, CircuitBreakerState};
