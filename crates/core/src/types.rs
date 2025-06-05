//! Core types for SpectreMesh

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

/// A fear score measurement with metadata
#[derive(Debug, Clone, PartialEq)]
pub struct FearScore {
    /// Normalized fear level [0.0, 1.0]
    pub value: f32,
    /// Raw emotion logits from the model
    pub emotion_logits: [f32; 7],
    /// Model confidence [0.0, 1.0]
    pub confidence: f32,
    /// Whether this score has been calibrated
    pub calibrated: bool,
    /// When this measurement was taken
    pub timestamp: Instant,
}

impl FearScore {
    /// Create a new calibrated fear score
    pub fn new_calibrated(value: f32, emotion_logits: [f32; 7], confidence: f32) -> Self {
        Self {
            value,
            emotion_logits,
            confidence,
            calibrated: true,
            timestamp: Instant::now(),
        }
    }

    /// Create a new uncalibrated fear score
    pub fn new_uncalibrated(value: f32, emotion_logits: [f32; 7], confidence: f32) -> Self {
        Self {
            value,
            emotion_logits,
            confidence,
            calibrated: false,
            timestamp: Instant::now(),
        }
    }

    /// Extract the fear logit from emotion logits
    /// Assumes: [angry, disgust, fear, happy, sad, surprise, neutral]
    pub fn extract_fear_logit(&self) -> f32 {
        const FEAR_INDEX: usize = 2;
        self.emotion_logits[FEAR_INDEX]
    }
}

/// A single fear measurement frame with timing information
#[derive(Debug, Clone, PartialEq)]
pub struct FearFrame {
    /// When this measurement was taken
    pub timestamp: Instant,
    /// The fear score measurement
    pub fear_score: f32,
    /// Raw emotion logits from the model
    pub emotion_logits: [f32; 7],
    /// Model confidence [0.0, 1.0]
    pub confidence: f32,
    /// Whether this score has been calibrated
    pub calibrated: bool,
    /// Inference latency for this frame
    pub inference_latency: Duration,
}

impl FearFrame {
    /// Create a new fear frame
    pub fn new(
        fear_score: f32,
        emotion_logits: [f32; 7],
        confidence: f32,
        calibrated: bool,
        inference_latency: Duration,
    ) -> Self {
        Self {
            timestamp: Instant::now(),
            fear_score,
            emotion_logits,
            confidence,
            calibrated,
            inference_latency,
        }
    }

    /// Get timestamp as microseconds since Unix epoch
    pub fn timestamp_us(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64
    }

    /// Extract the fear logit from emotion logits
    /// Assumes: [angry, disgust, fear, happy, sad, surprise, neutral]
    pub fn extract_fear_logit(&self) -> f32 {
        const FEAR_INDEX: usize = 2;
        self.emotion_logits[FEAR_INDEX]
    }
}

/// Fear bucket classification for terrain updates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FearBucket {
    Low,    // [0.0, 0.33)
    Medium, // [0.33, 0.66)
    High,   // [0.66, 1.0]
}

impl FearBucket {
    /// Classify a fear score into a bucket
    pub fn from_score(score: f32) -> Self {
        if score < 0.33 {
            FearBucket::Low
        } else if score < 0.66 {
            FearBucket::Medium
        } else {
            FearBucket::High
        }
    }

    /// Get the distortion intensity for shader uniforms
    pub fn distortion_intensity(&self) -> f32 {
        match self {
            FearBucket::Low => 0.1,
            FearBucket::Medium => 0.5,
            FearBucket::High => 1.0,
        }
    }
}

/// Camera device information
#[derive(Debug, Clone, PartialEq)]
pub struct CameraDevice {
    /// Device ID
    pub id: u32,
    /// Human-readable device name
    pub name: String,
    /// Supported resolution (width, height)
    pub resolution: (u32, u32),
}

impl CameraDevice {
    /// Create a new camera device
    pub fn new(id: u32, name: String, resolution: (u32, u32)) -> Self {
        Self { id, name, resolution }
    }
}

/// Camera configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraConfig {
    /// Camera device ID
    pub device_id: u32,
    /// Target frames per second
    pub fps: u32,
    /// Frame width
    pub width: u32,
    /// Frame height
    pub height: u32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            device_id: 0,
            fps: 30,
            width: 640,
            height: 480,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fear_bucket_classification() {
        assert_eq!(FearBucket::from_score(0.0), FearBucket::Low);
        assert_eq!(FearBucket::from_score(0.32), FearBucket::Low);
        assert_eq!(FearBucket::from_score(0.33), FearBucket::Medium);
        assert_eq!(FearBucket::from_score(0.65), FearBucket::Medium);
        assert_eq!(FearBucket::from_score(0.66), FearBucket::High);
        assert_eq!(FearBucket::from_score(1.0), FearBucket::High);
    }

    #[test]
    fn test_fear_bucket_distortion() {
        assert_eq!(FearBucket::Low.distortion_intensity(), 0.1);
        assert_eq!(FearBucket::Medium.distortion_intensity(), 0.5);
        assert_eq!(FearBucket::High.distortion_intensity(), 1.0);
    }

    #[test]
    fn test_fear_score_creation() {
        let emotion_logits = [0.1, 0.1, 0.8, 0.1, 0.1, 0.1, 0.1];
        let score = FearScore::new_calibrated(0.75, emotion_logits, 0.9);

        assert_eq!(score.value, 0.75);
        assert_eq!(score.extract_fear_logit(), 0.8);
        assert!(score.calibrated);
        assert_eq!(score.confidence, 0.9);
    }

    #[test]
    fn test_fear_frame_creation() {
        let emotion_logits = [0.1, 0.1, 0.8, 0.1, 0.1, 0.1, 0.1];
        let frame = FearFrame::new(
            0.75,
            emotion_logits,
            0.9,
            true,
            Duration::from_millis(5),
        );

        assert_eq!(frame.fear_score, 0.75);
        assert_eq!(frame.extract_fear_logit(), 0.8);
        assert!(frame.calibrated);
        assert_eq!(frame.inference_latency, Duration::from_millis(5));
    }

    #[test]
    fn test_camera_device() {
        let device = CameraDevice::new(0, "Test Camera".to_string(), (640, 480));
        assert_eq!(device.id, 0);
        assert_eq!(device.name, "Test Camera");
        assert_eq!(device.resolution, (640, 480));
    }
}
