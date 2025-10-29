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
    pub max_slippage_percent: f64, // Maximum slippage allowed on orders (e.g., 0.002 = 0.2%)

    // Symbol screening configuration
    pub screening_enabled: bool, // Enable symbol screening for scalping
    pub screening_interval_seconds: u64, // Interval between screening runs (seconds)
    pub screening_cache_ttl_seconds: u64, // Cache TTL for screening results (seconds)
    pub screening_score_threshold: f64, // Minimum overall score to consider (0.0-1.0)
    pub screening_volatility_weight: f64, // Weight for volatility in score aggregation
    pub screening_volume_weight: f64, // Weight for volume in score aggregation
    pub screening_spread_weight: f64, // Weight for spread in score aggregation
    pub screening_momentum_weight: f64, // Weight for momentum in score aggregation

    // Portfolio reconciliation configuration
    pub reconciliation_enabled: bool, // Enable portfolio reconciliation
    pub reconciliation_interval_seconds: u64, // How often to run reconciliation (seconds)
    pub reconciliation_threshold_percentage: f64, // Threshold for flagging discrepancies (percentage)
    pub reconciliation_timeout_milliseconds: u64, // API call timeout (milliseconds)
    pub reconciliation_max_retries: u32,          // Maximum number of retries on failure
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
            vec![
                "BTC".to_string(),
                "ETH".to_string(),
                "SOL".to_string(),
                "BNB".to_string(),
            ],
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

            // Symbol screening defaults
            screening_enabled: true,
            screening_interval_seconds: 60, // Screen every 60 seconds
            screening_cache_ttl_seconds: 300, // 5 minute cache TTL
            screening_score_threshold: 0.50, // Minimum 0.50 score
            screening_volatility_weight: 0.3, // 30% weight
            screening_volume_weight: 0.3,   // 30% weight
            screening_spread_weight: 0.2,   // 20% weight
            screening_momentum_weight: 0.2, // 20% weight

            // Portfolio reconciliation defaults
            reconciliation_enabled: true,
            reconciliation_interval_seconds: 300, // Every 5 minutes
            reconciliation_threshold_percentage: 0.01, // 1% threshold
            reconciliation_timeout_milliseconds: 10000, // 10 second timeout
            reconciliation_max_retries: 3,        // 3 retries
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
                        threshold,
                        e,
                        config.min_confidence_threshold
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

        // Symbol screening configuration from environment
        if let Ok(screening_enabled) = std::env::var("SCREENING_ENABLED") {
            config.screening_enabled =
                screening_enabled.to_lowercase() == "true" || screening_enabled == "1";
        }

        if let Ok(screening_interval) = std::env::var("SCREENING_INTERVAL_SECONDS") {
            if let Ok(value) = screening_interval.parse::<u64>() {
                if value >= 10 && value <= 3600 {
                    config.screening_interval_seconds = value;
                }
            }
        }

        if let Ok(screening_cache_ttl) = std::env::var("SCREENING_CACHE_TTL_SECONDS") {
            if let Ok(value) = screening_cache_ttl.parse::<u64>() {
                if value >= 60 && value <= 3600 {
                    config.screening_cache_ttl_seconds = value;
                }
            }
        }

        if let Ok(screening_threshold) = std::env::var("SCREENING_SCORE_THRESHOLD") {
            if let Ok(value) = screening_threshold.parse::<f64>() {
                if (0.0..=1.0).contains(&value) {
                    config.screening_score_threshold = value;
                }
            }
        }

        if let Ok(vol_weight) = std::env::var("SCREENING_VOLATILITY_WEIGHT") {
            if let Ok(value) = vol_weight.parse::<f64>() {
                if (0.0..=1.0).contains(&value) {
                    config.screening_volatility_weight = value;
                }
            }
        }

        if let Ok(vol_weight) = std::env::var("SCREENING_VOLUME_WEIGHT") {
            if let Ok(value) = vol_weight.parse::<f64>() {
                if (0.0..=1.0).contains(&value) {
                    config.screening_volume_weight = value;
                }
            }
        }

        if let Ok(spread_weight) = std::env::var("SCREENING_SPREAD_WEIGHT") {
            if let Ok(value) = spread_weight.parse::<f64>() {
                if (0.0..=1.0).contains(&value) {
                    config.screening_spread_weight = value;
                }
            }
        }

        if let Ok(momentum_weight) = std::env::var("SCREENING_MOMENTUM_WEIGHT") {
            if let Ok(value) = momentum_weight.parse::<f64>() {
                if (0.0..=1.0).contains(&value) {
                    config.screening_momentum_weight = value;
                }
            }
        }

        // Portfolio reconciliation configuration from environment
        if let Ok(reconciliation_enabled) = std::env::var("RECONCILIATION_ENABLED") {
            config.reconciliation_enabled =
                reconciliation_enabled.to_lowercase() == "true" || reconciliation_enabled == "1";
        }

        if let Ok(reconciliation_interval) = std::env::var("RECONCILIATION_INTERVAL_SECONDS") {
            if let Ok(value) = reconciliation_interval.parse::<u64>() {
                if value >= 60 && value <= 3600 {
                    config.reconciliation_interval_seconds = value;
                }
            }
        }

        if let Ok(reconciliation_threshold) = std::env::var("RECONCILIATION_THRESHOLD_PERCENTAGE") {
            if let Ok(value) = reconciliation_threshold.parse::<f64>() {
                if (0.0001..=0.1).contains(&value) {
                    config.reconciliation_threshold_percentage = value;
                }
            }
        }

        if let Ok(reconciliation_timeout) = std::env::var("RECONCILIATION_TIMEOUT_MILLISECONDS") {
            if let Ok(value) = reconciliation_timeout.parse::<u64>() {
                if value >= 1000 && value <= 60000 {
                    config.reconciliation_timeout_milliseconds = value;
                }
            }
        }

        if let Ok(reconciliation_max_retries) = std::env::var("RECONCILIATION_MAX_RETRIES") {
            if let Ok(value) = reconciliation_max_retries.parse::<u32>() {
                if value >= 0 && value <= 10 {
                    config.reconciliation_max_retries = value;
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
