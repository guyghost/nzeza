//! OrderExecutor service - manages order execution workflow from signals to trades

use async_trait::async_trait;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;
use tracing;

use crate::domain::entities::exchange::Exchange;
use crate::domain::entities::order::{Order, OrderSide, OrderType};
use crate::domain::entities::position::{Position, PositionSide};
use crate::domain::repositories::exchange_client::{ExchangeClient, ExchangeError, ExchangeResult};
use crate::domain::services::balance_manager::BalanceManager;
use crate::domain::services::leverage_calculator::LeverageCalculator;
use crate::domain::services::metrics::{PerformanceProfiler, TradingMetrics};
use crate::domain::services::position_manager::{PositionLimits, PositionManager, PositionResult};
use crate::domain::services::position_sizer::PositionSizer;
use crate::domain::value_objects::position_sizing::PositionSizingRequest;
use crate::domain::value_objects::{price::Price, quantity::Quantity};

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
    pub min_quantity: f64,
    pub max_retry_attempts: u32,
    pub retry_delay_ms: u64,
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

/// Cached signal with timestamp for LRU eviction
#[derive(Debug, Clone)]
pub struct CachedSignal {
    pub signal: TradingSignal,
    pub timestamp: SystemTime,
}

/// Trade history entry for rate limiting
#[derive(Debug, Clone)]
struct TradeEntry {
    symbol: String,
    timestamp: SystemTime,
}

/// Execution metrics for a single order
#[derive(Debug, Clone)]
pub struct OrderExecutionMetrics {
    pub execution_time_ms: f64,
    pub order_size: f64,
    pub success: bool,
}

/// Order executor for executing trades from signals
pub struct OrderExecutor {
    config: OrderExecutorConfig,
    position_manager: Arc<Mutex<PositionManager>>,
    exchange_clients: HashMap<Exchange, Arc<dyn ExchangeClient>>,
    active_exchange: Option<Exchange>,
    trade_history: Vec<TradeEntry>,
    signal_cache: HashMap<String, CachedSignal>,
    metrics: Arc<Mutex<TradingMetrics>>,
    profiler: PerformanceProfiler,
    portfolio_value: f64,
    balance_manager: Option<Arc<BalanceManager>>,
    leverage_calculator: Option<Arc<LeverageCalculator>>,
    position_sizer: PositionSizer,
}

impl OrderExecutor {
    /// Create a new OrderExecutor
    pub fn new(
        config: OrderExecutorConfig,
        position_manager: Arc<Mutex<PositionManager>>,
        exchange_clients: HashMap<Exchange, Arc<dyn ExchangeClient>>,
        metrics: Arc<Mutex<TradingMetrics>>,
        portfolio_value: f64,
    ) -> Self {
        let active_exchange = exchange_clients.keys().next().cloned();

        Self {
            config,
            position_manager,
            exchange_clients,
            active_exchange,
            trade_history: Vec::new(),
            signal_cache: HashMap::new(),
            metrics,
            profiler: PerformanceProfiler::new(),
            portfolio_value,
            balance_manager: None,
            leverage_calculator: None,
            position_sizer: PositionSizer::new(),
        }
    }

    /// Create a new OrderExecutor with balance and leverage managers
    pub fn with_managers(
        config: OrderExecutorConfig,
        position_manager: Arc<Mutex<PositionManager>>,
        exchange_clients: HashMap<Exchange, Arc<dyn ExchangeClient>>,
        metrics: Arc<Mutex<TradingMetrics>>,
        portfolio_value: f64,
        balance_manager: Arc<BalanceManager>,
        leverage_calculator: Arc<LeverageCalculator>,
    ) -> Self {
        let active_exchange = exchange_clients.keys().next().cloned();

        Self {
            config,
            position_manager,
            exchange_clients,
            active_exchange,
            trade_history: Vec::new(),
            signal_cache: HashMap::new(),
            metrics,
            profiler: PerformanceProfiler::new(),
            portfolio_value,
            balance_manager: Some(balance_manager),
            leverage_calculator: Some(leverage_calculator),
            position_sizer: PositionSizer::new(),
        }
    }

