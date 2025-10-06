use crate::domain::value_objects::price::Price;


#[derive(Debug, Clone)]
pub struct Candle {
    pub open: Price,
    pub high: Price,
    pub low: Price,
    pub close: Price,
    pub volume: f64,
}


impl Candle {
    pub fn new(open: f64, high: f64, low: f64, close: f64, volume: f64) -> Result<Self, String> {
        Ok(Candle {
            open: Price::new(open)?,
            high: Price::new(high)?,
            low: Price::new(low)?,
            close: Price::new(close)?,
            volume,
        })
    }
}


pub trait Indicator {
    fn calculate(&self, candles: &[Candle]) -> Vec<f64>;
}


pub struct EMA {
    pub period: usize,
}


impl EMA {
    pub fn new(period: usize) -> Self {
        EMA { period }
    }

    pub fn calculate_on_values(&self, values: &[f64]) -> Vec<f64> {
        if values.is_empty() || self.period == 0 {
            return vec![];
        }
        let mut ema_values = Vec::with_capacity(values.len());
        let multiplier = 2.0 / (self.period as f64 + 1.0);

        let initial_count = self.period.min(values.len());
        let mut sum = 0.0;
        for i in 0..initial_count {
            sum += values[i];
        }
        let mut ema = sum / initial_count as f64;
        ema_values.push(ema);

        for &val in values.iter().skip(self.period) {
            ema = (val - ema) * multiplier + ema;
            ema_values.push(ema);
        }

        ema_values
    }
}

impl Indicator for EMA {
    fn calculate(&self, candles: &[Candle]) -> Vec<f64> {
        if candles.is_empty() || self.period == 0 {
            return vec![];
        }
        let mut ema_values = Vec::with_capacity(candles.len());
        let multiplier = 2.0 / (self.period as f64 + 1.0);

        // First EMA is SMA
        let initial_count = self.period.min(candles.len());
        let mut sum = 0.0;
        for i in 0..initial_count {
            sum += candles[i].close.value();
        }
        let mut ema = sum / initial_count as f64;
        ema_values.push(ema);

        for candle in candles.iter().skip(self.period) {
            ema = (candle.close.value() - ema) * multiplier + ema;
            ema_values.push(ema);
        }

        ema_values
    }
}


pub struct RSI {
    pub period: usize,
}


impl RSI {
    pub fn new(period: usize) -> Self {
        RSI { period }
    }
}

impl Indicator for RSI {
    fn calculate(&self, candles: &[Candle]) -> Vec<f64> {
        if self.period == 0 || candles.len() < self.period + 1 {
            return vec![];
        }
        let mut gains = Vec::new();
        let mut losses = Vec::new();

        for i in 1..candles.len() {
            let change = candles[i].close.value() - candles[i - 1].close.value();
            if change > 0.0 {
                gains.push(change);
                losses.push(0.0);
            } else {
                gains.push(0.0);
                losses.push(change.abs());
            }
        }

        if self.period == 0 || gains.len() < self.period {
            return vec![];
        }

        let mut rsi_values = Vec::new();
        for i in self.period..=gains.len() {
            let start_idx = i - self.period;
            let end_idx = i - 1;
            let avg_gain = gains[start_idx..=end_idx].iter().sum::<f64>() / self.period as f64;
            let avg_loss = losses[start_idx..=end_idx].iter().sum::<f64>() / self.period as f64;
            let rs = if avg_loss == 0.0 {
                100.0
            } else {
                avg_gain / avg_loss
            };
            let rsi = 100.0 - (100.0 / (1.0 + rs));
            rsi_values.push(rsi);
        }

        rsi_values
    }
}


#[derive(Debug, Clone)]
pub struct BollingerBandsValues {
    pub upper: Vec<f64>,
    pub middle: Vec<f64>,
    pub lower: Vec<f64>,
}


pub struct BollingerBands {
    pub period: usize,
    pub std_dev: f64,
}


impl BollingerBands {
    pub fn new(period: usize, std_dev: f64) -> Self {
        BollingerBands { period, std_dev }
    }

    /// Calculate Bollinger Bands and return structured data
    pub fn calculate_detailed(&self, candles: &[Candle]) -> BollingerBandsValues {
        if self.period == 0 || candles.len() < self.period {
            return BollingerBandsValues {
                upper: vec![],
                middle: vec![],
                lower: vec![],
            };
        }

        let mut upper = Vec::new();
        let mut middle = Vec::new();
        let mut lower = Vec::new();

        for i in self.period..=candles.len() {
            let start_idx = i - self.period;
            let end_idx = i - 1;
            let slice = &candles[start_idx..=end_idx];
            let sma = slice.iter().map(|c| c.close.value()).sum::<f64>() / self.period as f64;
            let variance = slice
                .iter()
                .map(|c| (c.close.value() - sma).powi(2))
                .sum::<f64>()
                / self.period as f64;
            let std = variance.sqrt();

            upper.push(sma + self.std_dev * std);
            middle.push(sma);
            lower.push(sma - self.std_dev * std);
        }

        BollingerBandsValues {
            upper,
            middle,
            lower,
        }
    }
}

