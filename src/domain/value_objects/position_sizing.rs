//! Position sizing request and result value objects

/// Request for position sizing calculation
#[derive(Debug, Clone)]
pub struct PositionSizingRequest {
    /// Trading symbol (e.g., "BTC-USD")
    pub symbol: String,
    /// Available balance for trading
    pub available_balance: f64,
    /// Leverage to use for this trade
    pub leverage: f64,
    /// Current market price of the asset
    pub current_price: f64,
    /// Maximum portfolio exposure (e.g., 0.1 for 10% of balance)
    pub max_portfolio_exposure: f64,
    /// Minimum order size in USD
    pub min_order_size: f64,
    /// Maximum order size in USD
    pub max_order_size: f64,
}

impl PositionSizingRequest {
    /// Create a new position sizing request with validation
    ///
    /// # Arguments
    /// * `symbol` - Trading symbol
    /// * `available_balance` - Available balance (must be >= 0)
    /// * `leverage` - Leverage to use (must be >= 1)
    /// * `current_price` - Current asset price (must be > 0)
    /// * `max_portfolio_exposure` - Max exposure (0 < x <= 1)
    /// * `min_order_size` - Minimum order size in USD (>= 0)
    /// * `max_order_size` - Maximum order size in USD (>= min_order_size)
    ///
    /// # Returns
    /// Ok(PositionSizingRequest) if all parameters are valid, Err(String) otherwise
    pub fn new(
        symbol: String,
        available_balance: f64,
        leverage: f64,
        current_price: f64,
        max_portfolio_exposure: f64,
        min_order_size: f64,
        max_order_size: f64,
    ) -> Result<Self, String> {
        if symbol.is_empty() {
            return Err("symbol cannot be empty".to_string());
        }

        if available_balance < 0.0 {
            return Err("available_balance must be non-negative".to_string());
        }

        if leverage < 1.0 {
            return Err("leverage must be >= 1.0".to_string());
        }

        if current_price <= 0.0 {
            return Err("current_price must be positive".to_string());
        }

        if max_portfolio_exposure <= 0.0 || max_portfolio_exposure > 1.0 {
            return Err("max_portfolio_exposure must be in range (0, 1]".to_string());
        }

        if min_order_size < 0.0 {
            return Err("min_order_size must be non-negative".to_string());
        }

        if max_order_size < min_order_size {
            return Err("max_order_size must be >= min_order_size".to_string());
        }

        Ok(Self {
            symbol,
            available_balance,
            leverage,
            current_price,
            max_portfolio_exposure,
            min_order_size,
            max_order_size,
        })
    }

    /// Validate this request
    pub fn validate(&self) -> Result<(), String> {
        if self.symbol.is_empty() {
            return Err("symbol cannot be empty".to_string());
        }
        if self.available_balance < 0.0 {
            return Err("available_balance must be non-negative".to_string());
        }
        if self.leverage < 1.0 {
            return Err("leverage must be >= 1.0".to_string());
        }
        if self.current_price <= 0.0 {
            return Err("current_price must be positive".to_string());
        }
        if self.max_portfolio_exposure <= 0.0 || self.max_portfolio_exposure > 1.0 {
            return Err("max_portfolio_exposure must be in range (0, 1]".to_string());
        }
        if self.min_order_size < 0.0 {
            return Err("min_order_size must be non-negative".to_string());
        }
        if self.max_order_size < self.min_order_size {
            return Err("max_order_size must be >= min_order_size".to_string());
        }
        Ok(())
    }
}

/// Result of position sizing calculation
#[derive(Debug, Clone, PartialEq)]
pub struct PositionSizingResult {
    /// Calculated quantity to trade
    pub quantity: f64,
    /// Notional value = quantity * price
    pub notional_value: f64,
    /// Margin used = notional_value / leverage
    pub margin_used: f64,
    /// Reason for this sizing decision
    pub reason: String,
}

