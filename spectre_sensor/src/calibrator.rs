//! Adaptive fear calibrator with EMA updates
//! 
//! Implements continuous calibration using exponential moving averages
//! for mean and variance estimation, with optional freezing capability.

use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Calibration errors
#[derive(Debug, Error)]
pub enum CalibrationError {
    #[error("Calibration is frozen and cannot be updated")]
    Frozen,
    
    #[error("Insufficient samples for calibration: need at least {min_samples}")]
    InsufficientSamples { min_samples: usize },
    
    #[error("Invalid calibration parameters: {reason}")]
    InvalidParameters { reason: String },
}

/// Baseline statistics for calibration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineStats {
    /// Current mean of fear logits
    pub mean: f32,
    /// Current standard deviation
    pub std_dev: f32,
    /// Number of samples processed
    pub sample_count: u32,
    /// Last update timestamp (skipped in serialization)
    #[serde(skip, default = "Instant::now")]
    pub last_update: Instant,
}

impl Default for BaselineStats {
    fn default() -> Self {
        Self {
            mean: 0.0,
            std_dev: 1.0,
            sample_count: 0,
            last_update: Instant::now(),
        }
    }
}

/// Adaptive fear calibrator with EMA updates
pub struct AdaptiveCalibrator {
    /// Current baseline statistics
    baseline: BaselineStats,
    /// EMA alpha parameter for updates
    alpha: f32,
    /// Whether calibration is frozen
    frozen: bool,
    /// Minimum samples required for initial calibration
    min_samples: usize,
    /// Initial calibration period duration
    initial_period: Duration,
    /// When calibration started
    start_time: Instant,
    /// Whether initial calibration is complete
    initial_complete: bool,
    /// Previous mean for drift calculation
    previous_mean: f32,
}

impl AdaptiveCalibrator {
    /// Create a new adaptive calibrator
    pub fn new(initial_period: Duration, alpha: f32) -> Self {
        Self {
            baseline: BaselineStats::default(),
            alpha,
            frozen: false,
            min_samples: 30, // Minimum samples for stable statistics
            initial_period,
            start_time: Instant::now(),
            initial_complete: false,
            previous_mean: 0.0,
        }
    }

    /// Create with default parameters (alpha = 0.05)
    pub fn with_defaults(initial_period: Duration) -> Self {
        Self::new(initial_period, 0.05)
    }

    /// Add a new fear logit sample
    pub fn add_sample(&mut self, fear_logit: f32) -> Result<(), CalibrationError> {
        if self.frozen {
            return Err(CalibrationError::Frozen);
        }

        if !fear_logit.is_finite() {
            return Ok(()); // Skip invalid samples
        }

        self.baseline.sample_count += 1;
        self.baseline.last_update = Instant::now();

        if !self.initial_complete {
            // During initial calibration, collect samples for batch statistics
            self.update_initial_calibration(fear_logit)?;
        } else {
            // After initial calibration, use EMA updates
            self.update_ema(fear_logit);
        }

        Ok(())
    }

    /// Update during initial calibration period
    fn update_initial_calibration(&mut self, fear_logit: f32) -> Result<(), CalibrationError> {
        // For initial calibration, we need to collect samples and compute statistics
        // This is a simplified approach - in practice, you'd want to store samples
        // and compute proper mean/variance
        
        if self.baseline.sample_count == 1 {
            // First sample
            self.baseline.mean = fear_logit;
            self.baseline.std_dev = 1.0; // Default until we have more samples
        } else {
            // Update running statistics
            let delta = fear_logit - self.baseline.mean;
            self.baseline.mean += delta / self.baseline.sample_count as f32;
            
            // Simple variance estimation (not optimal but functional)
            if self.baseline.sample_count > 1 {
                let variance = self.baseline.std_dev.powi(2);
                let new_variance = variance + (delta * delta - variance) / self.baseline.sample_count as f32;
                self.baseline.std_dev = new_variance.sqrt().max(0.1); // Minimum std_dev
            }
        }

        // Check if initial calibration is complete
        let elapsed = self.start_time.elapsed();
        if elapsed >= self.initial_period && self.baseline.sample_count >= self.min_samples as u32 {
            self.initial_complete = true;
            self.previous_mean = self.baseline.mean;
            tracing::info!(
                "Initial calibration complete: mean={:.3}, std_dev={:.3}, samples={}",
                self.baseline.mean,
                self.baseline.std_dev,
                self.baseline.sample_count
            );
        }

        Ok(())
    }

