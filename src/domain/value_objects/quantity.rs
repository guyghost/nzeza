#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Quantity(f64);

impl Quantity {
    pub fn new(value: f64) -> Result<Self, String> {
        if value >= 0.0 {
            Ok(Quantity(value))
        } else {
            Err("Quantity must be non-negative".to_string())
        }
    }

    pub fn value(&self) -> f64 {
        self.0
    }

    pub fn add(&self, other: Quantity) -> Result<Quantity, String> {
        Quantity::new(self.0 + other.0)
    }

    pub fn subtract(&self, other: Quantity) -> Result<Quantity, String> {
        let result = self.0 - other.0;
        Quantity::new(result)
    }

    pub fn multiply(&self, factor: f64) -> Result<Quantity, String> {
        if !factor.is_finite() {
            return Err("Factor must be finite".to_string());
        }
        Quantity::new(self.0 * factor)
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
        assert_eq!(qty.unwrap_err(), "Quantity must be non-negative");
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
        assert_eq!(result.unwrap_err(), "Quantity must be non-negative");
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