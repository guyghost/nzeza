use crate::domain::value_objects::{price::Price, quantity::Quantity};
use crate::domain::errors::ValidationError;
use chrono::{DateTime, Utc};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum PositionSide {
    Long,
    Short,
}

impl std::fmt::Display for PositionSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PositionSide::Long => write!(f, "LONG"),
            PositionSide::Short => write!(f, "SHORT"),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Position {
    pub id: String,
    pub symbol: String,
    pub side: PositionSide,
    pub quantity: Quantity,
    pub entry_price: Price,
    pub entry_time: DateTime<Utc>,
    pub current_price: Option<Price>,
    pub stop_loss_price: Option<Price>,
    pub take_profit_price: Option<Price>,
}

impl Position {
    #[allow(dead_code)]
    pub fn new(
        id: String,
        symbol: String,
        side: PositionSide,
        quantity: Quantity,
        entry_price: Price,
    ) -> Self {
        Position {
            id,
            symbol,
            side,
            quantity,
            entry_price,
            entry_time: Utc::now(),
            current_price: None,
            stop_loss_price: None,
            take_profit_price: None,
        }
    }

    #[allow(dead_code)]
    pub fn new_with_stops(
        id: String,
        symbol: String,
        side: PositionSide,
        quantity: Quantity,
        entry_price: Price,
        stop_loss_percentage: Option<f64>,
        take_profit_percentage: Option<f64>,
    ) -> Result<Self, ValidationError> {
        let mut position = Self::new(id, symbol, side, quantity, entry_price);

        if let Some(sl_pct) = stop_loss_percentage {
            let sl_price = match position.side {
                PositionSide::Long => {
                    Price::new(position.entry_price.value() * (1.0 - sl_pct))?
                }
                PositionSide::Short => {
                    Price::new(position.entry_price.value() * (1.0 + sl_pct))?
                }
            };
            position.stop_loss_price = Some(sl_price);
        }

        if let Some(tp_pct) = take_profit_percentage {
            let tp_price = match position.side {
                PositionSide::Long => {
                    Price::new(position.entry_price.value() * (1.0 + tp_pct))?
                }
                PositionSide::Short => {
                    Price::new(position.entry_price.value() * (1.0 - tp_pct))?
                }
            };
            position.take_profit_price = Some(tp_price);
        }

        Ok(position)
    }

    #[allow(dead_code)]
    pub fn update_price(&mut self, price: Price) {
        self.current_price = Some(price);
    }

    #[allow(dead_code)]
    pub fn unrealized_pnl(&self) -> Option<Price> {
        self.current_price.and_then(|current_price| {
            let price_diff = match self.side {
                PositionSide::Long => current_price.value() - self.entry_price.value(),
                PositionSide::Short => self.entry_price.value() - current_price.value(),
            };
            Price::new(price_diff * self.quantity.value()).ok()
        })
    }

    #[allow(dead_code)]
    pub fn should_stop_loss(&self) -> bool {
        if let (Some(current_price), Some(stop_loss)) = (self.current_price, self.stop_loss_price) {
            match self.side {
                PositionSide::Long => current_price.value() <= stop_loss.value(),
                PositionSide::Short => current_price.value() >= stop_loss.value(),
            }
        } else {
            false
        }
    }

    #[allow(dead_code)]
    pub fn should_take_profit(&self) -> bool {
        if let (Some(current_price), Some(take_profit)) =
            (self.current_price, self.take_profit_price)
        {
            match self.side {
                PositionSide::Long => current_price.value() >= take_profit.value(),
                PositionSide::Short => current_price.value() <= take_profit.value(),
            }
        } else {
            false
        }
    }

