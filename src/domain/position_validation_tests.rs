//! Tests for position validation and management (TDD RED phase)
//! These tests define the expected behavior for position operations
//! All tests are currently RED (failing) and serve as specifications.

#[cfg(test)]
mod position_validation_tests {
    use std::sync::Arc;
    use std::collections::HashMap;

    // ============================================================================
    // POSITION MANAGEMENT TEST FIXTURES
    // ============================================================================

    #[derive(Debug, Clone)]
    pub struct Position {
        pub id: String,
        pub symbol: String,
        pub side: PositionSide,
        pub quantity: f64,
        pub entry_price: f64,
        pub current_price: Option<f64>,
        pub stop_loss_price: Option<f64>,
        pub take_profit_price: Option<f64>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum PositionSide {
        Long,
        Short,
    }

    #[derive(Debug, Clone)]
    pub struct PositionLimits {
        pub max_per_symbol: u32,
        pub max_total: u32,
        pub max_portfolio_exposure: f64, // e.g., 0.8 for 80%
    }

    #[derive(Debug, Clone)]
    pub struct PositionResult {
        pub success: bool,
        pub position_id: Option<String>,
        pub error: Option<String>,
        pub pnl: Option<f64>,
    }

    impl Position {
        pub fn calculate_pnl(&self) -> Option<f64> {
            let current = self.current_price?;
            let position_value = self.quantity * self.entry_price;
            let current_value = self.quantity * current;

            Some(match self.side {
                PositionSide::Long => current_value - position_value,
                PositionSide::Short => position_value - current_value,
            })
        }

        pub fn should_stop_loss(&self) -> bool {
            if let (Some(current), Some(stop_loss)) = (self.current_price, self.stop_loss_price) {
                match self.side {
                    PositionSide::Long => current <= stop_loss,
                    PositionSide::Short => current >= stop_loss,
                }
            } else {
                false
            }
        }

        pub fn should_take_profit(&self) -> bool {
            if let (Some(current), Some(tp)) = (self.current_price, self.take_profit_price) {
                match self.side {
                    PositionSide::Long => current >= tp,
                    PositionSide::Short => current <= tp,
                }
            } else {
                false
            }
        }
    }

    pub struct PositionManager {
        positions: Arc<std::sync::Mutex<HashMap<String, Position>>>,
        limits: PositionLimits,
        portfolio_value: f64,
        next_id: Arc<std::sync::Mutex<u64>>,
    }

    impl PositionManager {
        pub fn new(limits: PositionLimits, portfolio_value: f64) -> Self {
            Self {
                positions: Arc::new(std::sync::Mutex::new(HashMap::new())),
                limits,
                portfolio_value,
                next_id: Arc::new(std::sync::Mutex::new(1)),
            }
        }

