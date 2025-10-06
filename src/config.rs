use crate::domain::entities::exchange::Exchange;
use std::collections::HashMap;

/// Configuration for symbols to track on each exchange
pub struct TradingConfig {
    pub symbols: HashMap<Exchange, Vec<String>>,
}

impl TradingConfig {
    /// Default configuration with common trading pairs
    pub fn default() -> Self {
        let mut symbols = HashMap::new();

        // Binance symbols (uppercase, no separator)
        symbols.insert(
            Exchange::Binance,
            vec![
                "BTCUSDT".to_string(),
                "ETHUSDT".to_string(),
                "SOLUSDT".to_string(),
            ],
        );

        // Coinbase symbols (uppercase with dash)
        symbols.insert(
            Exchange::Coinbase,
            vec![
                "BTC-USD".to_string(),
                "ETH-USD".to_string(),
                "SOL-USD".to_string(),
            ],
        );

        // dYdX symbols (uppercase with dash)
        symbols.insert(
            Exchange::Dydx,
            vec![
                "BTC-USD".to_string(),
                "ETH-USD".to_string(),
                "SOL-USD".to_string(),
            ],
        );

        // Hyperliquid symbols (just the asset name)
        symbols.insert(
            Exchange::Hyperliquid,
            vec![
                "BTC".to_string(),
                "ETH".to_string(),
                "SOL".to_string(),
            ],
        );

        // Kraken symbols (with slash)
        symbols.insert(
            Exchange::Kraken,
            vec![
                "BTC/USD".to_string(),
                "ETH/USD".to_string(),
                "SOL/USD".to_string(),
            ],
        );

        TradingConfig { symbols }
    }

    /// Get all unique normalized symbols (BTC-USD format)
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
