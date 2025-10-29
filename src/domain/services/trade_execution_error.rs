//! Trade execution errors with detailed context

use std::fmt;
use thiserror::Error;

/// Detailed error type for trade execution failures
#[derive(Debug, Clone, Error, PartialEq)]
pub enum TradeExecutionError {
    /// Insufficient available balance to execute the trade
    #[error("Insufficient balance: required {required:.2}, available {available:.2}")]
    InsufficientBalance { required: f64, available: f64 },

    /// Insufficient leverage to execute the trade
    #[error("Insufficient leverage: required {required:.2}, available {available:.2}")]
    InsufficientLeverage { required: f64, available: f64 },

    /// Failed to fetch balance from exchange
    #[error("Failed to fetch balance: {reason}")]
    BalanceFetchFailed { reason: String },

    /// Failed to calculate available leverage
    #[error("Failed to calculate leverage: {reason}")]
    LeverageCalculationFailed { reason: String },

    /// Position sizing calculation failed
    #[error("Position sizing failed: {reason}")]
    PositionSizingFailed { reason: String },

    /// Order placement failed at exchange
    #[error("Order placement failed: {reason}")]
    OrderPlacementFailed { reason: String },

    /// Managers not properly configured
    #[error("Manager not configured: {manager_name}")]
    ManagerNotConfigured { manager_name: String },

    /// Invalid signal provided
    #[error("Invalid signal: {reason}")]
    InvalidSignal { reason: String },

    /// Position size invalid or below minimum
    #[error("Invalid position size: {reason}")]
    InvalidPositionSize { reason: String },
}

impl TradeExecutionError {
    /// Get the severity level of this error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            TradeExecutionError::InvalidSignal { .. } => ErrorSeverity::Minor,
            TradeExecutionError::InsufficientBalance { .. } => ErrorSeverity::Moderate,
            TradeExecutionError::InsufficientLeverage { .. } => ErrorSeverity::Moderate,
            TradeExecutionError::InvalidPositionSize { .. } => ErrorSeverity::Moderate,
            TradeExecutionError::PositionSizingFailed { .. } => ErrorSeverity::Moderate,
            TradeExecutionError::BalanceFetchFailed { .. } => ErrorSeverity::Moderate,
            TradeExecutionError::LeverageCalculationFailed { .. } => ErrorSeverity::Moderate,
            TradeExecutionError::OrderPlacementFailed { .. } => ErrorSeverity::Critical,
            TradeExecutionError::ManagerNotConfigured { .. } => ErrorSeverity::Critical,
        }
    }

    /// Check if this error is recoverable (retrying might help)
    pub fn is_recoverable(&self) -> bool {
        match self {
            // These might be temporary (network issues, temporary balance/leverage checks)
            TradeExecutionError::BalanceFetchFailed { .. } => true,
            TradeExecutionError::LeverageCalculationFailed { .. } => true,
            TradeExecutionError::OrderPlacementFailed { .. } => true,

            // These are permanent (configuration, validation, business logic)
            TradeExecutionError::ManagerNotConfigured { .. } => false,
            TradeExecutionError::InvalidSignal { .. } => false,
            TradeExecutionError::InsufficientBalance { .. } => false,
            TradeExecutionError::InsufficientLeverage { .. } => false,
            TradeExecutionError::PositionSizingFailed { .. } => false,
            TradeExecutionError::InvalidPositionSize { .. } => false,
        }
    }

    /// Get a short error code for logging/monitoring
    pub fn error_code(&self) -> &'static str {
        match self {
            TradeExecutionError::InsufficientBalance { .. } => "ERR_INSUFFICIENT_BALANCE",
            TradeExecutionError::InsufficientLeverage { .. } => "ERR_INSUFFICIENT_LEVERAGE",
            TradeExecutionError::BalanceFetchFailed { .. } => "ERR_BALANCE_FETCH",
            TradeExecutionError::LeverageCalculationFailed { .. } => "ERR_LEVERAGE_CALC",
            TradeExecutionError::PositionSizingFailed { .. } => "ERR_POSITION_SIZING",
            TradeExecutionError::OrderPlacementFailed { .. } => "ERR_ORDER_PLACEMENT",
            TradeExecutionError::ManagerNotConfigured { .. } => "ERR_MANAGER_NOT_CONFIGURED",
            TradeExecutionError::InvalidSignal { .. } => "ERR_INVALID_SIGNAL",
            TradeExecutionError::InvalidPositionSize { .. } => "ERR_INVALID_POSITION_SIZE",
        }
    }
}

