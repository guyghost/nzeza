use crate::domain::errors::ValidationError;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Quantity(f64);

impl Quantity {
    #[allow(dead_code)]
    pub fn new(value: f64) -> Result<Self, ValidationError> {
        if !value.is_finite() {
            return Err(ValidationError::MustBeFinite);
        }
        if value < 0.0 {
            return Err(ValidationError::MustBeNonNegative);
        }
        Ok(Quantity(value))
    }

    #[allow(dead_code)]
    pub fn value(&self) -> f64 {
        self.0
    }

    #[allow(dead_code)]
    pub fn add(&self, other: Quantity) -> Result<Quantity, ValidationError> {
        Quantity::new(self.0 + other.0)
    }

    #[allow(dead_code)]
    pub fn subtract(&self, other: Quantity) -> Result<Quantity, ValidationError> {
        let result = self.0 - other.0;
        Quantity::new(result)
    }

    #[allow(dead_code)]
    pub fn multiply(&self, factor: f64) -> Result<Quantity, ValidationError> {
        if !factor.is_finite() {
            return Err(ValidationError::MustBeFinite);
        }
        if factor < 0.0 {
            return Err(ValidationError::MustBeNonNegative);
        }
        Quantity::new(self.0 * factor)
    }
}

impl std::fmt::Display for Quantity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantity_new_valid() {
        let qty = Quantity::new(100.0);
        assert!(qty.is_ok());
        assert_eq!(qty.unwrap().value(), 100.0);
    }

    #[test]
    fn test_quantity_new_negative() {
        let qty = Quantity::new(-5.0);
        assert!(qty.is_err());
        assert!(matches!(
            qty.unwrap_err(),
            ValidationError::MustBeNonNegative
        ));
    }

    #[test]
    fn test_quantity_new_zero() {
        let qty = Quantity::new(0.0);
        assert!(qty.is_ok());
        assert_eq!(qty.unwrap().value(), 0.0);
    }

    #[test]
    fn test_quantity_add() {
        let q1 = Quantity::new(10.0).unwrap();
        let q2 = Quantity::new(5.0).unwrap();
        let result = q1.add(q2).unwrap();
        assert_eq!(result.value(), 15.0);
    }

    #[test]
    fn test_quantity_subtract_valid() {
        let q1 = Quantity::new(10.0).unwrap();
        let q2 = Quantity::new(3.0).unwrap();
        let result = q1.subtract(q2);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), 7.0);
    }

    #[test]
    fn test_quantity_subtract_insufficient() {
        let q1 = Quantity::new(5.0).unwrap();
        let q2 = Quantity::new(10.0).unwrap();
        let result = q1.subtract(q2);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::MustBeNonNegative
        ));
    }

    #[test]
    fn test_quantity_multiply() {
        let qty = Quantity::new(10.0).unwrap();
        let result = qty.multiply(2.5).unwrap();
        assert_eq!(result.value(), 25.0);
    }

    #[test]
    fn test_quantity_multiply_nan() {
        let qty = Quantity::new(10.0).unwrap();
        let result = qty.multiply(f64::NAN);
        assert!(result.is_err());
    }
}
