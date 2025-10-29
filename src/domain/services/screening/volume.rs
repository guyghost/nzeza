/// Calculates volume score based on recent trading volumes
pub trait VolumeScoreCalculator {
    /// Calculate volume score from volumes
    /// Returns score in [0.0, 1.0]
    fn calculate(&self, volumes: &[f64]) -> f64;
}

/// Default implementation using average-relative scoring
pub struct SimpleVolumeCalculator {
    /// Normalization factor - high volume threshold
    pub max_volume: f64,
}

impl Default for SimpleVolumeCalculator {
    fn default() -> Self {
        SimpleVolumeCalculator {
            max_volume: 1_000_000.0, // 1M units is considered "high volume"
        }
    }
}

impl VolumeScoreCalculator for SimpleVolumeCalculator {
    fn calculate(&self, volumes: &[f64]) -> f64 {
        if volumes.is_empty() {
            return 0.0;
        }

        if volumes.iter().all(|&v| v <= 0.0) {
            return 0.0;
        }

        // Calculate recent average (last 5 volumes or all if fewer)
        let recent_count = volumes.len().min(5);
        let recent_volumes = &volumes[volumes.len() - recent_count..];
        let avg_volume = recent_volumes.iter().sum::<f64>() / recent_count as f64;

        // Normalize to [0, 1]
        (avg_volume / self.max_volume).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_score_based_on_recent_average() {
        let calc = SimpleVolumeCalculator::default();
        let volumes = vec![100.0, 150.0, 200.0, 250.0, 300.0];
        let score = calc.calculate(&volumes);
        // recent_avg = (100+150+200+250+300)/5 = 200
        // score = 200 / 1_000_000 = 0.0002
        assert!(score > 0.0 && score < 1.0);
    }

    #[test]
    fn test_volume_score_normalization_with_mock_volumes() {
        let calc = SimpleVolumeCalculator::default();

        let low_volumes = vec![100.0, 100.0, 100.0];
        let high_volumes = vec![1_000_000.0, 1_000_000.0, 1_000_000.0];
        let extreme_volumes = vec![10_000_000.0, 10_000_000.0, 10_000_000.0];

        let low_score = calc.calculate(&low_volumes);
        let high_score = calc.calculate(&high_volumes);
        let extreme_score = calc.calculate(&extreme_volumes);

        assert!(low_score < high_score);
        assert_eq!(high_score, 1.0);
        assert_eq!(extreme_score, 1.0); // capped at 1.0
    }

    #[test]
    fn test_volume_score_zero_volume_edge_case() {
        let calc = SimpleVolumeCalculator::default();
        let volumes = vec![0.0, 0.0, 0.0];
        let score = calc.calculate(&volumes);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_volume_score_very_high_volume_spike() {
        let calc = SimpleVolumeCalculator::default();
        // 10x normal volume
        let volumes = vec![100_000.0, 100_000.0, 100_000.0, 1_000_000.0, 100_000.0];
        let score = calc.calculate(&volumes);
        assert_eq!(score, 1.0); // capped at 1.0
    }

    #[test]
    fn test_volume_score_consistent_volumes() {
        let calc = SimpleVolumeCalculator::default();
        let volumes = vec![500_000.0, 500_000.0, 500_000.0, 500_000.0, 500_000.0];
        let score = calc.calculate(&volumes);
        // avg = 500_000, score = 500_000 / 1_000_000 = 0.5
        assert!((score - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_volume_score_empty_volumes() {
        let calc = SimpleVolumeCalculator::default();
        let score = calc.calculate(&[]);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_volume_score_uses_recent_volumes() {
        let calc = SimpleVolumeCalculator::default();
        // old volumes ignored, only last 5 count
        let volumes = vec![
            10.0, 20.0, 30.0, 40.0, 50.0, 500_000.0, 600_000.0, 700_000.0, 800_000.0, 900_000.0,
        ];
        let score = calc.calculate(&volumes);
        // recent_avg = (500_000+600_000+700_000+800_000+900_000)/5 = 700_000
        // score = 700_000 / 1_000_000 = 0.7
        assert!((score - 0.7).abs() < 0.001);
    }
}