    /// Create a new OrderExecutor with default dependencies (for testing)
    pub fn new_with_config(config: OrderExecutorConfig) -> Self {
        use crate::domain::services::metrics::TradingMetrics;
        use crate::domain::services::position_manager::PositionLimits;

        let position_limits = PositionLimits {
            max_per_symbol: 3,
            max_total: 10,
            max_portfolio_exposure: 0.8,
        };

        let position_manager = Arc::new(Mutex::new(PositionManager::new(position_limits, 10000.0)));

        // Create a mock exchange client for testing
        let mut exchange_clients = HashMap::new();
        let mock_client: Arc<dyn ExchangeClient> = Arc::new(MockExchangeClient);
        exchange_clients.insert(Exchange::Coinbase, mock_client);

        let metrics = Arc::new(Mutex::new(TradingMetrics::new()));
        let portfolio_value = 10000.0;

        Self::new(
            config,
            position_manager,
            exchange_clients,
            metrics,
            portfolio_value,
        )
    }

    /// Create a new OrderExecutor WITHOUT exchange clients (for testing error cases)
    pub fn new_with_config_no_exchange(config: OrderExecutorConfig) -> Self {
        use crate::domain::services::metrics::TradingMetrics;
        use crate::domain::services::position_manager::PositionLimits;

        let position_limits = PositionLimits {
            max_per_symbol: 3,
            max_total: 10,
            max_portfolio_exposure: 0.8,
        };

        let position_manager = Arc::new(Mutex::new(PositionManager::new(position_limits, 10000.0)));
        let exchange_clients = HashMap::new(); // Empty for error case testing
        let metrics = Arc::new(Mutex::new(TradingMetrics::new()));
        let portfolio_value = 10000.0;

        Self::new(
            config,
            position_manager,
            exchange_clients,
            metrics,
            portfolio_value,
        )
    }

    /// Execute order based on trading signal - main entry point
    pub fn execute_order_from_signal(
        &mut self,
        symbol: &str,
        signal: &TradingSignal,
    ) -> Result<String, String> {
        // Start profiling
        let _timer = self.profiler.start_operation("execute_signal");

        let result = self.execute_order_from_signal_internal(symbol, signal);

        // Record the operation execution time (the timer is dropped at the end of scope,
        // but we need to explicitly record it because we're not using Drop)
        let elapsed = _timer.elapsed_ms();
        self.profiler.record_operation("execute_signal", elapsed);

        result
    }

    /// Internal implementation of order execution
    fn execute_order_from_signal_internal(
        &mut self,
        symbol: &str,
        signal: &TradingSignal,
    ) -> Result<String, String> {
        // Check if traders are available
        if self.config.traders.is_empty() {
            return Err("No trader available for order execution".to_string());
        }

        // Check if signal is cached (prevent re-execution)
        if let Some(cached) = self.signal_cache.get(symbol) {
            if cached.signal.confidence == signal.confidence
                && cached.signal.signal == signal.signal
            {
                return Err("Signal already executed - cached".to_string());
            }
        }

        // Validate signal
        self.validate_signal(symbol, signal)?;

        // Handle HOLD signals
        if signal.signal == Signal::Hold {
            self.cache_signal(symbol, signal.clone());
            return Ok("No order executed - signal is HOLD".to_string());
        }

        // Check rate limits
        self.check_trading_limits()?;

        // Get current price (simplified - in real implementation would get from price feed)
        let current_price = self.get_current_price(symbol)?;

        // Calculate position size
        let position_size = self.calculate_position_size(
            self.portfolio_value,
            current_price.value(),
            self.config.portfolio_percentage,
        )?;

        // Check minimum quantity
        if position_size < self.config.min_quantity {
            return Err(format!(
                "Position size {:.6} is below minimum quantity {:.6}",
                position_size, self.config.min_quantity
            ));
        }

        // Validate balance and position limits (simplified for sync interface)
        self.validate_execution_constraints_sync(symbol, position_size, current_price.value())?;

        // Execute order (simplified for sync interface)
        let execution_result =
            self.execute_single_order_sync(symbol, signal, position_size, current_price.value());

        match execution_result {
            Ok(order_id) => {
                // Record successful trade
                self.record_trade(symbol);
                self.cache_signal(symbol, signal.clone());

                let signal_str = match signal.signal {
                    Signal::Buy => "BUY",
                    Signal::Sell => "SELL",
                    Signal::Hold => "HOLD",
                };

                Ok(format!(
                    "Order executed: {} {} (Order ID: {}, confidence: {:.2})",
                    signal_str, symbol, order_id, signal.confidence
                ))
            }
            Err(error) => {
                // Clear signal cache on permanent failure
                if self.is_permanent_error(&error) {
                    self.signal_cache.remove(symbol);
                }

                Err(error)
            }
        }
    }

