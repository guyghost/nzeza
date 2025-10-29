//! Balance entity - represents account balance state

use std::time::SystemTime;

/// Account balance information from an exchange
#[derive(Debug, Clone, PartialEq)]
pub struct BalanceInfo {
    /// Total account balance
    pub total_balance: f64,
    /// Available balance for trading (not locked in positions)
    pub available_balance: f64,
    /// Balance locked in open positions or pending operations
    pub locked_balance: f64,
    /// When this balance was fetched
    pub timestamp: SystemTime,
}

impl BalanceInfo {
    /// Create a new BalanceInfo with validation
    ///
    /// # Arguments
    /// * `total_balance` - Total account balance (>= 0)
    /// * `available_balance` - Available balance (>= 0, <= total)
    /// * `locked_balance` - Locked balance (>= 0, <= total)
    ///
    /// # Returns
    /// Ok(BalanceInfo) if invariants are satisfied, Err(String) otherwise
    pub fn new(
        total_balance: f64,
        available_balance: f64,
        locked_balance: f64,
    ) -> Result<Self, String> {
        // Validate invariants
        if total_balance < 0.0 {
            return Err("total_balance must be non-negative".to_string());
        }
        if available_balance < 0.0 {
            return Err("available_balance must be non-negative".to_string());
        }
        if locked_balance < 0.0 {
            return Err("locked_balance must be non-negative".to_string());
        }

        // Check: total = available + locked
        let sum = available_balance + locked_balance;
        if (sum - total_balance).abs() > 1e-9 {
            return Err(format!(
                "Invariant violated: total_balance ({}) != available ({}) + locked ({})",
                total_balance, available_balance, locked_balance
            ));
        }

        Ok(Self {
            total_balance,
            available_balance,
            locked_balance,
            timestamp: SystemTime::now(),
        })
    }

    /// Check if this balance information is fresh (within TTL)
    pub fn is_fresh(&self, ttl: std::time::Duration) -> Result<bool, String> {
        let elapsed = self
            .timestamp
            .elapsed()
            .map_err(|e| format!("Failed to calculate elapsed time: {}", e))?;
        Ok(elapsed <= ttl)
    }

    /// Validate this balance info (check invariants)
    pub fn validate(&self) -> Result<(), String> {
        if self.total_balance < 0.0 {
            return Err("total_balance must be non-negative".to_string());
        }
        if self.available_balance < 0.0 {
            return Err("available_balance must be non-negative".to_string());
        }
        if self.locked_balance < 0.0 {
            return Err("locked_balance must be non-negative".to_string());
        }

        let sum = self.available_balance + self.locked_balance;
        if (sum - self.total_balance).abs() > 1e-9 {
            return Err(format!(
                "Invariant violated: total_balance ({}) != available ({}) + locked ({})",
                self.total_balance, self.available_balance, self.locked_balance
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balance_info_valid_creation() {
        let result = BalanceInfo::new(1000.0, 800.0, 200.0);
        assert!(result.is_ok());
        let balance = result.unwrap();
        assert_eq!(balance.total_balance, 1000.0);
        assert_eq!(balance.available_balance, 800.0);
        assert_eq!(balance.locked_balance, 200.0);
    }

    #[test]
    fn test_balance_info_negative_total() {
        let result = BalanceInfo::new(-100.0, 50.0, 50.0);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "total_balance must be non-negative"
        );
    }

    #[test]
    fn test_balance_info_negative_available() {
        let result = BalanceInfo::new(100.0, -50.0, 150.0);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "available_balance must be non-negative"
        );
    }

    #[test]
    fn test_balance_info_negative_locked() {
        let result = BalanceInfo::new(100.0, 100.0, -100.0);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "locked_balance must be non-negative"
        );
    }

    #[test]
    fn test_balance_info_invariant_violation() {
        // total (1000) != available (600) + locked (300)
        let result = BalanceInfo::new(1000.0, 600.0, 300.0);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Invariant violated"));
    }

    #[test]
    fn test_balance_info_zero_locked() {
        let result = BalanceInfo::new(1000.0, 1000.0, 0.0);
        assert!(result.is_ok());
        let balance = result.unwrap();
        assert_eq!(balance.locked_balance, 0.0);
    }

    #[test]
    fn test_balance_info_zero_available() {
        let result = BalanceInfo::new(1000.0, 0.0, 1000.0);
        assert!(result.is_ok());
        let balance = result.unwrap();
        assert_eq!(balance.available_balance, 0.0);
    }

    #[test]
    fn test_balance_info_is_fresh() {
        let balance = BalanceInfo::new(1000.0, 500.0, 500.0).unwrap();
        let ttl = std::time::Duration::from_secs(60);
        let is_fresh = balance.is_fresh(ttl).unwrap();
        assert!(is_fresh);
    }

    #[test]
    fn test_balance_info_validate() {
        let balance = BalanceInfo::new(1000.0, 700.0, 300.0).unwrap();
        let result = balance.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_balance_info_clone() {
        let balance1 = BalanceInfo::new(1000.0, 600.0, 400.0).unwrap();
        let balance2 = balance1.clone();
        assert_eq!(balance1, balance2);
    }
}
