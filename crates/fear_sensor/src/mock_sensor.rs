//! Mock fear sensor for testing without hardware dependencies

use async_trait::async_trait;
use spectremesh_core::{FearScore, FearConfig, CameraDevice, FearError, CameraError};
use crate::{FearSensor, FearCalibrator};
use std::time::Duration;
use std::sync::{Arc, Mutex};

/// Shared calibration state
#[derive(Debug, Clone)]
struct CalibrationState {
    calibrated: bool,
    progress: f32,
}

/// Mock fear sensor that generates predictable fear sequences
pub struct MockFearSensor {
    pub fear_sequence: Vec<f32>,
    pub current_index: usize,
    calibrator: Option<FearCalibrator>,
    running: bool,
    calibration_state: Arc<Mutex<CalibrationState>>,
}

impl MockFearSensor {
    pub fn new(fear_sequence: Vec<f32>) -> Self {
        Self {
            fear_sequence,
            current_index: 0,
            calibrator: None,
            running: false,
            calibration_state: Arc::new(Mutex::new(CalibrationState {
                calibrated: false,
                progress: 0.0,
            })),
        }
    }

    /// Create a mock sensor with a sine wave pattern
    pub fn sine_wave(amplitude: f32, frequency: f32, samples: usize) -> Self {
        let mut sequence = Vec::with_capacity(samples);
        for i in 0..samples {
            let t = i as f32 / samples as f32;
            let value = 0.5 + amplitude * (2.0 * std::f32::consts::PI * frequency * t).sin();
            sequence.push(value.clamp(0.0, 1.0));
        }
        Self::new(sequence)
    }

    /// Create a mock sensor with step changes for testing
    pub fn step_pattern() -> Self {
        Self::new(vec![0.1, 0.1, 0.1, 0.8, 0.8, 0.8, 0.3, 0.3, 0.9, 0.9])
    }


}

#[async_trait]
impl FearSensor for MockFearSensor {
    async fn initialize(&mut self, config: &FearConfig) -> Result<(), FearError> {
        self.calibrator = Some(FearCalibrator::new(
            config.calibration_duration,
            config.camera.fps as f32,
        ));
        self.running = false;

        // Reset calibration state
        if let Ok(mut state) = self.calibration_state.lock() {
            state.calibrated = false;
            state.progress = 0.0;
        }

        Ok(())
    }

    async fn start(&mut self) -> Result<async_channel::Receiver<FearScore>, FearError> {
        if self.calibrator.is_none() {
            return Err(FearError::OnnxRuntime {
                message: "Sensor not initialized".to_string(),
            });
        }

        let (sender, receiver) = async_channel::unbounded();
        self.running = true;

        // Clone data needed for the async task
        let fear_sequence = self.fear_sequence.clone();
        let mut current_index = 0;
        let mut calibrator = self.calibrator.take().unwrap();
        let calibration_state = self.calibration_state.clone();

        // Spawn task to send mock fear scores
        tokio::spawn(async move {
            let mut frame_count = 0;
            
            loop {
                // Generate mock emotion logits with fear component
                let fear_value = if fear_sequence.is_empty() {
                    0.3
                } else {
                    let value = fear_sequence[current_index];
                    current_index = (current_index + 1) % fear_sequence.len();
                    value
                };

                // Create mock emotion logits [angry, disgust, fear, happy, sad, surprise, neutral]
                let raw_logits = [0.1, 0.1, fear_value, 0.1, 0.1, 0.1, 0.5];
                let fear_logit = FearCalibrator::extract_fear_logit(&raw_logits);

                // Add to calibration if not yet calibrated
                if !calibrator.is_calibrated() {
                    let _ = calibrator.add_sample(fear_logit);

                    // Update shared calibration state
                    if let Ok(mut state) = calibration_state.lock() {
                        state.calibrated = calibrator.is_calibrated();
                        state.progress = calibrator.calibration_progress();
                    }
                }

                // Create fear score
                let score = if calibrator.is_calibrated() {
                    let normalized_fear = calibrator.normalize_fear(fear_logit);
                    FearScore::new_calibrated(normalized_fear, raw_logits, 0.9)
                } else {
                    FearScore::new_uncalibrated(raw_logits, 0.9)
                };

                if sender.send(score).await.is_err() {
                    break; // Receiver dropped
                }

                frame_count += 1;
                
                // Simulate ~30 FPS
                tokio::time::sleep(Duration::from_millis(33)).await;
                
                // Stop after reasonable number of frames for testing
                if frame_count > 1000 {
                    break;
                }
            }
        });

        Ok(receiver)
    }

