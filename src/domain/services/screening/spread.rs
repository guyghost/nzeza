/// Calculates spread score from bid-ask data
pub trait SpreadScoreCalculator {
    /// Calculate spread score from bid and ask prices
    /// Returns score in [0.0, 1.0] where higher = tighter spread = better
    fn calculate(&self, bid: f64, ask: f64) -> f64;
}

/// Default implementation using spread-relative scoring
pub struct SimpleSpreadCalculator {
    /// Normalization factor - spread considered "high"
    pub max_spread_percent: f64,
}

impl Default for SimpleSpreadCalculator {
    fn default() -> Self {
        SimpleSpreadCalculator {
            max_spread_percent: 0.5, // 0.5% spread is considered "wide"
        }
    }
}

impl SpreadScoreCalculator for SimpleSpreadCalculator {
    fn calculate(&self, bid: f64, ask: f64) -> f64 {
        if bid <= 0.0 || ask < bid {
            return 0.0;
        }

        let spread = ask - bid;
        let mid_price = (bid + ask) / 2.0;
        let spread_percent = (spread / mid_price) * 100.0;

        // Invert: tighter spreads = higher scores
        // If spread_percent = 0, score = 1.0
        // If spread_percent >= max_spread_percent, score = 0.0
        (1.0 - (spread_percent / self.max_spread_percent)).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spread_score_from_bid_ask_pairs() {
        let calc = SimpleSpreadCalculator::default();
        let score = calc.calculate(100.0, 100.5);
        assert!(score > 0.0 && score < 1.0);
    }

    #[test]
    fn test_spread_score_normalization_to_unit_range() {
        let calc = SimpleSpreadCalculator::default();

        let no_spread = calc.calculate(100.0, 100.0);
        let tight_spread = calc.calculate(100.0, 100.2);
        let wide_spread = calc.calculate(100.0, 100.5);
        let very_wide = calc.calculate(100.0, 110.0);

        assert_eq!(no_spread, 1.0);
        assert!(tight_spread > wide_spread);
        assert!(wide_spread > 0.0 || wide_spread == 0.0);
        assert_eq!(very_wide, 0.0); // capped at 0.0
    }

    #[test]
    fn test_spread_score_zero_spread_edge_case() {
        let calc = SimpleSpreadCalculator::default();
        let score = calc.calculate(100.0, 100.0);
        assert_eq!(score, 1.0);
    }

    #[test]
    fn test_spread_score_very_wide_spread() {
        let calc = SimpleSpreadCalculator::default();
        let score = calc.calculate(100.0, 110.0);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_spread_score_typical_spreads() {
        let calc = SimpleSpreadCalculator::default();

        // 0.1% spread (tight)
        let tight = calc.calculate(100.0, 100.1);
        // 0.3% spread (typical)
        let typical = calc.calculate(100.0, 100.3);
        // 0.5% spread (wide)
        let wide = calc.calculate(100.0, 100.5);

        assert!(tight > typical);
        assert!(typical > wide);
        assert!(wide >= 0.0);
    }

    #[test]
    fn test_spread_score_invalid_prices() {
        let calc = SimpleSpreadCalculator::default();

        // bid > ask
        let inverted = calc.calculate(100.5, 100.0);
        assert_eq!(inverted, 0.0);

        // negative bid
        let negative_bid = calc.calculate(-100.0, 100.0);
        assert_eq!(negative_bid, 0.0);
    }
}
