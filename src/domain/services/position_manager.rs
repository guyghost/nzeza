//! PositionManager service - manages opening, closing, and tracking positions
//! with proper validation and ACID properties

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

/// Position representation with all metadata
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

/// Limits for position management
#[derive(Debug, Clone)]
pub struct PositionLimits {
    pub max_per_symbol: u32,
    pub max_total: u32,
    pub max_portfolio_exposure: f64,
}

/// Result of position operation
#[derive(Debug, Clone)]
pub struct PositionResult {
    pub success: bool,
    pub position_id: Option<String>,
    pub error: Option<String>,
    pub pnl: Option<f64>,
}

impl Position {
    /// Calculate unrealized P&L for the position
    pub fn calculate_pnl(&self) -> Option<f64> {
        let current = self.current_price?;
        let entry_value = self.quantity * self.entry_price;
        let current_value = self.quantity * current;

        Some(match self.side {
            PositionSide::Long => current_value - entry_value,
            PositionSide::Short => entry_value - current_value,
        })
    }

    /// Check if stop-loss condition is met
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

    /// Check if take-profit condition is met
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

/// Manager for trading positions with validation and limits
pub struct PositionManager {
    positions: Arc<std::sync::Mutex<HashMap<String, Position>>>,
    limits: PositionLimits,
    portfolio_value: f64,
}

impl PositionManager {
    /// Create a new PositionManager
    pub fn new(limits: PositionLimits, portfolio_value: f64) -> Self {
        Self {
            positions: Arc::new(std::sync::Mutex::new(HashMap::new())),
            limits,
            portfolio_value,
        }
    }

    /// Open a new position with validation
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
        let symbol_count: u32 = positions.values().filter(|p| p.symbol == symbol).count() as u32;

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
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let position_id = format!("pos_{}_{}", symbol, timestamp);

        // Calculate stop-loss and take-profit prices
        let stop_loss_price = stop_loss_pct.map(|pct| match side {
            PositionSide::Long => entry_price * (1.0 - pct),
            PositionSide::Short => entry_price * (1.0 + pct),
        });

        let take_profit_price = take_profit_pct.map(|pct| match side {
            PositionSide::Long => entry_price * (1.0 + pct),
            PositionSide::Short => entry_price * (1.0 - pct),
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

    /// Close a position and calculate P&L
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

    /// Update position price (for market movements)
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

    /// Check for triggered stop-loss or take-profit
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

    /// Get all open positions
    pub fn get_positions(&self) -> Vec<Position> {
        match self.positions.lock() {
            Ok(positions) => positions.values().cloned().collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Get total number of open positions
    pub fn get_position_count(&self) -> usize {
        match self.positions.lock() {
            Ok(positions) => positions.len(),
            Err(_) => 0,
        }
    }

    /// Get number of open positions for a specific symbol
    pub fn get_symbol_position_count(&self, symbol: &str) -> u32 {
        match self.positions.lock() {
            Ok(positions) => positions.values().filter(|p| p.symbol == symbol).count() as u32,
            Err(_) => 0,
        }
    }

    /// Calculate current portfolio exposure (position value / portfolio value)
    pub fn get_portfolio_exposure(&self) -> f64 {
        match self.positions.lock() {
            Ok(positions) => {
                let total_position_value: f64 =
                    positions.values().map(|p| p.quantity * p.entry_price).sum();
                total_position_value / self.portfolio_value
            }
            Err(_) => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_manager_creation() {
        let limits = PositionLimits {
            max_per_symbol: 3,
            max_total: 10,
            max_portfolio_exposure: 0.8,
        };
        let manager = PositionManager::new(limits, 10000.0);
        assert_eq!(manager.get_position_count(), 0);
    }

    #[test]
    fn test_calculate_pnl_long() {
        let mut position = Position {
            id: "test".to_string(),
            symbol: "BTC-USD".to_string(),
            side: PositionSide::Long,
            quantity: 1.0,
            entry_price: 100.0,
            current_price: Some(110.0),
            stop_loss_price: None,
            take_profit_price: None,
        };

        let pnl = position.calculate_pnl().unwrap();
        assert_eq!(pnl, 10.0);
    }

    #[test]
    fn test_calculate_pnl_short() {
        let position = Position {
            id: "test".to_string(),
            symbol: "BTC-USD".to_string(),
            side: PositionSide::Short,
            quantity: 1.0,
            entry_price: 100.0,
            current_price: Some(90.0),
            stop_loss_price: None,
            take_profit_price: None,
        };

        let pnl = position.calculate_pnl().unwrap();
        assert_eq!(pnl, 10.0);
    }
}