    /// Update using exponential moving average
    fn update_ema(&mut self, fear_logit: f32) {
        let old_mean = self.baseline.mean;
        
        // Update mean using EMA
        self.baseline.mean = (1.0 - self.alpha) * self.baseline.mean + self.alpha * fear_logit;
        
        // Update variance using EMA
        let delta = fear_logit - old_mean;
        let variance = self.baseline.std_dev.powi(2);
        let new_variance = (1.0 - self.alpha) * variance + self.alpha * delta.powi(2);
        self.baseline.std_dev = new_variance.sqrt().max(0.1); // Minimum std_dev
    }

    /// Normalize a fear logit to [0, 1] range
    pub fn normalize_fear(&self, fear_logit: f32) -> f32 {
        if !self.is_calibrated() {
            // During calibration, return neutral fear
            return 0.3;
        }

        // Z-score normalization
        let z_score = (fear_logit - self.baseline.mean) / self.baseline.std_dev;
        
        // Convert to [0, 1] using sigmoid function
        let sigmoid = 1.0 / (1.0 + (-z_score).exp());
        
        // Clamp to [0, 1] for safety
        sigmoid.clamp(0.0, 1.0)
    }

    /// Check if calibration is complete
    pub fn is_calibrated(&self) -> bool {
        self.initial_complete
    }

    /// Get calibration progress [0.0, 1.0]
    pub fn progress(&self) -> f32 {
        if self.initial_complete {
            return 1.0;
        }

        let time_progress = self.start_time.elapsed().as_secs_f32() / self.initial_period.as_secs_f32();
        let sample_progress = self.baseline.sample_count as f32 / self.min_samples as f32;
        
        time_progress.min(sample_progress).min(1.0)
    }

    /// Freeze calibration to prevent further updates
    pub fn freeze(&mut self) {
        self.frozen = true;
        tracing::info!("Calibration frozen at mean={:.3}, std_dev={:.3}", 
                      self.baseline.mean, self.baseline.std_dev);
    }

    /// Unfreeze calibration to allow updates
    pub fn unfreeze(&mut self) {
        self.frozen = false;
        tracing::info!("Calibration unfrozen");
    }

    /// Reset calibration to initial state
    pub fn reset(&mut self) {
        self.baseline = BaselineStats::default();
        self.start_time = Instant::now();
        self.initial_complete = false;
        self.frozen = false;
        self.previous_mean = 0.0;
        tracing::info!("Calibration reset");
    }

    /// Get current baseline statistics
    pub fn baseline_stats(&self) -> &BaselineStats {
        &self.baseline
    }

    /// Calculate calibration drift (change in mean since last check)
    pub fn calculate_drift(&mut self) -> f32 {
        let current_drift = (self.baseline.mean - self.previous_mean).abs();
        self.previous_mean = self.baseline.mean;
        current_drift
    }

    /// Check if calibration is frozen
    pub fn is_frozen(&self) -> bool {
        self.frozen
    }

    /// Get the EMA alpha parameter
    pub fn alpha(&self) -> f32 {
        self.alpha
    }

