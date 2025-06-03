//! Core trait for fear detection implementations

use async_trait::async_trait;
use spectremesh_core::{FearScore, FearConfig, CameraDevice, FearError, CameraError};

/// Core trait for fear detection implementations
#[async_trait]
pub trait FearSensor: Send + Sync {
    /// Initialize the sensor with given configuration
    async fn initialize(&mut self, config: &FearConfig) -> Result<(), FearError>;

    /// Start continuous fear detection
    async fn start(&mut self) -> Result<async_channel::Receiver<FearScore>, FearError>;

    /// Stop detection and cleanup resources
    async fn stop(&mut self) -> Result<(), FearError>;

    /// Get available camera devices
    async fn enumerate_cameras(&self) -> Result<Vec<CameraDevice>, CameraError>;

    /// Check if sensor is currently calibrated
    fn is_calibrated(&self) -> bool;

    /// Get current calibration progress [0.0, 1.0]
    fn calibration_progress(&self) -> f32;
}

#[cfg(test)]
mod tests {
    use super::*;


    // Mock implementation for testing
    struct TestFearSensor {
        calibrated: bool,
        progress: f32,
    }

    #[async_trait]
    impl FearSensor for TestFearSensor {
        async fn initialize(&mut self, _config: &FearConfig) -> Result<(), FearError> {
            Ok(())
        }

        async fn start(&mut self) -> Result<async_channel::Receiver<FearScore>, FearError> {
            let (sender, receiver) = async_channel::unbounded();
            
            // Send a test fear score
            let test_score = FearScore::new_calibrated(
                0.5, 
                [0.1, 0.1, 0.5, 0.1, 0.1, 0.1, 0.1], 
                0.9
            );
            sender.send(test_score).await.unwrap();
            
            Ok(receiver)
        }

        async fn stop(&mut self) -> Result<(), FearError> {
            Ok(())
        }

        async fn enumerate_cameras(&self) -> Result<Vec<CameraDevice>, CameraError> {
            Ok(vec![CameraDevice::new(0, "Test Camera".to_string(), (640, 480))])
        }

        fn is_calibrated(&self) -> bool {
            self.calibrated
        }

        fn calibration_progress(&self) -> f32 {
            self.progress
        }
    }

    #[tokio::test]
    async fn test_fear_sensor_trait() {
        let mut sensor = TestFearSensor {
            calibrated: false,
            progress: 0.5,
        };

        // Test initialization
        let config = FearConfig::default();
        assert!(sensor.initialize(&config).await.is_ok());

        // Test calibration status
        assert!(!sensor.is_calibrated());
        assert_eq!(sensor.calibration_progress(), 0.5);

        // Test camera enumeration
        let cameras = sensor.enumerate_cameras().await.unwrap();
        assert_eq!(cameras.len(), 1);
        assert_eq!(cameras[0].name, "Test Camera");

        // Test starting sensor
        let receiver = sensor.start().await.unwrap();
        let fear_score = receiver.recv().await.unwrap();
        assert_eq!(fear_score.value, 0.5);
        assert!(fear_score.calibrated);

        // Test stopping sensor
        assert!(sensor.stop().await.is_ok());
    }
}
