use crate::domain::value_objects::{price::Price, quantity::Quantity};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum OrderSide {
    Buy,
    Sell,
}

impl std::fmt::Display for OrderSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderSide::Buy => write!(f, "BUY"),
            OrderSide::Sell => write!(f, "SELL"),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum OrderType {
    Market,
    Limit,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Order {
    pub id: String,
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub price: Option<Price>,
    pub quantity: Quantity,
}

impl Order {
    #[allow(dead_code)]
    pub fn new(
        id: String,
        symbol: String,
        side: OrderSide,
        order_type: OrderType,
        price: Option<f64>,
        quantity: f64,
    ) -> Result<Self, String> {
        let price = if let Some(p) = price {
            Some(Price::new(p)?)
        } else {
            None
        };
        let quantity = Quantity::new(quantity)?;

        // Validation: limit orders must have price
        if matches!(order_type, OrderType::Limit) && price.is_none() {
            return Err("Limit orders must have a price".to_string());
        }

        Ok(Order {
            id,
            symbol,
            side,
            order_type,
            price,
            quantity,
        })
    }

    #[allow(dead_code)]
    pub fn total_value(&self) -> Option<Price> {
        self.price.and_then(|p| p.multiply(self.quantity.value()).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_new_market_buy() {
        let order = Order::new(
            "123".to_string(),
            "BTCUSD".to_string(),
            OrderSide::Buy,
            OrderType::Market,
            None,
            1.0,
        );
        assert!(order.is_ok());
        let o = order.unwrap();
        assert_eq!(o.id, "123");
        assert_eq!(o.symbol, "BTCUSD");
        assert!(matches!(o.side, OrderSide::Buy));
        assert!(matches!(o.order_type, OrderType::Market));
        assert!(o.price.is_none());
        assert_eq!(o.quantity.value(), 1.0);
    }

    #[test]
    fn test_order_new_limit_sell() {
        let order = Order::new(
            "456".to_string(),
            "ETHUSD".to_string(),
            OrderSide::Sell,
            OrderType::Limit,
            Some(2000.0),
            0.5,
        );
        assert!(order.is_ok());
        let o = order.unwrap();
        assert_eq!(o.price.unwrap().value(), 2000.0);
        assert_eq!(o.quantity.value(), 0.5);
    }

    #[test]
    fn test_order_new_limit_without_price() {
        let order = Order::new(
            "789".to_string(),
            "BTCUSD".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            None,
            1.0,
        );
        assert!(order.is_err());
        assert_eq!(order.unwrap_err(), "Limit orders must have a price");
    }

    #[test]
    fn test_order_new_negative_quantity() {
        let order = Order::new(
            "101".to_string(),
            "BTCUSD".to_string(),
            OrderSide::Buy,
            OrderType::Market,
            None,
            -1.0,
        );
        assert!(order.is_err());
        assert_eq!(order.unwrap_err(), "Quantity must be non-negative");
    }

    #[test]
    fn test_order_total_value_limit() {
        let order = Order::new(
            "202".to_string(),
            "BTCUSD".to_string(),
            OrderSide::Buy,
            OrderType::Limit,
            Some(50000.0),
            0.1,
        ).unwrap();
        let total = order.total_value();
        assert!(total.is_some());
        assert_eq!(total.unwrap().value(), 5000.0);
    }

    #[test]
    fn test_order_total_value_market() {
        let order = Order::new(
            "303".to_string(),
            "BTCUSD".to_string(),
            OrderSide::Buy,
            OrderType::Market,
            None,
            0.1,
        ).unwrap();
        let total = order.total_value();
        assert!(total.is_none());
    }
}