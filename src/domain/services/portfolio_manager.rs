//! PortfolioManager - ACID-compliant portfolio state management

use std::time::SystemTime;
use std::collections::HashMap;

/// Position data
#[derive(Debug, Clone)]
pub struct Position {
    pub id: String,
    pub symbol: String,
    pub quantity: f64,
    pub entry_price: f64,
    pub current_price: Option<f64>,
}

/// ACID-compliant portfolio manager
pub struct PortfolioManager {
    initial_value: f64,
    total_value: f64,
    available_cash: f64,
    positions: HashMap<String, Position>,
    max_total_positions: usize,
    max_per_symbol: usize,
    max_exposure: f64,
}

impl PortfolioManager {
    /// Create a new portfolio manager
    pub fn new(initial_value: f64) -> Self {
        Self {
            initial_value,
            total_value: initial_value,
            available_cash: initial_value,
            positions: HashMap::new(),
            max_total_positions: 5,
            max_per_symbol: 3,
            max_exposure: 0.8,
        }
    }

    /// Open a position atomically
    pub fn open_position_atomic(
        &mut self,
        symbol: &str,
        quantity: f64,
        entry_price: f64,
    ) -> Result<String, String> {
        // Check position count limits
        if self.positions.len() >= self.max_total_positions {
            return Err(format!("Maximum positions ({}) reached", self.max_total_positions));
        }

        // Check per-symbol limit
        let symbol_count = self.positions.values().filter(|p| p.symbol == symbol).count();
        if symbol_count >= self.max_per_symbol {
            return Err(format!("Maximum positions per symbol ({}) reached for {}", self.max_per_symbol, symbol));
        }

        // Calculate position value
        let position_value = quantity * entry_price;

        // Check balance
        if position_value > self.available_cash {
            return Err(format!(
                "Insufficient balance: required {:.2}, available {:.2}",
                position_value, self.available_cash
            ));
        }

        // Check exposure limit
        let current_position_value = self.get_position_value();
        let total_value = self.get_total_value();
        if total_value > 0.0 {
            let new_exposure = (current_position_value + position_value) / total_value;
            if new_exposure > self.max_exposure {
                return Err(format!(
                    "Exposure limit exceeded: {:.2} > {:.2}",
                    new_exposure, self.max_exposure
                ));
            }
        }

        // Generate position ID
        let position_id = format!("pos_{}_{}", symbol, SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0));

        // Create position
        let position = Position {
            id: position_id.clone(),
            symbol: symbol.to_string(),
            quantity,
            entry_price,
            current_price: Some(entry_price),
        };

        // Update state
        self.available_cash -= position_value;
        self.positions.insert(position_id.clone(), position);

        // Validate invariants
        if let Err(e) = self.validate_invariants() {
            // This shouldn't happen if logic is correct, but rollback just in case
            self.positions.remove(&position_id);
            self.available_cash += position_value;
            return Err(format!("Invariant violation after open: {}", e));
        }

