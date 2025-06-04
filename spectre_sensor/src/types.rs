//! Core types for the spectre sensor

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

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

/// Performance metrics for monitoring
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Current frames per second
    pub current_fps: f32,
    /// 95th percentile inference latency
    pub p95_inference_latency: Duration,
    /// Total number of dropped frames
    pub dropped_frames: u64,
    /// Calibration drift (change in baseline mean)
    pub calibration_drift: f32,
    /// Last update timestamp
    pub last_update: Instant,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            current_fps: 0.0,
            p95_inference_latency: Duration::ZERO,
            dropped_frames: 0,
            calibration_drift: 0.0,
            last_update: Instant::now(),
        }
    }
}

impl PerformanceMetrics {
    /// Create new metrics
    pub fn new() -> Self {
        Self::default()
    }

    /// Update FPS calculation
    pub fn update_fps(&mut self, frame_count: u64, elapsed: Duration) {
        if elapsed.as_secs_f32() > 0.0 {
            self.current_fps = frame_count as f32 / elapsed.as_secs_f32();
        }
        self.last_update = Instant::now();
    }

    /// Record a dropped frame
    pub fn record_dropped_frame(&mut self) {
        self.dropped_frames += 1;
    }

    /// Update inference latency percentile
    pub fn update_inference_latency(&mut self, latencies: &[Duration]) {
        if !latencies.is_empty() {
            let mut sorted = latencies.to_vec();
            sorted.sort();
            let p95_index = (sorted.len() as f32 * 0.95) as usize;
            self.p95_inference_latency = sorted.get(p95_index).copied().unwrap_or_default();
        }
    }
}

/// Sensor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorConfig {
    /// Path to emotion model (can be overridden)
    pub emotion_model_path: Option<String>,
    /// Number of ONNX runtime threads
    pub onnx_threads: usize,
    /// Whether to freeze calibration after initial period
    pub freeze_calibration: bool,
    /// Camera device ID
    pub camera_id: u32,
    /// Target FPS
    pub target_fps: f32,
    /// Channel buffer size for back-pressure
    pub channel_buffer_size: usize,
    /// Metrics server port
    pub metrics_port: u16,
    /// gRPC server socket path
    pub grpc_socket_path: String,
}

impl Default for SensorConfig {
    fn default() -> Self {
        Self {
            emotion_model_path: None, // Use embedded model by default
            onnx_threads: num_cpus::get(),
            freeze_calibration: false,
            camera_id: 0,
            target_fps: 30.0,
            channel_buffer_size: 2,
            metrics_port: 9090,
            grpc_socket_path: "/tmp/spectre_sensor.sock".to_string(),
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
    fn test_performance_metrics() {
        let mut metrics = PerformanceMetrics::new();
        
        // Test FPS calculation
        metrics.update_fps(30, Duration::from_secs(1));
        assert_eq!(metrics.current_fps, 30.0);
        
        // Test dropped frame recording
        metrics.record_dropped_frame();
        assert_eq!(metrics.dropped_frames, 1);
        
        // Test latency percentile calculation
        let latencies = vec![
            Duration::from_millis(1),
            Duration::from_millis(2),
            Duration::from_millis(3),
            Duration::from_millis(4),
            Duration::from_millis(10), // This should be the p95
        ];
        metrics.update_inference_latency(&latencies);
        assert_eq!(metrics.p95_inference_latency, Duration::from_millis(10));
    }
}
