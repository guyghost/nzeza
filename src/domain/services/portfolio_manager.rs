//! PortfolioManager - ACID-compliant portfolio state management

use std::time::SystemTime;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Portfolio transaction record
#[derive(Debug, Clone)]
pub struct PortfolioTransaction {
    pub id: String,
    pub action: PortfolioAction,
    pub timestamp: SystemTime,
    pub status: TransactionStatus,
}

/// Portfolio action type
#[derive(Debug, Clone)]
pub enum PortfolioAction {
    OpenPosition {
        symbol: String,
        quantity: f64,
        price: f64,
    },
    ClosePosition {
        position_id: String,
        pnl: f64,
    },
    UpdatePrice {
        position_id: String,
        price: f64,
    },
}

/// Transaction status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionStatus {
    Pending,
    Committed,
    RolledBack,
}

/// Position data
#[derive(Debug, Clone)]
pub struct Position {
    pub id: String,
    pub symbol: String,
    pub quantity: f64,
    pub entry_price: f64,
    pub current_price: Option<f64>,
}

/// Portfolio snapshot for recovery
#[derive(Debug, Clone)]
struct PortfolioSnapshot {
    total_value: f64,
    available_cash: f64,
    position_value: f64,
    positions: HashMap<String, Position>,
}

/// ACID-compliant portfolio manager
pub struct PortfolioManager {
    total_value: f64,
    available_cash: f64,
    position_value: f64,
    open_positions: HashMap<String, Position>,
    position_locks: Arc<Mutex<HashMap<String, SystemTime>>>,
    transaction_log: Vec<PortfolioTransaction>,
    snapshots: Vec<PortfolioSnapshot>,
}

impl PortfolioManager {
    /// Create a new portfolio manager
    pub fn new(initial_value: f64) -> Self {
        Self {
            total_value: initial_value,
            available_cash: initial_value,
            position_value: 0.0,
            open_positions: HashMap::new(),
            position_locks: Arc::new(Mutex::new(HashMap::new())),
            transaction_log: Vec::new(),
            snapshots: vec![PortfolioSnapshot {
                total_value: initial_value,
                available_cash: initial_value,
                position_value: 0.0,
                positions: HashMap::new(),
            }],
        }
    }