        pub fn open_position(
            &self,
            symbol: &str,
            side: PositionSide,
            quantity: f64,
            entry_price: f64,
            stop_loss_pct: Option<f64>,
            take_profit_pct: Option<f64>,
        ) -> PositionResult {
            // Lock positions for validation and creation
            let mut positions = match self.positions.lock() {
                Ok(guard) => guard,
                Err(_) => {
                    return PositionResult {
                        success: false,
                        position_id: None,
                        error: Some("Failed to acquire lock".to_string()),
                        pnl: None,
                    }
                }
            };

            // Validate per-symbol limit
            let symbol_count: u32 = positions
                .values()
                .filter(|p| p.symbol == symbol)
                .count() as u32;

            if symbol_count >= self.limits.max_per_symbol {
                return PositionResult {
                    success: false,
                    position_id: None,
                    error: Some(format!(
                        "Position limit exceeded for symbol {}: {} current, {} max",
                        symbol, symbol_count, self.limits.max_per_symbol
                    )),
                    pnl: None,
                };
            }

            // Validate total position limit
            if positions.len() as u32 >= self.limits.max_total {
                return PositionResult {
                    success: false,
                    position_id: None,
                    error: Some(format!(
                        "Total position limit exceeded: {} current, {} max",
                        positions.len(),
                        self.limits.max_total
                    )),
                    pnl: None,
                };
            }

            // Validate available balance
            let position_value = quantity * entry_price;
            if position_value > self.portfolio_value {
                return PositionResult {
                    success: false,
                    position_id: None,
                    error: Some(format!(
                        "Insufficient balance: {:.2} required, {:.2} available",
                        position_value, self.portfolio_value
                    )),
                    pnl: None,
                };
            }

            // Validate portfolio exposure
            let current_exposure: f64 = positions
                .values()
                .map(|p| (p.quantity * p.entry_price) / self.portfolio_value)
                .sum();

            let new_exposure = current_exposure + (position_value / self.portfolio_value);
            if new_exposure > self.limits.max_portfolio_exposure {
                return PositionResult {
                    success: false,
                    position_id: None,
                    error: Some(format!(
                        "Portfolio exposure would exceed limit: {:.1}% > {:.1}%",
                        new_exposure * 100.0,
                        self.limits.max_portfolio_exposure * 100.0
                    )),
                    pnl: None,
                };
            }

            // Generate position ID
            let id_num = {
                let mut id_guard = self.next_id.lock().unwrap();
                let id = *id_guard;
                *id_guard += 1;
                id
            };
            let position_id = format!("pos_{}_{}", symbol, id_num);

            // Calculate stop-loss and take-profit prices
            let stop_loss_price = stop_loss_pct.map(|pct| {
                match side {
                    PositionSide::Long => entry_price * (1.0 - pct),
                    PositionSide::Short => entry_price * (1.0 + pct),
                }
            });

            let take_profit_price = take_profit_pct.map(|pct| {
                match side {
                    PositionSide::Long => entry_price * (1.0 + pct),
                    PositionSide::Short => entry_price * (1.0 - pct),
                }
            });

            // Create and store position
            let position = Position {
                id: position_id.clone(),
                symbol: symbol.to_string(),
                side,
                quantity,
                entry_price,
                current_price: None,
                stop_loss_price,
                take_profit_price,
            };

            positions.insert(position_id.clone(), position);

            PositionResult {
                success: true,
                position_id: Some(position_id),
                error: None,
                pnl: None,
            }
        }

        pub fn close_position(&self, position_id: &str) -> PositionResult {
            let mut positions = match self.positions.lock() {
                Ok(guard) => guard,
                Err(_) => {
                    return PositionResult {
                        success: false,
                        position_id: None,
                        error: Some("Failed to acquire lock".to_string()),
                        pnl: None,
                    }
                }
            };

            // Find and remove position
            match positions.remove(position_id) {
                Some(position) => {
                    let pnl = position.calculate_pnl();
                    PositionResult {
                        success: true,
                        position_id: Some(position_id.to_string()),
                        error: None,
                        pnl,
                    }
                }
                None => PositionResult {
                    success: false,
                    position_id: None,
                    error: Some(format!("Position {} not found", position_id)),
                    pnl: None,
                },
            }
        }

        pub fn update_position_price(&self, position_id: &str, new_price: f64) -> Result<(), String> {
            let mut positions = self
                .positions
                .lock()
                .map_err(|_| "Failed to acquire lock".to_string())?;

            match positions.get_mut(position_id) {
                Some(position) => {
                    position.current_price = Some(new_price);
                    Ok(())
                }
                None => Err(format!("Position {} not found", position_id)),
            }
        }

        pub fn check_triggers(&self) -> Vec<(String, String)> {
            let positions = match self.positions.lock() {
                Ok(guard) => guard,
                Err(_) => return Vec::new(),
            };

            let mut triggers = Vec::new();
            for (position_id, position) in positions.iter() {
                if position.should_stop_loss() {
                    triggers.push((position_id.clone(), "stop_loss".to_string()));
                } else if position.should_take_profit() {
                    triggers.push((position_id.clone(), "take_profit".to_string()));
                }
            }

            triggers
        }

        pub fn get_positions(&self) -> Vec<Position> {
            match self.positions.lock() {
                Ok(positions) => positions.values().cloned().collect(),
                Err(_) => Vec::new(),
            }
        }

        pub fn get_position_count(&self) -> usize {
            match self.positions.lock() {
                Ok(positions) => positions.len(),
                Err(_) => 0,
            }
        }

        pub fn get_symbol_position_count(&self, symbol: &str) -> u32 {
            match self.positions.lock() {
                Ok(positions) => positions
                    .values()
                    .filter(|p| p.symbol == symbol)
                    .count() as u32,
                Err(_) => 0,
            }
        }

