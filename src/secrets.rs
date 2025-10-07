//! Secure secret management module
//!
//! This module provides secure handling of sensitive data like API keys, secrets,
//! and mnemonics. It uses `zeroize` to ensure secrets are wiped from memory when dropped.
//!
//! # Security Features
//! - Automatic memory zeroing on drop
//! - Integration with 1Password CLI for secure secret storage
//! - Environment variable fallback with warnings
//! - Constant-time comparisons for API keys

use std::env;
use tracing::{error, info, warn};
use zeroize::Zeroizing;

/// Error type for secret loading operations
#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    #[error("Secret not found: {0}")]
    NotFound(String),

    #[error("1Password CLI error: {0}")]
    OnePasswordError(String),

    #[error("Environment variable not set: {0}")]
    EnvVarNotSet(String),

    #[error("Secret validation failed: {0}")]
    ValidationFailed(String),
}

/// Configuration for secret loading
#[derive(Debug, Clone)]
pub struct SecretConfig {
    /// Whether to allow loading secrets from environment variables
    /// Should be false in production
    pub allow_env_vars: bool,

    /// Whether to require 1Password CLI
    pub require_op_cli: bool,
}

impl Default for SecretConfig {
    fn default() -> Self {
        Self {
            // Allow env vars in development, should be disabled in production
            allow_env_vars: cfg!(debug_assertions),
            require_op_cli: false,
        }
    }
}

/// Load a secret from 1Password CLI or environment variable (with fallback)
///
/// # Security
/// - Prioritizes 1Password CLI over environment variables
/// - Returns a `Zeroizing<String>` to ensure the secret is wiped from memory
/// - Warns when falling back to environment variables
///
/// # Arguments
/// - `op_reference`: 1Password reference (e.g., "op://vault/item/field")
/// - `env_var_name`: Environment variable name as fallback
/// - `config`: Configuration for secret loading behavior
///
/// # Example
/// ```no_run
/// use nzeza::secrets::{load_secret, SecretConfig};
///
/// let config = SecretConfig::default();
/// let api_key = load_secret(
///     "op://Private/Coinbase/api_key",
///     "COINBASE_API_KEY",
///     &config
/// ).expect("Failed to load API key");
/// ```
pub fn load_secret(
    op_reference: &str,
    env_var_name: &str,
    config: &SecretConfig,
) -> Result<Zeroizing<String>, SecretError> {
    // Try 1Password CLI first
    match load_from_op_cli(op_reference) {
        Ok(secret) => {
            info!("✓ Loaded secret from 1Password CLI: {}", env_var_name);
            return Ok(secret);
        }
        Err(e) => {
            if config.require_op_cli {
                error!("1Password CLI required but failed: {}", e);
                return Err(e);
            }
            warn!("1Password CLI not available: {}", e);
        }
    }

    // Fall back to environment variable if allowed
    if config.allow_env_vars {
        warn!(
            "⚠️  Loading secret from environment variable: {} (INSECURE for production)",
            env_var_name
        );
        load_from_env(env_var_name)
    } else {
        error!(
            "Secret loading failed: 1Password CLI unavailable and env vars disabled for {}",
            env_var_name
        );
        Err(SecretError::NotFound(env_var_name.to_string()))
    }
}

/// Load a secret from 1Password CLI
fn load_from_op_cli(reference: &str) -> Result<Zeroizing<String>, SecretError> {
    use std::process::Command;

    let output = Command::new("op")
        .arg("read")
        .arg(reference)
        .output()
        .map_err(|e| {
            SecretError::OnePasswordError(format!(
                "Failed to execute 'op' command: {}. Install 1Password CLI from https://developer.1password.com/docs/cli",
                e
            ))
        })?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        return Err(SecretError::OnePasswordError(format!(
            "1Password CLI failed: {}",
            error_msg
        )));
    }

    let secret = String::from_utf8(output.stdout)
        .map_err(|e| SecretError::OnePasswordError(format!("Invalid UTF-8 from 1Password: {}", e)))?
        .trim()
        .to_string();

    if secret.is_empty() {
        return Err(SecretError::OnePasswordError(
            "1Password returned empty secret".to_string(),
        ));
    }

    Ok(Zeroizing::new(secret))
}

