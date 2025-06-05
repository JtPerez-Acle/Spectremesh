//! High-performance emotion detection sensor with gRPC streaming
//!
//! This crate provides the next-generation fear detection pipeline with:
//! - YuNet face detection (replacing Haar cascades)
//! - Optimized ONNX Runtime with configurable threading
//! - gRPC streaming with back-pressure handling
//! - Adaptive calibration with EMA updates
//! - Comprehensive metrics and monitoring

pub mod types;
pub mod yunet;
pub mod calibrator;
pub mod sensor;
pub mod grpc_server;
pub mod grpc_client;
pub mod metrics;
pub mod config;
pub mod compat;

// Re-export main types
pub use types::{FearFrame, FearBucket, PerformanceMetrics};
pub use sensor::{EmotionSensor, SensorError};
pub use calibrator::{AdaptiveCalibrator, CalibrationError, BaselineStats};
pub use config::SensorConfig;

// Re-export compatibility layer for legacy API
pub use compat::{YuNetFearSensor, MockFearSensor};

// gRPC generated code
pub mod proto {
    tonic::include_proto!("spectre.sensor.v1");
}

// Embedded YuNet model (345 KB)
pub const YUNET_MODEL_BYTES: &[u8] = include_bytes!("../models/face_detection_yunet.onnx");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yunet_model_embedded() {
        assert!(!YUNET_MODEL_BYTES.is_empty());
        assert_eq!(YUNET_MODEL_BYTES.len(), 232589); // Expected YuNet model size (2023mar version)
    }
}
