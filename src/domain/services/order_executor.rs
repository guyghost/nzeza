//! OrderExecutor service - manages order execution workflow from signals to trades

use std::time::{SystemTime, Duration};

/// Order execution configuration
#[derive(Debug, Clone)]
pub struct OrderExecutorConfig {
    pub confidence_threshold: f64,
    pub symbols: Vec<String>,
    pub traders: Vec<String>,
    pub max_per_hour: u32,
    pub max_per_day: u32,
    pub portfolio_percentage: f64,
    pub slippage_pct: f64,
}

/// Trading signal with confidence level
#[derive(Debug, Clone)]
pub struct TradingSignal {
    pub signal: Signal,
    pub confidence: f64,
}

/// Signal direction
#[derive(Debug, Clone, PartialEq)]
pub enum Signal {
    Buy,
    Sell,
    Hold,
}

/// Order execution side
#[derive(Debug, Clone, PartialEq)]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Order executor for executing trades from signals
pub struct OrderExecutor {
    config: OrderExecutorConfig,
    trade_history: Vec<(SystemTime, String)>,
}

impl OrderExecutor {
    /// Create a new OrderExecutor
    pub fn new(config: OrderExecutorConfig) -> Self {
        Self {
            config,
            trade_history: Vec::new(),
        }
    }

    /// Execute order based on trading signal
    pub fn execute_order_from_signal(
        &mut self,
        symbol: &str,
        signal: &TradingSignal,
    ) -> Result<String, String> {
        // Validate symbol is in whitelist
        self.validate_symbol(symbol)?;

        // Check if we have traders available
        if !self.can_execute_order() {
            return Err("No trader available to execute orders".to_string());
        }

        // Handle HOLD signals - not an error, just no order
        if signal.signal == Signal::Hold {
            return Ok("No order executed - signal is HOLD".to_string());
        }

        // Check confidence threshold
        if signal.confidence < self.config.confidence_threshold {
            return Ok(format!(
                "Signal confidence {:.2} too low for execution (minimum {:.2})",
                signal.confidence, self.config.confidence_threshold
            ));
        }

        // Check trading limits
        self.check_trading_limits()?;

        // Determine order side
        let order_side = match signal.signal {
            Signal::Buy => OrderSide::Buy,
            Signal::Sell => OrderSide::Sell,
            Signal::Hold => {
                return Ok("No order executed - signal is HOLD".to_string());
            }
        };

        // Generate order ID
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let order_id = format!("order_{}_{}", timestamp, symbol);

        // Record trade in history
        self.trade_history.push((SystemTime::now(), symbol.to_string()));

        // Clean up old trades (older than 24 hours)
        let one_day_ago = SystemTime::now()
            .checked_sub(Duration::from_secs(86400))
            .unwrap_or(SystemTime::now());
        self.trade_history
            .retain(|(timestamp, _)| *timestamp >= one_day_ago);

        Ok(format!(
            "Order executed: {:?} {} (Order ID: {}, confidence: {:.2})",
            order_side, symbol, order_id, signal.confidence
        ))
    }

    /// Validate symbol is in whitelist
    pub fn validate_symbol(&self, symbol: &str) -> Result<(), String> {
        if self.config.symbols.contains(&symbol.to_string()) {
            Ok(())
        } else {
            Err(format!(
                "Symbol '{}' is not in the configured whitelist for trading",
                symbol
            ))
        }
    }

    /// Check trading limits (hourly and daily)
    pub fn check_trading_limits(&self) -> Result<(), String> {
        let now = SystemTime::now();

        // Count trades in last hour
        let one_hour_ago = now
            .checked_sub(Duration::from_secs(3600))
            .unwrap_or(now);
        let trades_last_hour = self
            .trade_history
            .iter()
            .filter(|(timestamp, _)| *timestamp >= one_hour_ago)
            .count() as u32;

        if trades_last_hour >= self.config.max_per_hour {
            return Err(format!(
                "Hourly trade limit exceeded: {} trades in last hour (max: {})",
                trades_last_hour, self.config.max_per_hour
            ));
        }

        // Count trades in last 24 hours
        let one_day_ago = now
            .checked_sub(Duration::from_secs(86400))
            .unwrap_or(now);
        let trades_last_day = self
            .trade_history
            .iter()
            .filter(|(timestamp, _)| *timestamp >= one_day_ago)
            .count() as u32;

        if trades_last_day >= self.config.max_per_day {
            return Err(format!(
                "Daily trade limit exceeded: {} trades in last 24 hours (max: {})",
                trades_last_day, self.config.max_per_day
            ));
        }

        Ok(())
    }

    /// Check if orders can be executed (trader available)
    pub fn can_execute_order(&self) -> bool {
        !self.config.traders.is_empty()
    }