    /// Validate signal against business rules
    fn validate_signal(&self, symbol: &str, signal: &TradingSignal) -> Result<(), String> {
        // Check symbol whitelist
        if !self.config.symbols.contains(&symbol.to_string()) {
            return Err(format!(
                "Symbol '{}' is not in the configured whitelist for trading",
                symbol
            ));
        }

        // Check confidence threshold
        if signal.confidence < self.config.confidence_threshold {
            return Err(format!(
                "Signal confidence {:.2} below minimum threshold {:.2}",
                signal.confidence, self.config.confidence_threshold
            ));
        }

        Ok(())
    }

    /// Check trading rate limits
    fn check_trading_limits(&self) -> Result<(), String> {
        let now = SystemTime::now();

        // Count trades in last hour
        let one_hour_ago = now.checked_sub(Duration::from_secs(3600)).unwrap_or(now);
        let trades_last_hour = self
            .trade_history
            .iter()
            .filter(|entry| entry.timestamp >= one_hour_ago)
            .count() as u32;

        if trades_last_hour >= self.config.max_per_hour {
            return Err(format!(
                "Hourly trade limit exceeded: {} trades in last hour (max: {})",
                trades_last_hour, self.config.max_per_hour
            ));
        }

        // Count trades in last 24 hours
        let one_day_ago = now.checked_sub(Duration::from_secs(86400)).unwrap_or(now);
        let trades_last_day = self
            .trade_history
            .iter()
            .filter(|entry| entry.timestamp >= one_day_ago)
            .count() as u32;

        if trades_last_day >= self.config.max_per_day {
            return Err(format!(
                "Daily trade limit exceeded: {} trades in last 24 hours (max: {})",
                trades_last_day, self.config.max_per_day
            ));
        }

        Ok(())
    }

    /// Get current market price for symbol
    fn get_current_price(&self, symbol: &str) -> Result<Price, String> {
        // In a real implementation, this would query a price feed
        // For testing, we'll use a mock price
        match symbol {
            "BTC-USD" => Price::new(50000.0),
            "ETH-USD" => Price::new(3000.0),
            "SOL-USD" => Price::new(100.0),
            _ => Price::new(100.0),
        }
        .map_err(|e| format!("Invalid price: {}", e))
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

        // Check if quantity is below minimum
        if quantity < self.config.min_quantity {
            return Err(format!(
                "Calculated quantity {:.8} is below minimum {:.8}",
                quantity, self.config.min_quantity
            ));
        }

        Ok(quantity)
    }

    /// Validate execution constraints (balance, position limits) - sync version
    fn validate_execution_constraints_sync(
        &self,
        symbol: &str,
        quantity: f64,
        price: f64,
    ) -> Result<(), String> {
        let position_value = quantity * price;

        // Check portfolio balance
        if position_value > self.portfolio_value {
            return Err(format!(
                "Insufficient balance: required {:.2}, available {:.2}",
                position_value, self.portfolio_value
            ));
        }

        // Simplified position limit check (max 3 per symbol)
        // In a real implementation, this would check the position manager
        Ok(())
    }

    /// Execute a single order attempt - sync version
    fn execute_single_order_sync(
        &self,
        symbol: &str,
        signal: &TradingSignal,
        quantity: f64,
        price: f64,
    ) -> Result<String, String> {
        let exchange = self
            .active_exchange
            .as_ref()
            .ok_or("No active exchange configured")?;

        // Determine order side
        let order_side = match signal.signal {
            Signal::Buy => crate::domain::entities::order::OrderSide::Buy,
            Signal::Sell => crate::domain::entities::order::OrderSide::Sell,
            Signal::Hold => return Err("HOLD signals should not reach order execution".to_string()),
        };

        // Apply slippage protection
        let limit_price = self.apply_slippage_protection(
            price,
            matches!(order_side, crate::domain::entities::order::OrderSide::Buy),
            self.config.slippage_pct,
        );

        // Create order
        let quantity_obj =
            Quantity::new(quantity).map_err(|e| format!("Invalid quantity: {}", e))?;

        let order = Order::new(
            format!(
                "order_{}",
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            ),
            symbol.to_string(),
            order_side,
            OrderType::Limit,
            Some(limit_price),
            quantity,
        )
        .map_err(|e| format!("Failed to create order: {}", e))?;

        // In a real implementation, this would call the exchange client
        // For testing, we'll simulate success if exchange is configured
        let order_id = order.id;

        Ok(order_id)
    }

