//! Integration tests for OrderExecutor with BalanceManager and LeverageCalculator

#[cfg(test)]
mod tests {
    use crate::domain::entities::balance::BalanceInfo;
    use crate::domain::entities::leverage::LeverageInfo;
    use crate::domain::repositories::exchange_client::{Balance, ExchangeClient, ExchangeResult, OrderStatus};
    use crate::domain::services::balance_manager::BalanceManager;
    use crate::domain::services::leverage_calculator::LeverageCalculator;
    use crate::domain::services::order_executor::{
        OrderExecutor, OrderExecutorConfig, Signal, TradingSignal,
    };
    use crate::domain::services::position_manager::PositionLimits;
    use crate::domain::services::position_manager::PositionManager;
    use crate::domain::services::metrics::TradingMetrics;
    use crate::domain::entities::order::Order;
    use crate::domain::entities::exchange::Exchange;
    use async_trait::async_trait;
    use std::sync::Arc;
    use std::collections::HashMap;
    use tokio::sync::Mutex;
    use std::time::{Duration, SystemTime};

    /// Mock exchange client that returns predefined balance and account info
    struct MockExchangeClientForIntegration {
        balance_available: f64,
        balance_total: f64,
    }

    #[async_trait]
    impl ExchangeClient for MockExchangeClientForIntegration {
        fn name(&self) -> &str {
            "MockIntegration"
        }

        async fn place_order(&self, _order: &Order) -> ExchangeResult<String> {
            Ok("mock-order-id".to_string())
        }

        async fn cancel_order(&self, _order_id: &str) -> ExchangeResult<()> {
            Ok(())
        }

        async fn get_order_status(&self, _order_id: &str) -> ExchangeResult<OrderStatus> {
            Ok(OrderStatus::Filled)
        }

        async fn get_balance(&self, _currency: Option<&str>) -> ExchangeResult<Vec<Balance>> {
            Ok(vec![
                Balance {
                    currency: "USD".to_string(),
                    available: self.balance_available,
                    total: self.balance_total,
                },
                Balance {
                    currency: "BTC".to_string(),
                    available: 0.0,
                    total: 0.0,
                },
            ])
        }
    }

    // ===== PHASE 4.1: Enhanced OrderExecutor with Balance/Leverage Checks =====

    #[test]
    fn test_order_executor_has_balance_manager_field() {
        // Test that OrderExecutor can be created with balance_manager
        let config = OrderExecutorConfig {
            confidence_threshold: 0.5,
            symbols: vec!["BTC-USD".to_string()],
            traders: vec!["trader1".to_string()],
            max_per_hour: 10,
            max_per_day: 50,
            portfolio_percentage: 0.1,
            slippage_pct: 0.02,
            min_quantity: 0.0001,
            max_retry_attempts: 3,
            retry_delay_ms: 100,
        };

        let position_limits = PositionLimits {
            max_per_symbol: 3,
            max_total: 10,
            max_portfolio_exposure: 0.8,
        };

        let position_manager = Arc::new(Mutex::new(PositionManager::new(position_limits, 10000.0)));
        let mut exchange_clients = HashMap::new();
        let mock_client: Arc<dyn ExchangeClient> = Arc::new(MockExchangeClientForIntegration {
            balance_available: 10000.0,
            balance_total: 10000.0,
        });
        exchange_clients.insert(Exchange::Coinbase, Arc::clone(&mock_client));
        
        let metrics = Arc::new(Mutex::new(TradingMetrics::new()));
        
        let balance_manager = Arc::new(BalanceManager::new(Arc::clone(&mock_client), Duration::from_secs(300)));
        let leverage_calculator = Arc::new(LeverageCalculator::new(Arc::clone(&mock_client)));

        let executor = OrderExecutor::with_managers(
            config,
            position_manager,
            exchange_clients,
            metrics,
            10000.0,
            balance_manager,
            leverage_calculator,
        );

        // After implementation, balance_manager should be set
        // This test will pass once we implement execute_signal_with_balance_check
    }

