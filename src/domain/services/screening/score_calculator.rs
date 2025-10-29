/// Common trait for all score calculators
///
/// This trait provides a unified interface for all types of score calculations
/// used in symbol screening. All score calculators should return values in [0.0, 1.0].
pub trait ScoreCalculator: Send + Sync {
    /// Calculate a score based on the provided data
    ///
    /// # Returns
    /// A value in [0.0, 1.0] representing the calculated score.
    /// - 1.0 represents the best possible score for the metric
    /// - 0.0 represents the worst possible score for the metric
    fn calculate(&self) -> f64;

    /// Get a human-readable name for this calculator
    fn name(&self) -> &str;

    /// Get the weight this calculator contributes to the overall score
    /// Default: 1.0 (equal weight)
    fn weight(&self) -> f64 {
        1.0
    }

    /// Validate that the calculated score is in valid range [0.0, 1.0]
    /// This is called automatically after calculation
    fn validate_score(score: f64) -> bool {
        score >= 0.0 && score <= 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockCalculator {
        score: f64,
    }

    impl ScoreCalculator for MockCalculator {
        fn calculate(&self) -> f64 {
            self.score
        }

        fn name(&self) -> &str {
            "mock"
        }
    }

    #[test]
    fn test_score_calculator_validate_score_valid() {
        assert!(MockCalculator::validate_score(0.0));
        assert!(MockCalculator::validate_score(0.5));
        assert!(MockCalculator::validate_score(1.0));
    }

    #[test]
    fn test_score_calculator_validate_score_invalid() {
        assert!(!MockCalculator::validate_score(-0.1));
        assert!(!MockCalculator::validate_score(1.1));
    }

    #[test]
    fn test_score_calculator_default_weight() {
        let calc = MockCalculator { score: 0.5 };
        assert_eq!(calc.weight(), 1.0);
    }

    #[test]
    fn test_score_calculator_name() {
        let calc = MockCalculator { score: 0.5 };
        assert_eq!(calc.name(), "mock");
    }
}
