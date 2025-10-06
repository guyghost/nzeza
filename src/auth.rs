use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::collections::HashSet;
use std::sync::OnceLock;

/// Global store for valid API keys
static VALID_API_KEYS: OnceLock<HashSet<String>> = OnceLock::new();

/// Initialize the API keys from environment or default
pub fn init_api_keys() {
    let mut keys = HashSet::new();

    // Try to load from environment variable
    if let Ok(keys_env) = std::env::var("API_KEYS") {
        for key in keys_env.split(',') {
            let key = key.trim();
            if !key.is_empty() {
                keys.insert(key.to_string());
            }
        }
    }

    // If no keys were loaded, use a default key (for development only!)
    if keys.is_empty() {
        tracing::warn!("No API keys configured via API_KEYS env var, using default development key");
        keys.insert("dev_key_change_me_in_production".to_string());
    }

    VALID_API_KEYS.set(keys).expect("API keys already initialized");
    tracing::info!("API authentication initialized with {} valid key(s)",
        VALID_API_KEYS.get().unwrap().len());
}

/// Check if an API key is valid
fn is_valid_api_key(key: &str) -> bool {
    VALID_API_KEYS
        .get()
        .map(|keys| keys.contains(key))
        .unwrap_or(false)
}

/// Middleware to require authentication for protected endpoints
pub async fn require_auth(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract Authorization header
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok());

    // Check if it's a valid Bearer token
    match auth_header {
        Some(auth) if auth.starts_with("Bearer ") => {
            let key = &auth[7..]; // Skip "Bearer "
            if is_valid_api_key(key) {
                Ok(next.run(request).await)
            } else {
                tracing::warn!("Invalid API key attempted");
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        Some(_) => {
            tracing::warn!("Invalid Authorization header format (expected Bearer token)");
            Err(StatusCode::UNAUTHORIZED)
        }
        None => {
            tracing::warn!("Missing Authorization header");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_validation() {
        // Initialize with test keys
        let mut keys = HashSet::new();
        keys.insert("test_key_123".to_string());
        keys.insert("another_key_456".to_string());
        let _ = VALID_API_KEYS.set(keys);

        assert!(is_valid_api_key("test_key_123"));
        assert!(is_valid_api_key("another_key_456"));
        assert!(!is_valid_api_key("invalid_key"));
        assert!(!is_valid_api_key(""));
    }
}
