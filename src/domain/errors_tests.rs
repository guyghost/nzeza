//! Comprehensive tests for improved error handling in NZEZA trading system
//! These tests define the expected error types and context for various failure scenarios.
//! All tests are currently RED (failing) and serve as specifications for TDD.

#[cfg(test)]
mod error_handling_tests {
    use std::time::Duration;

    /// Error type that includes rich context information
    #[derive(Debug, Clone)]
    pub enum DetailedMpcError {
        /// Order validation failed with specific context
        OrderValidationFailed {
            symbol: String,
            reason: String,
        },
        /// Insufficient balance to execute order
        InsufficientBalance {
            required: f64,
            available: f64,
            currency: String,
        },
        /// Position limits exceeded
        PositionLimitExceeded {
            symbol: String,
            limit: u32,
            current: u32,
            limit_type: PositionLimitType,
        },
        /// No traders available to execute orders
        TraderUnavailable {
            reason: String,
            available_traders: Vec<String>,
        },
        /// Exchange connection issue
        ExchangeConnectionLost {
            exchange: String,
            last_contact: Duration,
            reason: String,
        },
        /// Insufficient data for signal generation
        InsufficientCandles {
            symbol: String,
            required: usize,
            current: usize,
        },
        /// Signal confidence too low (warning, not error)
        LowConfidenceSignal {
            symbol: String,
            confidence: f64,
            threshold: f64,
        },
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum PositionLimitType {
        PerSymbol,
        Total,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum ErrorSeverity {
        Minor,
        Moderate,
        Critical,
    }

    impl DetailedMpcError {
        pub fn severity(&self) -> ErrorSeverity {
            match self {
                DetailedMpcError::LowConfidenceSignal { .. } => ErrorSeverity::Minor,
                DetailedMpcError::InsufficientCandles { .. } => ErrorSeverity::Moderate,
                DetailedMpcError::PositionLimitExceeded { .. } => ErrorSeverity::Moderate,
                DetailedMpcError::OrderValidationFailed { .. } => ErrorSeverity::Moderate,
                DetailedMpcError::InsufficientBalance { .. } => ErrorSeverity::Moderate,
                DetailedMpcError::TraderUnavailable { .. } => ErrorSeverity::Critical,
                DetailedMpcError::ExchangeConnectionLost { .. } => ErrorSeverity::Critical,
            }
        }

        pub fn to_string(&self) -> String {
            match self {
                DetailedMpcError::OrderValidationFailed { symbol, reason } => {
                    format!("Order validation failed for {}: {}", symbol, reason)
                }
                DetailedMpcError::InsufficientBalance {
                    required,
                    available,
                    currency,
                } => {
                    format!(
                        "Insufficient {} balance: required {:.2}, available {:.2}",
                        currency, required, available
                    )
                }
                DetailedMpcError::PositionLimitExceeded {
                    symbol,
                    limit,
                    current,
                    limit_type,
                } => {
                    let limit_desc = match limit_type {
                        PositionLimitType::PerSymbol => format!("per symbol {}", symbol),
                        PositionLimitType::Total => "total".to_string(),
                    };
                    format!(
                        "Position limit exceeded ({}): {} current, {} limit",
                        limit_desc, current, limit
                    )
                }
                DetailedMpcError::TraderUnavailable {
                    reason,
                    available_traders,
                } => {
                    format!(
                        "No trader available: {}. Available traders: {:?}",
                        reason, available_traders
                    )
                }
                DetailedMpcError::ExchangeConnectionLost {
                    exchange,
                    last_contact,
                    reason,
                } => {
                    format!(
                        "Lost connection to {} (last contact: {:?}s ago): {}",
                        exchange,
                        last_contact.as_secs(),
                        reason
                    )
                }
                DetailedMpcError::InsufficientCandles {
                    symbol,
                    required,
                    current,
                } => {
                    format!(
                        "Insufficient candles for {}: need {}, have {}",
                        symbol, required, current
                    )
                }
                DetailedMpcError::LowConfidenceSignal {
                    symbol,
                    confidence,
                    threshold,
                } => {
                    format!(
                        "Signal confidence {:.3} below threshold {:.3} for {}",
                        confidence, threshold, symbol
                    )
                }
            }
        }
    }

    // ============================================================================
    // ORDER VALIDATION ERROR TESTS
    // ============================================================================

    /// Test that order validation errors include the symbol being validated
    #[test]
    fn test_order_validation_error_includes_symbol() {
        let error = DetailedMpcError::OrderValidationFailed {
            symbol: "BTC-USD".to_string(),
            reason: "Symbol not in whitelist".to_string(),
        };

        let error_msg = error.to_string();
        assert!(
            error_msg.contains("BTC-USD"),
            "Error message should include the symbol"
        );
    }

    /// Test that order validation errors include the validation failure reason
    #[test]
    fn test_order_validation_error_includes_reason() {
        let reason = "Symbol not in whitelist";
        let error = DetailedMpcError::OrderValidationFailed {
            symbol: "BTC-USD".to_string(),
            reason: reason.to_string(),
        };

        let error_msg = error.to_string();
        assert!(
            error_msg.contains(reason),
            "Error message should include the validation reason"
        );
    }

    /// Test that insufficient balance errors show both required and available amounts
    #[test]
    fn test_insufficient_balance_error_includes_amounts() {
        let error = DetailedMpcError::InsufficientBalance {
            required: 1000.50,
            available: 500.25,
            currency: "USD".to_string(),
        };

        let error_msg = error.to_string();
        assert!(
            error_msg.contains("1000.50"),
            "Error should include required amount"
        );
        assert!(
            error_msg.contains("500.25"),
            "Error should include available amount"
        );
    }

    // ============================================================================
    // POSITION LIMIT ERROR TESTS
    // ============================================================================

    /// Test that position limit errors include both current and limit values
    #[test]
    fn test_position_limit_exceeded_error_includes_limit_and_current() {
        let error = DetailedMpcError::PositionLimitExceeded {
            symbol: "ETH-USD".to_string(),
            limit: 3,
            current: 5,
            limit_type: PositionLimitType::PerSymbol,
        };

        let error_msg = error.to_string();
        assert!(
            error_msg.contains("3") && error_msg.contains("5"),
            "Error should show both limit and current position count"
        );
    }

    /// Test symbol-specific position limit error with context
    #[test]
    fn test_symbol_position_limit_error() {
        let symbol = "SOL-USD";
        let error = DetailedMpcError::PositionLimitExceeded {
            symbol: symbol.to_string(),
            limit: 2,
            current: 2,
            limit_type: PositionLimitType::PerSymbol,
        };

        let error_msg = error.to_string();
        assert!(
            error_msg.contains(symbol),
            "Error should reference the specific symbol"
        );
        assert!(
            error_msg.contains("per symbol"),
            "Error should indicate it's a per-symbol limit"
        );
    }

    /// Test total portfolio position limit error
    #[test]
    fn test_total_position_limit_error() {
        let error = DetailedMpcError::PositionLimitExceeded {
            symbol: "BTC-USD".to_string(),
            limit: 10,
            current: 10,
            limit_type: PositionLimitType::Total,
        };

        let error_msg = error.to_string();
        assert!(
            error_msg.contains("total"),
            "Error should indicate total portfolio limit"
        );
    }

    // ============================================================================
    // TRADER AVAILABILITY ERROR TESTS
    // ============================================================================

    /// Test that trader unavailable error includes the specific reason
    #[test]
    fn test_trader_unavailable_error_includes_reason() {
        let reason = "All traders disconnected from exchange";
        let error = DetailedMpcError::TraderUnavailable {
            reason: reason.to_string(),
            available_traders: vec![],
        };

        let error_msg = error.to_string();
        assert!(
            error_msg.contains(reason),
            "Error should include the unavailability reason"
        );
    }

    /// Test no traders available error with context about available traders
    #[test]
    fn test_no_traders_available_error_context() {
        let available = vec!["trader_fast".to_string(), "trader_momentum".to_string()];
        let error = DetailedMpcError::TraderUnavailable {
            reason: "All available traders are busy".to_string(),
            available_traders: available.clone(),
        };

        let error_msg = error.to_string();
        assert!(
            error_msg.contains("trader_fast"),
            "Error should reference available traders"
        );
    }

    // ============================================================================
    // EXCHANGE CONNECTION ERROR TESTS
    // ============================================================================

    /// Test exchange connection loss error includes exchange name and last contact time
    #[test]
    fn test_exchange_connection_lost_includes_exchange_and_duration() {
        let error = DetailedMpcError::ExchangeConnectionLost {
            exchange: "Coinbase".to_string(),
            last_contact: Duration::from_secs(45),
            reason: "WebSocket closed unexpectedly".to_string(),
        };

        let error_msg = error.to_string();
        assert!(
            error_msg.contains("Coinbase"),
            "Error should include exchange name"
        );
        assert!(
            error_msg.contains("45"),
            "Error should include time since last contact"
        );
    }

    /// Test exchange timeout error with detailed context
    #[test]
    fn test_exchange_timeout_error_with_context() {
        let error = DetailedMpcError::ExchangeConnectionLost {
            exchange: "dYdX".to_string(),
            last_contact: Duration::from_secs(120),
            reason: "Request timeout after 5 retries".to_string(),
        };

        let error_msg = error.to_string();
        assert!(
            error_msg.contains("dYdX"),
            "Error should specify which exchange"
        );
        assert!(
            error_msg.contains("120"),
            "Error should show how long connection has been lost"
        );
    }

    // ============================================================================
    // SIGNAL GENERATION ERROR TESTS
    // ============================================================================

    /// Test insufficient candles error includes current and required counts
    #[test]
    fn test_insufficient_candles_error_includes_current_and_required() {
        let error = DetailedMpcError::InsufficientCandles {
            symbol: "BTC-USD".to_string(),
            required: 20,
            current: 5,
        };

        let error_msg = error.to_string();
        assert!(
            error_msg.contains("BTC-USD"),
            "Error should include symbol"
        );
        assert!(
            error_msg.contains("20") && error_msg.contains("5"),
            "Error should show both required and current candle count"
        );
    }

    /// Test that low confidence signals are warnings, not errors
    #[test]
    fn test_low_confidence_signal_not_an_error() {
        let error = DetailedMpcError::LowConfidenceSignal {
            symbol: "ETH-USD".to_string(),
            confidence: 0.35,
            threshold: 0.5,
        };

        // Should be classified as Minor severity (warning level), not critical
        assert_eq!(
            error.severity(),
            ErrorSeverity::Minor,
            "Low confidence should be a warning, not a critical error"
        );
    }

    // ============================================================================
    // ERROR CONTEXT & SEVERITY TESTS
    // ============================================================================

    /// Test error severity classification
    #[test]
    fn test_error_severity_classification() {
        let critical_error = DetailedMpcError::TraderUnavailable {
            reason: "All traders down".to_string(),
            available_traders: vec![],
        };

        let moderate_error = DetailedMpcError::PositionLimitExceeded {
            symbol: "BTC-USD".to_string(),
            limit: 5,
            current: 5,
            limit_type: PositionLimitType::Total,
        };

        let minor_error = DetailedMpcError::LowConfidenceSignal {
            symbol: "BTC-USD".to_string(),
            confidence: 0.3,
            threshold: 0.5,
        };

        assert_eq!(critical_error.severity(), ErrorSeverity::Critical);
        assert_eq!(moderate_error.severity(), ErrorSeverity::Moderate);
        assert_eq!(minor_error.severity(), ErrorSeverity::Minor);
    }

    /// Test error context preservation through operations
    #[test]
    fn test_error_context_preservation() {
        let original_symbol = "BTC-USD";
        let original_reason = "Invalid price format";

        let error = DetailedMpcError::OrderValidationFailed {
            symbol: original_symbol.to_string(),
            reason: original_reason.to_string(),
        };

        let error_msg = error.to_string();

        // Verify context is preserved
        assert!(error_msg.contains(original_symbol));
        assert!(error_msg.contains(original_reason));
    }

    // ============================================================================
    // HELPER FUNCTION TESTS
    // ============================================================================

    /// Test creating insufficient balance error with realistic values
    #[test]
    fn test_create_insufficient_balance_error_realistic_values() {
        let portfolio_value = 1000.0;
        let required_for_position = 2500.0;

        let error = DetailedMpcError::InsufficientBalance {
            required: required_for_position,
            available: portfolio_value,
            currency: "USD".to_string(),
        };

        assert_eq!(error.severity(), ErrorSeverity::Moderate);
        let msg = error.to_string();
        assert!(msg.contains("2500.00"));
        assert!(msg.contains("1000.00"));
    }

    /// Test creating position limit error with realistic values
    #[test]
    fn test_create_position_limit_error_realistic_values() {
        let max_per_symbol = 3;
        let current_positions = 3;

        let error = DetailedMpcError::PositionLimitExceeded {
            symbol: "BTC-USD".to_string(),
            limit: max_per_symbol,
            current: current_positions,
            limit_type: PositionLimitType::PerSymbol,
        };

        let msg = error.to_string();
        assert!(msg.contains("3"));
        assert!(msg.contains("BTC-USD"));
    }

    /// Test creating trader unavailable error
    #[test]
    fn test_create_trader_unavailable_error() {
        let available_traders = vec![
            "trader_fast".to_string(),
            "trader_momentum".to_string(),
        ];

        let error = DetailedMpcError::TraderUnavailable {
            reason: "All traders at maximum concurrent orders".to_string(),
            available_traders: available_traders.clone(),
        };

        let msg = error.to_string();
        assert!(msg.contains("All traders at maximum"));
        assert!(msg.len() > 0);
    }

    /// Test creating exchange connection error with realistic timing
    #[test]
    fn test_create_exchange_connection_error_realistic_timing() {
        let disconnection_duration = Duration::from_secs(300); // 5 minutes

        let error = DetailedMpcError::ExchangeConnectionLost {
            exchange: "Coinbase".to_string(),
            last_contact: disconnection_duration,
            reason: "WebSocket connection timeout".to_string(),
        };

        let msg = error.to_string();
        assert!(msg.contains("300"));
        assert!(msg.contains("Coinbase"));
    }

    /// Test creating insufficient candles error
    #[test]
    fn test_create_insufficient_candles_error() {
        let symbol = "ETH-USD";
        let required = 50;
        let current = 10;

        let error = DetailedMpcError::InsufficientCandles {
            symbol: symbol.to_string(),
            required,
            current,
        };

        let msg = error.to_string();
        assert!(msg.contains(symbol));
        assert!(msg.contains("50"));
        assert!(msg.contains("10"));
    }

    // ============================================================================
    // EDGE CASE TESTS
    // ============================================================================

    /// Test error message generation for zero available balance
    #[test]
    fn test_error_message_zero_balance() {
        let error = DetailedMpcError::InsufficientBalance {
            required: 100.0,
            available: 0.0,
            currency: "USD".to_string(),
        };

        let msg = error.to_string();
        assert!(msg.contains("0.00"), "Should handle zero balance correctly");
    }

    /// Test error with maximum values
    #[test]
    fn test_error_with_large_values() {
        let error = DetailedMpcError::PositionLimitExceeded {
            symbol: "BTC-USD".to_string(),
            limit: 1000,
            current: 1000,
            limit_type: PositionLimitType::Total,
        };

        let msg = error.to_string();
        assert!(msg.contains("1000"));
    }

    /// Test error messages with special characters in symbol names
    #[test]
    fn test_error_with_special_symbol_characters() {
        let symbol = "BTC/USD";
        let error = DetailedMpcError::OrderValidationFailed {
            symbol: symbol.to_string(),
            reason: "Unsupported pair format".to_string(),
        };

        let msg = error.to_string();
        assert!(msg.contains(symbol));
    }

    /// Test multiple errors can be created independently
    #[test]
    fn test_multiple_errors_independent() {
        let error1 = DetailedMpcError::InsufficientBalance {
            required: 1000.0,
            available: 500.0,
            currency: "USD".to_string(),
        };

        let error2 = DetailedMpcError::PositionLimitExceeded {
            symbol: "BTC-USD".to_string(),
            limit: 5,
            current: 5,
            limit_type: PositionLimitType::Total,
        };

        // Errors should be independent
        assert_ne!(error1.to_string(), error2.to_string());
    }
}
