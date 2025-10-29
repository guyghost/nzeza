/// Calculates volatility score based on price range in candles
pub trait VolatilityScoreCalculator {
    /// Calculate volatility score from candle data
    /// Returns score in [0.0, 1.0]
    fn calculate(&self, high: f64, low: f64, close: f64) -> f64;
}

/// Default implementation using simple price range normalization
pub struct SimpleVolatilityCalculator {
    /// Normalization factor - spreads mapping (0, 100%_range] to [0, 1]
    pub max_volatility_range_percent: f64,
}

impl Default for SimpleVolatilityCalculator {
    fn default() -> Self {
        SimpleVolatilityCalculator {
            max_volatility_range_percent: 5.0, // 5% is considered "high volatility"
        }
    }
}

impl VolatilityScoreCalculator for SimpleVolatilityCalculator {
    fn calculate(&self, high: f64, low: f64, close: f64) -> f64 {
        if close <= 0.0 || low < 0.0 || high < low {
            return 0.0;
        }

        let range = high - low;
        let range_percent = (range / close) * 100.0;

        // Normalize range_percent to [0, 1]
        // If range_percent > max_volatility_range_percent, score = 1.0
        (range_percent / self.max_volatility_range_percent).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volatility_score_zero_for_flat_candles() {
        let calc = SimpleVolatilityCalculator::default();
        let score = calc.calculate(100.0, 100.0, 100.0);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_volatility_score_increases_with_price_range() {
        let calc = SimpleVolatilityCalculator::default();
        let score1 = calc.calculate(101.0, 99.0, 100.0);
        let score2 = calc.calculate(110.0, 90.0, 100.0);
        assert!(score2 > score1);
        assert!(score1 > 0.0);
    }

    #[test]
    fn test_volatility_score_normalization_to_unit_range() {
        let calc = SimpleVolatilityCalculator::default();

        let flat = calc.calculate(100.0, 100.0, 100.0);
        let low_vol = calc.calculate(101.0, 99.0, 100.0);
        let high_vol = calc.calculate(110.0, 90.0, 100.0);
        let extreme_vol = calc.calculate(200.0, 50.0, 100.0);

        assert_eq!(flat, 0.0);
        assert!(flat < low_vol);
        assert!(low_vol < high_vol);
        assert_eq!(extreme_vol, 1.0); // capped at 1.0
    }

    #[test]
    fn test_volatility_score_edge_case_single_candle() {
        let calc = SimpleVolatilityCalculator::default();
        let score = calc.calculate(100.0, 100.0, 100.0);
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn test_volatility_score_high_volatility() {
        let calc = SimpleVolatilityCalculator::default();
        // 150% range should be capped at 1.0
        let score = calc.calculate(250.0, 100.0, 200.0);
        assert_eq!(score, 1.0);
    }

    #[test]
    fn test_volatility_score_typical_market() {
        let calc = SimpleVolatilityCalculator::default();
        // Typical 2% range
        let score = calc.calculate(102.0, 100.0, 101.0);
        assert!((score - 0.4).abs() < 0.01); // 2% / 5% = 0.4
    }
}