/// Severity levels for trade execution errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    /// Non-critical, expected to occur occasionally
    Minor,
    /// Moderate issues that indicate problems
    Moderate,
    /// Critical issues that require immediate attention
    Critical,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorSeverity::Minor => write!(f, "Minor"),
            ErrorSeverity::Moderate => write!(f, "Moderate"),
            ErrorSeverity::Critical => write!(f, "Critical"),
        }
    }
}

// Implement conversion from TradeExecutionError to String for compatibility
impl From<TradeExecutionError> for String {
    fn from(error: TradeExecutionError) -> Self {
        error.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insufficient_balance_error() {
        let error = TradeExecutionError::InsufficientBalance {
            required: 1000.0,
            available: 500.0,
        };
        assert_eq!(
            error.to_string(),
            "Insufficient balance: required 1000.00, available 500.00"
        );
    }

    #[test]
    fn test_insufficient_leverage_error() {
        let error = TradeExecutionError::InsufficientLeverage {
            required: 5.0,
            available: 2.0,
        };
        assert_eq!(
            error.to_string(),
            "Insufficient leverage: required 5.00, available 2.00"
        );
    }

    #[test]
    fn test_balance_fetch_failed_error() {
        let error = TradeExecutionError::BalanceFetchFailed {
            reason: "Connection timeout".to_string(),
        };
        assert_eq!(error.to_string(), "Failed to fetch balance: Connection timeout");
    }

    #[test]
    fn test_leverage_calculation_failed_error() {
        let error = TradeExecutionError::LeverageCalculationFailed {
            reason: "Invalid position data".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "Failed to calculate leverage: Invalid position data"
        );
    }

    #[test]
    fn test_position_sizing_failed_error() {
        let error = TradeExecutionError::PositionSizingFailed {
            reason: "Price too high".to_string(),
        };
        assert_eq!(error.to_string(), "Position sizing failed: Price too high");
    }

    #[test]
    fn test_order_placement_failed_error() {
        let error = TradeExecutionError::OrderPlacementFailed {
            reason: "Network error".to_string(),
        };
        assert_eq!(error.to_string(), "Order placement failed: Network error");
    }

    #[test]
    fn test_manager_not_configured_error() {
        let error = TradeExecutionError::ManagerNotConfigured {
            manager_name: "BalanceManager".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "Manager not configured: BalanceManager"
        );
    }

    #[test]
    fn test_invalid_signal_error() {
        let error = TradeExecutionError::InvalidSignal {
            reason: "Confidence below threshold".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "Invalid signal: Confidence below threshold"
        );
    }

    #[test]
    fn test_invalid_position_size_error() {
        let error = TradeExecutionError::InvalidPositionSize {
            reason: "Quantity below minimum".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "Invalid position size: Quantity below minimum"
        );
    }

    #[test]
    fn test_error_severity_insufficient_balance() {
        let error = TradeExecutionError::InsufficientBalance {
            required: 1000.0,
            available: 500.0,
        };
        assert_eq!(error.severity(), ErrorSeverity::Moderate);
    }

    #[test]
    fn test_error_severity_order_placement_failed() {
        let error = TradeExecutionError::OrderPlacementFailed {
            reason: "Network error".to_string(),
        };
        assert_eq!(error.severity(), ErrorSeverity::Critical);
    }

    #[test]
    fn test_error_severity_manager_not_configured() {
        let error = TradeExecutionError::ManagerNotConfigured {
            manager_name: "BalanceManager".to_string(),
        };
        assert_eq!(error.severity(), ErrorSeverity::Critical);
    }

    #[test]
    fn test_error_is_recoverable_balance_fetch_failed() {
        let error = TradeExecutionError::BalanceFetchFailed {
            reason: "Network timeout".to_string(),
        };
        assert!(error.is_recoverable());
    }

    #[test]
    fn test_error_is_not_recoverable_insufficient_balance() {
        let error = TradeExecutionError::InsufficientBalance {
            required: 1000.0,
            available: 500.0,
        };
        assert!(!error.is_recoverable());
    }

    #[test]
    fn test_error_is_not_recoverable_manager_not_configured() {
        let error = TradeExecutionError::ManagerNotConfigured {
            manager_name: "BalanceManager".to_string(),
        };
        assert!(!error.is_recoverable());
    }

    #[test]
    fn test_error_code_insufficient_balance() {
        let error = TradeExecutionError::InsufficientBalance {
            required: 1000.0,
            available: 500.0,
        };
        assert_eq!(error.error_code(), "ERR_INSUFFICIENT_BALANCE");
    }

    #[test]
    fn test_error_code_order_placement_failed() {
        let error = TradeExecutionError::OrderPlacementFailed {
            reason: "Network error".to_string(),
        };
        assert_eq!(error.error_code(), "ERR_ORDER_PLACEMENT");
    }

    #[test]
    fn test_error_code_position_sizing_failed() {
        let error = TradeExecutionError::PositionSizingFailed {
            reason: "Price too high".to_string(),
        };
        assert_eq!(error.error_code(), "ERR_POSITION_SIZING");
    }

    #[test]
    fn test_error_equality() {
        let error1 = TradeExecutionError::InsufficientBalance {
            required: 1000.0,
            available: 500.0,
        };
        let error2 = TradeExecutionError::InsufficientBalance {
            required: 1000.0,
            available: 500.0,
        };
        assert_eq!(error1, error2);
    }

    #[test]
    fn test_error_inequality() {
        let error1 = TradeExecutionError::InsufficientBalance {
            required: 1000.0,
            available: 500.0,
        };
        let error2 = TradeExecutionError::InsufficientLeverage {
            required: 5.0,
            available: 2.0,
        };
        assert_ne!(error1, error2);
    }

    #[test]
    fn test_error_severity_display() {
        assert_eq!(ErrorSeverity::Minor.to_string(), "Minor");
        assert_eq!(ErrorSeverity::Moderate.to_string(), "Moderate");
        assert_eq!(ErrorSeverity::Critical.to_string(), "Critical");
    }

    #[test]
    fn test_error_severity_ordering() {
        assert!(ErrorSeverity::Minor < ErrorSeverity::Moderate);
        assert!(ErrorSeverity::Moderate < ErrorSeverity::Critical);
        assert!(ErrorSeverity::Minor < ErrorSeverity::Critical);
    }

    #[test]
    fn test_error_conversion_to_string() {
        let error = TradeExecutionError::InsufficientBalance {
            required: 1000.0,
            available: 500.0,
        };
        let error_string: String = error.into();
        assert_eq!(
            error_string,
            "Insufficient balance: required 1000.00, available 500.00"
        );
    }

    #[test]
    fn test_recoverable_errors() {
        let recoverable_errors = vec![
            TradeExecutionError::BalanceFetchFailed {
                reason: "timeout".to_string(),
            },
            TradeExecutionError::LeverageCalculationFailed {
                reason: "timeout".to_string(),
            },
            TradeExecutionError::OrderPlacementFailed {
                reason: "network".to_string(),
            },
        ];

        for error in recoverable_errors {
            assert!(error.is_recoverable(), "Expected error to be recoverable: {:?}", error);
        }
    }

    #[test]
    fn test_non_recoverable_errors() {
        let non_recoverable_errors = vec![
            TradeExecutionError::ManagerNotConfigured {
                manager_name: "BalanceManager".to_string(),
            },
            TradeExecutionError::InvalidSignal {
                reason: "confidence".to_string(),
            },
            TradeExecutionError::InsufficientBalance {
                required: 1000.0,
                available: 500.0,
            },
            TradeExecutionError::InsufficientLeverage {
                required: 5.0,
                available: 2.0,
            },
            TradeExecutionError::PositionSizingFailed {
                reason: "price".to_string(),
            },
            TradeExecutionError::InvalidPositionSize {
                reason: "minimum".to_string(),
            },
        ];

        for error in non_recoverable_errors {
            assert!(
                !error.is_recoverable(),
                "Expected error to be non-recoverable: {:?}",
                error
            );
        }
    }

    #[test]
    fn test_all_error_codes_unique() {
        let errors = vec![
            TradeExecutionError::InsufficientBalance {
                required: 1.0,
                available: 0.5,
            },
            TradeExecutionError::InsufficientLeverage {
                required: 5.0,
                available: 2.0,
            },
            TradeExecutionError::BalanceFetchFailed {
                reason: "test".to_string(),
            },
            TradeExecutionError::LeverageCalculationFailed {
                reason: "test".to_string(),
            },
            TradeExecutionError::PositionSizingFailed {
                reason: "test".to_string(),
            },
            TradeExecutionError::OrderPlacementFailed {
                reason: "test".to_string(),
            },
            TradeExecutionError::ManagerNotConfigured {
                manager_name: "test".to_string(),
            },
            TradeExecutionError::InvalidSignal {
                reason: "test".to_string(),
            },
            TradeExecutionError::InvalidPositionSize {
                reason: "test".to_string(),
            },
        ];

        let mut codes = vec![];
        for error in errors {
            let code = error.error_code();
            assert!(
                !codes.contains(&code),
                "Duplicate error code: {}",
                code
            );
            codes.push(code);
        }
    }
}
