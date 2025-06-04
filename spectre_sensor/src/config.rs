//! Configuration management for the spectre sensor

use serde::{Deserialize, Serialize};
use std::env;

/// Sensor configuration with environment variable overrides
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorConfig {
    /// Path to emotion model (can be overridden with --model-path)
    pub emotion_model_path: Option<String>,
    /// Number of ONNX runtime threads (overridable with SPECTRE_THREADS)
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
            onnx_threads: Self::get_thread_count(),
            freeze_calibration: false,
            camera_id: 0,
            target_fps: 30.0,
            channel_buffer_size: 2,
            metrics_port: 9090,
            grpc_socket_path: "/tmp/spectre_sensor.sock".to_string(),
        }
    }
}

impl SensorConfig {
    /// Create configuration with environment variable overrides
    pub fn from_env() -> Self {
        let mut config = Self::default();
        
        // Override thread count from environment
        config.onnx_threads = Self::get_thread_count();
        
        // Override other settings from environment variables
        if let Ok(freeze) = env::var("SPECTRE_FREEZE_CALIBRATION") {
            config.freeze_calibration = freeze.parse().unwrap_or(false);
        }
        
        if let Ok(camera_id) = env::var("SPECTRE_CAMERA_ID") {
            config.camera_id = camera_id.parse().unwrap_or(0);
        }
        
        if let Ok(fps) = env::var("SPECTRE_TARGET_FPS") {
            config.target_fps = fps.parse().unwrap_or(30.0);
        }
        
        if let Ok(buffer_size) = env::var("SPECTRE_BUFFER_SIZE") {
            config.channel_buffer_size = buffer_size.parse().unwrap_or(2);
        }
        
        if let Ok(port) = env::var("SPECTRE_METRICS_PORT") {
            config.metrics_port = port.parse().unwrap_or(9090);
        }
        
        if let Ok(socket_path) = env::var("SPECTRE_GRPC_SOCKET") {
            config.grpc_socket_path = socket_path;
        }
        
        config
    }
    
    /// Get thread count from environment or default to CPU count
    fn get_thread_count() -> usize {
        env::var("SPECTRE_THREADS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(num_cpus::get)
            .max(1) // Ensure at least 1 thread
    }
    
    /// Set emotion model path (for --model-path override)
    pub fn with_model_path(mut self, path: String) -> Self {
        self.emotion_model_path = Some(path);
        self
    }
    
    /// Set freeze calibration flag
    pub fn with_freeze_calibration(mut self, freeze: bool) -> Self {
        self.freeze_calibration = freeze;
        self
    }
    
    /// Set camera ID
    pub fn with_camera_id(mut self, camera_id: u32) -> Self {
        self.camera_id = camera_id;
        self
    }
    
    /// Set target FPS
    pub fn with_target_fps(mut self, fps: f32) -> Self {
        self.target_fps = fps.max(1.0).min(120.0); // Reasonable bounds
        self
    }
    
    /// Set ONNX thread count
    pub fn with_onnx_threads(mut self, threads: usize) -> Self {
        self.onnx_threads = threads.max(1); // Ensure at least 1 thread
        self
    }
    
    /// Set channel buffer size
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.channel_buffer_size = size.max(1); // Ensure at least 1
        self
    }
    
    /// Set metrics port
    pub fn with_metrics_port(mut self, port: u16) -> Self {
        self.metrics_port = port;
        self
    }
    
    /// Set gRPC socket path
    pub fn with_grpc_socket(mut self, path: String) -> Self {
        self.grpc_socket_path = path;
        self
    }
    
    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.onnx_threads == 0 {
            return Err("ONNX thread count must be at least 1".to_string());
        }
        
        if self.target_fps <= 0.0 {
            return Err("Target FPS must be positive".to_string());
        }
        
        if self.channel_buffer_size == 0 {
            return Err("Channel buffer size must be at least 1".to_string());
        }
        