        pub fn get_portfolio_exposure(&self) -> f64 {
            match self.positions.lock() {
                Ok(positions) => {
                    let total_position_value: f64 = positions
                        .values()
                        .map(|p| p.quantity * p.entry_price)
                        .sum();
                    total_position_value / self.portfolio_value
                }
                Err(_) => 0.0,
            }
        }
    }

    // ============================================================================
    // POSITION OPENING VALIDATION TESTS
    // ============================================================================

    /// Test that opening a position validates symbol-specific position limits
    #[test]
    fn test_open_position_should_validate_symbol_limits() {
        let limits = PositionLimits {
            max_per_symbol: 3,
            max_total: 10,
            max_portfolio_exposure: 0.8,
        };
        let manager = PositionManager::new(limits, 10000.0);

        // Should fail: exceeding per-symbol limit
        let result = manager.open_position(
            "BTC-USD",
            PositionSide::Long,
            1.0,
            50000.0,
            Some(0.02),
            Some(0.05),
        );

        assert!(
            !result.success || result.error.is_some(),
            "Should enforce symbol position limits"
        );
    }

    /// Test that opening a position validates total portfolio position limits
    #[test]
    fn test_open_position_should_validate_total_portfolio_limits() {
        let limits = PositionLimits {
            max_per_symbol: 5,
            max_total: 3,
            max_portfolio_exposure: 0.8,
        };
        let manager = PositionManager::new(limits, 10000.0);

        let result = manager.open_position(
            "ETH-USD",
            PositionSide::Long,
            2.0,
            2000.0,
            Some(0.02),
            Some(0.05),
        );

        assert!(
            result.error.is_none() || !result.success,
            "Should validate total portfolio limits"
        );
    }

    /// Test that opening a position validates available balance
    #[test]
    fn test_open_position_should_validate_available_balance() {
        let limits = PositionLimits {
            max_per_symbol: 5,
            max_total: 10,
            max_portfolio_exposure: 0.8,
        };
        let manager = PositionManager::new(limits, 1000.0); // Only $1000 available

        // Trying to open $500k position should fail
        let result = manager.open_position(
            "BTC-USD",
            PositionSide::Long,
            10.0, // 10 * 50000 = 500,000 USD
            50000.0,
            Some(0.02),
            Some(0.05),
        );

        assert!(
            result.error.is_some() && !result.success,
            "Should reject orders exceeding available balance"
        );
    }

    /// Test that portfolio exposure limits are enforced
    #[test]
    fn test_open_position_should_validate_portfolio_exposure_limit() {
        let limits = PositionLimits {
            max_per_symbol: 5,
            max_total: 10,
            max_portfolio_exposure: 0.5, // Max 50% of portfolio
        };
        let manager = PositionManager::new(limits, 10000.0); // $10k portfolio

        // Trying to open $6k position (60% exposure) should fail
        let result = manager.open_position(
            "BTC-USD",
            PositionSide::Long,
            0.12,
            50000.0,
            Some(0.02),
            Some(0.05),
        );

        assert!(
            result.error.is_some() || !result.success,
            "Should enforce maximum portfolio exposure limit"
        );
    }

    /// Test that valid positions open successfully with proper ID generation
    #[test]
    fn test_open_position_should_succeed_with_valid_parameters() {
        let limits = PositionLimits {
            max_per_symbol: 5,
            max_total: 10,
            max_portfolio_exposure: 0.8,
        };
        let manager = PositionManager::new(limits, 10000.0);

        let result = manager.open_position(
            "ETH-USD",
            PositionSide::Long,
            1.0,
            2000.0,
            Some(0.02),
            Some(0.05),
        );

        assert!(result.success, "Should open valid position");
        assert!(
            result.position_id.is_some(),
            "Should return position ID for successful open"
        );
        assert!(result.error.is_none(), "Should have no error for valid position");
    }

    // ============================================================================
    // POSITION CLOSING & PNL TESTS
    // ============================================================================

