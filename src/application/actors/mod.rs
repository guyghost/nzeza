pub mod trader_actor;
pub mod websocket_handler;
pub mod websocket_client;
pub mod circuit_breaker;

#[cfg(test)]
pub mod tests;

// Re-exports for convenience
pub use websocket_handler::{WebSocketClient, ConnectionState, CircuitBreaker, CircuitBreakerState, PriceUpdate};
pub use websocket_client::*;
pub use circuit_breaker::*;
