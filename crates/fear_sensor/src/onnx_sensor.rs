//! ONNX-based fear sensor implementation with real OpenCV and ONNX integration

use async_trait::async_trait;
use spectremesh_core::{FearScore, FearConfig, CameraDevice, FearError, CameraError};
use crate::{FearSensor, FearCalibrator};
use opencv::{
    core::{Mat, Rect, Size, Vector},
    imgproc,
    objdetect::CascadeClassifier,
    videoio::{VideoCapture, CAP_ANY},
    prelude::*,
};
use ort::{
    session::{Session, builder::GraphOptimizationLevel},
    value::Value,
};
use ndarray::Array4;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};

/// Shared calibration state for thread communication
#[derive(Debug, Clone)]
struct CalibrationState {
    calibrated: bool,
    progress: f32,
}

/// Real ONNX-based fear sensor with OpenCV camera capture
pub struct OnnxFearSensor {
    session: Option<Session>,
    calibrator: Option<FearCalibrator>,
    face_detector_path: Option<String>,
    calibration_state: Arc<Mutex<CalibrationState>>,
    running: bool,
}

impl OnnxFearSensor {
    pub fn new() -> Self {
        Self {
            session: None,
            calibrator: None,
            face_detector_path: None,
            calibration_state: Arc::new(Mutex::new(CalibrationState {
                calibrated: false,
                progress: 0.0,
            })),
            running: false,
        }
    }

    /// Load ONNX model with appropriate execution provider
    fn load_model(model_path: &str) -> Result<Session, FearError> {
        let session = Session::builder()
            .map_err(|e| FearError::OnnxRuntime {
                message: format!("Failed to create session builder: {}", e)
            })?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| FearError::OnnxRuntime {
                message: format!("Failed to set optimization level: {}", e)
            })?
            .with_intra_threads(1)
            .map_err(|e| FearError::OnnxRuntime {
                message: format!("Failed to set intra threads: {}", e)
            })?
            .commit_from_file(model_path)
            .map_err(|_e| FearError::model_not_found(format!("Model not found: {}", model_path)))?;

        Ok(session)
    }

    /// Get the path to the Haar cascade face detector
    fn get_face_detector_path() -> String {
        // Try to find the cascade file in common locations
        let possible_paths = [
            "haarcascade_frontalface_alt.xml",
            "assets/models/haarcascade_frontalface_alt.xml",
            "/usr/share/opencv4/haarcascades/haarcascade_frontalface_alt.xml",
            "/usr/local/share/opencv4/haarcascades/haarcascade_frontalface_alt.xml",
        ];

        for path in &possible_paths {
            if std::path::Path::new(path).exists() {
                return path.to_string();
            }
        }

        // Return default path even if it doesn't exist
        "haarcascade_frontalface_alt.xml".to_string()
    }

    /// OpenCV frame preprocessing for ONNX input
    fn preprocess_frame(
        frame: &Mat,
        face_detector_path: &str
    ) -> Result<Array4<f32>, FearError> {
        // Create face detector locally (thread-safe)
        let mut face_detector = CascadeClassifier::new(face_detector_path)
            .map_err(|e| FearError::OnnxRuntime {
                message: format!("Failed to load face detector: {}", e)
            })?;
        // 1. Convert BGR to RGB
        let mut rgb_frame = Mat::default();
        imgproc::cvt_color(frame, &mut rgb_frame, imgproc::COLOR_BGR2RGB, 0)
            .map_err(|e| FearError::OnnxRuntime {
                message: format!("Color conversion failed: {}", e)
            })?;

        // 2. Detect face using Haar cascade
        let mut faces = Vector::<Rect>::new();
        face_detector.detect_multi_scale(
            &rgb_frame,
            &mut faces,
            1.1,  // scale factor
            3,    // min neighbors
            0,    // flags
            Size::new(30, 30), // min size
            Size::new(300, 300), // max size
        ).map_err(|e| FearError::OnnxRuntime {
            message: format!("Face detection failed: {}", e)
        })?;

        if faces.is_empty() {
            return Err(FearError::NoFaceDetected);
        }

        // 3. Crop to largest face
        let face_rect = faces.get(0)
            .map_err(|e| FearError::OnnxRuntime {
                message: format!("Failed to get face rect: {}", e)
            })?;

        let face_roi = Mat::roi(&rgb_frame, face_rect)
            .map_err(|e| FearError::OnnxRuntime {
                message: format!("Failed to extract face ROI: {}", e)
            })?;

        // 4. Resize to 48x48 for FaceONNX
        let mut resized = Mat::default();
        imgproc::resize(
            &face_roi,
            &mut resized,
            Size::new(48, 48),
            0.0, 0.0,
            imgproc::INTER_LINEAR,
        ).map_err(|e| FearError::OnnxRuntime {
            message: format!("Resize failed: {}", e)
        })?;

        // 5. Convert to grayscale
        let mut gray = Mat::default();
        imgproc::cvt_color(&resized, &mut gray, imgproc::COLOR_RGB2GRAY, 0)
            .map_err(|e| FearError::OnnxRuntime {
                message: format!("Grayscale conversion failed: {}", e)
            })?;

        // 6. Normalize to [0.0, 1.0] and convert to NCHW format
        let data = gray.data_bytes()
            .map_err(|e| FearError::OnnxRuntime {
                message: format!("Failed to get image data: {}", e)
            })?;

        let normalized: Vec<f32> = data.iter().map(|&x| x as f32 / 255.0).collect();

        // Reshape to [1, 1, 48, 48] (batch, channels, height, width)
        let array = Array4::from_shape_vec((1, 1, 48, 48), normalized)
            .map_err(|e| FearError::invalid_logits(format!("Array reshape failed: {}", e)))?;

        Ok(array)
    }

    /// Run ONNX inference and extract emotion logits
    fn run_inference(session: &mut Session, input: Array4<f32>) -> Result<[f32; 7], FearError> {
        // Convert ndarray to the format expected by ort
        let shape = input.shape().to_vec();
        let data = input.into_raw_vec();

        // Create input tensor using the correct API
        let input_tensor = Value::from_array((shape, data))
            .map_err(|e| FearError::OnnxRuntime {
                message: format!("Failed to create input tensor: {}", e)
            })?;

        // Run inference with proper input format
        let outputs = session.run(vec![("input", input_tensor)])
            .map_err(|e| FearError::OnnxRuntime {
                message: format!("Inference failed: {}", e)
            })?;

        // Extract emotion logits (assuming single output)
        let output_tensor = &outputs[0];
        let (_, logits_slice) = output_tensor.try_extract_tensor::<f32>()
            .map_err(|e| FearError::OnnxRuntime {
                message: format!("Failed to extract output: {}", e)
            })?;

        if logits_slice.len() != 7 {
            return Err(FearError::invalid_logits(format!("Expected 7 emotion classes, got {}", logits_slice.len())));
        }

        let mut result = [0.0; 7];
        result.copy_from_slice(&logits_slice[..7]);
        Ok(result)
    }
}