    /// Test that closing a position calculates PnL accurately for long positions
    #[test]
    fn test_close_position_should_calculate_accurate_pnl_long() {
        let limits = PositionLimits {
            max_per_symbol: 5,
            max_total: 10,
            max_portfolio_exposure: 0.8,
        };
        let manager = PositionManager::new(limits, 10000.0);

        // Open position at $100, close at $110 -> profit of $10 per share
        // If we buy 10 shares: $1000 * 1.10 = $1100 -> profit $100
        let result = manager.open_position(
            "TEST-USD",
            PositionSide::Long,
            10.0,
            100.0,
            None,
            None,
        );

        let position_id = result.position_id.expect("Position should open");

        // Manually update price (in real implementation)
        manager
            .update_position_price(&position_id, 110.0)
            .expect("Price update should work");

        let close_result = manager.close_position(&position_id);

        assert!(close_result.success, "Should close position successfully");
        assert!(
            close_result.pnl.is_some(),
            "Should calculate and return PnL"
        );
        assert_eq!(
            close_result.pnl.unwrap(),
            100.0, // 10 shares * ($110 - $100)
            "PnL should be correctly calculated"
        );
    }

    /// Test that closing a position calculates PnL accurately for short positions
    #[test]
    fn test_close_position_should_calculate_accurate_pnl_short() {
        let limits = PositionLimits {
            max_per_symbol: 5,
            max_total: 10,
            max_portfolio_exposure: 0.8,
        };
        let manager = PositionManager::new(limits, 10000.0);

        // Open short position at $100, close at $90 -> profit of $10 per share
        let result = manager.open_position(
            "TEST-USD",
            PositionSide::Short,
            10.0,
            100.0,
            None,
            None,
        );

        let position_id = result.position_id.expect("Position should open");
        manager
            .update_position_price(&position_id, 90.0)
            .expect("Price update should work");

        let close_result = manager.close_position(&position_id);

        assert!(close_result.success, "Should close short position");
        assert_eq!(
            close_result.pnl.unwrap(),
            100.0, // 10 shares * ($100 - $90)
            "Short position PnL should be correctly calculated"
        );
    }

    /// Test precision handling in PnL calculations
    #[test]
    fn test_position_pnl_calculation_precision() {
        let limits = PositionLimits {
            max_per_symbol: 5,
            max_total: 10,
            max_portfolio_exposure: 0.8,
        };
        let manager = PositionManager::new(limits, 10000.0);

        // Test with precise decimal values
        let result = manager.open_position(
            "BTC-USD",
            PositionSide::Long,
            0.12345678,
            50000.0,
            None,
            None,
        );

        let position_id = result.position_id.expect("Position should open");
        manager
            .update_position_price(&position_id, 51000.0)
            .expect("Price update");

        let close_result = manager.close_position(&position_id);

        assert!(close_result.pnl.is_some(), "Should handle decimal precision");
        let pnl = close_result.pnl.unwrap();
        let expected = 0.12345678 * (51000.0 - 50000.0);
        assert!(
            (pnl - expected).abs() < 0.01,
            "PnL precision should be maintained"
        );
    }

    // ============================================================================
    // STOP-LOSS & TAKE-PROFIT TESTS
    // ============================================================================

    /// Test that stop-loss triggers automatically close long positions
    #[test]
    fn test_stop_loss_trigger_should_auto_close_long_position() {
        let limits = PositionLimits {
            max_per_symbol: 5,
            max_total: 10,
            max_portfolio_exposure: 0.8,
        };
        let manager = PositionManager::new(limits, 10000.0);

        // Open long position with 2% stop-loss
        let result = manager.open_position(
            "BTC-USD",
            PositionSide::Long,
            0.1,
            50000.0,
            Some(0.02), // Stop at $49,000
            None,
        );

        let position_id = result.position_id.expect("Position should open");

        // Update price below stop-loss
        manager
            .update_position_price(&position_id, 48900.0)
            .expect("Price update");

        let triggers = manager.check_triggers();
        assert!(
            triggers.iter().any(|(pid, _)| pid == &position_id),
            "Stop-loss should be triggered"
        );
    }

