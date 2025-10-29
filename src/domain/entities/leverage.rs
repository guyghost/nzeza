//! Leverage entity - represents account leverage state

use std::time::SystemTime;

/// Leverage information for an account
#[derive(Debug, Clone, PartialEq)]
pub struct LeverageInfo {
    /// Maximum allowed leverage (e.g., 20x for dYdX)
    pub max_leverage: f64,
    /// Current leverage being used
    pub current_leverage: f64,
    /// Available leverage (max - current)
    pub available_leverage: f64,
    /// Current margin ratio (equity / required margin)
    pub margin_ratio: f64,
    /// Minimum required margin ratio to avoid liquidation
    pub maintenance_margin_ratio: f64,
    /// When this leverage info was calculated
    pub timestamp: SystemTime,
}

impl LeverageInfo {
    /// Create a new LeverageInfo with validation
    ///
    /// # Arguments
    /// * `max_leverage` - Maximum allowed leverage (>= 1.0)
    /// * `current_leverage` - Current leverage used (>= 0.0, <= max)
    /// * `margin_ratio` - Current margin ratio (> 0)
    /// * `maintenance_margin_ratio` - Minimum margin ratio (> 0, < margin_ratio)
    ///
    /// # Returns
    /// Ok(LeverageInfo) if invariants are satisfied, Err(String) otherwise
    pub fn new(
        max_leverage: f64,
        current_leverage: f64,
        margin_ratio: f64,
        maintenance_margin_ratio: f64,
    ) -> Result<Self, String> {
        // Validate max_leverage
        if max_leverage < 1.0 {
            return Err("max_leverage must be >= 1.0".to_string());
        }

        // Validate current_leverage
        if current_leverage < 0.0 {
            return Err("current_leverage must be non-negative".to_string());
        }
        if current_leverage > max_leverage {
            return Err(format!(
                "current_leverage ({}) cannot exceed max_leverage ({})",
                current_leverage, max_leverage
            ));
        }

        // Validate margin_ratio
        if margin_ratio <= 0.0 {
            return Err("margin_ratio must be positive".to_string());
        }

        // Validate maintenance_margin_ratio
        if maintenance_margin_ratio <= 0.0 {
            return Err("maintenance_margin_ratio must be positive".to_string());
        }
        if maintenance_margin_ratio >= margin_ratio {
            return Err(format!(
                "maintenance_margin_ratio ({}) must be less than margin_ratio ({})",
                maintenance_margin_ratio, margin_ratio
            ));
        }

        let available_leverage = max_leverage - current_leverage;

        Ok(Self {
            max_leverage,
            current_leverage,
            available_leverage,
            margin_ratio,
            maintenance_margin_ratio,
            timestamp: SystemTime::now(),
        })
    }

    /// Check if this leverage info is fresh (within TTL)
    pub fn is_fresh(&self, ttl: std::time::Duration) -> Result<bool, String> {
        let elapsed = self
            .timestamp
            .elapsed()
            .map_err(|e| format!("Failed to calculate elapsed time: {}", e))?;
        Ok(elapsed <= ttl)
    }

    /// Check if leverage is healthy (margin_ratio > maintenance_margin_ratio)
    pub fn is_healthy(&self) -> bool {
        self.margin_ratio > self.maintenance_margin_ratio
    }

    /// Get the margin safety factor (how far from liquidation)
    /// Returns ratio of margin_ratio / maintenance_margin_ratio
    pub fn margin_safety_factor(&self) -> f64 {
        if self.maintenance_margin_ratio == 0.0 {
            f64::INFINITY
        } else {
            self.margin_ratio / self.maintenance_margin_ratio
        }
    }

