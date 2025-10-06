#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Exchange {
    Dydx,
    Hyperliquid,
    Coinbase,
    Binance,
    Kraken,
}

impl Exchange {
    pub fn name(&self) -> &str {
        match self {
            Exchange::Dydx => "dydx",
            Exchange::Hyperliquid => "hyperliquid",
            Exchange::Coinbase => "coinbase",
            Exchange::Binance => "binance",
            Exchange::Kraken => "kraken",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exchange_name_dydx() {
        assert_eq!(Exchange::Dydx.name(), "dydx");
    }

    #[test]
    fn test_exchange_name_binance() {
        assert_eq!(Exchange::Binance.name(), "binance");
    }

    #[test]
    fn test_exchange_name_kraken() {
        assert_eq!(Exchange::Kraken.name(), "kraken");
    }

    #[test]
    fn test_exchange_equality() {
        assert_eq!(Exchange::Binance, Exchange::Binance);
        assert_ne!(Exchange::Binance, Exchange::Dydx);
    }
}