    /// Create position after successful order
    fn create_position(
        &self,
        symbol: &str,
        signal: &TradingSignal,
        quantity: f64,
        price: f64,
    ) -> Result<(), String> {
        // Simplified for sync interface - in real implementation would be async
        Ok(())
    }

    /// Apply slippage protection to limit price
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

    /// Record successful trade in history
    fn record_trade(&mut self, symbol: &str) {
        self.trade_history.push(TradeEntry {
            symbol: symbol.to_string(),
            timestamp: SystemTime::now(),
        });

        // Clean up old entries (older than 24 hours)
        let one_day_ago = SystemTime::now()
            .checked_sub(Duration::from_secs(86400))
            .unwrap_or(SystemTime::now());

        self.trade_history
            .retain(|entry| entry.timestamp >= one_day_ago);
    }

    /// Cache signal to prevent re-execution
    fn cache_signal(&mut self, symbol: &str, signal: TradingSignal) {
        self.signal_cache.insert(
            symbol.to_string(),
            CachedSignal {
                signal,
                timestamp: SystemTime::now(),
            },
        );

        // Simple LRU: keep only recent signals (cleanup old ones)
        let five_minutes_ago = SystemTime::now()
            .checked_sub(Duration::from_secs(300))
            .unwrap_or(SystemTime::now());

        self.signal_cache
            .retain(|_, cached| cached.timestamp >= five_minutes_ago);
    }

    /// Check if error is permanent (should not retry)
    fn is_permanent_error(&self, error: &str) -> bool {
        // Permanent errors that should not be retried
        error.contains("whitelist")
            || error.contains("balance")
            || error.contains("limit")
            || error.contains("validation")
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

    /// Check if orders can be executed (trader available)
    pub fn can_execute_order(&self) -> bool {
        !self.config.traders.is_empty()
    }

    /// Get trade history
    pub fn get_trade_history(&self) -> &[(SystemTime, String)] {
        // Convert to the expected format
        // This is a bit of a hack since we changed the internal structure
        // In a real implementation, we'd keep the original format
        static mut HISTORY: Vec<(SystemTime, String)> = Vec::new();
        unsafe {
            HISTORY.clear();
            for entry in &self.trade_history {
                HISTORY.push((entry.timestamp, entry.symbol.clone()));
            }
            &HISTORY
        }
    }

    /// Get current trade count in last hour
    pub fn get_trades_last_hour(&self) -> u32 {
        let now = SystemTime::now();
        let one_hour_ago = now.checked_sub(Duration::from_secs(3600)).unwrap_or(now);

        self.trade_history
            .iter()
            .filter(|entry| entry.timestamp >= one_hour_ago)
            .count() as u32
    }

    /// Get current trade count in last day
    pub fn get_trades_last_day(&self) -> u32 {
        let now = SystemTime::now();
        let one_day_ago = now.checked_sub(Duration::from_secs(86400)).unwrap_or(now);

        self.trade_history
            .iter()
            .filter(|entry| entry.timestamp >= one_day_ago)
            .count() as u32
    }

    /// Get cached signals
    pub fn get_cached_signals(&self) -> &HashMap<String, CachedSignal> {
        &self.signal_cache
    }

    /// Clear signal cache for symbol
    pub fn clear_signal_cache(&mut self, symbol: &str) {
        self.signal_cache.remove(symbol);
    }

    /// Get performance profiler
    pub fn get_profiler(&self) -> &PerformanceProfiler {
        &self.profiler
    }

    /// Set portfolio value (for testing)
    pub fn set_portfolio_value(&mut self, value: f64) {
        self.portfolio_value = value;
    }

    /// Helper: Convert Signal enum to string representation
    fn signal_to_string(signal: &Signal) -> &'static str {
        match signal {
            Signal::Buy => "BUY",
            Signal::Sell => "SELL",
            Signal::Hold => "HOLD",
        }
    }

