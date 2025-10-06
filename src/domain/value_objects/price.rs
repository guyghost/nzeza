#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Price(f64);

impl Price {
    pub fn new(value: f64) -> Result<Self, String> {
        if value >= 0.0 {
            Ok(Price(value))
        } else {
            Err("Price must be non-negative".to_string())
        }
    }

    pub fn value(&self) -> f64 {
        self.0
    }

    pub fn add(&self, other: Price) -> Result<Price, String> {
        Price::new(self.0 + other.0)
    }

    pub fn multiply(&self, factor: f64) -> Result<Price, String> {
        if !factor.is_finite() {
            return Err("Factor must be finite".to_string());
        }
        Price::new(self.0 * factor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_new_valid() {
        let price = Price::new(100.0);
        assert!(price.is_ok());
        assert_eq!(price.unwrap().value(), 100.0);
    }

    #[test]
    fn test_price_new_negative() {
        let price = Price::new(-10.0);
        assert!(price.is_err());
        assert_eq!(price.unwrap_err(), "Price must be non-negative");
    }

    #[test]
    fn test_price_new_zero() {
        let price = Price::new(0.0);
        assert!(price.is_ok());
        assert_eq!(price.unwrap().value(), 0.0);
    }

    #[test]
    fn test_price_add() {
        let p1 = Price::new(50.0).unwrap();
        let p2 = Price::new(25.0).unwrap();
        let result = p1.add(p2).unwrap();
        assert_eq!(result.value(), 75.0);
    }

    #[test]
    fn test_price_multiply() {
        let price = Price::new(10.0).unwrap();
        let result = price.multiply(2.5).unwrap();
        assert_eq!(result.value(), 25.0);
    }

    #[test]
    fn test_price_multiply_negative_factor() {
        let price = Price::new(10.0).unwrap();
        let result = price.multiply(-2.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_price_multiply_nan() {
        let price = Price::new(10.0).unwrap();
        let result = price.multiply(f64::NAN);
        assert!(result.is_err());
    }
}