    async fn stop(&mut self) -> Result<(), FearError> {
        self.running = false;
        Ok(())
    }

    async fn enumerate_cameras(&self) -> Result<Vec<CameraDevice>, CameraError> {
        Ok(vec![CameraDevice::new(
            999,
            "Mock Camera".to_string(),
            (640, 480),
        )])
    }

    fn is_calibrated(&self) -> bool {
        self.calibration_state
            .lock()
            .map(|state| state.calibrated)
            .unwrap_or(false)
    }

    fn calibration_progress(&self) -> f32 {
        self.calibration_state
            .lock()
            .map(|state| state.progress)
            .unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_sensor_initialization() {
        let mut sensor = MockFearSensor::new(vec![0.1, 0.5, 0.9]);
        let config = FearConfig::default();
        
        assert!(sensor.initialize(&config).await.is_ok());
        assert!(!sensor.is_calibrated());
        assert_eq!(sensor.calibration_progress(), 0.0);
    }

    #[tokio::test]
    async fn test_mock_sensor_camera_enumeration() {
        let sensor = MockFearSensor::new(vec![0.5]);
        let cameras = sensor.enumerate_cameras().await.unwrap();
        
        assert_eq!(cameras.len(), 1);
        assert_eq!(cameras[0].id, 999);
        assert_eq!(cameras[0].name, "Mock Camera");
    }

    #[tokio::test]
    async fn test_mock_sensor_fear_generation() {
        let mut sensor = MockFearSensor::new(vec![0.1, 0.8, 0.3]);
        let config = FearConfig {
            calibration_duration: 0.1, // Very short for testing
            ..FearConfig::default()
        };
        
        sensor.initialize(&config).await.unwrap();
        let receiver = sensor.start().await.unwrap();
        
        // Receive a few fear scores
        let score1 = receiver.recv().await.unwrap();
        let score2 = receiver.recv().await.unwrap();
        let score3 = receiver.recv().await.unwrap();
        
        // Should cycle through the sequence
        assert_eq!(score1.extract_fear_logit(), 0.1);
        assert_eq!(score2.extract_fear_logit(), 0.8);
        assert_eq!(score3.extract_fear_logit(), 0.3);
    }

    #[test]
    fn test_sine_wave_generation() {
        let sensor = MockFearSensor::sine_wave(0.4, 1.0, 100);
        assert_eq!(sensor.fear_sequence.len(), 100);
        
        // Check that values are in valid range
        for &value in &sensor.fear_sequence {
            assert!(value >= 0.0 && value <= 1.0);
        }
        
        // First value should be around 0.5 (sine starts at 0)
        assert!((sensor.fear_sequence[0] - 0.5).abs() < 0.1);
    }

    #[test]
    fn test_step_pattern() {
        let sensor = MockFearSensor::step_pattern();
        assert_eq!(sensor.fear_sequence.len(), 10);
        assert_eq!(sensor.fear_sequence[0], 0.1);
        assert_eq!(sensor.fear_sequence[3], 0.8);
        assert_eq!(sensor.fear_sequence[8], 0.9);
    }

    #[tokio::test]
    async fn test_mock_sensor_calibration() {
        let mut sensor = MockFearSensor::new(vec![0.2; 10]); // Constant fear for calibration
        let config = FearConfig {
            calibration_duration: 0.3, // 0.3 seconds
            camera: spectremesh_core::CameraConfig {
                fps: 30, // 30 FPS = ~9 samples for calibration
                ..Default::default()
            },
            ..FearConfig::default()
        };
        
        sensor.initialize(&config).await.unwrap();
        let receiver = sensor.start().await.unwrap();
        
        // Wait for calibration to complete
        let mut calibrated = false;
        for _ in 0..20 {
            let _score = receiver.recv().await.unwrap();
            if sensor.is_calibrated() {
                calibrated = true;
                break;
            }
        }
        
        assert!(calibrated, "Sensor should be calibrated after receiving enough samples");
    }
}