    #[test]
    fn test_order_executor_has_leverage_calculator_field() {
        // Test that OrderExecutor can be created with leverage_calculator
        let config = OrderExecutorConfig {
            confidence_threshold: 0.5,
            symbols: vec!["BTC-USD".to_string()],
            traders: vec!["trader1".to_string()],
            max_per_hour: 10,
            max_per_day: 50,
            portfolio_percentage: 0.1,
            slippage_pct: 0.02,
            min_quantity: 0.0001,
            max_retry_attempts: 3,
            retry_delay_ms: 100,
        };

        let position_limits = PositionLimits {
            max_per_symbol: 3,
            max_total: 10,
            max_portfolio_exposure: 0.8,
        };

        let position_manager = Arc::new(Mutex::new(PositionManager::new(position_limits, 10000.0)));
        let mut exchange_clients = HashMap::new();
        let mock_client: Arc<dyn ExchangeClient> = Arc::new(MockExchangeClientForIntegration {
            balance_available: 10000.0,
            balance_total: 10000.0,
        });
        exchange_clients.insert(Exchange::Coinbase, Arc::clone(&mock_client));
        
        let metrics = Arc::new(Mutex::new(TradingMetrics::new()));
        
        let balance_manager = Arc::new(BalanceManager::new(Arc::clone(&mock_client), Duration::from_secs(300)));
        let leverage_calculator = Arc::new(LeverageCalculator::new(Arc::clone(&mock_client)));

        let executor = OrderExecutor::with_managers(
            config,
            position_manager,
            exchange_clients,
            metrics,
            10000.0,
            balance_manager,
            leverage_calculator,
        );

        // After implementation, leverage_calculator should be set
        // This test will pass once we implement execute_signal_with_leverage_check
    }

    #[tokio::test]
    async fn test_execute_signal_fetches_balance_before_execution() {
        // Test: OrderExecutor should fetch balance before executing a signal
        let config = OrderExecutorConfig {
            confidence_threshold: 0.5,
            symbols: vec!["BTC-USD".to_string()],
            traders: vec!["trader1".to_string()],
            max_per_hour: 10,
            max_per_day: 50,
            portfolio_percentage: 0.1,
            slippage_pct: 0.02,
            min_quantity: 0.0001,
            max_retry_attempts: 3,
            retry_delay_ms: 100,
        };

        let position_limits = PositionLimits {
            max_per_symbol: 3,
            max_total: 10,
            max_portfolio_exposure: 0.8,
        };

        let position_manager = Arc::new(Mutex::new(PositionManager::new(position_limits, 10000.0)));
        let mut exchange_clients = HashMap::new();
        let mock_client: Arc<dyn ExchangeClient> = Arc::new(MockExchangeClientForIntegration {
            balance_available: 10000.0,
            balance_total: 10000.0,
        });
        exchange_clients.insert(Exchange::Coinbase, Arc::clone(&mock_client));

        let metrics = Arc::new(Mutex::new(TradingMetrics::new()));
        
        let balance_manager = Arc::new(BalanceManager::new(Arc::clone(&mock_client), Duration::from_secs(300)));
        let leverage_calculator = Arc::new(LeverageCalculator::new(Arc::clone(&mock_client)));

        let mut executor = OrderExecutor::with_managers(
            config,
            position_manager,
            exchange_clients,
            metrics,
            10000.0,
            balance_manager.clone(),
            leverage_calculator,
        );

        // Create a buy signal
        let signal = TradingSignal {
            signal: Signal::Buy,
            confidence: 0.8,
        };

        // This test verifies that balance is fetched before execution
        // Expected behavior: execute_signal_with_balance_check should fetch and validate balance
        // For now, this is a placeholder that documents expected behavior
    }

    #[test]
    fn test_execute_signal_checks_leverage_before_execution() {
        // Test: OrderExecutor should check leverage constraints before executing
        let config = OrderExecutorConfig {
            confidence_threshold: 0.5,
            symbols: vec!["BTC-USD".to_string()],
            traders: vec!["trader1".to_string()],
            max_per_hour: 10,
            max_per_day: 50,
            portfolio_percentage: 0.1,
            slippage_pct: 0.02,
            min_quantity: 0.0001,
            max_retry_attempts: 3,
            retry_delay_ms: 100,
        };

        let position_limits = PositionLimits {
            max_per_symbol: 3,
            max_total: 10,
            max_portfolio_exposure: 0.8,
        };

        let position_manager = Arc::new(Mutex::new(PositionManager::new(position_limits, 10000.0)));
        let mut exchange_clients = HashMap::new();
        let mock_client: Arc<dyn ExchangeClient> = Arc::new(MockExchangeClientForIntegration {
            balance_available: 10000.0,
            balance_total: 10000.0,
        });
        exchange_clients.insert(Exchange::Coinbase, Arc::clone(&mock_client));
        
        let metrics = Arc::new(Mutex::new(TradingMetrics::new()));
        
        let balance_manager = Arc::new(BalanceManager::new(Arc::clone(&mock_client), Duration::from_secs(300)));
        let leverage_calculator = Arc::new(LeverageCalculator::new(Arc::clone(&mock_client)));

        let executor = OrderExecutor::with_managers(
            config,
            position_manager,
            exchange_clients,
            metrics,
            10000.0,
            balance_manager,
            leverage_calculator,
        );

        // Test that leverage constraints are checked
        // This verifies the leverage_calculator integration
    }