    /// Set a new EMA alpha parameter
    pub fn set_alpha(&mut self, alpha: f32) -> Result<(), CalibrationError> {
        if alpha <= 0.0 || alpha >= 1.0 {
            return Err(CalibrationError::InvalidParameters {
                reason: "Alpha must be between 0 and 1".to_string(),
            });
        }
        self.alpha = alpha;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_calibrator_creation() {
        let calibrator = AdaptiveCalibrator::new(Duration::from_secs(30), 0.05);
        assert!(!calibrator.is_calibrated());
        assert_eq!(calibrator.progress(), 0.0);
        assert!(!calibrator.is_frozen());
        assert_eq!(calibrator.alpha(), 0.05);
    }

    #[test]
    fn test_calibrator_with_defaults() {
        let calibrator = AdaptiveCalibrator::with_defaults(Duration::from_secs(30));
        assert_eq!(calibrator.alpha(), 0.05);
    }

    #[test]
    fn test_initial_calibration() {
        let mut calibrator = AdaptiveCalibrator::new(Duration::from_millis(100), 0.05);
        
        // Add samples during initial period
        for i in 0..50 {
            let sample = 0.5 + (i as f32 * 0.01); // Gradually increasing samples
            calibrator.add_sample(sample).unwrap();
        }

        // Wait for initial period to complete
        std::thread::sleep(Duration::from_millis(150));
        
        // Add one more sample to trigger completion check
        calibrator.add_sample(0.6).unwrap();
        
        assert!(calibrator.is_calibrated());
        assert_eq!(calibrator.progress(), 1.0);
    }

    #[test]
    fn test_fear_normalization() {
        let mut calibrator = AdaptiveCalibrator::new(Duration::from_millis(1), 0.05);
        
        // Before calibration
        assert_eq!(calibrator.normalize_fear(0.8), 0.3);
        
        // Simulate calibration completion
        calibrator.baseline.mean = 0.5;
        calibrator.baseline.std_dev = 0.2;
        calibrator.initial_complete = true;
        
        // After calibration
        let normalized = calibrator.normalize_fear(0.7); // Above mean
        assert!(normalized > 0.5);
        
        let normalized_low = calibrator.normalize_fear(0.3); // Below mean
        assert!(normalized_low < 0.5);
    }

    #[test]
    fn test_freeze_unfreeze() {
        let mut calibrator = AdaptiveCalibrator::new(Duration::from_secs(30), 0.05);
        
        // Normal operation
        assert!(calibrator.add_sample(0.5).is_ok());
        
        // Freeze and try to add sample
        calibrator.freeze();
        assert!(calibrator.is_frozen());
        assert!(calibrator.add_sample(0.6).is_err());
        
        // Unfreeze and try again
        calibrator.unfreeze();
        assert!(!calibrator.is_frozen());
        assert!(calibrator.add_sample(0.6).is_ok());
    }

    #[test]
    fn test_reset() {
        let mut calibrator = AdaptiveCalibrator::new(Duration::from_secs(30), 0.05);
        
        // Add some samples
        calibrator.add_sample(0.5).unwrap();
        calibrator.add_sample(0.6).unwrap();
        
        let initial_count = calibrator.baseline_stats().sample_count;
        assert!(initial_count > 0);
        
        // Reset
        calibrator.reset();
        assert_eq!(calibrator.baseline_stats().sample_count, 0);
        assert!(!calibrator.is_calibrated());
        assert!(!calibrator.is_frozen());
    }

    #[test]
    fn test_invalid_alpha() {
        let mut calibrator = AdaptiveCalibrator::new(Duration::from_secs(30), 0.05);
        
        // Test invalid alpha values
        assert!(calibrator.set_alpha(0.0).is_err());
        assert!(calibrator.set_alpha(1.0).is_err());
        assert!(calibrator.set_alpha(-0.1).is_err());
        assert!(calibrator.set_alpha(1.1).is_err());
        
        // Test valid alpha
        assert!(calibrator.set_alpha(0.1).is_ok());
        assert_eq!(calibrator.alpha(), 0.1);
    }

    #[test]
    fn test_drift_calculation() {
        let mut calibrator = AdaptiveCalibrator::new(Duration::from_secs(30), 0.05);
        calibrator.baseline.mean = 0.5;
        calibrator.previous_mean = 0.4;
        
        let drift = calibrator.calculate_drift();
        assert!((drift - 0.1).abs() < f32::EPSILON);
        assert_eq!(calibrator.previous_mean, 0.5); // Should be updated
    }
}
