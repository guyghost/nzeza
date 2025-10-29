//! Position sizing service that calculates optimal position sizes for trading
//!
//! This service implements position sizing algorithms that respect:
//! - Available balance and leverage
//! - Portfolio exposure limits
//! - Order size constraints (min/max)
//! - Price precision

use crate::domain::value_objects::position_sizing::{PositionSizingRequest, PositionSizingResult};

/// PositionSizer service for calculating optimal position sizes
#[derive(Debug, Clone)]
pub struct PositionSizer;

impl PositionSizer {
    /// Create a new PositionSizer instance
    pub fn new() -> Self {
        Self
    }

    /// Size a position based on request parameters
    ///
    /// This algorithm calculates the optimal quantity respecting:
    /// 1. Balance × Leverage / Price (maximum based on balance)
    /// 2. Portfolio exposure limit (max % of balance)
    /// 3. Order size constraints (min/max order sizes in USD)
    ///
    /// # Arguments
    /// * `req` - The position sizing request with parameters
    ///
    /// # Returns
    /// Ok(PositionSizingResult) with the calculated quantity, or Err if sizing fails
    pub fn size_position(&self, req: &PositionSizingRequest) -> Result<PositionSizingResult, String> {
        // Validate the request
        req.validate()?;

        // Calculate maximum quantity based on balance and leverage
        let max_quantity_by_balance = (req.available_balance * req.leverage) / req.current_price;

        // Calculate maximum quantity based on portfolio exposure limit
        let max_exposure_amount = req.available_balance * req.max_portfolio_exposure;
        let max_quantity_by_exposure = max_exposure_amount / req.current_price;

        // The actual maximum is the minimum of these two constraints
        let max_quantity = max_quantity_by_balance.min(max_quantity_by_exposure);

        // Calculate the notional value and margin used at max quantity
        let notional_value_at_max = max_quantity * req.current_price;

        // Check if the maximum quantity respects the min/max order size constraints
        let final_quantity = if notional_value_at_max >= req.min_order_size {
            // The max is still >= min, so use it
            if notional_value_at_max <= req.max_order_size {
                // It's also <= max, so perfect
                max_quantity
            } else {
                // It exceeds max, so cap it at max
                req.max_order_size / req.current_price
            }
        } else {
            // The max is < min, so we can't size the position
            0.0
        };

        // Calculate the final notional value and margin used
        let final_notional_value = final_quantity * req.current_price;
        let final_margin_used = final_notional_value / req.leverage;

        // Determine the reason for the sizing
        let reason = if final_quantity == 0.0 {
            format!(
                "Position too small: notional value {} < minimum {}",
                final_notional_value, req.min_order_size
            )
        } else if final_quantity == max_quantity && notional_value_at_max <= req.max_order_size {
            "Sized by balance and leverage constraints".to_string()
        } else if final_quantity < max_quantity && final_notional_value >= req.max_order_size {
            "Sized by maximum order size constraint".to_string()
        } else {
            "Sized by portfolio exposure constraint".to_string()
        };

        PositionSizingResult::new(final_quantity, final_notional_value, final_margin_used, reason)
    }

    /// Calculate if the available balance is sufficient for a desired notional value
    ///
    /// # Arguments
    /// * `available_balance` - Available balance for trading
    /// * `leverage` - Leverage to use
    /// * `desired_notional_value` - The desired notional value for the trade
    ///
    /// # Returns
    /// true if balance is sufficient, false otherwise
    pub fn is_sufficient_balance(
        &self,
        available_balance: f64,
        leverage: f64,
        desired_notional_value: f64,
    ) -> bool {
        if available_balance < 0.0 || leverage < 1.0 || desired_notional_value < 0.0 {
            return false;
        }

        let required_margin = desired_notional_value / leverage;
        available_balance >= required_margin
    }

    /// Calculate the margin required for a given notional value and leverage
    ///
    /// # Arguments
    /// * `notional_value` - The notional value of the position
    /// * `leverage` - Leverage to use
    ///
    /// # Returns
    /// The required margin amount
    pub fn calculate_required_margin(&self, notional_value: f64, leverage: f64) -> f64 {
        if notional_value < 0.0 || leverage < 1.0 {
            0.0
        } else {
            notional_value / leverage
        }
    }

