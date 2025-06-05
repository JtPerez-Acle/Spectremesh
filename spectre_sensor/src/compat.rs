//! Compatibility layer for legacy FearSensor trait
//! 
//! This module provides a compatibility wrapper that implements the legacy FearSensor trait
//! for the modern EmotionSensor, enabling seamless migration from Haar cascade to YuNet
//! face detection without breaking existing code.

use async_trait::async_trait;
use spectremesh_core::{FearScore, FearConfig, CameraDevice, FearError, CameraError};
use crate::{
    sensor::{EmotionSensor, SensorError},
    types::FearFrame,
    config::SensorConfig,
};
use async_channel::Receiver;
use std::time::Duration;
use std::sync::{Arc, Mutex};

/// Legacy FearSensor trait for compatibility
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

/// YuNet-based fear sensor that implements the legacy FearSensor trait
pub struct YuNetFearSensor {
    emotion_sensor: EmotionSensor,
    frame_receiver: Option<Receiver<FearFrame>>,
}

impl YuNetFearSensor {
    /// Create a new YuNet fear sensor
    pub fn new() -> Self {
        let config = SensorConfig::default();
        Self {
            emotion_sensor: EmotionSensor::new(config),
            frame_receiver: None,
        }
    }
}

impl Default for YuNetFearSensor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FearSensor for YuNetFearSensor {
    async fn initialize(&mut self, config: &FearConfig) -> Result<(), FearError> {
        // Convert FearConfig to SensorConfig
        let sensor_config = convert_fear_config_to_sensor_config(config);
        
        // Update the emotion sensor's configuration
        self.emotion_sensor = EmotionSensor::new(sensor_config);
        
        // Initialize the emotion sensor
        self.emotion_sensor.initialize().await
            .map_err(convert_sensor_error_to_fear_error)?;
        
        Ok(())
    }

    async fn start(&mut self) -> Result<async_channel::Receiver<FearScore>, FearError> {
        // Start the emotion sensor
        let frame_receiver = self.emotion_sensor.start().await
            .map_err(convert_sensor_error_to_fear_error)?;
        
        // Create a channel for FearScore output
        let (score_sender, score_receiver) = async_channel::bounded(2);
        
        // Store the frame receiver for later cleanup
        self.frame_receiver = Some(frame_receiver.clone());
        
        // Spawn a task to convert FearFrame to FearScore
        tokio::spawn(async move {
            while let Ok(fear_frame) = frame_receiver.recv().await {
                let fear_score = convert_fear_frame_to_fear_score(fear_frame);
                
                // Try to send with back-pressure handling
                match score_sender.try_send(fear_score) {
                    Ok(_) => {},
                    Err(async_channel::TrySendError::Full(_)) => {
                        tracing::debug!("Dropped frame due to back-pressure in YuNet sensor");
                    },
                    Err(async_channel::TrySendError::Closed(_)) => {
                        break; // Receiver dropped
                    }
                }
            }
        });
        
        Ok(score_receiver)
    }

    async fn stop(&mut self) -> Result<(), FearError> {
        self.emotion_sensor.stop().await
            .map_err(convert_sensor_error_to_fear_error)?;
        
        // Clear the frame receiver
        self.frame_receiver = None;
        
        Ok(())
    }

    async fn enumerate_cameras(&self) -> Result<Vec<CameraDevice>, CameraError> {
        // For now, return a simple camera enumeration
        // The EmotionSensor doesn't have a direct camera enumeration method
        // so we'll provide a basic implementation
        Ok(vec![
            CameraDevice::new(0, "Default Camera".to_string(), (640, 480)),
        ])
    }

    fn is_calibrated(&self) -> bool {
        // Get calibration status from the emotion sensor state
        let state = self.emotion_sensor.get_state();
        state.calibrated
    }

    fn calibration_progress(&self) -> f32 {
        // Get calibration progress from the emotion sensor state
        let state = self.emotion_sensor.get_state();
        state.calibration_progress
    }
}

/// Convert FearConfig to SensorConfig
fn convert_fear_config_to_sensor_config(fear_config: &FearConfig) -> SensorConfig {
    SensorConfig {
        emotion_model_path: Some(fear_config.model_path.clone()),
        onnx_threads: num_cpus::get().min(4), // Reasonable default
        freeze_calibration: false,
        camera_id: fear_config.camera.device_id,
        target_fps: fear_config.camera.fps as f32,
        channel_buffer_size: 2,
        metrics_port: 9090,
        grpc_socket_path: "/tmp/spectre_sensor.sock".to_string(),
    }
}

