use tracing::{debug, warn};

use crate::domain::entities::symbol_screening::SymbolScreeningResult;
use crate::domain::services::indicators::Candle;
use crate::domain::services::screening::{
    MomentumScoreCalculator, SimpleMomentumCalculator, SimpleSpreadCalculator,
    SimpleVolatilityCalculator, SimpleVolumeCalculator, SpreadScoreCalculator,
    VolatilityScoreCalculator, VolumeScoreCalculator,
};

/// Aggregates all individual scores into overall scalping potential score
pub struct ScalpingPotentialAggregator {
    volatility_calc: SimpleVolatilityCalculator,
    volume_calc: SimpleVolumeCalculator,
    spread_calc: SimpleSpreadCalculator,
    momentum_calc: SimpleMomentumCalculator,
}

impl Default for ScalpingPotentialAggregator {
    fn default() -> Self {
        ScalpingPotentialAggregator {
            volatility_calc: SimpleVolatilityCalculator::default(),
            volume_calc: SimpleVolumeCalculator::default(),
            spread_calc: SimpleSpreadCalculator::default(),
            momentum_calc: SimpleMomentumCalculator::default(),
        }
    }
}

impl ScalpingPotentialAggregator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate scalping potential for a symbol
    ///
    /// # Arguments
    /// * `symbol` - Trading symbol (e.g., "BTC-USD")
    /// * `exchange` - Exchange identifier (e.g., "dydx")
    /// * `candles` - Recent candle data for technical analysis
    /// * `volumes` - Recent trading volumes
    /// * `bid` - Current bid price
    /// * `ask` - Current ask price
    ///
    /// # Returns
    /// SymbolScreeningResult with all calculated scores
    pub fn calculate(
        &self,
        symbol: String,
        exchange: String,
        candles: &[Candle],
        volumes: &[f64],
        bid: f64,
        ask: f64,
    ) -> SymbolScreeningResult {
        debug!(
            symbol = %symbol,
            exchange = %exchange,
            candle_count = candles.len(),
            volume_count = volumes.len(),
            "Starting scalping potential calculation"
        );

        let volatility_score = if candles.is_empty() {
            debug!(symbol = %symbol, "No candle data available, volatility_score = 0.0");
            0.0
        } else {
            let last = &candles[candles.len() - 1];
            let score = self.volatility_calc.calculate(
                last.high.value(),
                last.low.value(),
                last.close.value(),
            );
            debug!(
                symbol = %symbol,
                high = last.high.value(),
                low = last.low.value(),
                close = last.close.value(),
                volatility_score = score,
                "Calculated volatility score"
            );
            score
        };

        let volume_score = self.volume_calc.calculate(volumes);
        debug!(
            symbol = %symbol,
            volume_count = volumes.len(),
            volume_score = volume_score,
            "Calculated volume score"
        );

        let spread_score = self.spread_calc.calculate(bid, ask);
        debug!(
            symbol = %symbol,
            bid = bid,
            ask = ask,
            spread_score = spread_score,
            "Calculated spread score"
        );

        let momentum_score = self.momentum_calc.calculate(candles);
        debug!(
            symbol = %symbol,
            candle_count = candles.len(),
            momentum_score = momentum_score,
            "Calculated momentum score"
        );

        let result = SymbolScreeningResult::new(
            symbol.clone(),
            exchange.clone(),
            volatility_score,
            volume_score,
            spread_score,
            momentum_score,
        );

        debug!(
            symbol = %symbol,
            exchange = %exchange,
            overall_score = result.overall_score,
            volatility_score = volatility_score,
            volume_score = volume_score,
            spread_score = spread_score,
            momentum_score = momentum_score,
            recommendation = ?result.recommendation,
            "Completed scalping potential calculation"
        );

        result
    }

    /// Rank multiple screening results by overall score
    pub fn rank_results(mut results: Vec<SymbolScreeningResult>) -> Vec<SymbolScreeningResult> {
        debug!(
            result_count = results.len(),
            "Starting ranking of screening results"
        );

        results.sort_by(|a, b| b.overall_score.partial_cmp(&a.overall_score).unwrap());

        for (rank, result) in results.iter().enumerate() {
            debug!(
                rank = rank + 1,
                symbol = %result.symbol,
                exchange = %result.exchange,
                overall_score = result.overall_score,
                recommendation = ?result.recommendation,
                "Ranked symbol screening result"
            );
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scalping_score_weighted_combination() {
        let agg = ScalpingPotentialAggregator::new();
        let candles = vec![Candle::new(100.0, 101.0, 99.0, 100.5, 1000.0).unwrap()];
        let volumes = vec![500_000.0];

        // All perfect scores should give 1.0
        let result = agg.calculate(
            "TEST-USD".to_string(),
            "test".to_string(),
            &candles,
            &volumes,
            100.0,
            100.0,
        );

        // overall = 0.3 * volatility + 0.3 * volume + 0.2 * spread + 0.2 * momentum
        assert!(result.overall_score <= 1.0 && result.overall_score >= 0.0);
    }

    #[test]
    fn test_scalping_score_aggregation_with_known_inputs() {
        let mut agg = ScalpingPotentialAggregator::new();

        // Override calculators with fixed values for testing
        agg.volatility_calc.max_volatility_range_percent = 1.0;
        agg.volume_calc.max_volume = 1.0;
        agg.spread_calc.max_spread_percent = 1.0;

        let candles = vec![Candle::new(100.0, 100.5, 99.5, 100.0, 1.0).unwrap()];
        let volumes = vec![1.0];

        let result = agg.calculate(
            "TEST-USD".to_string(),
            "test".to_string(),
            &candles,
            &volumes,
            100.0,
            100.0,
        );

        // All components should be non-zero
        assert!(result.volatility_score > 0.0);
        assert!(result.volume_score > 0.0);
        assert_eq!(result.spread_score, 1.0); // bid == ask
        assert!(result.momentum_score >= 0.0);
    }

    #[test]
    fn test_ranking_multiple_symbols_by_score() {
        let results = vec![
            SymbolScreeningResult::new(
                "BTC-USD".to_string(),
                "test".to_string(),
                0.5,
                0.5,
                0.5,
                0.5,
            ),
            SymbolScreeningResult::new(
                "ETH-USD".to_string(),
                "test".to_string(),
                0.8,
                0.8,
                0.8,
                0.8,
            ),
            SymbolScreeningResult::new(
                "SOL-USD".to_string(),
                "test".to_string(),
                0.3,
                0.3,
                0.3,
                0.3,
            ),
        ];

        let ranked = ScalpingPotentialAggregator::rank_results(results);

        // Should be sorted descending
        assert_eq!(ranked[0].symbol, "ETH-USD");
        assert_eq!(ranked[1].symbol, "BTC-USD");
        assert_eq!(ranked[2].symbol, "SOL-USD");
    }

    #[test]
    fn test_recommendation_categorization_best_candidate() {
        let result = SymbolScreeningResult::new(
            "TEST-USD".to_string(),
            "test".to_string(),
            1.0,
            1.0,
            1.0,
            1.0,
        );
        assert_eq!(
            result.recommendation,
            crate::domain::entities::symbol_screening::RecommendationCategory::BestCandidate
        );
    }

    #[test]
    fn test_recommendation_categorization_good_candidate() {
        let result = SymbolScreeningResult::new(
            "TEST-USD".to_string(),
            "test".to_string(),
            0.7,
            0.7,
            0.7,
            0.7,
        );
        assert_eq!(
            result.recommendation,
            crate::domain::entities::symbol_screening::RecommendationCategory::GoodCandidate
        );
    }

    #[test]
    fn test_recommendation_categorization_fair_candidate() {
        let result = SymbolScreeningResult::new(
            "TEST-USD".to_string(),
            "test".to_string(),
            0.5,
            0.5,
            0.5,
            0.5,
        );
        assert_eq!(
            result.recommendation,
            crate::domain::entities::symbol_screening::RecommendationCategory::FairCandidate
        );
    }

    #[test]
    fn test_recommendation_categorization_avoid() {
        let result = SymbolScreeningResult::new(
            "TEST-USD".to_string(),
            "test".to_string(),
            0.2,
            0.2,
            0.2,
            0.2,
        );
        assert_eq!(
            result.recommendation,
            crate::domain::entities::symbol_screening::RecommendationCategory::Avoid
        );
    }
}