        if self.grpc_socket_path.is_empty() {
            return Err("gRPC socket path cannot be empty".to_string());
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_default_config() {
        let config = SensorConfig::default();
        
        assert!(config.emotion_model_path.is_none());
        assert!(config.onnx_threads > 0);
        assert!(!config.freeze_calibration);
        assert_eq!(config.camera_id, 0);
        assert_eq!(config.target_fps, 30.0);
        assert_eq!(config.channel_buffer_size, 2);
        assert_eq!(config.metrics_port, 9090);
        assert_eq!(config.grpc_socket_path, "/tmp/spectre_sensor.sock");
    }

    #[test]
    fn test_config_validation() {
        let mut config = SensorConfig::default();
        
        // Valid config should pass
        assert!(config.validate().is_ok());
        
        // Invalid thread count
        config.onnx_threads = 0;
        assert!(config.validate().is_err());
        config.onnx_threads = 1;
        
        // Invalid FPS
        config.target_fps = 0.0;
        assert!(config.validate().is_err());
        config.target_fps = 30.0;
        
        // Invalid buffer size
        config.channel_buffer_size = 0;
        assert!(config.validate().is_err());
        config.channel_buffer_size = 2;
        
        // Invalid socket path
        config.grpc_socket_path = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_builders() {
        let config = SensorConfig::default()
            .with_model_path("test_model.onnx".to_string())
            .with_freeze_calibration(true)
            .with_camera_id(1)
            .with_target_fps(60.0)
            .with_onnx_threads(4)
            .with_buffer_size(5)
            .with_metrics_port(8080)
            .with_grpc_socket("/tmp/test.sock".to_string());
        
        assert_eq!(config.emotion_model_path, Some("test_model.onnx".to_string()));
        assert!(config.freeze_calibration);
        assert_eq!(config.camera_id, 1);
        assert_eq!(config.target_fps, 60.0);
        assert_eq!(config.onnx_threads, 4);
        assert_eq!(config.channel_buffer_size, 5);
        assert_eq!(config.metrics_port, 8080);
        assert_eq!(config.grpc_socket_path, "/tmp/test.sock");
    }

    #[test]
    fn test_env_override() {
        // Set environment variables
        env::set_var("SPECTRE_THREADS", "8");
        env::set_var("SPECTRE_FREEZE_CALIBRATION", "true");
        env::set_var("SPECTRE_CAMERA_ID", "2");
        env::set_var("SPECTRE_TARGET_FPS", "60.0");
        env::set_var("SPECTRE_BUFFER_SIZE", "4");
        env::set_var("SPECTRE_METRICS_PORT", "8080");
        env::set_var("SPECTRE_GRPC_SOCKET", "/tmp/test.sock");
        
        let config = SensorConfig::from_env();
        
        assert_eq!(config.onnx_threads, 8);
        assert!(config.freeze_calibration);
        assert_eq!(config.camera_id, 2);
        assert_eq!(config.target_fps, 60.0);
        assert_eq!(config.channel_buffer_size, 4);
        assert_eq!(config.metrics_port, 8080);
        assert_eq!(config.grpc_socket_path, "/tmp/test.sock");
        
        // Clean up environment variables
        env::remove_var("SPECTRE_THREADS");
        env::remove_var("SPECTRE_FREEZE_CALIBRATION");
        env::remove_var("SPECTRE_CAMERA_ID");
        env::remove_var("SPECTRE_TARGET_FPS");
        env::remove_var("SPECTRE_BUFFER_SIZE");
        env::remove_var("SPECTRE_METRICS_PORT");
        env::remove_var("SPECTRE_GRPC_SOCKET");
    }

    #[test]
    fn test_bounds_checking() {
        let config = SensorConfig::default()
            .with_target_fps(-10.0) // Should be clamped to 1.0
            .with_target_fps(200.0) // Should be clamped to 120.0
            .with_onnx_threads(0)   // Should be set to 1
            .with_buffer_size(0);   // Should be set to 1
        
        assert_eq!(config.target_fps, 120.0);
        assert_eq!(config.onnx_threads, 1);
        assert_eq!(config.channel_buffer_size, 1);
    }
}
