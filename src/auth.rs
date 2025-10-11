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

/// Initialize the API keys from environment
///
/// # Security
/// This function requires the `API_KEYS` environment variable to be set.
/// If no API keys are configured, the application will **panic** to prevent
/// running in an insecure state. This is intentional fail-secure behavior.
///
/// # Panics
/// Panics if the `API_KEYS` environment variable is not set or is empty.
/// This prevents accidentally running the application without authentication.
pub fn init_api_keys() {
    let mut keys = HashSet::new();

    // Load from environment variable (REQUIRED)
    let keys_env = std::env::var("API_KEYS")
        .expect("SECURITY ERROR: API_KEYS environment variable is not set. \
                 Set API_KEYS to a comma-separated list of secure API keys. \
                 Example: API_KEYS=your_secure_key_here,another_key");

    // Minimum required key length for security (256 bits = 32 bytes)
    const MIN_KEY_LENGTH: usize = 32;

    for key in keys_env.split(',') {
        let key = key.trim();
        if !key.is_empty() {
            // ENFORCE minimum key length for security
            if key.len() < MIN_KEY_LENGTH {
                tracing::error!(
                    "SECURITY ERROR: API key is too weak (length: {}, minimum: {})",
                    key.len(),
                    MIN_KEY_LENGTH
                );
                panic!(
                    "SECURITY ERROR: API key must be at least {} characters long. \
                     Found key with length {}. \
                     Generate a secure key with: openssl rand -base64 32",
                    MIN_KEY_LENGTH,
                    key.len()
                );
            }
            keys.insert(key.to_string());
        }
    }

    // Fail if no valid keys were loaded
    if keys.is_empty() {
        panic!("SECURITY ERROR: No valid API keys found in API_KEYS environment variable. \
                At least one API key with length >= {} characters is required. \
                Generate a secure key with: openssl rand -base64 32",
                MIN_KEY_LENGTH);
    }

    VALID_API_KEYS.set(keys).expect("API keys already initialized");
    tracing::info!("✓ API authentication initialized with {} valid key(s)",
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