    /// Calculate position size based on portfolio percentage
    pub fn calculate_position_size(
        &self,
        portfolio_value: f64,
        current_price: f64,
        portfolio_percentage: f64,
    ) -> Result<f64, String> {
        if portfolio_value <= 0.0 {
            return Err("Portfolio value must be positive".to_string());
        }

        if current_price <= 0.0 {
            return Err("Current price must be positive".to_string());
        }

        let position_value = portfolio_value * portfolio_percentage;
        let quantity = position_value / current_price;

        if !quantity.is_finite() {
            return Err(format!(
                "Invalid quantity calculated: {} (portfolio: {}, price: {})",
                quantity, portfolio_value, current_price
            ));
        }

        if quantity < 0.0 {
            return Err(format!("Negative quantity calculated: {}", quantity));
        }

        Ok(quantity)
    }

    /// Apply slippage protection to market orders
    pub fn apply_slippage_protection(
        &self,
        base_price: f64,
        is_buy: bool,
        slippage_pct: f64,
    ) -> f64 {
        if is_buy {
            // For buys, allow price to go up
            base_price * (1.0 + slippage_pct)
        } else {
            // For sells, allow price to go down
            base_price * (1.0 - slippage_pct)
        }
    }

    /// Get trade history
    pub fn get_trade_history(&self) -> &[(SystemTime, String)] {
        &self.trade_history
    }

    /// Get current trade count in last hour
    pub fn get_trades_last_hour(&self) -> u32 {
        let now = SystemTime::now();
        let one_hour_ago = now
            .checked_sub(Duration::from_secs(3600))
            .unwrap_or(now);

        self.trade_history
            .iter()
            .filter(|(timestamp, _)| *timestamp >= one_hour_ago)
            .count() as u32
    }

    /// Get current trade count in last day
    pub fn get_trades_last_day(&self) -> u32 {
        let now = SystemTime::now();
        let one_day_ago = now
            .checked_sub(Duration::from_secs(86400))
            .unwrap_or(now);

        self.trade_history
            .iter()
            .filter(|(timestamp, _)| *timestamp >= one_day_ago)
            .count() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_executor_creation() {
        let config = OrderExecutorConfig {
            confidence_threshold: 0.5,
            symbols: vec!["BTC-USD".to_string()],
            traders: vec!["trader1".to_string()],
            max_per_hour: 10,
            max_per_day: 50,
            portfolio_percentage: 0.05,
            slippage_pct: 0.02,
        };

        let executor = OrderExecutor::new(config);
        assert_eq!(executor.get_trade_history().len(), 0);
    }

    #[test]
    fn test_validate_symbol_success() {
        let config = OrderExecutorConfig {
            confidence_threshold: 0.5,
            symbols: vec!["BTC-USD".to_string()],
            traders: vec!["trader1".to_string()],
            max_per_hour: 10,
            max_per_day: 50,
            portfolio_percentage: 0.05,
            slippage_pct: 0.02,
        };

        let executor = OrderExecutor::new(config);
        assert!(executor.validate_symbol("BTC-USD").is_ok());
    }

    #[test]
    fn test_validate_symbol_failure() {
        let config = OrderExecutorConfig {
            confidence_threshold: 0.5,
            symbols: vec!["BTC-USD".to_string()],
            traders: vec!["trader1".to_string()],
            max_per_hour: 10,
            max_per_day: 50,
            portfolio_percentage: 0.05,
            slippage_pct: 0.02,
        };

        let executor = OrderExecutor::new(config);
        assert!(executor.validate_symbol("ETH-USD").is_err());
    }

    #[test]
    fn test_calculate_position_size() {
        let config = OrderExecutorConfig {
            confidence_threshold: 0.5,
            symbols: vec!["BTC-USD".to_string()],
            traders: vec!["trader1".to_string()],
            max_per_hour: 10,
            max_per_day: 50,
            portfolio_percentage: 0.05,
            slippage_pct: 0.02,
        };

        let executor = OrderExecutor::new(config);
        let size = executor
            .calculate_position_size(10000.0, 50000.0, 0.05)
            .unwrap();

        // 10000 * 0.05 / 50000 = 0.01
        assert_eq!(size, 0.01);
    }

    #[test]
    fn test_slippage_protection_buy() {
        let config = OrderExecutorConfig {
            confidence_threshold: 0.5,
            symbols: vec!["BTC-USD".to_string()],
            traders: vec!["trader1".to_string()],
            max_per_hour: 10,
            max_per_day: 50,
            portfolio_percentage: 0.05,
            slippage_pct: 0.02,
        };

        let executor = OrderExecutor::new(config);
        let price = executor.apply_slippage_protection(50000.0, true, 0.02);

        // 50000 * 1.02 = 51000
        assert_eq!(price, 51000.0);
    }

    #[test]
    fn test_slippage_protection_sell() {
        let config = OrderExecutorConfig {
            confidence_threshold: 0.5,
            symbols: vec!["BTC-USD".to_string()],
            traders: vec!["trader1".to_string()],
            max_per_hour: 10,
            max_per_day: 50,
            portfolio_percentage: 0.05,
            slippage_pct: 0.02,
        };

        let executor = OrderExecutor::new(config);
        let price = executor.apply_slippage_protection(50000.0, false, 0.02);

        // 50000 * 0.98 = 49000
        assert_eq!(price, 49000.0);
    }
}