    /// Open a position atomically
    pub fn open_position_atomic(
        &mut self,
        symbol: &str,
        quantity: f64,
        entry_price: f64,
    ) -> Result<String, String> {
        // Take snapshot before operation
        let snapshot = self.take_snapshot();

        // Calculate position value
        let position_value = quantity * entry_price;

        // Validate invariants before operation
        self.validate_invariants()?;

        // Check balance
        if position_value > self.available_cash {
            return Err(format!(
                "Insufficient balance: required {:.2}, available {:.2}",
                position_value, self.available_cash
            ));
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

        // Reserve cash and update state
        self.available_cash -= position_value;
        self.position_value += position_value;
        self.open_positions.insert(position_id.clone(), position);

        // Validate invariants after operation
        if let Err(e) = self.validate_invariants() {
            // Rollback on invariant violation
            self.restore_snapshot(snapshot);
            return Err(format!("Operation failed invariant check: {}", e));
        }

        // Record transaction
        self.record_transaction(
            PortfolioAction::OpenPosition {
                symbol: symbol.to_string(),
                quantity,
                price: entry_price,
            },
            TransactionStatus::Committed,
        );

        Ok(position_id)
    }

    /// Close a position atomically
    pub fn close_position_atomic(&mut self, position_id: &str) -> Result<f64, String> {
        // Take snapshot before operation
        let snapshot = self.take_snapshot();

        // Find position
        let position = self
            .open_positions
            .get(position_id)
            .ok_or_else(|| format!("Position {} not found", position_id))?
            .clone();

        // Calculate PnL
        let entry_value = position.quantity * position.entry_price;
        let current_price = position.current_price.unwrap_or(position.entry_price);
        let current_value = position.quantity * current_price;
        let pnl = current_value - entry_value;

        // Release cash and update state
        self.available_cash += entry_value + pnl;
        self.position_value -= entry_value;
        self.total_value += pnl;
        self.open_positions.remove(position_id);

        // Validate invariants after operation
        if let Err(e) = self.validate_invariants() {
            // Rollback on invariant violation
            self.restore_snapshot(snapshot);
            return Err(format!("Operation failed invariant check: {}", e));
        }

        // Record transaction
        self.record_transaction(
            PortfolioAction::ClosePosition {
                position_id: position_id.to_string(),
                pnl,
            },
            TransactionStatus::Committed,
        );

        Ok(pnl)
    }

    /// Validate all portfolio invariants
    pub fn validate_invariants(&self) -> Result<(), String> {
        // Invariant 1: total_value >= 0
        if self.total_value < 0.0 {
            return Err(format!("Invariant 1 violated: total_value {} < 0", self.total_value));
        }

        // Invariant 2: available_cash >= 0
        if self.available_cash < 0.0 {
            return Err(format!(
                "Invariant 2 violated: available_cash {} < 0",
                self.available_cash
            ));
        }

        // Invariant 3: position_value >= 0
        if self.position_value < 0.0 {
            return Err(format!(
                "Invariant 3 violated: position_value {} < 0",
                self.position_value
            ));
        }

        // Invariant 4: total_value == available_cash + position_value
        let expected_total = self.available_cash + self.position_value;
        if (self.total_value - expected_total).abs() > 0.01 {
            return Err(format!(
                "Invariant 4 violated: total_value {} != available_cash {} + position_value {}",
                self.total_value, self.available_cash, self.position_value
            ));
        }

        // Invariant 5: No negative values and all finite
        if !self.total_value.is_finite()
            || !self.available_cash.is_finite()
            || !self.position_value.is_finite()
        {
            return Err("Invariant 5 violated: Non-finite values detected".to_string());
        }

        Ok(())
    }

    /// Quick consistency check
    pub fn validate_consistency(&self) -> bool {
        let expected_total = self.available_cash + self.position_value;
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
        self.position_value
    }

    /// Get all open positions
    pub fn get_open_positions(&self) -> HashMap<String, Position> {
        self.open_positions.clone()
    }

    /// Get position count
    pub fn get_position_count(&self) -> usize {
        self.open_positions.len()
    }

    /// Get symbol position count
    pub fn get_symbol_position_count(&self, symbol: &str) -> u32 {
        self.open_positions
            .values()
            .filter(|p| p.symbol == symbol)
            .count() as u32
    }

    /// Get portfolio exposure
    pub fn get_portfolio_exposure(&self, portfolio_value: f64) -> f64 {
        if portfolio_value <= 0.0 {
            return 0.0;
        }
        self.position_value / portfolio_value
    }

    /// Update position price
    pub fn update_position_price(&mut self, position_id: &str, new_price: f64) -> Result<(), String> {
        if let Some(position) = self.open_positions.get_mut(position_id) {
            position.current_price = Some(new_price);
            Ok(())
        } else {
            Err(format!("Position {} not found", position_id))
        }
    }

    /// Recover from failure to last known good state
    pub fn recover_from_failure(&mut self) {
        if let Some(snapshot) = self.snapshots.last() {
            self.restore_snapshot(snapshot.clone());
        }
    }

    // ========== Private Helper Methods ==========

    /// Take a snapshot of current state
    fn take_snapshot(&self) -> PortfolioSnapshot {
        PortfolioSnapshot {
            total_value: self.total_value,
            available_cash: self.available_cash,
            position_value: self.position_value,
            positions: self.open_positions.clone(),
        }
    }

    /// Restore from snapshot
    fn restore_snapshot(&mut self, snapshot: PortfolioSnapshot) {
        self.total_value = snapshot.total_value;
        self.available_cash = snapshot.available_cash;
        self.position_value = snapshot.position_value;
        self.open_positions = snapshot.positions;
    }

    /// Record transaction in log
    fn record_transaction(&mut self, action: PortfolioAction, status: TransactionStatus) {
        let transaction = PortfolioTransaction {
            id: format!(
                "txn_{}",
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0)
            ),
            action,
            timestamp: SystemTime::now(),
            status,
        };

        self.transaction_log.push(transaction);

        // Store snapshot after each committed transaction
        if status == TransactionStatus::Committed {
            self.snapshots.push(self.take_snapshot());
        }
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