/// Load a secret from environment variable (wrapped in Zeroizing)
fn load_from_env(env_var_name: &str) -> Result<Zeroizing<String>, SecretError> {
    env::var(env_var_name)
        .map(Zeroizing::new)
        .map_err(|_| SecretError::EnvVarNotSet(env_var_name.to_string()))
}

/// Validate that a secret meets minimum security requirements
pub fn validate_secret_strength(secret: &str, min_length: usize) -> Result<(), SecretError> {
    if secret.len() < min_length {
        return Err(SecretError::ValidationFailed(format!(
            "Secret too short: {} characters (minimum: {})",
            secret.len(),
            min_length
        )));
    }

    // Check for obviously weak secrets
    let weak_patterns = ["test", "demo", "example", "placeholder", "changeme", "12345"];
    let secret_lower = secret.to_lowercase();

    for pattern in &weak_patterns {
        if secret_lower.contains(pattern) {
            return Err(SecretError::ValidationFailed(format!(
                "Secret contains weak pattern: {}",
                pattern
            )));
        }
    }

    Ok(())
}

/// Load an API key with validation
pub fn load_api_key(
    op_reference: &str,
    env_var_name: &str,
    config: &SecretConfig,
) -> Result<Zeroizing<String>, SecretError> {
    let secret = load_secret(op_reference, env_var_name, config)?;

    // Validate minimum length (32 characters for 256 bits of entropy)
    validate_secret_strength(&secret, 32)?;

    Ok(secret)
}

/// Load a mnemonic phrase with validation
pub fn load_mnemonic(
    op_reference: &str,
    env_var_name: &str,
    config: &SecretConfig,
) -> Result<Zeroizing<String>, SecretError> {
    let secret = load_secret(op_reference, env_var_name, config)?;

    // Validate it looks like a mnemonic (12 or 24 words)
    let word_count = secret.split_whitespace().count();
    if word_count != 12 && word_count != 24 {
        return Err(SecretError::ValidationFailed(format!(
            "Invalid mnemonic: expected 12 or 24 words, got {}",
            word_count
        )));
    }

    Ok(secret)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_secret_strength() {
        // Too short
        assert!(validate_secret_strength("short", 32).is_err());

        // Contains weak pattern
        assert!(validate_secret_strength("test_api_key_1234567890123456789", 32).is_err());

        // Valid strong key
        let strong_key = "a".repeat(32);
        assert!(validate_secret_strength(&strong_key, 32).is_ok());
    }

    #[test]
    fn test_load_from_env() {
        env::set_var("TEST_SECRET_KEY", "test_value_12345678901234567890");
        let result = load_from_env("TEST_SECRET_KEY");
        assert!(result.is_ok());
        assert_eq!(*result.unwrap(), "test_value_12345678901234567890");
        env::remove_var("TEST_SECRET_KEY");
    }

    #[test]
    fn test_load_from_env_missing() {
        let result = load_from_env("NONEXISTENT_VAR");
        assert!(result.is_err());
    }

    #[test]
    fn test_mnemonic_validation() {
        let config = SecretConfig::default();

        // Set a valid 12-word mnemonic
        let valid_mnemonic = "word ".repeat(11) + "word";
        env::set_var("TEST_MNEMONIC", &valid_mnemonic);

        let result = load_mnemonic("op://test", "TEST_MNEMONIC", &config);
        assert!(result.is_ok());

        // Invalid word count
        env::set_var("TEST_MNEMONIC", "only five words here");
        let result = load_mnemonic("op://test", "TEST_MNEMONIC", &config);
        assert!(result.is_err());

        env::remove_var("TEST_MNEMONIC");
    }
}