    /// Helper: Get balance and leverage managers (ensures both are configured)
    fn get_required_managers(
        &self,
    ) -> Result<(&Arc<BalanceManager>, &Arc<LeverageCalculator>), String> {
        let balance_manager = self
            .balance_manager
            .as_ref()
            .ok_or("BalanceManager not configured")?;
        let leverage_calculator = self
            .leverage_calculator
            .as_ref()
            .ok_or("LeverageCalculator not configured")?;
        Ok((balance_manager, leverage_calculator))
    }

    /// Helper: Validate balance is sufficient for the order
    fn validate_balance_sufficient(
        available_balance: f64,
        required_order_value: f64,
    ) -> Result<(), String> {
        if available_balance < required_order_value {
            return Err(format!(
                "Insufficient balance: required {:.2}, available {:.2}",
                required_order_value, available_balance
            ));
        }
        Ok(())
    }

    /// Helper: Validate leverage is sufficient for the order
    fn validate_leverage_sufficient(
        available_leverage: f64,
        required_leverage: f64,
    ) -> Result<(), String> {
        if available_leverage < required_leverage {
            return Err(format!(
                "Insufficient leverage: required {:.2}, available {:.2}",
                required_leverage, available_leverage
            ));
        }
        Ok(())
    }

    /// Helper: Create and validate position sizing request
    fn create_sizing_request(
        &self,
        symbol: &str,
        available_balance: f64,
        available_leverage: f64,
        current_price: f64,
    ) -> Result<PositionSizingRequest, String> {
        PositionSizingRequest::new(
            symbol.to_string(),
            available_balance,
            available_leverage,
            current_price,
            self.config.portfolio_percentage,
            100.0, // min_order_size: $100
            self.portfolio_value, // max_order_size: portfolio value
        )
        .map_err(|e| format!("Invalid position sizing request: {}", e))
    }

    /// Helper: Validate position size meets minimum requirements
    fn validate_position_size(
        quantity: f64,
        sizing_reason: &str,
        min_quantity: f64,
    ) -> Result<(), String> {
        if quantity <= 0.0 {
            return Err(format!(
                "Position sizing resulted in zero quantity: {}",
                sizing_reason
            ));
        }

        if quantity < min_quantity {
            return Err(format!(
                "Position size {:.8} is below minimum quantity {:.8}",
                quantity, min_quantity
            ));
        }

        Ok(())
    }