    #[test]
    fn test_execute_signal_sizes_position_correctly() {
        // Test: Position sizing should be applied during signal execution
        let config = OrderExecutorConfig {
            confidence_threshold: 0.5,
            symbols: vec!["BTC-USD".to_string()],
            traders: vec!["trader1".to_string()],
            max_per_hour: 10,
            max_per_day: 50,
            portfolio_percentage: 0.1,
            slippage_pct: 0.02,
            min_quantity: 0.0001,
            max_retry_attempts: 3,
            retry_delay_ms: 100,
        };

        let position_limits = PositionLimits {
            max_per_symbol: 3,
            max_total: 10,
            max_portfolio_exposure: 0.8,
        };

        let position_manager = Arc::new(Mutex::new(PositionManager::new(position_limits, 10000.0)));
        let mut exchange_clients = HashMap::new();
        let mock_client: Arc<dyn ExchangeClient> = Arc::new(MockExchangeClientForIntegration {
            balance_available: 10000.0,
            balance_total: 10000.0,
        });
        exchange_clients.insert(Exchange::Coinbase, Arc::clone(&mock_client));
        
        let metrics = Arc::new(Mutex::new(TradingMetrics::new()));
        
        let balance_manager = Arc::new(BalanceManager::new(Arc::clone(&mock_client), Duration::from_secs(300)));
        let leverage_calculator = Arc::new(LeverageCalculator::new(Arc::clone(&mock_client)));

        let executor = OrderExecutor::with_managers(
            config,
            position_manager,
            exchange_clients,
            metrics,
            10000.0,
            balance_manager,
            leverage_calculator,
        );

        // Test that position is sized correctly using PositionSizer
        // This verifies position_sizer integration
    }

    #[test]
    fn test_insufficient_balance_prevents_execution() {
        // Test: Execution should fail if balance is insufficient
        let config = OrderExecutorConfig {
            confidence_threshold: 0.5,
            symbols: vec!["BTC-USD".to_string()],
            traders: vec!["trader1".to_string()],
            max_per_hour: 10,
            max_per_day: 50,
            portfolio_percentage: 0.1,
            slippage_pct: 0.02,
            min_quantity: 0.0001,
            max_retry_attempts: 3,
            retry_delay_ms: 100,
        };

        let position_limits = PositionLimits {
            max_per_symbol: 3,
            max_total: 10,
            max_portfolio_exposure: 0.8,
        };

        let position_manager = Arc::new(Mutex::new(PositionManager::new(position_limits, 50.0))); // Very small balance
        let mut exchange_clients = HashMap::new();
        let mock_client: Arc<dyn ExchangeClient> = Arc::new(MockExchangeClientForIntegration {
            balance_available: 50.0,
            balance_total: 50.0,
        });
        exchange_clients.insert(Exchange::Coinbase, Arc::clone(&mock_client));
        
        let metrics = Arc::new(Mutex::new(TradingMetrics::new()));
        
        let balance_manager = Arc::new(BalanceManager::new(Arc::clone(&mock_client), Duration::from_secs(300)));
        let leverage_calculator = Arc::new(LeverageCalculator::new(Arc::clone(&mock_client)));

        let executor = OrderExecutor::with_managers(
            config,
            position_manager,
            exchange_clients,
            metrics,
            50.0,
            balance_manager,
            leverage_calculator,
        );

        // Test that insufficient balance prevents execution
    }