    #[allow(dead_code)]
    pub fn set_stop_loss_percentage(&mut self, percentage: f64) -> Result<(), ValidationError> {
        let sl_price = match self.side {
            PositionSide::Long => {
                Price::new(self.entry_price.value() * (1.0 - percentage))?
            }
            PositionSide::Short => {
                Price::new(self.entry_price.value() * (1.0 + percentage))?
            }
        };
        self.stop_loss_price = Some(sl_price);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn set_take_profit_percentage(&mut self, percentage: f64) -> Result<(), ValidationError> {
        let tp_price = match self.side {
            PositionSide::Long => {
                Price::new(self.entry_price.value() * (1.0 + percentage))?
            }
            PositionSide::Short => {
                Price::new(self.entry_price.value() * (1.0 - percentage))?
            }
        };
        self.take_profit_price = Some(tp_price);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_new() {
        let quantity = Quantity::new(1.0).unwrap();
        let entry_price = Price::new(50000.0).unwrap();
        let position = Position::new(
            "pos_1".to_string(),
            "BTC-USD".to_string(),
            PositionSide::Long,
            quantity,
            entry_price,
        );

        assert_eq!(position.id, "pos_1");
        assert_eq!(position.symbol, "BTC-USD");
        assert!(matches!(position.side, PositionSide::Long));
        assert_eq!(position.quantity.value(), 1.0);
        assert_eq!(position.entry_price.value(), 50000.0);
        assert!(position.current_price.is_none());
        assert!(position.stop_loss_price.is_none());
        assert!(position.take_profit_price.is_none());
    }

    #[test]
    fn test_position_update_price() {
        let quantity = Quantity::new(1.0).unwrap();
        let entry_price = Price::new(50000.0).unwrap();
        let mut position = Position::new(
            "pos_1".to_string(),
            "BTC-USD".to_string(),
            PositionSide::Long,
            quantity,
            entry_price,
        );

        let current_price = Price::new(55000.0).unwrap();
        position.update_price(current_price.clone());

        assert_eq!(position.current_price.unwrap().value(), 55000.0);
    }

    #[test]
    fn test_position_unrealized_pnl_long() {
        let quantity = Quantity::new(1.0).unwrap();
        let entry_price = Price::new(50000.0).unwrap();
        let mut position = Position::new(
            "pos_1".to_string(),
            "BTC-USD".to_string(),
            PositionSide::Long,
            quantity,
            entry_price,
        );

        let current_price = Price::new(55000.0).unwrap();
        position.update_price(current_price);

        let pnl = position.unrealized_pnl();
        assert!(pnl.is_some());
        assert_eq!(pnl.unwrap().value(), 5000.0); // 1.0 * (55000 - 50000)
    }

    #[test]
    fn test_position_unrealized_pnl_short() {
        let quantity = Quantity::new(1.0).unwrap();
        let entry_price = Price::new(50000.0).unwrap();
        let mut position = Position::new(
            "pos_1".to_string(),
            "BTC-USD".to_string(),
            PositionSide::Short,
            quantity,
            entry_price,
        );

        let current_price = Price::new(45000.0).unwrap();
        position.update_price(current_price);

        let pnl = position.unrealized_pnl();
        assert!(pnl.is_some());
        assert_eq!(pnl.unwrap().value(), 5000.0); // 1.0 * (50000 - 45000)
    }

    #[test]
    fn test_position_with_stops() {
        let quantity = Quantity::new(1.0).unwrap();
        let entry_price = Price::new(50000.0).unwrap();
        let position = Position::new_with_stops(
            "pos_1".to_string(),
            "BTC-USD".to_string(),
            PositionSide::Long,
            quantity,
            entry_price,
            Some(0.05), // 5% stop loss
            Some(0.10), // 10% take profit
        )
        .unwrap();

        let sl_price = position.stop_loss_price.unwrap().value();
        let tp_price = position.take_profit_price.unwrap().value();

        assert!((sl_price - 47500.0).abs() < 0.001); // 50000 * (1 - 0.05)
        assert!((tp_price - 55000.0).abs() < 0.001); // 50000 * (1 + 0.10)
    }

    #[test]
    fn test_position_stop_loss_long() {
        let quantity = Quantity::new(1.0).unwrap();
        let entry_price = Price::new(50000.0).unwrap();
        let mut position = Position::new_with_stops(
            "pos_1".to_string(),
            "BTC-USD".to_string(),
            PositionSide::Long,
            quantity,
            entry_price,
            Some(0.05), // 5% stop loss
            None,
        )
        .unwrap();

        // Price above stop loss - should not trigger
        position.update_price(Price::new(48000.0).unwrap());
        assert!(!position.should_stop_loss());

        // Price at stop loss - should trigger
        position.update_price(Price::new(47500.0).unwrap());
        assert!(position.should_stop_loss());
    }

    #[test]
    fn test_position_take_profit_short() {
        let quantity = Quantity::new(1.0).unwrap();
        let entry_price = Price::new(50000.0).unwrap();
        let mut position = Position::new_with_stops(
            "pos_1".to_string(),
            "BTC-USD".to_string(),
            PositionSide::Short,
            quantity,
            entry_price,
            None,
            Some(0.10), // 10% take profit
        )
        .unwrap();

        // Price above take profit - should not trigger
        position.update_price(Price::new(46000.0).unwrap());
        assert!(!position.should_take_profit());

        // Price at take profit - should trigger
        position.update_price(Price::new(45000.0).unwrap());
        assert!(position.should_take_profit());
    }
}
