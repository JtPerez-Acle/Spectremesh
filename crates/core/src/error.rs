//! Error types for SpectreMesh

use thiserror::Error;

/// Main error type for fear detection operations
#[derive(Error, Debug)]
pub enum FearError {
    #[error("Camera error: {0}")]
    Camera(#[from] CameraError),

    #[error("ONNX Runtime error: {message}")]
    OnnxRuntime { message: String },

    #[error("Model loading error: {message}")]
    ModelLoading { message: String },

    #[error("Face detection error: {message}")]
    FaceDetection { message: String },

    #[error("Calibration error: {message}")]
    Calibration { message: String },

    #[error("Configuration error: {message}")]
    Configuration { message: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Channel error: {message}")]
    Channel { message: String },

    #[error("Sensor not initialized")]
    NotInitialized,

    #[error("Sensor already running")]
    AlreadyRunning,

    #[error("Sensor not running")]
    NotRunning,

    #[error("Calibration incomplete")]
    CalibrationIncomplete,

    #[error("Model not found: {message}")]
    ModelNotFound { message: String },

    #[error("No face detected")]
    NoFaceDetected,

    #[error("Invalid logits: {message}")]
    InvalidLogits { message: String },
}

/// Camera-specific error types
#[derive(Error, Debug)]
pub enum CameraError {
    #[error("Camera not found: device_id={device_id}")]
    NotFound { device_id: u32 },

    #[error("Camera access denied: device_id={device_id}")]
    AccessDenied { device_id: u32 },

    #[error("Camera initialization failed: {message}")]
    InitializationFailed { message: String },

    #[error("Camera capture failed: {message}")]
    CaptureFailed { message: String },

    #[error("No cameras available")]
    NoCamerasAvailable,

    #[error("No cameras found")]
    NoCamerasFound,

    #[error("Invalid camera configuration: {message}")]
    InvalidConfiguration { message: String },
}

/// Terrain generation error types
#[derive(Error, Debug)]
pub enum TerrainError {
    #[error("Chunk generation failed: {message}")]
    ChunkGeneration { message: String },

    #[error("Noise generation failed: {message}")]
    NoiseGeneration { message: String },

    #[error("Mesh generation failed: {message}")]
    MeshGeneration { message: String },

    #[error("Invalid chunk coordinates: x={x}, z={z}")]
    InvalidChunkCoordinates { x: i32, z: i32 },
}

/// Configuration error types
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Invalid configuration file: {message}")]
    InvalidFile { message: String },

    #[error("Missing required field: {field}")]
    MissingField { field: String },

    #[error("Invalid value for field '{field}': {message}")]
    InvalidValue { field: String, message: String },

    #[error("Serialization error: {0}")]
    Serialization(#[from] toml::ser::Error),

    #[error("Deserialization error: {0}")]
    Deserialization(#[from] toml::de::Error),
}

impl FearError {
    /// Create a new ONNX Runtime error
    pub fn onnx_runtime(message: impl Into<String>) -> Self {
        Self::OnnxRuntime {
            message: message.into(),
        }
    }

    /// Create a new model loading error
    pub fn model_loading(message: impl Into<String>) -> Self {
        Self::ModelLoading {
            message: message.into(),
        }
    }

    /// Create a new face detection error
    pub fn face_detection(message: impl Into<String>) -> Self {
        Self::FaceDetection {
            message: message.into(),
        }
    }

    /// Create a new calibration error
    pub fn calibration(message: impl Into<String>) -> Self {
        Self::Calibration {
            message: message.into(),
        }
    }

    /// Create a new configuration error
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration {
            message: message.into(),
        }
    }

    /// Create a new channel error
    pub fn channel(message: impl Into<String>) -> Self {
        Self::Channel {
            message: message.into(),
        }
    }

    /// Create a new model not found error
    pub fn model_not_found(message: impl Into<String>) -> Self {
        Self::ModelNotFound {
            message: message.into(),
        }
    }

    /// Create a new invalid logits error
    pub fn invalid_logits(message: impl Into<String>) -> Self {
        Self::InvalidLogits {
            message: message.into(),
        }
    }
}

impl CameraError {
    /// Create a camera not found error
    pub fn not_found(device_id: u32) -> Self {
        Self::NotFound { device_id }
    }

    /// Create a camera access denied error
    pub fn access_denied(device_id: u32) -> Self {
        Self::AccessDenied { device_id }
    }

    /// Create a camera initialization failed error
    pub fn initialization_failed(message: impl Into<String>) -> Self {
        Self::InitializationFailed {
            message: message.into(),
        }
    }

    /// Create a camera capture failed error
    pub fn capture_failed(message: impl Into<String>) -> Self {
        Self::CaptureFailed {
            message: message.into(),
        }
    }

    /// Create an invalid configuration error
    pub fn invalid_configuration(message: impl Into<String>) -> Self {
        Self::InvalidConfiguration {
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fear_error_creation() {
        let error = FearError::onnx_runtime("Test error");
        assert!(matches!(error, FearError::OnnxRuntime { .. }));
        assert_eq!(error.to_string(), "ONNX Runtime error: Test error");
    }

    #[test]
    fn test_camera_error_creation() {
        let error = CameraError::not_found(0);
        assert!(matches!(error, CameraError::NotFound { device_id: 0 }));
        assert_eq!(error.to_string(), "Camera not found: device_id=0");
    }

    #[test]
    fn test_error_conversion() {
        let camera_error = CameraError::not_found(0);
        let fear_error: FearError = camera_error.into();
        assert!(matches!(fear_error, FearError::Camera(_)));
    }
}