impl Indicator for BollingerBands {
    fn calculate(&self, candles: &[Candle]) -> Vec<f64> {
        if self.period == 0 || candles.len() < self.period {
            return vec![];
        }
        let mut bands = Vec::new();

        for i in self.period..=candles.len() {
            let start_idx = i - self.period;
            let end_idx = i - 1;
            let slice = &candles[start_idx..=end_idx];
            let sma = slice.iter().map(|c| c.close.value()).sum::<f64>() / self.period as f64;
            let variance = slice
                .iter()
                .map(|c| (c.close.value() - sma).powi(2))
                .sum::<f64>()
                / self.period as f64;
            let std = variance.sqrt();
            bands.push(sma + self.std_dev * std); // Upper band
            bands.push(sma); // Middle (SMA)
            bands.push(sma - self.std_dev * std); // Lower band
        }

        bands
    }
}


pub struct MACD {
    pub fast_period: usize,
    pub slow_period: usize,
    pub signal_period: usize,
}


impl MACD {
    pub fn new(fast_period: usize, slow_period: usize, signal_period: usize) -> Self {
        MACD {
            fast_period,
            slow_period,
            signal_period,
        }
    }
}

impl Indicator for MACD {
    fn calculate(&self, candles: &[Candle]) -> Vec<f64> {
        let fast_ema = EMA::new(self.fast_period);
        let slow_ema = EMA::new(self.slow_period);
        let fast_values = fast_ema.calculate(candles);
        let slow_values = slow_ema.calculate(candles);

        let macd_line: Vec<f64> = fast_values
            .iter()
            .zip(slow_values.iter())
            .map(|(f, s)| f - s)
            .collect();

        let signal_ema = EMA::new(self.signal_period);
        signal_ema.calculate_on_values(&macd_line)
    }
}


pub struct StochasticOscillator {
    pub k_period: usize,
    pub d_period: usize,
}


impl StochasticOscillator {
    pub fn new(k_period: usize, d_period: usize) -> Self {
        StochasticOscillator { k_period, d_period }
    }
}

impl Indicator for StochasticOscillator {
    fn calculate(&self, candles: &[Candle]) -> Vec<f64> {
        if candles.len() < self.k_period {
            return vec![];
        }
        let mut k_values = Vec::new();

        for i in self.k_period..=candles.len() {
            let start_idx = i - self.k_period;
            let end_idx = i - 1;
            let slice = &candles[start_idx..=end_idx];
            let highest = slice
                .iter()
                .map(|c| c.high.value())
                .fold(f64::NEG_INFINITY, f64::max);
            let lowest = slice
                .iter()
                .map(|c| c.low.value())
                .fold(f64::INFINITY, f64::min);
            let current_close = candles[end_idx].close.value();

            // Handle division by zero when highest == lowest (no price movement)
            let range = highest - lowest;
            let k = if range > f64::EPSILON {
                100.0 * (current_close - lowest) / range
            } else {
                50.0 // Neutral value when no price movement
            };
            k_values.push(k);
        }

        // D is SMA of K
        let mut d_values = Vec::new();
        if self.d_period == 0 || k_values.len() < self.d_period {
            return vec![];
        }
        for i in self.d_period..=k_values.len() {
            let start_idx = i - self.d_period;
            let end_idx = i - 1;
            let sum: f64 = k_values[start_idx..=end_idx].iter().sum();
            d_values.push(sum / self.d_period as f64);
        }

        d_values
    }
}

pub struct VWAP;

impl Indicator for VWAP {
    fn calculate(&self, candles: &[Candle]) -> Vec<f64> {
        let mut vwap_values = Vec::new();
        let mut cumulative_volume = 0.0;
        let mut cumulative_volume_price = 0.0;

        for candle in candles {
            let typical_price =
                (candle.high.value() + candle.low.value() + candle.close.value()) / 3.0;
            cumulative_volume += candle.volume;
            cumulative_volume_price += typical_price * candle.volume;

            // Handle division by zero when cumulative volume is zero
            let vwap = if cumulative_volume > f64::EPSILON {
                cumulative_volume_price / cumulative_volume
            } else {
                typical_price // Use typical price if no volume
            };
            vwap_values.push(vwap);
        }

        vwap_values
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ema_calculation() {
        let candles = vec![
            Candle::new(100.0, 105.0, 95.0, 102.0, 1000.0).unwrap(),
            Candle::new(102.0, 108.0, 98.0, 105.0, 1100.0).unwrap(),
            Candle::new(105.0, 110.0, 100.0, 108.0, 1200.0).unwrap(),
        ];
        let ema = EMA::new(2);
        let values = ema.calculate(&candles);
        assert!(!values.is_empty());
        // Basic check
        assert!(values[0] > 100.0);
    }

    #[test]
    fn test_rsi_calculation() {
        let candles = vec![
            Candle::new(100.0, 105.0, 95.0, 102.0, 1000.0).unwrap(),
            Candle::new(102.0, 108.0, 98.0, 105.0, 1100.0).unwrap(),
            Candle::new(105.0, 110.0, 100.0, 108.0, 1200.0).unwrap(),
            Candle::new(108.0, 112.0, 103.0, 106.0, 1300.0).unwrap(),
            Candle::new(106.0, 111.0, 102.0, 109.0, 1400.0).unwrap(),
        ];
        let rsi = RSI::new(2);
        let values = rsi.calculate(&candles);
        assert!(!values.is_empty());
        assert!(values[0] >= 0.0 && values[0] <= 100.0);
    }

    #[test]
    fn test_vwap_calculation() {
        let candles = vec![
            Candle::new(100.0, 105.0, 95.0, 102.0, 1000.0).unwrap(),
            Candle::new(102.0, 108.0, 98.0, 105.0, 1100.0).unwrap(),
        ];
        let vwap = VWAP;
        let values = vwap.calculate(&candles);
        assert_eq!(values.len(), 2);
        assert!(values[1] > values[0]); // Should increase with volume
    }
}
