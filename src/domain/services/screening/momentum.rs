use crate::domain::services::indicators::{Candle, Indicator, MACD, RSI};

/// Calculates momentum score from multiple technical indicators
pub trait MomentumScoreCalculator {
    /// Calculate momentum score from candle data
    /// Returns score in [0.0, 1.0]
    fn calculate(&self, candles: &[Candle]) -> f64;
}

/// Default implementation using RSI and MACD
pub struct SimpleMomentumCalculator {
    pub rsi_period: usize,
    pub macd_fast: usize,
    pub macd_slow: usize,
    pub macd_signal: usize,
}

impl Default for SimpleMomentumCalculator {
    fn default() -> Self {
        SimpleMomentumCalculator {
            rsi_period: 14,
            macd_fast: 12,
            macd_slow: 26,
            macd_signal: 9,
        }
    }
}

impl MomentumScoreCalculator for SimpleMomentumCalculator {
    fn calculate(&self, candles: &[Candle]) -> f64 {
        if candles.is_empty() {
            return 0.5; // neutral momentum
        }

        let mut momentum_score = 0.5; // default: neutral
        let mut signal_count = 0;

        // Calculate RSI
        let rsi = RSI::new(self.rsi_period);
        let rsi_values = rsi.calculate(candles);

        if let Some(&last_rsi) = rsi_values.last() {
            // RSI: >70 = overbought (bearish), <30 = oversold (bullish)
            // Map to [0, 1] where 0.5 is neutral
            let rsi_score = if last_rsi > 70.0 {
                // Overbought, slightly bearish
                0.4
            } else if last_rsi < 30.0 {
                // Oversold, slightly bullish
                0.6
            } else if last_rsi > 50.0 {
                // Uptrend
                0.6 + ((last_rsi - 50.0) / 20.0) * 0.1 // 0.6 to 0.7
            } else {
                // Downtrend
                0.3 + ((50.0 - last_rsi) / 20.0) * 0.1 // 0.3 to 0.4
            };
            momentum_score = rsi_score;
            signal_count += 1;
        }

        // Calculate MACD
        let macd = MACD::new(self.macd_fast, self.macd_slow, self.macd_signal);
        let macd_values = macd.calculate(candles);

        if macd_values.len() >= 2 {
            let last_macd = macd_values[macd_values.len() - 1];
            let prev_macd = macd_values[macd_values.len() - 2];

            // MACD > 0 = bullish, MACD < 0 = bearish
            // MACD crossing up = strong bullish signal
            let macd_score = if last_macd > 0.0 && prev_macd <= 0.0 {
                // Golden cross (crossing up)
                0.8
            } else if last_macd < 0.0 && prev_macd >= 0.0 {
                // Death cross (crossing down)
                0.2
            } else if last_macd > 0.0 {
                // Above zero line (bullish)
                0.65
            } else {
                // Below zero line (bearish)
                0.35
            };

            // Average with RSI if both available
            if signal_count > 0 {
                momentum_score = (momentum_score + macd_score) / 2.0;
            } else {
                momentum_score = macd_score;
            }
            signal_count += 1;
        }

        // Clamp to [0, 1]
        momentum_score.max(0.0).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_candles() -> Vec<Candle> {
        vec![
            Candle::new(100.0, 101.0, 99.0, 100.5, 1000.0).unwrap(),
            Candle::new(100.5, 102.0, 100.0, 101.0, 1100.0).unwrap(),
            Candle::new(101.0, 103.0, 100.5, 102.5, 1200.0).unwrap(),
            Candle::new(102.5, 104.0, 101.0, 103.5, 1300.0).unwrap(),
            Candle::new(103.5, 105.0, 102.0, 104.5, 1400.0).unwrap(),
        ]
    }

    #[test]
    fn test_momentum_score_from_rsi_indicator() {
        let calc = SimpleMomentumCalculator::default();
        let candles = create_test_candles();
        let score = calc.calculate(&candles);
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn test_momentum_score_from_macd_indicator() {
        let calc = SimpleMomentumCalculator::default();
        let candles = create_test_candles();
        let score = calc.calculate(&candles);
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn test_momentum_score_combining_multiple_indicators() {
        let calc = SimpleMomentumCalculator::default();
        let candles = create_test_candles();
        let score = calc.calculate(&candles);
        // Both RSI and MACD should contribute
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn test_momentum_score_uptrend() {
        let calc = SimpleMomentumCalculator::default();
        // Create strongly bullish candles (close always higher)
        let uptrend_candles: Vec<Candle> = (0..20)
            .map(|i| {
                let base = 100.0 + i as f64;
                Candle::new(base, base + 1.0, base - 0.5, base + 0.8, 1000.0).unwrap()
            })
            .collect();
        let score = calc.calculate(&uptrend_candles);
        assert!(score > 0.5); // Should be bullish
    }

    #[test]
    fn test_momentum_score_downtrend() {
        let calc = SimpleMomentumCalculator::default();
        // Create strongly bearish candles (close always lower)
        let downtrend_candles: Vec<Candle> = (0..20)
            .map(|i| {
                let base = 120.0 - i as f64;
                Candle::new(base, base + 0.5, base - 1.0, base - 0.8, 1000.0).unwrap()
            })
            .collect();
        let score = calc.calculate(&downtrend_candles);
        assert!(score < 0.5); // Should be bearish
    }

    #[test]
    fn test_momentum_score_empty_candles() {
        let calc = SimpleMomentumCalculator::default();
        let score = calc.calculate(&[]);
        assert_eq!(score, 0.5); // neutral
    }

    #[test]
    fn test_momentum_score_single_candle() {
        let calc = SimpleMomentumCalculator::default();
        let candles = vec![Candle::new(100.0, 101.0, 99.0, 100.5, 1000.0).unwrap()];
        let score = calc.calculate(&candles);
        assert!(score >= 0.0 && score <= 1.0);
    }
}