/// Convert FearFrame to FearScore
fn convert_fear_frame_to_fear_score(fear_frame: FearFrame) -> FearScore {
    if fear_frame.calibrated {
        FearScore::new_calibrated(
            fear_frame.fear_score,
            fear_frame.emotion_logits,
            fear_frame.confidence,
        )
    } else {
        FearScore::new_uncalibrated(
            fear_frame.fear_score,
            fear_frame.emotion_logits,
            fear_frame.confidence,
        )
    }
}

/// Convert SensorError to FearError
fn convert_sensor_error_to_fear_error(sensor_error: SensorError) -> FearError {
    match sensor_error {
        SensorError::OnnxEnvironment(msg) => FearError::OnnxRuntime { message: msg },
        SensorError::ModelLoading(msg) => FearError::model_not_found(msg),
        SensorError::CameraInit(msg) => FearError::OnnxRuntime { message: format!("Camera init: {}", msg) },
        SensorError::FrameProcessing(msg) => FearError::OnnxRuntime { message: format!("Frame processing: {}", msg) },
        SensorError::FaceDetection(_) => FearError::NoFaceDetected,
        SensorError::Calibration(_) => FearError::OnnxRuntime { message: "Calibration error".to_string() },
        SensorError::ChannelError => FearError::OnnxRuntime { message: "Channel communication error".to_string() },
        SensorError::NotInitialized => FearError::OnnxRuntime { message: "Sensor not initialized".to_string() },
    }
}

/// Shared state for mock sensor calibration tracking
#[derive(Debug, Clone)]
struct MockCalibrationState {
    calibrated: bool,
    progress: f32,
    samples: usize,
    target: usize,
}

/// Mock fear sensor for testing without hardware dependencies
pub struct MockFearSensor {
    pub fear_sequence: Vec<f32>,
    pub current_index: usize,
    calibration_state: Arc<Mutex<MockCalibrationState>>,
}

impl MockFearSensor {
    /// Create a new mock sensor with a constant fear level
    pub fn new(fear_sequence: Vec<f32>) -> Self {
        Self {
            fear_sequence,
            current_index: 0,
            calibration_state: Arc::new(Mutex::new(MockCalibrationState {
                calibrated: false,
                progress: 0.0,
                samples: 0,
                target: 20, // 20 samples for calibration
            })),
        }
    }

    /// Create a step pattern sensor (low → high → low)
    pub fn step_pattern() -> Self {
        Self::new(vec![0.1, 0.2, 0.3, 0.7, 0.8, 0.9, 0.8, 0.7, 0.3, 0.2, 0.1])
    }

    /// Create a sine wave pattern sensor
    pub fn sine_pattern(center: f32, amplitude: f32, period: f32) -> Self {
        let mut sequence = Vec::new();
        for i in 0..100 {
            let t = i as f32 * period / 100.0;
            let value = center + amplitude * (t * 2.0 * std::f32::consts::PI).sin();
            sequence.push(value.clamp(0.0, 1.0));
        }
        Self::new(sequence)
    }
}

#[async_trait]
impl FearSensor for MockFearSensor {
    async fn initialize(&mut self, _config: &FearConfig) -> Result<(), FearError> {
        self.current_index = 0;
        let mut state = self.calibration_state.lock().unwrap();
        state.calibrated = false;
        state.progress = 0.0;
        state.samples = 0;
        Ok(())
    }

