//! Fear calibration system with z-score normalization

use spectremesh_core::FearError;

/// Implements the z-score normalization for fear calibration
pub struct FearCalibrator {
    baseline_samples: Vec<f32>,
    target_samples: usize,
    mean: f32,
    std_dev: f32,
    calibrated: bool,
}

impl FearCalibrator {
    pub fn new(calibration_duration: f32, fps: f32) -> Self {
        let target_samples = (calibration_duration * fps) as usize;
        Self {
            baseline_samples: Vec::with_capacity(target_samples),
            target_samples,
            mean: 0.0,
            std_dev: 1.0,
            calibrated: false,
        }
    }

    /// Add a raw emotion logit sample during calibration period
    pub fn add_sample(&mut self, raw_fear_logit: f32) -> Result<(), FearError> {
        if self.calibrated {
            return Ok(()); // Already calibrated, ignore
        }

        self.baseline_samples.push(raw_fear_logit);

        if self.baseline_samples.len() >= self.target_samples {
            self.compute_baseline()?;
        }

        Ok(())
    }

    /// Compute mean and standard deviation from baseline samples
    fn compute_baseline(&mut self) -> Result<(), FearError> {
        if self.baseline_samples.is_empty() {
            return Err(FearError::CalibrationIncomplete);
        }

        // Calculate mean
        self.mean = self.baseline_samples.iter().sum::<f32>() / self.baseline_samples.len() as f32;

        // Calculate standard deviation
        let variance = self.baseline_samples
            .iter()
            .map(|x| (x - self.mean).powi(2))
            .sum::<f32>() / self.baseline_samples.len() as f32;

        self.std_dev = variance.sqrt();

        // Handle edge case: if std_dev is too small, use default
        if self.std_dev < 0.01 {
            self.std_dev = 1.0;
        }

        self.calibrated = true;
        Ok(())
    }

    /// Convert raw fear logit to normalized fear score [0.0, 1.0]
    pub fn normalize_fear(&self, raw_fear_logit: f32) -> f32 {
        if !self.calibrated {
            // During calibration, return neutral fear
            return 0.3;
        }

        // Z-score normalization
        let z_score = (raw_fear_logit - self.mean) / self.std_dev;

        // Convert z-score to [0, 1] using sigmoid function
        // This maps z-scores: -3 -> ~0.05, 0 -> 0.5, +3 -> ~0.95
        let sigmoid = 1.0 / (1.0 + (-z_score).exp());

        // Clamp to [0, 1] for safety
        sigmoid.clamp(0.0, 1.0)
    }

    /// Extract fear component from 7-class emotion logits
    /// Assumes: [angry, disgust, fear, happy, sad, surprise, neutral]
    pub fn extract_fear_logit(emotion_logits: &[f32; 7]) -> f32 {
        // FaceONNX emotion classes (typical order)
        const FEAR_INDEX: usize = 2;
        emotion_logits[FEAR_INDEX]
    }

    pub fn is_calibrated(&self) -> bool {
        self.calibrated
    }

    pub fn calibration_progress(&self) -> f32 {
        (self.baseline_samples.len() as f32 / self.target_samples as f32).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fear_calibrator_creation() {
        let calibrator = FearCalibrator::new(30.0, 1.0); // 30 samples at 1 FPS
        assert!(!calibrator.is_calibrated());
        assert_eq!(calibrator.calibration_progress(), 0.0);
        assert_eq!(calibrator.target_samples, 30);
    }

    #[test]
    fn test_fear_calibration_mathematics() {
        let mut calibrator = FearCalibrator::new(10.0, 1.0); // 10 samples for quick test

        // Add baseline samples (neutral emotion)
        let baseline_samples = vec![0.2, 0.25, 0.18, 0.22, 0.19, 0.21, 0.23, 0.20, 0.24, 0.17];
        for sample in baseline_samples {
            calibrator.add_sample(sample).unwrap();
        }

        // Should now be calibrated
        assert!(calibrator.is_calibrated());
        assert_eq!(calibrator.calibration_progress(), 1.0);

        // Test normalization
        let neutral_fear = calibrator.normalize_fear(0.2); // Baseline value
        assert!((neutral_fear - 0.5).abs() < 0.1); // Should be near 0.5

        let high_fear = calibrator.normalize_fear(0.8); // High fear
        assert!(high_fear > 0.7); // Should be high

        let low_fear = calibrator.normalize_fear(0.05); // Low fear
        assert!(low_fear < 0.3); // Should be low
    }

    #[test]
    fn test_calibration_incomplete() {
        let mut calibrator = FearCalibrator::new(30.0, 1.0); // Need 30 samples

        // Add only a few samples
        for _ in 0..5 {
            calibrator.add_sample(0.2).unwrap();
        }

        // Should not be calibrated yet
        assert!(!calibrator.is_calibrated());
        assert!((calibrator.calibration_progress() - 5.0/30.0).abs() < 0.01);

        // Normalization should return default value
        assert_eq!(calibrator.normalize_fear(0.8), 0.3);
    }

    #[test]
    fn test_extract_fear_logit() {
        let emotion_logits = [0.1, 0.1, 0.8, 0.1, 0.1, 0.1, 0.1]; // High fear at index 2
        let fear_logit = FearCalibrator::extract_fear_logit(&emotion_logits);
        assert_eq!(fear_logit, 0.8);
    }

    #[test]
    fn test_small_std_dev_handling() {
        let mut calibrator = FearCalibrator::new(5.0, 1.0);

        // Add identical samples (zero variance)
        for _ in 0..5 {
            calibrator.add_sample(0.5).unwrap();
        }

        assert!(calibrator.is_calibrated());
        // Should handle zero variance gracefully
        let normalized = calibrator.normalize_fear(0.6);
        assert!(normalized >= 0.0 && normalized <= 1.0);
    }

    #[test]
    fn test_sigmoid_mapping() {
        let mut calibrator = FearCalibrator::new(3.0, 1.0);
        
        // Set up known mean and std_dev
        calibrator.baseline_samples = vec![0.0, 0.0, 0.0];
        calibrator.compute_baseline().unwrap();
        
        // Test sigmoid mapping
        let result = calibrator.normalize_fear(0.0); // Should be ~0.5 (mean)
        assert!((result - 0.5).abs() < 0.1);
    }
}