impl PositionSizingResult {
    /// Create a new position sizing result
    pub fn new(
        quantity: f64,
        notional_value: f64,
        margin_used: f64,
        reason: String,
    ) -> Result<Self, String> {
        if quantity < 0.0 {
            return Err("quantity must be non-negative".to_string());
        }

        if notional_value < 0.0 {
            return Err("notional_value must be non-negative".to_string());
        }

        if margin_used < 0.0 {
            return Err("margin_used must be non-negative".to_string());
        }

        if reason.is_empty() {
            return Err("reason cannot be empty".to_string());
        }

        // Verify notional_value = quantity * price (loosely)
        if quantity > 0.0 && notional_value == 0.0 {
            return Err("notional_value should be > 0 when quantity > 0".to_string());
        }

        Ok(Self {
            quantity,
            notional_value,
            margin_used,
            reason,
        })
    }

    /// Validate this result
    pub fn validate(&self) -> Result<(), String> {
        if self.quantity < 0.0 {
            return Err("quantity must be non-negative".to_string());
        }
        if self.notional_value < 0.0 {
            return Err("notional_value must be non-negative".to_string());
        }
        if self.margin_used < 0.0 {
            return Err("margin_used must be non-negative".to_string());
        }
        if self.reason.is_empty() {
            return Err("reason cannot be empty".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_sizing_request_valid_creation() {
        let result = PositionSizingRequest::new(
            "BTC-USD".to_string(),
            1000.0,
            5.0,
            45000.0,
            0.1,
            100.0,
            5000.0,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_position_sizing_request_empty_symbol() {
        let result = PositionSizingRequest::new(
            "".to_string(),
            1000.0,
            5.0,
            45000.0,
            0.1,
            100.0,
            5000.0,
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "symbol cannot be empty");
    }

    #[test]
    fn test_position_sizing_request_negative_balance() {
        let result = PositionSizingRequest::new(
            "BTC-USD".to_string(),
            -100.0,
            5.0,
            45000.0,
            0.1,
            100.0,
            5000.0,
        );
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "available_balance must be non-negative"
        );
    }

    #[test]
    fn test_position_sizing_request_low_leverage() {
        let result = PositionSizingRequest::new(
            "BTC-USD".to_string(),
            1000.0,
            0.5,
            45000.0,
            0.1,
            100.0,
            5000.0,
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "leverage must be >= 1.0");
    }

    #[test]
    fn test_position_sizing_request_zero_price() {
        let result = PositionSizingRequest::new(
            "BTC-USD".to_string(),
            1000.0,
            5.0,
            0.0,
            0.1,
            100.0,
            5000.0,
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "current_price must be positive");
    }

    #[test]
    fn test_position_sizing_request_invalid_exposure() {
        let result = PositionSizingRequest::new(
            "BTC-USD".to_string(),
            1000.0,
            5.0,
            45000.0,
            1.5,
            100.0,
            5000.0,
        );
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "max_portfolio_exposure must be in range (0, 1]"
        );
    }

    #[test]
    fn test_position_sizing_request_max_less_than_min() {
        let result = PositionSizingRequest::new(
            "BTC-USD".to_string(),
            1000.0,
            5.0,
            45000.0,
            0.1,
            5000.0,
            100.0,
        );
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "max_order_size must be >= min_order_size"
        );
    }

    #[test]
    fn test_position_sizing_result_valid_creation() {
        let result =
            PositionSizingResult::new(1.0, 45000.0, 9000.0, "Based on balance".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_position_sizing_result_negative_quantity() {
        let result = PositionSizingResult::new(-1.0, 45000.0, 9000.0, "Test".to_string());
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "quantity must be non-negative");
    }

    #[test]
    fn test_position_sizing_result_empty_reason() {
        let result = PositionSizingResult::new(1.0, 45000.0, 9000.0, "".to_string());
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "reason cannot be empty");
    }

    #[test]
    fn test_position_sizing_result_zero_quantity() {
        let result = PositionSizingResult::new(0.0, 0.0, 0.0, "Insufficient balance".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_position_sizing_result_validate() {
        let result =
            PositionSizingResult::new(1.0, 45000.0, 9000.0, "Test".to_string()).unwrap();
        assert!(result.validate().is_ok());
    }
}