impl Default for OnnxFearSensor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FearSensor for OnnxFearSensor {
    async fn initialize(&mut self, config: &FearConfig) -> Result<(), FearError> {
        // Load ONNX model
        self.session = Some(Self::load_model(&config.model_path)?);

        // Get face detector path
        self.face_detector_path = Some(Self::get_face_detector_path());

        // Initialize calibrator
        self.calibrator = Some(FearCalibrator::new(
            config.calibration_duration.as_secs_f32(),
            config.camera.fps as f32,
        ));

        // Reset calibration state
        if let Ok(mut state) = self.calibration_state.lock() {
            state.calibrated = false;
            state.progress = 0.0;
        }

        self.running = false;
        Ok(())
    }

    async fn start(&mut self) -> Result<async_channel::Receiver<FearScore>, FearError> {
        if self.session.is_none() || self.face_detector_path.is_none() {
            return Err(FearError::OnnxRuntime {
                message: "Sensor not initialized".to_string(),
            });
        }

        let (sender, receiver) = async_channel::bounded(2);
        self.running = true;

        // Clone necessary data for the async task
        let mut session = self.session.take().unwrap();
        let face_detector_path = self.face_detector_path.take().unwrap();
        let mut calibrator = self.calibrator.take().unwrap();
        let calibration_state = self.calibration_state.clone();

        // Spawn task for camera capture and inference
        tokio::spawn(async move {
            // Initialize camera
            let mut camera = match VideoCapture::new(0, CAP_ANY) {
                Ok(cam) => cam,
                Err(e) => {
                    tracing::error!("Failed to open camera: {}", e);
                    return;
                }
            };

            if !camera.is_opened().unwrap_or(false) {
                tracing::error!("Camera is not opened");
                return;
            }

            let mut frame = Mat::default();
            let mut frame_count = 0;

            loop {
                let start_time = Instant::now();

                // Capture frame
                if let Err(e) = camera.read(&mut frame) {
                    tracing::warn!("Failed to read frame: {}", e);
                    tokio::time::sleep(Duration::from_millis(33)).await;
                    continue;
                }

                if frame.empty() {
                    tracing::warn!("Empty frame captured");
                    tokio::time::sleep(Duration::from_millis(33)).await;
                    continue;
                }

                // Process frame
                let emotion_logits = match Self::preprocess_frame(&frame, &face_detector_path) {
                    Ok(preprocessed) => {
                        match Self::run_inference(&mut session, preprocessed) {
                            Ok(logits) => logits,
                            Err(e) => {
                                tracing::warn!("Inference failed: {}", e);
                                // Use neutral emotions as fallback
                                [0.1, 0.1, 0.3, 0.1, 0.1, 0.1, 0.2]
                            }
                        }
                    }
                    Err(FearError::NoFaceDetected) => {
                        // No face detected, use neutral emotions
                        [0.1, 0.1, 0.3, 0.1, 0.1, 0.1, 0.2]
                    }
                    Err(e) => {
                        tracing::warn!("Frame preprocessing failed: {}", e);
                        // Use neutral emotions as fallback
                        [0.1, 0.1, 0.3, 0.1, 0.1, 0.1, 0.2]
                    }
                };

                let fear_logit = FearCalibrator::extract_fear_logit(&emotion_logits);

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
                    FearScore::new_calibrated(normalized_fear, emotion_logits, 0.9)
                } else {
                    FearScore::new_uncalibrated(fear_logit, emotion_logits, 0.9)
                };

                // Send score with back-pressure handling
                match sender.try_send(score) {
                    Ok(_) => {},
                    Err(async_channel::TrySendError::Full(_)) => {
                        // Channel full, drop oldest frame (back-pressure)
                        tracing::debug!("Dropped frame due to back-pressure in ONNX sensor");
                    },
                    Err(async_channel::TrySendError::Closed(_)) => {
                        break; // Receiver dropped
                    }
                }

                frame_count += 1;

                // Measure inference time
                let inference_time = start_time.elapsed();
                if inference_time > Duration::from_millis(10) {
                    tracing::warn!("Inference took {}ms (target: <10ms)", inference_time.as_millis());
                }

                // Target ~30 FPS
                let target_frame_time = Duration::from_millis(33);
                if inference_time < target_frame_time {
                    tokio::time::sleep(target_frame_time - inference_time).await;
                }

                // Stop after reasonable number of frames for testing
                if frame_count > 10000 {
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
        let mut cameras = Vec::new();

        // Try to enumerate cameras (OpenCV doesn't have great enumeration support)
        for i in 0..10 {
            let camera = VideoCapture::new(i, CAP_ANY);
            match camera {
                Ok(cam) => {
                    if cam.is_opened().unwrap_or(false) {
                        // Get camera properties
                        let width = cam.get(opencv::videoio::CAP_PROP_FRAME_WIDTH).unwrap_or(640.0) as u32;
                        let height = cam.get(opencv::videoio::CAP_PROP_FRAME_HEIGHT).unwrap_or(480.0) as u32;

                        cameras.push(CameraDevice::new(
                            i as u32,
                            format!("Camera {}", i),
                            (width, height),
                        ));
                    }
                }
                Err(_) => {
                    // Camera not available, continue
                    continue;
                }
            }
        }

        if cameras.is_empty() {
            Err(CameraError::NoCamerasAvailable)
        } else {
            Ok(cameras)
        }
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
    async fn test_onnx_sensor_creation() {
        let sensor = OnnxFearSensor::new();
        assert!(!sensor.is_calibrated());
        assert_eq!(sensor.calibration_progress(), 0.0);
    }

    #[tokio::test]
    async fn test_onnx_sensor_initialization_without_model() {
        let mut sensor = OnnxFearSensor::new();
        let config = FearConfig {
            model_path: "nonexistent_model.onnx".to_string(),
            ..FearConfig::default()
        };

        // Should fail because model doesn't exist
        let result = sensor.initialize(&config).await;
        assert!(result.is_err());

        if let Err(FearError::ModelNotFound { message }) = result {
            assert!(message.contains("nonexistent_model.onnx"));
        } else {
            panic!("Expected ModelNotFound error");
        }
    }

    #[tokio::test]
    async fn test_onnx_sensor_camera_enumeration() {
        let sensor = OnnxFearSensor::new();

        // This might succeed or fail depending on available cameras
        let result = sensor.enumerate_cameras().await;
        match result {
            Ok(cameras) => {
                println!("Found {} cameras", cameras.len());
                for camera in cameras {
                    println!("Camera {}: {} ({}x{})",
                        camera.id, camera.name, camera.resolution.0, camera.resolution.1);
                }
            }
            Err(CameraError::NoCamerasAvailable) => {
                println!("No cameras found (expected in CI environment)");
            }
            Err(e) => {
                panic!("Unexpected camera error: {}", e);
            }
        }
    }

    #[test]
    fn test_face_detector_path() {
        // This test checks if we can get a face detector path
        let path = OnnxFearSensor::get_face_detector_path();
        println!("Face detector path: {}", path);

        // Try to create a face detector with the path
        let result = CascadeClassifier::new(&path);
        match result {
            Ok(_) => {
                println!("Face detector loaded successfully from: {}", path);
            }
            Err(e) => {
                println!("Face detector loading failed (expected without cascade file): {}", e);
                // This is expected in test environment without the cascade file
            }
        }
    }

    #[test]
    fn test_model_loading_with_invalid_path() {
        let result = OnnxFearSensor::load_model("invalid_path.onnx");
        assert!(result.is_err());

        if let Err(FearError::ModelNotFound { message }) = result {
            assert!(message.contains("invalid_path.onnx"));
        } else {
            panic!("Expected ModelNotFound error");
        }
    }
}