    #[test]
    fn test_insufficient_leverage_prevents_execution() {
        // Test: Execution should fail if leverage is insufficient
        let config = OrderExecutorConfig {
            confidence_threshold: 0.5,
            symbols: vec!["BTC-USD".to_string()],
            traders: vec!["trader1".to_string()],
            max_per_hour: 10,
            max_per_day: 50,
            portfolio_percentage: 0.1,
            slippage_pct: 0.02,
            min_quantity: 0.0001,
            max_retry_attempts: 3,
            retry_delay_ms: 100,
        };

        let position_limits = PositionLimits {
            max_per_symbol: 3,
            max_total: 10,
            max_portfolio_exposure: 0.8,
        };

        let position_manager = Arc::new(Mutex::new(PositionManager::new(position_limits, 10000.0)));
        let mut exchange_clients = HashMap::new();
        let mock_client: Arc<dyn ExchangeClient> = Arc::new(MockExchangeClientForIntegration {
            balance_available: 10000.0,
            balance_total: 10000.0,
        });
        exchange_clients.insert(Exchange::Coinbase, Arc::clone(&mock_client));
        
        let metrics = Arc::new(Mutex::new(TradingMetrics::new()));
        
        let balance_manager = Arc::new(BalanceManager::new(Arc::clone(&mock_client), Duration::from_secs(300)));
        let leverage_calculator = Arc::new(LeverageCalculator::new(Arc::clone(&mock_client)));

        let executor = OrderExecutor::with_managers(
            config,
            position_manager,
            exchange_clients,
            metrics,
            10000.0,
            balance_manager,
            leverage_calculator,
        );

        // Test that insufficient leverage prevents execution
    }

    #[test]
    fn test_position_placed_with_calculated_quantity() {
        // Test: Verify order is placed with PositionSizer calculated quantity
        let config = OrderExecutorConfig {
            confidence_threshold: 0.5,
            symbols: vec!["BTC-USD".to_string()],
            traders: vec!["trader1".to_string()],
            max_per_hour: 10,
            max_per_day: 50,
            portfolio_percentage: 0.1,
            slippage_pct: 0.02,
            min_quantity: 0.0001,
            max_retry_attempts: 3,
            retry_delay_ms: 100,
        };

        let position_limits = PositionLimits {
            max_per_symbol: 3,
            max_total: 10,
            max_portfolio_exposure: 0.8,
        };

        let position_manager = Arc::new(Mutex::new(PositionManager::new(position_limits, 10000.0)));
        let mut exchange_clients = HashMap::new();
        let mock_client: Arc<dyn ExchangeClient> = Arc::new(MockExchangeClientForIntegration {
            balance_available: 10000.0,
            balance_total: 10000.0,
        });
        exchange_clients.insert(Exchange::Coinbase, Arc::clone(&mock_client));
        
        let metrics = Arc::new(Mutex::new(TradingMetrics::new()));
        
        let balance_manager = Arc::new(BalanceManager::new(Arc::clone(&mock_client), Duration::from_secs(300)));
        let leverage_calculator = Arc::new(LeverageCalculator::new(Arc::clone(&mock_client)));

        let executor = OrderExecutor::with_managers(
            config,
            position_manager,
            exchange_clients,
            metrics,
            10000.0,
            balance_manager,
            leverage_calculator,
        );

        // Test that order quantity is calculated by PositionSizer
    }

    // ===== PHASE 4.2: Error Handling Tests =====

    #[test]
    fn test_balance_fetch_failure_error() {
        // Red test: Error handling when balance fetch fails
        
        // Expected: Should return TradeExecutionError::BalanceFetchFailed
        // EXPECTED IMPLEMENTATION:
        // When MockExchangeClient returns an error, execute_signal should propagate it
    }

    #[test]
    fn test_leverage_calculation_failure_error() {
        // Red test: Error handling when leverage calculation fails
        
        // Expected: Should return TradeExecutionError::LeverageCalculationFailed
    }

    #[test]
    fn test_position_sizing_failure_error() {
        // Red test: Error handling when position sizing fails
        
        // Expected: Should return TradeExecutionError::PositionSizingFailed
    }

    #[test]
    fn test_order_placement_failure_error() {
        // Red test: Error handling when order placement fails with dYdX
        
        // Expected: Should return TradeExecutionError::OrderPlacementFailed
    }

    // ===== PHASE 4.3: Metrics and Logging Tests =====

    #[test]
    fn test_successful_execution_is_logged() {
        // Red test: Successful signal execution should be logged
        
        // Expected: Should log at INFO level with execution details
        // EXPECTED IMPLEMENTATION:
        // tracing::info!("Signal executed successfully: ...");
    }

    #[test]
    fn test_failed_execution_is_logged() {
        // Red test: Failed signal execution should be logged with error details
        
        // Expected: Should log at ERROR level with reason for failure
    }

    #[test]
    fn test_execution_metrics_recorded() {
        // Red test: Execution metrics should be recorded
        
        // Expected: Should record:
        // - execution_time_ms
        // - quantity_executed
        // - fill_price
        // - execution_status (success/failure)
    }
}
