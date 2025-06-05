//! High-performance sensor implementation with YuNet and optimized ONNX Runtime

use crate::{
    types::*,
    yunet::{YuNetDetector, YuNetError},
    calibrator::{AdaptiveCalibrator, CalibrationError},
    config::SensorConfig,
};
use opencv::{
    core::{Mat, Rect, Size},
    imgproc,
    videoio::{VideoCapture, CAP_ANY},
    prelude::*,
};
use ort::{
    session::{Session, builder::GraphOptimizationLevel},
    value::Tensor,
};

use async_channel::{Sender, Receiver, bounded};
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use tokio::time::sleep;
use thiserror::Error;

/// Sensor errors
#[derive(Debug, Error)]
pub enum SensorError {
    #[error("Camera initialization failed: {0}")]
    CameraInit(String),
    
    #[error("ONNX environment creation failed: {0}")]
    OnnxEnvironment(String),
    
    #[error("Emotion model loading failed: {0}")]
    ModelLoading(String),
    
    #[error("Face detection error: {0}")]
    FaceDetection(#[from] YuNetError),
    
    #[error("Calibration error: {0}")]
    Calibration(#[from] CalibrationError),
    
    #[error("Frame processing failed: {0}")]
    FrameProcessing(String),
    
    #[error("Channel communication error")]
    ChannelError,
    
    #[error("Sensor not initialized")]
    NotInitialized,
}

/// Shared sensor state for thread communication
#[derive(Debug, Clone)]
pub struct SensorState {
    pub running: bool,
    pub calibration_progress: f32,
    pub calibrated: bool,
    pub last_error: Option<String>,
    pub metrics: PerformanceMetrics,
}

impl Default for SensorState {
    fn default() -> Self {
        Self {
            running: false,
            calibration_progress: 0.0,
            calibrated: false,
            last_error: None,
            metrics: PerformanceMetrics::new(),
        }
    }
}

/// High-performance emotion sensor with YuNet face detection
pub struct EmotionSensor {
    /// YuNet face detector
    face_detector: Option<YuNetDetector>,
    /// Emotion recognition session
    emotion_session: Option<Session>,
    /// Adaptive calibrator
    calibrator: Option<AdaptiveCalibrator>,
    /// Sensor configuration
    config: SensorConfig,
    /// Shared state for monitoring
    state: Arc<Mutex<SensorState>>,
    /// Performance metrics tracking
    #[allow(dead_code)]
    latency_samples: Vec<Duration>,
}

impl EmotionSensor {
    /// Create a new emotion sensor
    pub fn new(config: SensorConfig) -> Self {
        Self {
            face_detector: None,
            emotion_session: None,
            calibrator: None,
            config,
            state: Arc::new(Mutex::new(SensorState::default())),
            latency_samples: Vec::new(),
        }
    }

    /// Initialize the sensor with ONNX environment and models
    pub async fn initialize(&mut self) -> Result<(), SensorError> {
        // Initialize ONNX Runtime environment (global initialization)
        ort::init()
            .commit()
            .map_err(|e| SensorError::OnnxEnvironment(e.to_string()))?;

        // Initialize YuNet face detector
        let face_detector = if let Some(model_path) = &self.config.emotion_model_path {
            YuNetDetector::from_file(model_path, self.config.onnx_threads)?
        } else {
            YuNetDetector::new(self.config.onnx_threads)?
        };

        // Load emotion recognition model
        let emotion_session = self.load_emotion_model()?;

        // Initialize adaptive calibrator
        let calibrator = AdaptiveCalibrator::with_defaults(Duration::from_secs(30));

        self.face_detector = Some(face_detector);
        self.emotion_session = Some(emotion_session);
        self.calibrator = Some(calibrator);

        tracing::info!("Sensor initialized with {} ONNX threads", self.config.onnx_threads);
        Ok(())
    }

    /// Start the sensor and return a channel receiver for fear frames
    pub async fn start(&mut self) -> Result<Receiver<FearFrame>, SensorError> {
        if self.face_detector.is_none() || self.emotion_session.is_none() {
            return Err(SensorError::NotInitialized);
        }

        let (sender, receiver) = bounded(self.config.channel_buffer_size);
        
        // Update state
        {
            let mut state = self.state.lock().unwrap();
            state.running = true;
            state.last_error = None;
        }

        // Spawn processing task
        let face_detector = self.face_detector.take().unwrap();
        let emotion_session = self.emotion_session.take().unwrap();
        let mut calibrator = self.calibrator.take().unwrap();
        let config = self.config.clone();
        let state = Arc::clone(&self.state);

        tokio::spawn(async move {
            if let Err(e) = Self::processing_loop(
                face_detector,
                emotion_session,
                &mut calibrator,
                sender,
                config,
                state,
            ).await {
                tracing::error!("Sensor processing loop failed: {}", e);
            }
        });

        Ok(receiver)
    }

    /// Main processing loop
    async fn processing_loop(
        mut face_detector: YuNetDetector,
        mut emotion_session: Session,
        calibrator: &mut AdaptiveCalibrator,
        sender: Sender<FearFrame>,
        config: SensorConfig,
        state: Arc<Mutex<SensorState>>,
    ) -> Result<(), SensorError> {
        // Check camera permissions first
        if let Err(e) = crate::permissions::check_camera_permissions().await {
            tracing::warn!("Camera permission check failed: {}", e);
            crate::permissions::provide_camera_troubleshooting_guidance();
        }

        // Initialize camera with enhanced error reporting
        let mut camera = Self::initialize_camera_with_backend_detection(config.camera_id)?;

        let frame_duration = Duration::from_secs_f32(1.0 / config.target_fps);
        let mut frame_count = 0u64;
        let mut last_metrics_update = Instant::now();
        let mut latency_samples = Vec::new();

        loop {
            let frame_start = Instant::now();

            // Check if we should stop
            {
                let state_guard = state.lock().unwrap();
                if !state_guard.running {
                    break;
                }
            }

            // Capture frame
            let mut frame = Mat::default();
            if !camera.read(&mut frame).unwrap_or(false) || frame.empty() {
                sleep(frame_duration).await;
                continue;
            }

            // Process frame
            match Self::process_frame(
                &frame,
                &mut face_detector,
                &mut emotion_session,
                calibrator,
            ).await {
                Ok(fear_frame) => {
                    latency_samples.push(fear_frame.inference_latency);
                    
                    // Try to send frame (non-blocking with back-pressure)
                    match sender.try_send(fear_frame) {
                        Ok(_) => {},
                        Err(async_channel::TrySendError::Full(_)) => {
                            // Channel full, drop oldest frame
                            let mut state_guard = state.lock().unwrap();
                            state_guard.metrics.record_dropped_frame();
                            tracing::debug!("Dropped frame due to back-pressure");
                        },
                        Err(async_channel::TrySendError::Closed(_)) => {
                            tracing::info!("Receiver closed, stopping sensor");
                            break;
                        }
                    }
                },
                Err(e) => {
                    tracing::warn!("Frame processing failed: {}", e);
                    let mut state_guard = state.lock().unwrap();
                    state_guard.last_error = Some(e.to_string());
                }
            }

            frame_count += 1;

            // Update metrics periodically
            if last_metrics_update.elapsed() >= Duration::from_secs(1) {
                let mut state_guard = state.lock().unwrap();
                state_guard.metrics.update_fps(frame_count, last_metrics_update.elapsed());
                state_guard.metrics.update_inference_latency(&latency_samples);
                state_guard.calibration_progress = calibrator.progress();
                state_guard.calibrated = calibrator.is_calibrated();
                state_guard.metrics.calibration_drift = calibrator.calculate_drift();
                
                last_metrics_update = Instant::now();
                frame_count = 0;
                latency_samples.clear();
            }

            // Maintain target FPS
            let elapsed = frame_start.elapsed();
            if elapsed < frame_duration {
                sleep(frame_duration - elapsed).await;
            }
        }

        // Update state on exit
        {
            let mut state_guard = state.lock().unwrap();
            state_guard.running = false;
        }

        Ok(())
    }

    /// Process a single frame to extract fear score
    async fn process_frame(
        frame: &Mat,
        face_detector: &mut YuNetDetector,
        emotion_session: &mut Session,
        calibrator: &mut AdaptiveCalibrator,
    ) -> Result<FearFrame, SensorError> {
        let inference_start = Instant::now();

        // Detect largest face
        let face_detection = face_detector.get_largest_face(frame)?;

        // Crop face region
        let face_roi = Self::crop_face_region(frame, &face_detection.bbox)?;

        // Run emotion recognition
        let emotion_logits = Self::run_emotion_inference(&face_roi, emotion_session).await?;

        let inference_latency = inference_start.elapsed();

        // Extract fear logit and update calibrator
        let fear_logit = emotion_logits[2]; // Fear is at index 2
        calibrator.add_sample(fear_logit)?;

        // Normalize fear score
        let normalized_fear = calibrator.normalize_fear(fear_logit);

        Ok(FearFrame::new(
            normalized_fear,
            emotion_logits,
            face_detection.confidence,
            calibrator.is_calibrated(),
            inference_latency,
        ))
    }

    /// Load emotion recognition model
    fn load_emotion_model(&self) -> Result<Session, SensorError> {
        // For this implementation, we'll assume the emotion model is also embedded
        // In practice, you'd load from a file or embed it like YuNet
        let model_path = self.config.emotion_model_path
            .as_deref()
            .unwrap_or("assets/models/face_emotion.onnx");

        Session::builder()
            .map_err(|e| SensorError::ModelLoading(e.to_string()))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| SensorError::ModelLoading(e.to_string()))?
            .with_intra_threads(self.config.onnx_threads)
            .map_err(|e| SensorError::ModelLoading(e.to_string()))?
            .commit_from_file(model_path)
            .map_err(|e| SensorError::ModelLoading(e.to_string()))
    }

    /// Crop face region from frame
    fn crop_face_region(frame: &Mat, bbox: &Rect) -> Result<Mat, SensorError> {
        let roi = Mat::roi(frame, *bbox)
            .map_err(|e| SensorError::FrameProcessing(e.to_string()))?;
        
        // Resize to emotion model input size (typically 48x48)
        let mut resized = Mat::default();
        imgproc::resize(
            &roi,
            &mut resized,
            Size::new(48, 48),
            0.0,
            0.0,
            imgproc::INTER_LINEAR,
        )
        .map_err(|e| SensorError::FrameProcessing(e.to_string()))?;

        Ok(resized)
    }

    /// Run emotion inference on face image
    async fn run_emotion_inference(
        face_image: &Mat,
        session: &mut Session,
    ) -> Result<[f32; 7], SensorError> {
        // Convert to grayscale
        let mut gray = Mat::default();
        imgproc::cvt_color(face_image, &mut gray, imgproc::COLOR_BGR2GRAY, 0)
            .map_err(|e| SensorError::FrameProcessing(e.to_string()))?;

        // Convert to float and normalize
        let mut float_img = Mat::default();
        gray.convert_to(&mut float_img, opencv::core::CV_32F, 1.0 / 255.0, 0.0)
            .map_err(|e| SensorError::FrameProcessing(e.to_string()))?;

        // Convert to ndarray format (NCHW)
        let data = float_img.data_typed::<f32>()
            .map_err(|e| SensorError::FrameProcessing(e.to_string()))?;
        
        let input_tensor = Tensor::from_array(([1, 1, 48, 48], data.to_vec()))
            .map_err(|e| SensorError::FrameProcessing(e.to_string()))?;

        // Run inference
        let outputs = session
            .run(ort::inputs!["input" => input_tensor])
            .map_err(|e| SensorError::FrameProcessing(e.to_string()))?;

        // Extract emotion logits
        let output = outputs.get("output")
            .ok_or_else(|| SensorError::FrameProcessing("Missing output tensor".to_string()))?;
        let (_, output_data) = output
            .try_extract_tensor::<f32>()
            .map_err(|_| SensorError::FrameProcessing("Invalid output format".to_string()))?;

        if output_data.len() < 7 {
            return Err(SensorError::FrameProcessing("Insufficient output dimensions".to_string()));
        }

        let mut emotion_logits = [0.0f32; 7];
        for (i, &value) in output_data.iter().take(7).enumerate() {
            emotion_logits[i] = value;
        }

        Ok(emotion_logits)
    }

    /// Stop the sensor
    pub async fn stop(&mut self) -> Result<(), SensorError> {
        let mut state = self.state.lock().unwrap();
        state.running = false;
        Ok(())
    }

    /// Get current sensor state
    pub fn get_state(&self) -> SensorState {
        self.state.lock().unwrap().clone()
    }

    /// Control calibration
    pub fn control_calibration(&mut self, freeze: bool) -> Result<(), SensorError> {
        if let Some(calibrator) = &mut self.calibrator {
            if freeze {
                calibrator.freeze();
            } else {
                calibrator.unfreeze();
            }
        }
        Ok(())
    }

    /// Reset calibration
    pub fn reset_calibration(&mut self) -> Result<(), SensorError> {
        if let Some(calibrator) = &mut self.calibrator {
            calibrator.reset();
        }
        Ok(())
    }

    /// Initialize camera with enhanced error reporting and backend detection
    fn initialize_camera_with_backend_detection(camera_id: u32) -> Result<VideoCapture, SensorError> {
        let camera = VideoCapture::new(camera_id as i32, CAP_ANY)
            .map_err(|e| SensorError::CameraInit(format!("Failed to create camera {}: {}", camera_id, e)))?;

        if !camera.is_opened().unwrap_or(false) {
            return Err(SensorError::CameraInit(format!(
                "Camera {} failed to open. Available backends: {}",
                camera_id,
                Self::get_available_backends()
            )));
        }

        // Log camera backend information
        let backend_name = Self::get_camera_backend_name(&camera);
        tracing::info!("Camera {} initialized with backend: {}", camera_id, backend_name);

        // Validate camera properties
        let width = camera.get(opencv::videoio::CAP_PROP_FRAME_WIDTH).unwrap_or(0.0);
        let height = camera.get(opencv::videoio::CAP_PROP_FRAME_HEIGHT).unwrap_or(0.0);
        tracing::info!("Camera resolution: {}x{}", width, height);

        Ok(camera)
    }

    /// Get camera backend name
    fn get_camera_backend_name(camera: &VideoCapture) -> String {
        // Try to get backend name from OpenCV
        match camera.get_backend_name() {
            Ok(name) => name,
            Err(_) => {
                #[cfg(target_os = "windows")]
                return "DirectShow".to_string();
                #[cfg(target_os = "macos")]
                return "AVFoundation".to_string();
                #[cfg(target_os = "linux")]
                return "V4L2".to_string();
                #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
                return "Unknown".to_string();
            }
        }
    }

    /// Get available camera backends for the platform
    fn get_available_backends() -> String {
        #[cfg(target_os = "windows")]
        return "DirectShow, MSMF".to_string();
        #[cfg(target_os = "macos")]
        return "AVFoundation".to_string();
        #[cfg(target_os = "linux")]
        return "V4L2, GStreamer".to_string();
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        return "Platform-specific".to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensor_creation() {
        let config = SensorConfig::default();
        let sensor = EmotionSensor::new(config);
        
        let state = sensor.get_state();
        assert!(!state.running);
        assert!(!state.calibrated);
        assert_eq!(state.calibration_progress, 0.0);
    }

    #[tokio::test]
    async fn test_sensor_initialization() {
        let config = SensorConfig::default();
        let mut sensor = EmotionSensor::new(config);
        
        // Note: This test will fail without proper model files
        // In a real test environment, you'd mock the ONNX components
        let result = sensor.initialize().await;
        
        // We expect this to fail in the test environment due to missing models
        assert!(result.is_err());
    }
}
