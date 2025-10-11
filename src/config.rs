use crate::domain::entities::exchange::Exchange;
use std::collections::HashMap;

/// Configuration for symbols to track on each exchange
#[derive(Clone)]
pub struct TradingConfig {
    pub symbols: HashMap<Exchange, Vec<String>>,
    pub min_confidence_threshold: f64,
    pub max_positions_per_symbol: usize,
    pub max_total_positions: usize,
    pub default_position_size: f64,
    pub enable_automated_trading: bool,
    pub stop_loss_percentage: Option<f64>,
    pub take_profit_percentage: Option<f64>,
    pub portfolio_percentage_per_position: f64, // Pourcentage du portefeuille par position
    pub max_trades_per_hour: usize,             // Limite de trades par heure
    pub max_trades_per_day: usize,              // Limite de trades par jour
    pub max_slippage_percent: f64,              // Maximum slippage allowed on orders (e.g., 0.002 = 0.2%)
}

impl TradingConfig {
    /// Default configuration with common trading pairs
    pub fn default() -> TradingConfig {
        let mut symbols = HashMap::new();

        // Binance symbols (uppercase, no separator)
        symbols.insert(
            Exchange::Binance,
            vec![
                "BTCUSDT".to_string(),
                "ETHUSDT".to_string(),
                "SOLUSDT".to_string(),
                "BNBUSDT".to_string(),
            ],
        );

        // Coinbase symbols (uppercase with dash)
        symbols.insert(
            Exchange::Coinbase,
            vec![
                "BTC-USD".to_string(),
                "ETH-USD".to_string(),
                "SOL-USD".to_string(),
                "BN-USD".to_string(),
            ],
        );

        // dYdX symbols (uppercase with dash)
        symbols.insert(
            Exchange::Dydx,
            vec![
                "BTC-USD".to_string(),
                "ETH-USD".to_string(),
                "SOL-USD".to_string(),
                "BN-USD".to_string(),
            ],
        );

        // Hyperliquid symbols (just the asset name)
        symbols.insert(
            Exchange::Hyperliquid,
            vec!["BTC".to_string(), "ETH".to_string(), "SOL".to_string(), "BNB".to_string()],
        );

        // Kraken symbols (with slash)
        symbols.insert(
            Exchange::Kraken,
            vec![
                "BTC/USD".to_string(),
                "ETH/USD".to_string(),
                "SOL/USD".to_string(),
                "BNB/USD".to_string(),
            ],
        );

        TradingConfig {
            symbols,
            min_confidence_threshold: 0.7,
            max_positions_per_symbol: 1,
            max_total_positions: 5,
            default_position_size: 0.001,
            enable_automated_trading: true,
            stop_loss_percentage: Some(0.05),        // 5% stop loss
            take_profit_percentage: Some(0.10),      // 10% take profit
            portfolio_percentage_per_position: 0.02, // 2% du portefeuille par position
            max_trades_per_hour: 10,                 // 10 trades par heure max
            max_trades_per_day: 50,                  // 50 trades par jour max
            max_slippage_percent: 0.002,             // 0.2% maximum slippage protection
        }
    }

