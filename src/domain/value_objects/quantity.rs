#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Quantity(pub f64);

#[allow(dead_code)]
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

    pub fn add(&self, other: Quantity) -> Quantity {
        Quantity(self.0 + other.0)
    }

    pub fn subtract(&self, other: Quantity) -> Result<Quantity, String> {
        if self.0 >= other.0 {
            Ok(Quantity(self.0 - other.0))
        } else {
            Err("Cannot subtract more than available".to_string())
        }
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
        let result = q1.add(q2);
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
        assert_eq!(result.unwrap_err(), "Cannot subtract more than available");
    }
}