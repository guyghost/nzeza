use crate::domain::errors::ValidationError;

/// Profit and Loss value object
///
/// Unlike Price, PnL can be negative to represent losses.
/// This type ensures PnL values are valid (finite) but allows negative values.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct PnL(f64);

impl PnL {
    /// Create a new PnL value
    ///
    /// # Arguments
    /// * `value` - The PnL amount (can be positive for profit or negative for loss)
    ///
    /// # Errors
    /// Returns ValidationError::MustBeFinite if the value is NaN or infinite
    pub fn new(value: f64) -> Result<Self, ValidationError> {
        if !value.is_finite() {
            return Err(ValidationError::MustBeFinite);
        }
        Ok(PnL(value))
    }

    /// Get the raw value
    pub fn value(&self) -> f64 {
        self.0
    }

    /// Check if this represents a profit (positive value)
    pub fn is_profit(&self) -> bool {
        self.0 > 0.0
    }

    /// Check if this represents a loss (negative value)
    pub fn is_loss(&self) -> bool {
        self.0 < 0.0
    }

    /// Get the absolute value
    pub fn abs(&self) -> f64 {
        self.0.abs()
    }

    /// Create a zero PnL (breakeven)
    pub fn zero() -> Self {
        PnL(0.0)
    }
}

impl std::fmt::Display for PnL {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 >= 0.0 {
            write!(f, "+${:.2}", self.0)
        } else {
            write!(f, "-${:.2}", self.0.abs())
        }
    }
}

impl std::ops::Add for PnL {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        // Safe: sum of finite numbers is finite
        PnL(self.0 + other.0)
    }
}

impl std::ops::Sub for PnL {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        // Safe: difference of finite numbers is finite
        PnL(self.0 - other.0)
    }
}

impl std::ops::Mul<f64> for PnL {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self {
        // Multiplication could produce infinity, so we need validation
        let result = self.0 * rhs;
        if result.is_finite() {
            PnL(result)
        } else {
            // Return max/min value instead of infinity
            if result.is_sign_positive() {
                PnL(f64::MAX)
            } else {
                PnL(f64::MIN)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pnl_profit() {
        let pnl = PnL::new(1000.0).unwrap();
        assert_eq!(pnl.value(), 1000.0);
        assert!(pnl.is_profit());
        assert!(!pnl.is_loss());
    }

    #[test]
    fn test_pnl_loss() {
        let pnl = PnL::new(-500.0).unwrap();
        assert_eq!(pnl.value(), -500.0);
        assert!(!pnl.is_profit());
        assert!(pnl.is_loss());
    }

    #[test]
    fn test_pnl_zero() {
        let pnl = PnL::zero();
        assert_eq!(pnl.value(), 0.0);
        assert!(!pnl.is_profit());
        assert!(!pnl.is_loss());
    }

    #[test]
    fn test_pnl_add() {
        let pnl1 = PnL::new(1000.0).unwrap();
        let pnl2 = PnL::new(-300.0).unwrap();
        let total = pnl1 + pnl2;
        assert_eq!(total.value(), 700.0);
    }

    #[test]
    fn test_pnl_invalid() {
        assert!(PnL::new(f64::NAN).is_err());
        assert!(PnL::new(f64::INFINITY).is_err());
        assert!(PnL::new(f64::NEG_INFINITY).is_err());
    }

    #[test]
    fn test_pnl_display() {
        let profit = PnL::new(1234.56).unwrap();
        assert_eq!(format!("{}", profit), "+$1234.56");

        let loss = PnL::new(-789.12).unwrap();
        assert_eq!(format!("{}", loss), "-$789.12");
    }
}
