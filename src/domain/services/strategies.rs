use crate::domain::services::indicators::{Indicator, EMA, RSI, BollingerBands, MACD, StochasticOscillator, VWAP, Candle};

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum Signal {
    Buy,
    Sell,
    Hold,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TradingSignal {
    pub signal: Signal,
    pub confidence: f64, // 0.0 to 1.0
}

#[allow(dead_code)]
pub trait Strategy {
    fn generate_signal(&self, candles: &[Candle]) -> Option<TradingSignal>;
}

#[allow(dead_code)]
pub struct FastScalping {
    pub ema_short: EMA,
    pub ema_long: EMA,
}

#[allow(dead_code)]
impl FastScalping {
    pub fn new() -> Self {
        FastScalping {
            ema_short: EMA::new(5),
            ema_long: EMA::new(10),
        }
    }
}

impl Strategy for FastScalping {
    fn generate_signal(&self, candles: &[Candle]) -> Option<TradingSignal> {
        if candles.len() < 10 {
            return None;
        }
        let short_ema = self.ema_short.calculate(candles);
        let long_ema = self.ema_long.calculate(candles);

        let last_short = *short_ema.last()?;
        let last_long = *long_ema.last()?;

        if last_short > last_long {
            Some(TradingSignal { signal: Signal::Buy, confidence: 0.8 })
        } else if last_short < last_long {
            Some(TradingSignal { signal: Signal::Sell, confidence: 0.8 })
        } else {
            Some(TradingSignal { signal: Signal::Hold, confidence: 0.5 })
        }
    }
}

#[allow(dead_code)]
pub struct MomentumScalping {
    pub rsi: RSI,
    pub macd: MACD,
}

#[allow(dead_code)]
impl MomentumScalping {
    pub fn new() -> Self {
        MomentumScalping {
            rsi: RSI::new(14),
            macd: MACD::new(12, 26, 9),
        }
    }
}

impl Strategy for MomentumScalping {
    fn generate_signal(&self, candles: &[Candle]) -> Option<TradingSignal> {
        if candles.len() < 26 {
            return None;
        }
        let rsi_values = self.rsi.calculate(candles);
        let macd_values = self.macd.calculate(candles);

        let last_rsi = *rsi_values.last()?;
        let last_macd = *macd_values.last()?;

        let mut confidence = 0.5;

        if last_rsi < 30.0 && last_macd > 0.0 {
            confidence = 0.9;
            Some(TradingSignal { signal: Signal::Buy, confidence })
        } else if last_rsi > 70.0 && last_macd < 0.0 {
            confidence = 0.9;
            Some(TradingSignal { signal: Signal::Sell, confidence })
        } else {
            Some(TradingSignal { signal: Signal::Hold, confidence })
        }
    }
}

#[allow(dead_code)]
pub struct ConservativeScalping {
    pub bollinger: BollingerBands,
    pub stoch: StochasticOscillator,
    pub vwap: VWAP,
}

#[allow(dead_code)]
impl ConservativeScalping {
    pub fn new() -> Self {
        ConservativeScalping {
            bollinger: BollingerBands::new(20, 2.0),
            stoch: StochasticOscillator::new(14, 3),
            vwap: VWAP,
        }
    }
}

impl Strategy for ConservativeScalping {
    fn generate_signal(&self, candles: &[Candle]) -> Option<TradingSignal> {
        if candles.len() < 20 {
            return None;
        }
        let bb_values = self.bollinger.calculate_detailed(candles);
        let stoch_values = self.stoch.calculate(candles);
        let vwap_values = self.vwap.calculate(candles);

        let last_close = candles.last()?.close.value();
        let last_bb_upper = *bb_values.upper.last()?;
        let last_bb_lower = *bb_values.lower.last()?;
        let last_stoch = *stoch_values.last()?;
        let last_vwap = *vwap_values.last()?;

        let mut confidence = 0.6;

        if last_close < last_bb_lower && last_stoch < 20.0 && last_close < last_vwap {
            confidence = 0.7;
            Some(TradingSignal { signal: Signal::Buy, confidence })
        } else if last_close > last_bb_upper && last_stoch > 80.0 && last_close > last_vwap {
            confidence = 0.7;
            Some(TradingSignal { signal: Signal::Sell, confidence })
        } else {
            Some(TradingSignal { signal: Signal::Hold, confidence })
        }
    }
}

#[allow(dead_code)]
pub struct SignalCombiner {
    pub strategies: Vec<Box<dyn Strategy + Send + Sync>>,
    pub weights: Vec<f64>,
}

#[allow(dead_code)]
impl SignalCombiner {
    pub fn new(
        strategies: Vec<Box<dyn Strategy + Send + Sync>>,
        weights: Vec<f64>
    ) -> Result<Self, String> {
        if strategies.len() != weights.len() {
            return Err(format!(
                "Strategies count ({}) must match weights count ({})",
                strategies.len(),
                weights.len()
            ));
        }
        if strategies.is_empty() {
            return Err("At least one strategy is required".to_string());
        }
        Ok(SignalCombiner { strategies, weights })
    }

    pub fn combine_signals(&self, candles: &[Candle]) -> Option<TradingSignal> {
        let mut buy_score = 0.0;
        let mut sell_score = 0.0;
        let mut total_weight = 0.0;

        for (strategy, &weight) in self.strategies.iter().zip(&self.weights) {
            if let Some(signal) = strategy.generate_signal(candles) {
                match signal.signal {
                    Signal::Buy => buy_score += signal.confidence * weight,
                    Signal::Sell => sell_score += signal.confidence * weight,
                    Signal::Hold => {} // Hold doesn't affect score
                }
                total_weight += weight;
            }
        }

        if total_weight == 0.0 {
            return None;
        }

        let buy_confidence = buy_score / total_weight;
        let sell_confidence = sell_score / total_weight;

        if buy_confidence > sell_confidence && buy_confidence > 0.5 {
            Some(TradingSignal { signal: Signal::Buy, confidence: buy_confidence })
        } else if sell_confidence > buy_confidence && sell_confidence > 0.5 {
            Some(TradingSignal { signal: Signal::Sell, confidence: sell_confidence })
        } else {
            Some(TradingSignal { signal: Signal::Hold, confidence: 0.5 })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_candles() -> Vec<Candle> {
        vec![
            Candle::new(100.0, 105.0, 95.0, 102.0, 1000.0).unwrap(),
            Candle::new(102.0, 108.0, 98.0, 105.0, 1100.0).unwrap(),
            Candle::new(105.0, 110.0, 100.0, 108.0, 1200.0).unwrap(),
            Candle::new(108.0, 112.0, 103.0, 106.0, 1300.0).unwrap(),
            Candle::new(106.0, 111.0, 102.0, 109.0, 1400.0).unwrap(),
            Candle::new(109.0, 115.0, 105.0, 112.0, 1500.0).unwrap(),
            Candle::new(112.0, 118.0, 108.0, 115.0, 1600.0).unwrap(),
            Candle::new(115.0, 120.0, 110.0, 117.0, 1700.0).unwrap(),
            Candle::new(117.0, 122.0, 112.0, 119.0, 1800.0).unwrap(),
            Candle::new(119.0, 125.0, 115.0, 122.0, 1900.0).unwrap(),
            Candle::new(122.0, 128.0, 118.0, 125.0, 2000.0).unwrap(),
            Candle::new(125.0, 130.0, 120.0, 127.0, 2100.0).unwrap(),
            Candle::new(127.0, 132.0, 122.0, 129.0, 2200.0).unwrap(),
            Candle::new(129.0, 135.0, 125.0, 132.0, 2300.0).unwrap(),
            Candle::new(132.0, 138.0, 128.0, 135.0, 2400.0).unwrap(),
            Candle::new(135.0, 140.0, 130.0, 137.0, 2500.0).unwrap(),
            Candle::new(137.0, 142.0, 132.0, 139.0, 2600.0).unwrap(),
            Candle::new(139.0, 144.0, 134.0, 141.0, 2700.0).unwrap(),
            Candle::new(141.0, 146.0, 136.0, 143.0, 2800.0).unwrap(),
            Candle::new(143.0, 148.0, 138.0, 145.0, 2900.0).unwrap(),
            Candle::new(145.0, 150.0, 140.0, 147.0, 3000.0).unwrap(),
            Candle::new(147.0, 152.0, 142.0, 149.0, 3100.0).unwrap(),
            Candle::new(149.0, 154.0, 144.0, 151.0, 3200.0).unwrap(),
            Candle::new(151.0, 156.0, 146.0, 153.0, 3300.0).unwrap(),
            Candle::new(153.0, 158.0, 148.0, 155.0, 3400.0).unwrap(),
            Candle::new(155.0, 160.0, 150.0, 157.0, 3500.0).unwrap(),
            Candle::new(157.0, 162.0, 152.0, 159.0, 3600.0).unwrap(),
            Candle::new(159.0, 164.0, 154.0, 161.0, 3700.0).unwrap(),
            Candle::new(161.0, 166.0, 156.0, 163.0, 3800.0).unwrap(),
            Candle::new(163.0, 168.0, 158.0, 165.0, 3900.0).unwrap(),
        ]
    }

    #[test]
    fn test_fast_scalping_signal() {
        let strategy = FastScalping::new();
        let candles = create_test_candles();
        let signal = strategy.generate_signal(&candles);
        assert!(signal.is_some());
        assert!(matches!(signal.unwrap().signal, Signal::Buy | Signal::Sell | Signal::Hold));
    }

    #[test]
    fn test_momentum_scalping_signal() {
        let strategy = MomentumScalping::new();
        let candles = create_test_candles();
        let signal = strategy.generate_signal(&candles);
        assert!(signal.is_some());
    }

    #[test]
    fn test_conservative_scalping_signal() {
        let strategy = ConservativeScalping::new();
        let candles = create_test_candles();
        let signal = strategy.generate_signal(&candles);
        assert!(signal.is_some());
    }

    #[test]
    fn test_signal_combiner() {
        let strategies: Vec<Box<dyn Strategy + Send + Sync>> = vec![
            Box::new(FastScalping::new()),
            Box::new(MomentumScalping::new()),
        ];
        let weights = vec![0.6, 0.4];
        let combiner = SignalCombiner::new(strategies, weights).unwrap();
        let candles = create_test_candles();
        let combined_signal = combiner.combine_signals(&candles);
        assert!(combined_signal.is_some());
        let signal = combined_signal.unwrap();
        assert!(signal.confidence >= 0.0 && signal.confidence <= 1.0);
    }
}