    /// Load configuration from environment variables
    pub fn from_env() -> TradingConfig {
        let mut config = TradingConfig::default();

        if let Ok(threshold) = std::env::var("MIN_CONFIDENCE_THRESHOLD") {
            match threshold.parse::<f64>() {
                Ok(value) if (0.0..=1.0).contains(&value) => {
                    config.min_confidence_threshold = value;
                }
                Ok(value) => {
                    tracing::warn!(
                        "Invalid MIN_CONFIDENCE_THRESHOLD value: {} (must be between 0.0 and 1.0), using default: {}",
                        value, config.min_confidence_threshold
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to parse MIN_CONFIDENCE_THRESHOLD '{}': {}, using default: {}",
                        threshold, e, config.min_confidence_threshold
                    );
                }
            }
        }

        if let Ok(max_pos) = std::env::var("MAX_POSITIONS_PER_SYMBOL") {
            if let Ok(value) = max_pos.parse::<usize>() {
                config.max_positions_per_symbol = value;
            }
        }

        if let Ok(max_total) = std::env::var("MAX_TOTAL_POSITIONS") {
            if let Ok(value) = max_total.parse::<usize>() {
                config.max_total_positions = value;
            }
        }

        if let Ok(size) = std::env::var("DEFAULT_POSITION_SIZE") {
            if let Ok(value) = size.parse::<f64>() {
                if value > 0.0 {
                    config.default_position_size = value;
                }
            }
        }

        if let Ok(enabled) = std::env::var("ENABLE_AUTOMATED_TRADING") {
            config.enable_automated_trading = enabled.to_lowercase() == "true" || enabled == "1";
        }

        if let Ok(sl) = std::env::var("STOP_LOSS_PERCENTAGE") {
            if let Ok(value) = sl.parse::<f64>() {
                if value > 0.0 && value < 1.0 {
                    config.stop_loss_percentage = Some(value);
                }
            }
        }

        if let Ok(tp) = std::env::var("TAKE_PROFIT_PERCENTAGE") {
            if let Ok(value) = tp.parse::<f64>() {
                if value > 0.0 {
                    config.take_profit_percentage = Some(value);
                }
            }
        }

        if let Ok(portfolio_pct) = std::env::var("PORTFOLIO_PERCENTAGE_PER_POSITION") {
            if let Ok(value) = portfolio_pct.parse::<f64>() {
                if (0.001..=0.1).contains(&value) {
                    // Entre 0.1% et 10%
                    config.portfolio_percentage_per_position = value;
                }
            }
        }

        if let Ok(max_hourly) = std::env::var("MAX_TRADES_PER_HOUR") {
            if let Ok(value) = max_hourly.parse::<usize>() {
                if value > 0 && value <= 100 {
                    config.max_trades_per_hour = value;
                }
            }
        }

        if let Ok(max_daily) = std::env::var("MAX_TRADES_PER_DAY") {
            if let Ok(value) = max_daily.parse::<usize>() {
                if value > 0 && value <= 500 {
                    config.max_trades_per_day = value;
                }
            }
        }

        if let Ok(slippage) = std::env::var("MAX_SLIPPAGE_PERCENT") {
            if let Ok(value) = slippage.parse::<f64>() {
                if (0.0001..=0.05).contains(&value) {
                    // Entre 0.01% et 5%
                    config.max_slippage_percent = value;
                }
            }
        }

        config
    }

    /// Get all unique normalized symbols (BTC-USD format)

    pub fn get_normalized_symbols(&self) -> Vec<String> {
        let mut normalized = std::collections::HashSet::new();

        for symbols in self.symbols.values() {
            for symbol in symbols {
                let norm = Self::normalize_symbol(symbol);
                normalized.insert(norm);
            }
        }

        normalized.into_iter().collect()
    }

    /// Normalize symbol to standard format (BTC-USD)

    pub fn normalize_symbol(symbol: &str) -> String {
        // Handle different symbol formats to standardize them
        let normalized = if symbol.contains("USDT") {
            symbol.replace("USDT", "-USD")
        } else if symbol.contains("/") {
            symbol.replace("/", "-")
        } else if symbol == "BTC" {
            "BTC-USD".to_string()
        } else if symbol == "ETH" {
            "ETH-USD".to_string()
        } else {
            symbol.to_string()
        };

        normalized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TradingConfig::default();
        assert!(config.symbols.contains_key(&Exchange::Binance));
        assert!(config.symbols.contains_key(&Exchange::Coinbase));
        assert!(config.symbols.contains_key(&Exchange::Dydx));
        assert_eq!(config.symbols.get(&Exchange::Binance).unwrap().len(), 3);
        assert_eq!(config.symbols.get(&Exchange::Coinbase).unwrap().len(), 3);
        assert_eq!(config.symbols.get(&Exchange::Dydx).unwrap().len(), 3);
    }

    #[test]
    fn test_normalize_symbol() {
        assert_eq!(TradingConfig::normalize_symbol("BTCUSDT"), "BTC-USD");
        assert_eq!(TradingConfig::normalize_symbol("BTC-USD"), "BTC-USD");
        assert_eq!(TradingConfig::normalize_symbol("BTC/USD"), "BTC-USD");
        assert_eq!(TradingConfig::normalize_symbol("BTC"), "BTC-USD");
        assert_eq!(TradingConfig::normalize_symbol("ETHUSDT"), "ETH-USD");
        assert_eq!(TradingConfig::normalize_symbol("ETH-USD"), "ETH-USD");
        assert_eq!(TradingConfig::normalize_symbol("ETH/USD"), "ETH-USD");
        assert_eq!(TradingConfig::normalize_symbol("ETH"), "ETH-USD");
    }

    #[test]
    fn test_get_normalized_symbols() {
        let config = TradingConfig::default();
        let symbols = config.get_normalized_symbols();
        assert!(symbols.contains(&"BTC-USD".to_string()));
        assert!(symbols.contains(&"ETH-USD".to_string()));
    }
}
