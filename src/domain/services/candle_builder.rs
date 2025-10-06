use crate::domain::services::indicators::Candle;
use crate::domain::value_objects::price::Price;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, SystemTime};

/// Price update with timestamp
#[derive(Debug, Clone)]
pub struct PriceUpdate {
    pub price: Price,
    pub timestamp: SystemTime,
}

/// Builds candles from price stream
pub struct CandleBuilder {
    /// Window duration for each candle (default: 1 minute)
    pub window_duration: Duration,
    /// Maximum number of candles to keep in history
    pub max_history: usize,
    /// Price updates buffer per symbol
    price_updates: HashMap<String, VecDeque<PriceUpdate>>,
    /// Completed candles per symbol
    candles: HashMap<String, VecDeque<Candle>>,
}

impl CandleBuilder {
    pub fn new(window_duration: Duration, max_history: usize) -> Self {
        Self {
            window_duration,
            max_history,
            price_updates: HashMap::new(),
            candles: HashMap::new(),
        }
    }

    /// Add a price update for a symbol
    pub fn add_price(&mut self, symbol: String, price: Price) {
        let update = PriceUpdate {
            price,
            timestamp: SystemTime::now(),
        };

        self.price_updates
            .entry(symbol.clone())
            .or_insert_with(VecDeque::new)
            .push_back(update);

        // Try to build a candle if we have enough data
        self.try_build_candle(&symbol);
    }

    /// Try to build a candle from accumulated price updates
    fn try_build_candle(&mut self, symbol: &str) {
        if let Some(updates) = self.price_updates.get_mut(symbol) {
            if updates.is_empty() {
                return;
            }

            let now = SystemTime::now();
            let first_timestamp = updates.front().unwrap().timestamp;

            // Check if enough time has passed for a complete candle
            if let Ok(elapsed) = now.duration_since(first_timestamp) {
                if elapsed >= self.window_duration {
                    // Collect all updates within the window
                    let window_end = first_timestamp + self.window_duration;
                    let mut window_updates = Vec::new();

                    while let Some(update) = updates.front() {
                        if update.timestamp <= window_end {
                            window_updates.push(updates.pop_front().unwrap());
                        } else {
                            break;
                        }
                    }

                    if !window_updates.is_empty() {
                        if let Some(candle) = Self::build_candle_from_updates(&window_updates) {
                            self.candles
                                .entry(symbol.to_string())
                                .or_insert_with(VecDeque::new)
                                .push_back(candle);

                            // Trim to max history
                            if let Some(candle_history) = self.candles.get_mut(symbol) {
                                while candle_history.len() > self.max_history {
                                    candle_history.pop_front();
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Build a candle from price updates
    fn build_candle_from_updates(updates: &[PriceUpdate]) -> Option<Candle> {
        if updates.is_empty() {
            return None;
        }

        let open = updates.first()?.price.value();
        let close = updates.last()?.price.value();
        let high = updates
            .iter()
            .map(|u| u.price.value())
            .fold(f64::NEG_INFINITY, f64::max);
        let low = updates
            .iter()
            .map(|u| u.price.value())
            .fold(f64::INFINITY, f64::min);

        // Volume is approximated as the number of updates (in a real system, this would be actual volume)
        let volume = updates.len() as f64;

        Candle::new(open, high, low, close, volume).ok()
    }

    /// Get candles for a symbol
    pub fn get_candles(&self, symbol: &str) -> Vec<Candle> {
        self.candles
            .get(symbol)
            .map(|deque| deque.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get number of candles for a symbol

    pub fn candle_count(&self, symbol: &str) -> usize {
        self.candles
            .get(symbol)
            .map(|deque| deque.len())
            .unwrap_or(0)
    }

    /// Clear all data for a symbol

    pub fn clear_symbol(&mut self, symbol: &str) {
        self.price_updates.remove(symbol);
        self.candles.remove(symbol);
    }

    /// Get all tracked symbols

    pub fn get_symbols(&self) -> Vec<String> {
        self.candles.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_candle_builder_new() {
        let builder = CandleBuilder::new(Duration::from_secs(60), 100);
        assert_eq!(builder.window_duration, Duration::from_secs(60));
        assert_eq!(builder.max_history, 100);
    }

    #[test]
    fn test_add_price() {
        let mut builder = CandleBuilder::new(Duration::from_secs(1), 100);
        let price = Price::new(50000.0).unwrap();
        builder.add_price("BTC-USD".to_string(), price);

        // Updates should be stored
        assert!(builder.price_updates.contains_key("BTC-USD"));
    }

    #[test]
    fn test_build_candle_from_updates() {
        let updates = vec![
            PriceUpdate {
                price: Price::new(100.0).unwrap(),
                timestamp: SystemTime::now(),
            },
            PriceUpdate {
                price: Price::new(105.0).unwrap(),
                timestamp: SystemTime::now(),
            },
            PriceUpdate {
                price: Price::new(95.0).unwrap(),
                timestamp: SystemTime::now(),
            },
            PriceUpdate {
                price: Price::new(102.0).unwrap(),
                timestamp: SystemTime::now(),
            },
        ];

        let candle = CandleBuilder::build_candle_from_updates(&updates).unwrap();
        assert_eq!(candle.open.value(), 100.0);
        assert_eq!(candle.close.value(), 102.0);
        assert_eq!(candle.high.value(), 105.0);
        assert_eq!(candle.low.value(), 95.0);
        assert_eq!(candle.volume, 4.0);
    }

    #[test]
    fn test_get_candles() {
        let builder = CandleBuilder::new(Duration::from_secs(60), 100);
        let candles = builder.get_candles("BTC-USD");
        assert!(candles.is_empty());
    }
}