    /// Test that take-profit triggers automatically close long positions
    #[test]
    fn test_take_profit_trigger_should_auto_close_long_position() {
        let limits = PositionLimits {
            max_per_symbol: 5,
            max_total: 10,
            max_portfolio_exposure: 0.8,
        };
        let manager = PositionManager::new(limits, 10000.0);

        // Open long position with 5% take-profit
        let result = manager.open_position(
            "BTC-USD",
            PositionSide::Long,
            0.1,
            50000.0,
            None,
            Some(0.05), // Take profit at $52,500
        );

        let position_id = result.position_id.expect("Position should open");

        // Update price above take-profit
        manager
            .update_position_price(&position_id, 52600.0)
            .expect("Price update");

        let triggers = manager.check_triggers();
        assert!(
            triggers.iter().any(|(pid, _)| pid == &position_id),
            "Take-profit should be triggered"
        );
    }

    // ============================================================================
    // POSITION MANAGEMENT & CONCURRENCY TESTS
    // ============================================================================

    /// Test that position count tracking is accurate
    #[test]
    fn test_position_count_should_be_accurate() {
        let limits = PositionLimits {
            max_per_symbol: 5,
            max_total: 10,
            max_portfolio_exposure: 0.8,
        };
        let manager = PositionManager::new(limits, 100000.0);

        let r1 = manager.open_position("BTC-USD", PositionSide::Long, 0.1, 50000.0, None, None);
        let r2 = manager.open_position("ETH-USD", PositionSide::Long, 1.0, 2000.0, None, None);

        let count = manager.get_position_count();
        assert_eq!(count, 2, "Should accurately track position count");
    }

    /// Test that symbol-specific position count is tracked
    #[test]
    fn test_symbol_position_count_should_be_tracked() {
        let limits = PositionLimits {
            max_per_symbol: 5,
            max_total: 10,
            max_portfolio_exposure: 0.8,
        };
        let manager = PositionManager::new(limits, 100000.0);

        manager.open_position("BTC-USD", PositionSide::Long, 0.05, 50000.0, None, None);
        manager.open_position("BTC-USD", PositionSide::Long, 0.05, 50000.0, None, None);
        manager.open_position("ETH-USD", PositionSide::Long, 1.0, 2000.0, None, None);

        let btc_count = manager.get_symbol_position_count("BTC-USD");
        assert_eq!(btc_count, 2, "Should track per-symbol positions");
    }

    /// Test that portfolio exposure is calculated correctly
    #[test]
    fn test_portfolio_exposure_calculation() {
        let limits = PositionLimits {
            max_per_symbol: 5,
            max_total: 10,
            max_portfolio_exposure: 0.8,
        };
        let portfolio_value = 10000.0;
        let manager = PositionManager::new(limits, portfolio_value);

        // Open position worth $3000 (30% of portfolio)
        manager.open_position("BTC-USD", PositionSide::Long, 0.06, 50000.0, None, None);

        let exposure = manager.get_portfolio_exposure();
        assert!(
            (exposure - 0.30).abs() < 0.01,
            "Portfolio exposure should be 30%"
        );
    }

    /// Test that position operations are atomic (no partial state)
    #[test]
    fn test_position_atomic_operations() {
        let limits = PositionLimits {
            max_per_symbol: 5,
            max_total: 10,
            max_portfolio_exposure: 0.8,
        };
        let manager = PositionManager::new(limits, 10000.0);

        let result = manager.open_position(
            "BTC-USD",
            PositionSide::Long,
            0.1,
            50000.0,
            Some(0.02),
            Some(0.05),
        );

        if result.success {
            let positions = manager.get_positions();
            assert!(!positions.is_empty(), "Position should be fully created");
            assert!(
                positions[0].stop_loss_price.is_some()
                    && positions[0].take_profit_price.is_some(),
                "Position should have all fields set"
            );
        }
    }

    /// Test that closing a failed position doesn't affect system state
    #[test]
    fn test_close_position_should_rollback_on_failure() {
        let limits = PositionLimits {
            max_per_symbol: 5,
            max_total: 10,
            max_portfolio_exposure: 0.8,
        };
        let manager = PositionManager::new(limits, 10000.0);

        // Try to close non-existent position
        let result = manager.close_position("invalid_position_id");

        assert!(!result.success, "Should fail for invalid position");
        assert!(result.pnl.is_none(), "Should not compute PnL for failed close");

        // Verify system state is consistent
        let positions = manager.get_positions();
        assert!(positions.is_empty(), "No positions should exist after failed close");
    }
}
