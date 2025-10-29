use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Recommendation category for a symbol's scalping potential
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecommendationCategory {
    /// Score >= 0.75 - Excellent scalping potential
    BestCandidate,
    /// Score 0.60-0.75 - Good scalping potential
    GoodCandidate,
    /// Score 0.50-0.60 - Fair scalping potential
    FairCandidate,
    /// Score < 0.50 - Not recommended for scalping
    Avoid,
}

impl RecommendationCategory {
    pub fn from_score(score: f64) -> Self {
        match score {
            s if s >= 0.75 => RecommendationCategory::BestCandidate,
            s if s >= 0.60 => RecommendationCategory::GoodCandidate,
            s if s >= 0.50 => RecommendationCategory::FairCandidate,
            _ => RecommendationCategory::Avoid,
        }
    }
}

/// Result of screening a symbol for scalping potential
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolScreeningResult {
    /// Trading symbol (e.g., "BTC-USD")
    pub symbol: String,
    /// Exchange identifier (e.g., "dydx", "coinbase")
    pub exchange: String,
    /// Volatility score [0.0, 1.0]
    pub volatility_score: f64,
    /// Volume score [0.0, 1.0]
    pub volume_score: f64,
    /// Spread score [0.0, 1.0] (higher = tighter spread = better)
    pub spread_score: f64,
    /// Momentum score [0.0, 1.0]
    pub momentum_score: f64,
    /// Overall scalping potential score [0.0, 1.0]
    /// Formula: 0.3 * volatility + 0.3 * volume + 0.2 * spread + 0.2 * momentum
    pub overall_score: f64,
    /// Recommendation category based on overall_score
    pub recommendation: RecommendationCategory,
    /// Timestamp when this screening was performed
    pub screened_at: DateTime<Utc>,
}

impl SymbolScreeningResult {
    /// Create a new screening result with validation
    ///
    /// # Panics
    /// Panics if any score is outside [0.0, 1.0]
    pub fn new(
        symbol: String,
        exchange: String,
        volatility_score: f64,
        volume_score: f64,
        spread_score: f64,
        momentum_score: f64,
    ) -> Self {
        assert!(
            (0.0..=1.0).contains(&volatility_score),
            "volatility_score must be in [0.0, 1.0], got {}",
            volatility_score
        );
        assert!(
            (0.0..=1.0).contains(&volume_score),
            "volume_score must be in [0.0, 1.0], got {}",
            volume_score
        );
        assert!(
            (0.0..=1.0).contains(&spread_score),
            "spread_score must be in [0.0, 1.0], got {}",
            spread_score
        );
        assert!(
            (0.0..=1.0).contains(&momentum_score),
            "momentum_score must be in [0.0, 1.0], got {}",
            momentum_score
        );

        let overall_score =
            0.3 * volatility_score + 0.3 * volume_score + 0.2 * spread_score + 0.2 * momentum_score;

        let recommendation = RecommendationCategory::from_score(overall_score);

        SymbolScreeningResult {
            symbol,
            exchange,
            volatility_score,
            volume_score,
            spread_score,
            momentum_score,
            overall_score,
            recommendation,
            screened_at: Utc::now(),
        }
    }

    /// Get volatility score
    pub fn volatility_score(&self) -> f64 {
        self.volatility_score
    }

    /// Get volume score
    pub fn volume_score(&self) -> f64 {
        self.volume_score
    }

    /// Get spread score
    pub fn spread_score(&self) -> f64 {
        self.spread_score
    }

    /// Get momentum score
    pub fn momentum_score(&self) -> f64 {
        self.momentum_score
    }

    /// Get overall score
    pub fn overall_score(&self) -> f64 {
        self.overall_score
    }

    /// Get recommendation category
    pub fn recommendation(&self) -> RecommendationCategory {
        self.recommendation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_screening_result_creation() {
        let result = SymbolScreeningResult::new(
            "BTC-USD".to_string(),
            "dydx".to_string(),
            0.8,
            0.7,
            0.9,
            0.6,
        );

        assert_eq!(result.symbol, "BTC-USD");
        assert_eq!(result.exchange, "dydx");
        assert_eq!(result.volatility_score, 0.8);
        assert_eq!(result.volume_score, 0.7);
        assert_eq!(result.spread_score, 0.9);
        assert_eq!(result.momentum_score, 0.6);

        // overall = 0.3*0.8 + 0.3*0.7 + 0.2*0.9 + 0.2*0.6 = 0.24 + 0.21 + 0.18 + 0.12 = 0.75
        assert!((result.overall_score - 0.75).abs() < 0.001);
        assert_eq!(result.recommendation, RecommendationCategory::BestCandidate);
    }

    #[test]
    fn test_recommendation_category_best_candidate() {
        let result = SymbolScreeningResult::new(
            "BTC-USD".to_string(),
            "dydx".to_string(),
            1.0,
            1.0,
            1.0,
            1.0,
        );
        assert_eq!(result.recommendation, RecommendationCategory::BestCandidate);
    }

    #[test]
    fn test_recommendation_category_good_candidate() {
        let result = SymbolScreeningResult::new(
            "ETH-USD".to_string(),
            "dydx".to_string(),
            0.7,
            0.7,
            0.7,
            0.7,
        );
        assert_eq!(result.recommendation, RecommendationCategory::GoodCandidate);
    }

    #[test]
    fn test_recommendation_category_fair_candidate() {
        let result = SymbolScreeningResult::new(
            "SOL-USD".to_string(),
            "dydx".to_string(),
            0.5,
            0.5,
            0.5,
            0.5,
        );
        assert_eq!(result.recommendation, RecommendationCategory::FairCandidate);
    }

    #[test]
    fn test_recommendation_category_avoid() {
        let result = SymbolScreeningResult::new(
            "LOW-USD".to_string(),
            "dydx".to_string(),
            0.2,
            0.2,
            0.2,
            0.2,
        );
        assert_eq!(result.recommendation, RecommendationCategory::Avoid);
    }

    #[test]
    #[should_panic(expected = "volatility_score must be in [0.0, 1.0]")]
    fn test_volatility_score_validation_too_high() {
        SymbolScreeningResult::new(
            "BTC-USD".to_string(),
            "dydx".to_string(),
            1.5,
            0.5,
            0.5,
            0.5,
        );
    }

    #[test]
    #[should_panic(expected = "volatility_score must be in [0.0, 1.0]")]
    fn test_volatility_score_validation_negative() {
        SymbolScreeningResult::new(
            "BTC-USD".to_string(),
            "dydx".to_string(),
            -0.1,
            0.5,
            0.5,
            0.5,
        );
    }

    #[test]
    #[should_panic(expected = "volume_score must be in [0.0, 1.0]")]
    fn test_volume_score_validation() {
        SymbolScreeningResult::new(
            "BTC-USD".to_string(),
            "dydx".to_string(),
            0.5,
            1.5,
            0.5,
            0.5,
        );
    }

    #[test]
    #[should_panic(expected = "spread_score must be in [0.0, 1.0]")]
    fn test_spread_score_validation() {
        SymbolScreeningResult::new(
            "BTC-USD".to_string(),
            "dydx".to_string(),
            0.5,
            0.5,
            -0.1,
            0.5,
        );
    }

    #[test]
    #[should_panic(expected = "momentum_score must be in [0.0, 1.0]")]
    fn test_momentum_score_validation() {
        SymbolScreeningResult::new(
            "BTC-USD".to_string(),
            "dydx".to_string(),
            0.5,
            0.5,
            0.5,
            2.0,
        );
    }
}
