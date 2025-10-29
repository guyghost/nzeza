use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use std::num::NonZeroU32;
use std::sync::Arc;

/// Rate limiter configuration
pub struct RateLimiterConfig {
    /// Maximum requests per minute
    pub requests_per_minute: u32,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 100, // Default: 100 requests per minute
        }
    }
}

/// Global rate limiter
pub type GlobalRateLimiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>;

/// Create a new rate limiter
pub fn create_rate_limiter(config: RateLimiterConfig) -> GlobalRateLimiter {
    let quota = Quota::per_minute(
        NonZeroU32::new(config.requests_per_minute).expect("Requests per minute must be non-zero"),
    );
    Arc::new(RateLimiter::direct(quota))
}

/// Middleware to apply rate limiting
pub async fn rate_limit_middleware(
    limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
    request: Request,
    next: Next,
) -> Response {
    match limiter.check() {
        Ok(_) => next.run(request).await,
        Err(_) => {
            tracing::warn!("Rate limit exceeded");
            (
                StatusCode::TOO_MANY_REQUESTS,
                "Rate limit exceeded. Please try again later.",
            )
                .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_creation() {
        let config = RateLimiterConfig {
            requests_per_minute: 50,
        };
        let limiter = create_rate_limiter(config);

        // Should allow first request
        assert!(limiter.check().is_ok());
    }

    #[test]
    fn test_default_config() {
        let config = RateLimiterConfig::default();
        assert_eq!(config.requests_per_minute, 100);
    }
}