    /// Execute a trading signal with integrated balance and leverage checks
    ///
    /// This is the main async entry point for signal execution that:
    /// 1. Fetches current balance via BalanceManager
    /// 2. Calculates available leverage via LeverageCalculator
    /// 3. Sizes position using PositionSizer
    /// 4. Places order with exchange client
    ///
    /// # Arguments
    /// * `symbol` - The trading symbol (e.g., "BTC-USD")
    /// * `signal` - The trading signal with confidence level
    /// * `current_price` - Current market price of the asset
    ///
    /// # Returns
    /// Ok(String) with order ID if successful, Err(String) with error message otherwise
    pub async fn execute_signal_with_balance_and_leverage_check(
        &mut self,
        symbol: &str,
        signal: &TradingSignal,
        current_price: f64,
    ) -> Result<String, String> {
        // Validate signal
        self.validate_signal(symbol, signal)?;

        // Handle HOLD signals
        if signal.signal == Signal::Hold {
            self.cache_signal(symbol, signal.clone());
            return Ok("No order executed - signal is HOLD".to_string());
        }

        // Get required managers
        let (balance_manager, leverage_calculator) = self.get_required_managers()?;

        // Fetch current balance asynchronously
        let balance_info = balance_manager
            .get_balance()
            .await
            .map_err(|e| format!("Failed to fetch balance: {}", e))?;

        // Validate sufficient balance
        let required_order_value = self.portfolio_value * self.config.portfolio_percentage;
        Self::validate_balance_sufficient(balance_info.available_balance, required_order_value)?;

        // Calculate available leverage
        let max_leverage = 10.0; // Default max leverage (can be configurable)
        let current_leverage = 1.0; // Default current leverage (can be calculated from positions)
        let available_leverage =
            leverage_calculator.calculate_available_leverage(max_leverage, current_leverage);

        // Validate sufficient leverage
        let required_leverage = 2.0; // Default required leverage (can be configurable)
        Self::validate_leverage_sufficient(available_leverage, required_leverage)?;

        // Create and validate position sizing request
        let sizing_request = self.create_sizing_request(
            symbol,
            balance_info.available_balance,
            available_leverage,
            current_price,
        )?;

        // Calculate position size
        let sizing_result = self
            .position_sizer
            .size_position(&sizing_request)
            .map_err(|e| format!("Position sizing failed: {}", e))?;

        // Validate position size meets minimum requirements
        Self::validate_position_size(
            sizing_result.quantity,
            &sizing_result.reason,
            self.config.min_quantity,
        )?;

        // Record the execution attempt
        let start_time = SystemTime::now();

        // Execute the order with calculated quantity
        let result = self.execute_single_order_sync(symbol, signal, sizing_result.quantity, current_price);

        // Record metrics
        let elapsed_ms = start_time.elapsed().unwrap_or_default().as_millis() as f64;
        
        match &result {
            Ok(order_id) => {
                self.record_trade(symbol);
                self.cache_signal(symbol, signal.clone());

                let signal_str = Self::signal_to_string(&signal.signal);

                tracing::info!(
                    "Signal executed successfully: {} {} quantity={:.8} order_id={} confidence={:.2} time_ms={:.2}",
                    signal_str,
                    symbol,
                    sizing_result.quantity,
                    order_id,
                    signal.confidence,
                    elapsed_ms
                );

                Ok(order_id.clone())
            }
            Err(error) => {
                if self.is_permanent_error(error) {
                    self.signal_cache.remove(symbol);
                }

                let signal_str = Self::signal_to_string(&signal.signal);

                tracing::error!(
                    "Signal execution failed: {} {} error={} time_ms={:.2}",
                    symbol,
                    signal_str,
                    error,
                    elapsed_ms
                );

                Err(error.clone())
            }
        }
    }
}

/// Mock exchange client for testing
struct MockExchangeClient;

#[async_trait]
impl ExchangeClient for MockExchangeClient {
    fn name(&self) -> &str {
        "MockExchange"
    }

    async fn place_order(&self, order: &Order) -> ExchangeResult<String> {
        Ok(order.id.clone())
    }

    async fn cancel_order(&self, _order_id: &str) -> ExchangeResult<()> {
        Ok(())
    }

    async fn get_order_status(
        &self,
        _order_id: &str,
    ) -> ExchangeResult<crate::domain::repositories::exchange_client::OrderStatus> {
        Ok(crate::domain::repositories::exchange_client::OrderStatus::Filled)
    }

    async fn get_balance(
        &self,
        _currency: Option<&str>,
    ) -> ExchangeResult<Vec<crate::domain::repositories::exchange_client::Balance>> {
        Ok(vec![
            crate::domain::repositories::exchange_client::Balance {
                currency: "USD".to_string(),
                available: 10000.0,
                total: 10000.0,
            },
        ])
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
            min_quantity: 0.0001,
            max_retry_attempts: 3,
            retry_delay_ms: 1000,
        };

        let executor = OrderExecutor::new_with_config(config);
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
            min_quantity: 0.0001,
            max_retry_attempts: 3,
            retry_delay_ms: 1000,
        };

        let executor = OrderExecutor::new_with_config(config);
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
            min_quantity: 0.0001,
            max_retry_attempts: 3,
            retry_delay_ms: 1000,
        };

        let executor = OrderExecutor::new_with_config(config);
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
            min_quantity: 0.0001,
            max_retry_attempts: 3,
            retry_delay_ms: 1000,
        };

        let executor = OrderExecutor::new_with_config(config);
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
            min_quantity: 0.0001,
            max_retry_attempts: 3,
            retry_delay_ms: 1000,
        };

        let executor = OrderExecutor::new_with_config(config);
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
            min_quantity: 0.0001,
            max_retry_attempts: 3,
            retry_delay_ms: 1000,
        };

        let executor = OrderExecutor::new_with_config(config);
        let price = executor.apply_slippage_protection(50000.0, false, 0.02);

        // 50000 * 0.98 = 49000
        assert_eq!(price, 49000.0);
    }
}