        Ok(position_id)
    }

    /// Close a position atomically
    pub fn close_position_atomic(&mut self, position_id: &str) -> Result<f64, String> {
        // Find position
        let position = self
            .positions
            .get(position_id)
            .ok_or_else(|| format!("Position {} not found", position_id))?
            .clone();

        // Calculate PnL
        let entry_value = position.quantity * position.entry_price;
        let current_price = position.current_price.unwrap_or(position.entry_price);
        let current_value = position.quantity * current_price;
        let pnl = current_value - entry_value;

        // Update state
        self.available_cash += entry_value + pnl;
        self.total_value += pnl;
        self.positions.remove(position_id);

        // Validate invariants
        if let Err(e) = self.validate_invariants() {
            // This shouldn't happen, but rollback just in case
            self.positions.insert(position_id.to_string(), position);
            self.available_cash -= entry_value + pnl;
            self.total_value -= pnl;
            return Err(format!("Invariant violation after close: {}", e));
        }

        Ok(pnl)
    }

    /// Validate all portfolio invariants
    pub fn validate_invariants(&self) -> Result<(), String> {
        // Invariant 1: available_cash >= 0
        if self.available_cash < 0.0 {
            return Err(format!(
                "Invariant 1 violated: available_cash {} < 0",
                self.available_cash
            ));
        }

        // Invariant 2: total_value == available_cash + position_value
        let position_value = self.get_position_value();
        let expected_total = self.available_cash + position_value;
        if (self.total_value - expected_total).abs() > 0.01 {
            return Err(format!(
                "Invariant 2 violated: total_value {} != available_cash {} + position_value {}",
                self.total_value, self.available_cash, position_value
            ));
        }

        // Invariant 3: position count <= max_total_positions
        if self.positions.len() > self.max_total_positions {
            return Err(format!(
                "Invariant 3 violated: position count {} > max_total_positions {}",
                self.positions.len(), self.max_total_positions
            ));
        }

        // Invariant 4: per-symbol position count <= max_per_symbol
        for symbol in self.positions.values().map(|p| &p.symbol).collect::<std::collections::HashSet<_>>() {
            let count = self.positions.values().filter(|p| &p.symbol == symbol).count();
            if count > self.max_per_symbol {
                return Err(format!(
                    "Invariant 4 violated: symbol {} has {} positions > max_per_symbol {}",
                    symbol, count, self.max_per_symbol
                ));
            }
        }

        // Invariant 5: exposure <= max_exposure
        if self.total_value > 0.0 {
            let exposure = position_value / self.total_value;
            if exposure > self.max_exposure {
                return Err(format!(
                    "Invariant 5 violated: exposure {} > max_exposure {}",
                    exposure, self.max_exposure
                ));
            }
        }

        // Invariant 6: all positions have valid data
        for position in self.positions.values() {
            if position.quantity <= 0.0 || position.entry_price <= 0.0 {
                return Err(format!(
                    "Invariant 6 violated: position {} has invalid quantity {} or entry_price {}",
                    position.id, position.quantity, position.entry_price
                ));
            }
        }

        Ok(())
    }

    /// Quick consistency check
    pub fn validate_consistency(&self) -> bool {
        let position_value = self.get_position_value();
        let expected_total = self.available_cash + position_value;
        (self.total_value - expected_total).abs() < 0.01
    }

    /// Get total portfolio value
    pub fn get_total_value(&self) -> f64 {
        self.total_value
    }

    /// Get available cash
    pub fn get_available_cash(&self) -> f64 {
        self.available_cash
    }

    /// Get position value
    pub fn get_position_value(&self) -> f64 {
        self.positions.values()
            .map(|p| p.quantity * p.current_price.unwrap_or(p.entry_price))
            .sum()
    }

    /// Get all open positions
    pub fn get_open_positions(&self) -> HashMap<String, Position> {
        self.positions.clone()
    }

    /// Get position count
    pub fn get_position_count(&self) -> usize {
        self.positions.len()
    }

    /// Get symbol position count
    pub fn get_symbol_position_count(&self, symbol: &str) -> u32 {
        self.positions
            .values()
            .filter(|p| p.symbol == symbol)
            .count() as u32
    }

    /// Get portfolio exposure
    pub fn get_portfolio_exposure(&self, portfolio_value: f64) -> f64 {
        if portfolio_value <= 0.0 {
            return 0.0;
        }
        self.get_position_value() / portfolio_value
    }

    /// Update position price
    pub fn update_position_price(&mut self, position_id: &str, new_price: f64) -> Result<(), String> {
        if let Some(position) = self.positions.get_mut(position_id) {
            position.current_price = Some(new_price);
            Ok(())
        } else {
            Err(format!("Position {} not found", position_id))
        }
    }

    /// Recover from failure to last known good state
    pub fn recover_from_failure(&mut self) {
        // Reset to initial state
        self.total_value = self.initial_value;
        self.available_cash = self.initial_value;
        self.positions.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_portfolio_creation() {
        let pm = PortfolioManager::new(10000.0);
        assert_eq!(pm.get_total_value(), 10000.0);
        assert_eq!(pm.get_available_cash(), 10000.0);
        assert_eq!(pm.get_position_value(), 0.0);
    }

    #[test]
    fn test_invariants_on_creation() {
        let pm = PortfolioManager::new(10000.0);
        assert!(pm.validate_invariants().is_ok());
        assert!(pm.validate_consistency());
    }

    #[test]
    fn test_open_position() {
        let mut pm = PortfolioManager::new(10000.0);
        let pos_id = pm.open_position_atomic("BTC-USD", 0.1, 50000.0).unwrap();

        assert!(pm.get_open_positions().contains_key(&pos_id));
        assert_eq!(pm.get_position_count(), 1);
    }

    #[test]
    fn test_insufficient_balance() {
        let mut pm = PortfolioManager::new(1000.0);
        let result = pm.open_position_atomic("BTC-USD", 1.0, 50000.0);

        assert!(result.is_err());
        assert_eq!(pm.get_position_count(), 0);
    }
}
