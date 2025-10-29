//! LeverageCalculator - Calculates available leverage based on account state

use crate::domain::entities::leverage::LeverageInfo;
use crate::domain::repositories::exchange_client::ExchangeClient;
use std::sync::Arc;
use std::time::Duration;

/// Calculates and caches leverage information
pub struct LeverageCalculator {
    exchange_client: Arc<dyn ExchangeClient>,
}

impl LeverageCalculator {
    /// Create a new LeverageCalculator
    pub fn new(exchange_client: Arc<dyn ExchangeClient>) -> Self {
        Self { exchange_client }
    }

    /// Calculate available leverage given max and current leverage
    ///
    /// # Arguments
    /// * `max_leverage` - Maximum allowed leverage
    /// * `current_leverage` - Current leverage being used
    ///
    /// # Returns
    /// Available leverage = max - current
    pub fn calculate_available_leverage(
        &self,
        max_leverage: f64,
        current_leverage: f64,
    ) -> f64 {
        max_leverage - current_leverage
    }

    /// Check if an account has sufficient leverage for a trade
    ///
    /// # Arguments
    /// * `required_leverage` - Leverage needed for the trade
    /// * `available_leverage` - Current available leverage
    ///
    /// # Returns
    /// true if available_leverage >= required_leverage
    pub fn has_sufficient_leverage(
        &self,
        required_leverage: f64,
        available_leverage: f64,
    ) -> bool {
        available_leverage >= required_leverage
    }

    /// Calculate margin ratio from equity and notional value
    ///
    /// # Arguments
    /// * `equity` - Total account equity
    /// * `notional_value` - Total notional value of positions
    ///
    /// # Returns
    /// Margin ratio = equity / (notional_value / max_leverage)
    pub fn calculate_margin_ratio(
        &self,
        equity: f64,
        notional_value: f64,
        max_leverage: f64,
    ) -> f64 {
        if notional_value == 0.0 || max_leverage == 0.0 {
            f64::INFINITY
        } else {
            equity / (notional_value / max_leverage)
        }
    }

    /// Calculate current leverage from positions
    ///
    /// # Arguments
    /// * `notional_value` - Total notional value of all positions
    /// * `equity` - Account equity
    ///
    /// # Returns
    /// Current leverage = notional_value / equity
    pub fn calculate_current_leverage(&self, notional_value: f64, equity: f64) -> f64 {
        if equity == 0.0 {
            0.0
        } else {
            notional_value / equity
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::domain::entities::order::Order;
    use crate::domain::repositories::exchange_client::{ExchangeError, OrderStatus};

    /// Mock exchange client for testing
    struct MockExchangeClient;

    #[async_trait]
    impl ExchangeClient for MockExchangeClient {
        fn name(&self) -> &str {
            "MockExchange"
        }

        async fn get_balance(
            &self,
            _currency: Option<&str>,
        ) -> Result<Vec<crate::domain::repositories::exchange_client::Balance>, ExchangeError> {
            Ok(vec![])
        }

        async fn place_order(&self, _order: &Order) -> Result<String, ExchangeError> {
            Ok("mock-order-id".to_string())
        }

        async fn cancel_order(&self, _order_id: &str) -> Result<(), ExchangeError> {
            Ok(())
        }

        async fn get_order_status(&self, _order_id: &str) -> Result<OrderStatus, ExchangeError> {
            Ok(OrderStatus::Pending)
        }
    }

    #[test]
    fn test_leverage_calculator_calculate_available_leverage() {
        let client = Arc::new(MockExchangeClient);
        let calc = LeverageCalculator::new(client);

        let available = calc.calculate_available_leverage(20.0, 5.0);
        assert_eq!(available, 15.0);
    }

    #[test]
    fn test_leverage_calculator_calculate_available_leverage_no_current() {
        let client = Arc::new(MockExchangeClient);
        let calc = LeverageCalculator::new(client);

        let available = calc.calculate_available_leverage(20.0, 0.0);
        assert_eq!(available, 20.0);
    }

    #[test]
    fn test_leverage_calculator_calculate_available_leverage_full() {
        let client = Arc::new(MockExchangeClient);
        let calc = LeverageCalculator::new(client);

        let available = calc.calculate_available_leverage(20.0, 20.0);
        assert_eq!(available, 0.0);
    }

    #[test]
    fn test_leverage_calculator_has_sufficient_leverage() {
        let client = Arc::new(MockExchangeClient);
        let calc = LeverageCalculator::new(client);

        assert!(calc.has_sufficient_leverage(5.0, 10.0));
        assert!(calc.has_sufficient_leverage(10.0, 10.0));
        assert!(!calc.has_sufficient_leverage(11.0, 10.0));
    }

    #[test]
    fn test_leverage_calculator_calculate_margin_ratio_no_positions() {
        let client = Arc::new(MockExchangeClient);
        let calc = LeverageCalculator::new(client);

        let ratio = calc.calculate_margin_ratio(1000.0, 0.0, 20.0);
        assert!(ratio.is_infinite());
    }

    #[test]
    fn test_leverage_calculator_calculate_margin_ratio_with_positions() {
        let client = Arc::new(MockExchangeClient);
        let calc = LeverageCalculator::new(client);

        // equity: 1000, notional: 10000, max_leverage: 20
        // margin_ratio = 1000 / (10000 / 20) = 1000 / 500 = 2.0
        let ratio = calc.calculate_margin_ratio(1000.0, 10000.0, 20.0);
        assert!((ratio - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_leverage_calculator_calculate_current_leverage_no_positions() {
        let client = Arc::new(MockExchangeClient);
        let calc = LeverageCalculator::new(client);

        let current = calc.calculate_current_leverage(0.0, 1000.0);
        assert_eq!(current, 0.0);
    }

    #[test]
    fn test_leverage_calculator_calculate_current_leverage_with_positions() {
        let client = Arc::new(MockExchangeClient);
        let calc = LeverageCalculator::new(client);

        // notional: 10000, equity: 1000
        // current_leverage = 10000 / 1000 = 10.0
        let current = calc.calculate_current_leverage(10000.0, 1000.0);
        assert!((current - 10.0).abs() < 1e-9);
    }

    #[test]
    fn test_leverage_calculator_calculate_current_leverage_zero_equity() {
        let client = Arc::new(MockExchangeClient);
        let calc = LeverageCalculator::new(client);

        let current = calc.calculate_current_leverage(10000.0, 0.0);
        assert_eq!(current, 0.0);
    }
}