    /// Calculate the maximum notional value based on balance and leverage
    ///
    /// # Arguments
    /// * `available_balance` - Available balance for trading
    /// * `leverage` - Leverage to use
    ///
    /// # Returns
    /// The maximum notional value achievable
    pub fn calculate_max_notional_value(&self, available_balance: f64, leverage: f64) -> f64 {
        if available_balance < 0.0 || leverage < 1.0 {
            0.0
        } else {
            available_balance * leverage
        }
    }
}

impl Default for PositionSizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_sizer_creation() {
        let sizer = PositionSizer::new();
        assert_eq!(format!("{:?}", sizer), "PositionSizer");
    }

    #[test]
    fn test_size_position_basic() {
        let sizer = PositionSizer::new();
        let req = PositionSizingRequest::new(
            "BTC-USD".to_string(),
            10000.0,  // balance
            5.0,      // leverage
            45000.0,  // price
            0.1,      // 10% max exposure
            100.0,    // min order
            50000.0,  // max order
        )
        .unwrap();

        let result = sizer.size_position(&req).unwrap();

        // max by balance = 10000 * 5 / 45000 = 1.111
        // max by exposure = 10000 * 0.1 / 45000 = 0.0222
        // So the real max is 0.0222 * 45000 = 1000 USD
        // But that's wrong. Let me recalculate:
        // max by balance = (10000 * 5) / 45000 = 50000 / 45000 = 1.111 BTC
        // max by exposure = (10000 * 0.1) / 45000 = 1000 / 45000 = 0.0222 BTC
        // So max_quantity = min(1.111, 0.0222) = 0.0222
        // notional_value = 0.0222 * 45000 = 1000 USD
        // This is >= 100 (min) and <= 50000 (max), so final quantity = 0.0222

        assert!(result.quantity > 0.0);
        assert!(result.notional_value > 100.0);
        assert!(result.notional_value <= 1000.0);
    }

    #[test]
    fn test_size_position_insufficient_balance() {
        let sizer = PositionSizer::new();
        let req = PositionSizingRequest::new(
            "BTC-USD".to_string(),
            50.0,     // very small balance
            2.0,      // leverage
            45000.0,  // price
            0.1,      // max exposure
            100.0,    // min order
            50000.0,  // max order
        )
        .unwrap();

        let result = sizer.size_position(&req).unwrap();

        // max by balance = (50 * 2) / 45000 = 0.00222 BTC
        // notional = 0.00222 * 45000 = 100 USD
        // This is < 100, so result should be 0
        assert_eq!(result.quantity, 0.0);
        assert_eq!(result.notional_value, 0.0);
    }

    #[test]
    fn test_size_position_max_order_constraint() {
        let sizer = PositionSizer::new();
        let req = PositionSizingRequest::new(
            "BTC-USD".to_string(),
            100000.0, // large balance
            10.0,     // high leverage
            45000.0,  // price
            0.5,      // 50% max exposure
            100.0,    // min order
            30000.0,  // max order (restrictive)
        )
        .unwrap();

        let result = sizer.size_position(&req).unwrap();

        // max by balance = (100000 * 10) / 45000 = 22.22 BTC
        // max by exposure = (100000 * 0.5) / 45000 = 1.111 BTC
        // max_quantity = min(22.22, 1.111) = 1.111
        // notional = 1.111 * 45000 = 50000 USD (exceeds max of 30000)
        // So final = 30000 / 45000 = 0.667 BTC
        // notional = 0.667 * 45000 = 30000 USD

        assert!(result.quantity > 0.0);
        assert!(result.notional_value <= 30000.0);
        assert!(result.reason.contains("maximum order size"));
    }

    #[test]
    fn test_is_sufficient_balance_positive() {
        let sizer = PositionSizer::new();
        assert!(sizer.is_sufficient_balance(10000.0, 5.0, 40000.0)); // margin needed = 8000
    }

    #[test]
    fn test_is_sufficient_balance_exact() {
        let sizer = PositionSizer::new();
        assert!(sizer.is_sufficient_balance(8000.0, 5.0, 40000.0)); // margin needed = 8000
    }

    #[test]
    fn test_is_sufficient_balance_insufficient() {
        let sizer = PositionSizer::new();
        assert!(!sizer.is_sufficient_balance(7000.0, 5.0, 40000.0)); // margin needed = 8000
    }

    #[test]
    fn test_is_sufficient_balance_negative_balance() {
        let sizer = PositionSizer::new();
        assert!(!sizer.is_sufficient_balance(-1000.0, 5.0, 40000.0));
    }

    #[test]
    fn test_is_sufficient_balance_low_leverage() {
        let sizer = PositionSizer::new();
        assert!(!sizer.is_sufficient_balance(10000.0, 0.5, 40000.0));
    }

    #[test]
    fn test_calculate_required_margin() {
        let sizer = PositionSizer::new();
        assert_eq!(sizer.calculate_required_margin(40000.0, 5.0), 8000.0);
    }

    #[test]
    fn test_calculate_required_margin_invalid_inputs() {
        let sizer = PositionSizer::new();
        assert_eq!(sizer.calculate_required_margin(-40000.0, 5.0), 0.0);
        assert_eq!(sizer.calculate_required_margin(40000.0, 0.5), 0.0);
    }

    #[test]
    fn test_calculate_max_notional_value() {
        let sizer = PositionSizer::new();
        assert_eq!(sizer.calculate_max_notional_value(10000.0, 5.0), 50000.0);
    }

    #[test]
    fn test_calculate_max_notional_value_invalid_inputs() {
        let sizer = PositionSizer::new();
        assert_eq!(sizer.calculate_max_notional_value(-10000.0, 5.0), 0.0);
        assert_eq!(sizer.calculate_max_notional_value(10000.0, 0.5), 0.0);
    }

    #[test]
    fn test_size_position_exposure_limit() {
        let sizer = PositionSizer::new();
        let req = PositionSizingRequest::new(
            "BTC-USD".to_string(),
            10000.0,  // balance
            10.0,     // high leverage
            45000.0,  // price
            0.05,     // only 5% max exposure
            10.0,     // low min order
            100000.0, // high max order
        )
        .unwrap();

        let result = sizer.size_position(&req).unwrap();

        // max by balance = (10000 * 10) / 45000 = 2.222 BTC
        // max by exposure = (10000 * 0.05) / 45000 = 0.0111 BTC
        // max_quantity = 0.0111
        // notional = 0.0111 * 45000 = 500 USD
        // This is >= 10 and <= 100000, so final = 0.0111

        assert!(result.notional_value <= 10000.0 * 0.05);
    }

    #[test]
    fn test_size_position_with_invalid_request() {
        let sizer = PositionSizer::new();
        let req = PositionSizingRequest::new(
            "BTC-USD".to_string(),
            -1000.0, // invalid: negative balance
            5.0,
            45000.0,
            0.1,
            100.0,
            50000.0,
        );

        assert!(req.is_err());
    }

    #[test]
    fn test_size_position_default_constructor() {
        let sizer = PositionSizer::default();
        let req = PositionSizingRequest::new(
            "ETH-USD".to_string(),
            5000.0,
            3.0,
            2500.0,
            0.2,
            50.0,
            10000.0,
        )
        .unwrap();

        let result = sizer.size_position(&req).unwrap();
        assert!(result.quantity >= 0.0);
    }

    #[test]
    fn test_size_position_zero_balance() {
        let sizer = PositionSizer::new();
        let req = PositionSizingRequest::new(
            "BTC-USD".to_string(),
            0.0,      // zero balance
            5.0,
            45000.0,
            0.1,
            100.0,
            50000.0,
        )
        .unwrap();

        let result = sizer.size_position(&req).unwrap();
        assert_eq!(result.quantity, 0.0);
        assert_eq!(result.notional_value, 0.0);
    }

    #[test]
    fn test_size_position_reason_contains_detail() {
        let sizer = PositionSizer::new();
        let req = PositionSizingRequest::new(
            "BTC-USD".to_string(),
            10000.0,
            5.0,
            45000.0,
            0.1,
            100.0,
            50000.0,
        )
        .unwrap();

        let result = sizer.size_position(&req).unwrap();
        assert!(!result.reason.is_empty());
    }
}