    /// Validate this leverage info (check invariants)
    pub fn validate(&self) -> Result<(), String> {
        if self.max_leverage < 1.0 {
            return Err("max_leverage must be >= 1.0".to_string());
        }
        if self.current_leverage < 0.0 {
            return Err("current_leverage must be non-negative".to_string());
        }
        if self.current_leverage > self.max_leverage {
            return Err("current_leverage cannot exceed max_leverage".to_string());
        }
        if self.margin_ratio <= 0.0 {
            return Err("margin_ratio must be positive".to_string());
        }
        if self.maintenance_margin_ratio <= 0.0 {
            return Err("maintenance_margin_ratio must be positive".to_string());
        }
        if self.maintenance_margin_ratio >= self.margin_ratio {
            return Err("maintenance_margin_ratio must be less than margin_ratio".to_string());
        }

        // Verify available_leverage calculation
        let expected_available = self.max_leverage - self.current_leverage;
        if (expected_available - self.available_leverage).abs() > 1e-9 {
            return Err(format!(
                "Invariant violated: available_leverage ({}) != max ({}) - current ({})",
                self.available_leverage, self.max_leverage, self.current_leverage
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leverage_info_valid_creation() {
        let result = LeverageInfo::new(20.0, 5.0, 10.0, 3.0);
        assert!(result.is_ok());
        let leverage = result.unwrap();
        assert_eq!(leverage.max_leverage, 20.0);
        assert_eq!(leverage.current_leverage, 5.0);
        assert_eq!(leverage.available_leverage, 15.0);
        assert_eq!(leverage.margin_ratio, 10.0);
        assert_eq!(leverage.maintenance_margin_ratio, 3.0);
    }

    #[test]
    fn test_leverage_info_max_leverage_too_low() {
        let result = LeverageInfo::new(0.5, 0.0, 10.0, 3.0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "max_leverage must be >= 1.0");
    }

    #[test]
    fn test_leverage_info_current_negative() {
        let result = LeverageInfo::new(20.0, -1.0, 10.0, 3.0);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "current_leverage must be non-negative"
        );
    }

    #[test]
    fn test_leverage_info_current_exceeds_max() {
        let result = LeverageInfo::new(20.0, 25.0, 10.0, 3.0);
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("current_leverage") && err_msg.contains("max_leverage"));
    }

    #[test]
    fn test_leverage_info_margin_ratio_not_positive() {
        let result = LeverageInfo::new(20.0, 5.0, 0.0, 3.0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "margin_ratio must be positive");
    }

    #[test]
    fn test_leverage_info_maintenance_margin_negative() {
        let result = LeverageInfo::new(20.0, 5.0, 10.0, -1.0);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "maintenance_margin_ratio must be positive"
        );
    }

    #[test]
    fn test_leverage_info_maintenance_exceeds_margin() {
        let result = LeverageInfo::new(20.0, 5.0, 10.0, 15.0);
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("maintenance_margin_ratio") && err_msg.contains("margin_ratio"));
    }

    #[test]
    fn test_leverage_info_is_healthy() {
        let leverage = LeverageInfo::new(20.0, 5.0, 10.0, 3.0).unwrap();
        assert!(leverage.is_healthy());
    }

    #[test]
    fn test_leverage_info_is_not_healthy() {
        let leverage = LeverageInfo::new(20.0, 5.0, 3.0, 10.0);
        // This should fail in new() due to invariant check, but if we somehow got here:
        // The test should verify that margin_ratio < maintenance_margin_ratio indicates unhealthy state
        assert!(leverage.is_err()); // Construction fails first
    }

    #[test]
    fn test_leverage_info_margin_safety_factor() {
        let leverage = LeverageInfo::new(20.0, 5.0, 10.0, 2.0).unwrap();
        let safety = leverage.margin_safety_factor();
        assert!((safety - 5.0).abs() < 1e-9); // 10.0 / 2.0 = 5.0
    }

    #[test]
    fn test_leverage_info_is_fresh() {
        let leverage = LeverageInfo::new(20.0, 5.0, 10.0, 3.0).unwrap();
        let ttl = std::time::Duration::from_secs(60);
        let is_fresh = leverage.is_fresh(ttl).unwrap();
        assert!(is_fresh);
    }

    #[test]
    fn test_leverage_info_validate() {
        let leverage = LeverageInfo::new(20.0, 5.0, 10.0, 3.0).unwrap();
        let result = leverage.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_leverage_info_clone() {
        let leverage1 = LeverageInfo::new(20.0, 5.0, 10.0, 3.0).unwrap();
        let leverage2 = leverage1.clone();
        assert_eq!(leverage1, leverage2);
    }

    #[test]
    fn test_leverage_info_zero_current_leverage() {
        let result = LeverageInfo::new(20.0, 0.0, 10.0, 3.0);
        assert!(result.is_ok());
        let leverage = result.unwrap();
        assert_eq!(leverage.current_leverage, 0.0);
        assert_eq!(leverage.available_leverage, 20.0);
    }

    #[test]
    fn test_leverage_info_max_current_leverage() {
        let result = LeverageInfo::new(20.0, 20.0, 10.0, 3.0);
        assert!(result.is_ok());
        let leverage = result.unwrap();
        assert_eq!(leverage.current_leverage, 20.0);
        assert_eq!(leverage.available_leverage, 0.0);
    }
}
