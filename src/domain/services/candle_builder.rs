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
    /// Maximum number of price updates to buffer per symbol
    max_price_updates: usize,
    /// Price updates buffer per symbol
    price_updates: HashMap<String, VecDeque<PriceUpdate>>,
    /// Completed candles per symbol
    candles: HashMap<String, VecDeque<Candle>>,
}

impl CandleBuilder {
    pub fn new(window_duration: Duration, max_history: usize) -> Self {
        // Price updates buffer should be large enough to build multiple candles
        // but not unbounded. Default to 10x the window (e.g., 10 minutes of data for 1min candles)
        let max_price_updates = max_history * 60; // Reasonable upper bound

        Self {
            window_duration,
            max_history,
            max_price_updates,
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

        let updates = self
            .price_updates
            .entry(symbol.clone())
            .or_insert_with(VecDeque::new);

        updates.push_back(update);

        // Trim price updates buffer to prevent unbounded growth
        while updates.len() > self.max_price_updates {
            updates.pop_front();
        }

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

    /// Clean up old price updates that are beyond the retention period
    ///
    /// Call this periodically to free memory from stale price updates
    pub fn cleanup_old_updates(&mut self, max_age: Duration) {
        let now = SystemTime::now();

        self.price_updates.retain(|_, updates| {
            // Remove updates older than max_age
            updates.retain(|update| {
                now.duration_since(update.timestamp)
                    .map(|age| age <= max_age)
                    .unwrap_or(false)
            });

            // Keep the symbol if it still has updates
            !updates.is_empty()
        });
    }

    /// Remove inactive symbols (no candles and no updates)
    ///
    /// Useful for freeing memory when symbols are no longer being tracked
    pub fn remove_inactive_symbols(&mut self) {
        let active_symbols: std::collections::HashSet<String> = self
            .price_updates
            .keys()
            .chain(self.candles.keys())
            .cloned()
            .collect();

        self.price_updates
            .retain(|symbol, _| active_symbols.contains(symbol));
        self.candles
            .retain(|symbol, candles| !candles.is_empty() && active_symbols.contains(symbol));
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

    #[test]
    fn test_price_updates_bounded() {
        let mut builder = CandleBuilder::new(Duration::from_secs(60), 10);
        // max_price_updates should be 10 * 60 = 600

        // Add more updates than the limit
        for i in 0..1000 {
            let price = Price::new(50000.0 + i as f64).unwrap();
            builder.add_price("BTC-USD".to_string(), price);
        }

        // Should not exceed max_price_updates
        let updates = builder.price_updates.get("BTC-USD").unwrap();
        assert!(updates.len() <= 600);
    }

    #[test]
    fn test_cleanup_old_updates() {
        use std::thread::sleep;
        use std::time::Duration as StdDuration;

        let mut builder = CandleBuilder::new(Duration::from_secs(60), 100);

        // Add some updates
        for i in 0..5 {
            let price = Price::new(50000.0 + i as f64).unwrap();
            builder.add_price("BTC-USD".to_string(), price);
        }

        // Wait a bit
        sleep(StdDuration::from_millis(100));

        // Cleanup updates older than 50ms (should remove all)
        builder.cleanup_old_updates(Duration::from_millis(50));

        // All old updates should be removed
        assert!(builder
            .price_updates
            .get("BTC-USD")
            .map(|u| u.is_empty())
            .unwrap_or(true));
    }

    #[test]
    fn test_remove_inactive_symbols() {
        let mut builder = CandleBuilder::new(Duration::from_secs(60), 100);

        // Add data for multiple symbols
        builder.add_price("BTC-USD".to_string(), Price::new(50000.0).unwrap());
        builder.add_price("ETH-USD".to_string(), Price::new(3000.0).unwrap());

        // Clear one symbol
        builder.clear_symbol("BTC-USD");

        // Remove inactive symbols
        builder.remove_inactive_symbols();

        // BTC-USD should be removed, ETH-USD should remain
        assert!(builder.price_updates.contains_key("ETH-USD"));
        assert!(!builder.price_updates.contains_key("BTC-USD"));
    }
}
