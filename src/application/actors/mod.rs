pub mod trader_actor;
pub mod websocket_handler;

#[cfg(test)]
pub mod tests;

// Re-exports for convenience
pub use websocket_handler::{WebSocketClient, ConnectionState, CircuitBreaker, CircuitBreakerState, PriceUpdate};