    async fn start(&mut self) -> Result<async_channel::Receiver<FearScore>, FearError> {
        let (sender, receiver) = async_channel::bounded(2);
        let fear_sequence = self.fear_sequence.clone();
        let mut current_index = self.current_index;
        let calibration_state = Arc::clone(&self.calibration_state);

        tokio::spawn(async move {
            loop {
                // Get next fear value
                let fear_value = fear_sequence[current_index % fear_sequence.len()];
                current_index += 1;

                // Update calibration progress
                let calibrated = {
                    let mut state = calibration_state.lock().unwrap();
                    state.samples += 1;
                    state.progress = (state.samples as f32 / state.target as f32).min(1.0);
                    state.calibrated = state.samples >= state.target;
                    state.calibrated
                };

                // Create mock emotion logits with fear at index 2
                let mut emotion_logits = [0.1; 7];
                emotion_logits[2] = fear_value; // Fear is at index 2

                // Create fear score
                let score = if calibrated {
                    FearScore::new_calibrated(fear_value, emotion_logits, 0.9)
                } else {
                    FearScore::new_uncalibrated(fear_value, emotion_logits, 0.9)
                };

                // Try to send with back-pressure handling
                match sender.try_send(score) {
                    Ok(_) => {},
                    Err(async_channel::TrySendError::Full(_)) => {
                        tracing::debug!("Dropped frame due to back-pressure in mock sensor");
                    },
                    Err(async_channel::TrySendError::Closed(_)) => {
                        break; // Receiver dropped
                    }
                }

                // Simulate ~30 FPS
                tokio::time::sleep(Duration::from_millis(33)).await;
            }
        });

        Ok(receiver)
    }

    async fn stop(&mut self) -> Result<(), FearError> {
        Ok(())
    }

    async fn enumerate_cameras(&self) -> Result<Vec<CameraDevice>, CameraError> {
        Ok(vec![
            CameraDevice::new(0, "Mock Camera".to_string(), (640, 480)),
        ])
    }

    fn is_calibrated(&self) -> bool {
        self.calibration_state.lock().unwrap().calibrated
    }

    fn calibration_progress(&self) -> f32 {
        self.calibration_state.lock().unwrap().progress
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_fear_config_to_sensor_config_conversion() {
        let fear_config = FearConfig {
            model_path: "test_model.onnx".to_string(),
            camera: spectremesh_core::CameraConfig {
                device_id: 1,
                fps: 60,
                width: 1280,
                height: 720,
            },
            calibration_duration: Duration::from_secs(45),
            debug: true,
            inference_timeout: Duration::from_millis(200),
        };

        let sensor_config = convert_fear_config_to_sensor_config(&fear_config);

        assert_eq!(sensor_config.emotion_model_path, Some("test_model.onnx".to_string()));
        assert_eq!(sensor_config.camera_id, 1);
        assert_eq!(sensor_config.target_fps, 60.0);
    }

    #[test]
    fn test_fear_frame_to_fear_score_conversion() {
        let emotion_logits = [0.1, 0.1, 0.8, 0.1, 0.1, 0.1, 0.1];
        let fear_frame = FearFrame::new(
            0.75,
            emotion_logits,
            0.9,
            true,
            Duration::from_millis(5),
        );

        let fear_score = convert_fear_frame_to_fear_score(fear_frame);

        assert_eq!(fear_score.value, 0.75);
        assert_eq!(fear_score.emotion_logits, emotion_logits);
        assert_eq!(fear_score.confidence, 0.9);
        assert!(fear_score.calibrated);
        assert_eq!(fear_score.extract_fear_logit(), 0.8);
    }

    #[tokio::test]
    async fn test_mock_fear_sensor_basic_functionality() {
        let mut sensor = MockFearSensor::new(vec![0.3, 0.5, 0.7]);
        let config = FearConfig::default();

        // Test initialization
        assert!(sensor.initialize(&config).await.is_ok());
        assert!(!sensor.is_calibrated());
        assert_eq!(sensor.calibration_progress(), 0.0);

        // Test camera enumeration
        let cameras = sensor.enumerate_cameras().await.unwrap();
        assert_eq!(cameras.len(), 1);
        assert_eq!(cameras[0].name, "Mock Camera");

        // Test stop
        assert!(sensor.stop().await.is_ok());
    }

    #[test]
    fn test_mock_fear_sensor_patterns() {
        let step_sensor = MockFearSensor::step_pattern();
        assert!(!step_sensor.fear_sequence.is_empty());

        let sine_sensor = MockFearSensor::sine_pattern(0.5, 0.3, 2.0);
        assert_eq!(sine_sensor.fear_sequence.len(), 100);

        // Check that values are within valid range
        for &value in &sine_sensor.fear_sequence {
            assert!(value >= 0.0 && value <= 1.0);
        }
    }

    #[test]
    fn test_yunet_fear_sensor_creation() {
        let sensor = YuNetFearSensor::new();
        assert!(!sensor.is_calibrated());
        assert_eq!(sensor.calibration_progress(), 0.0);
    }
}
