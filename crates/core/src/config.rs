//! Configuration types for SpectreMesh

use std::time::Duration;
use serde::{Deserialize, Serialize};
use crate::{CameraConfig, ConfigError};

/// Main configuration for fear detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FearConfig {
    /// Path to the ONNX emotion model
    pub model_path: String,
    /// Camera configuration
    pub camera: CameraConfig,
    /// Calibration duration in seconds
    pub calibration_duration: Duration,
    /// Whether to enable debug logging
    pub debug: bool,
    /// Maximum inference timeout
    pub inference_timeout: Duration,
}

impl Default for FearConfig {
    fn default() -> Self {
        Self {
            model_path: "assets/models/face_emotion.onnx".to_string(),
            camera: CameraConfig::default(),
            calibration_duration: Duration::from_secs(30),
            debug: false,
            inference_timeout: Duration::from_millis(100),
        }
    }
}

impl FearConfig {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the model path
    pub fn with_model_path(mut self, path: impl Into<String>) -> Self {
        self.model_path = path.into();
        self
    }

    /// Set the camera device ID
    pub fn with_camera_device(mut self, device_id: u32) -> Self {
        self.camera.device_id = device_id;
        self
    }

    /// Set the calibration duration
    pub fn with_calibration_duration(mut self, duration: Duration) -> Self {
        self.calibration_duration = duration;
        self
    }

    /// Enable debug logging
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.model_path.is_empty() {
            return Err(ConfigError::MissingField {
                field: "model_path".to_string(),
            });
        }

        if self.calibration_duration.as_secs() == 0 {
            return Err(ConfigError::InvalidValue {
                field: "calibration_duration".to_string(),
                message: "must be greater than 0".to_string(),
            });
        }

        if self.inference_timeout.as_millis() == 0 {
            return Err(ConfigError::InvalidValue {
                field: "inference_timeout".to_string(),
                message: "must be greater than 0".to_string(),
            });
        }

        Ok(())
    }

    /// Load configuration from TOML file
    pub fn from_file(path: &str) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::InvalidFile {
                message: format!("Failed to read file '{}': {}", path, e),
            })?;

        let config: Self = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Save configuration to TOML file
    pub fn to_file(&self, path: &str) -> Result<(), ConfigError> {
        self.validate()?;
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)
            .map_err(|e| ConfigError::InvalidFile {
                message: format!("Failed to write file '{}': {}", path, e),
            })?;
        Ok(())
    }
}

/// Terrain generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainConfig {
    /// Chunk size in blocks
    pub chunk_size: u32,
    /// Render distance in chunks
    pub render_distance: u32,
    /// Base terrain height
    pub base_height: f32,
    /// Fear influence multiplier
    pub fear_multiplier: f32,
    /// Noise scale
    pub noise_scale: f32,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            chunk_size: 16,
            render_distance: 8,
            base_height: 64.0,
            fear_multiplier: 10.0,
            noise_scale: 0.01,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_fear_config_default() {
        let config = FearConfig::default();
        assert_eq!(config.model_path, "assets/models/face_emotion.onnx");
        assert_eq!(config.camera.device_id, 0);
        assert_eq!(config.calibration_duration, Duration::from_secs(30));
        assert!(!config.debug);
    }

    #[test]
    fn test_fear_config_builder() {
        let config = FearConfig::new()
            .with_model_path("custom/model.onnx")
            .with_camera_device(1)
            .with_calibration_duration(Duration::from_secs(60))
            .with_debug(true);

        assert_eq!(config.model_path, "custom/model.onnx");
        assert_eq!(config.camera.device_id, 1);
        assert_eq!(config.calibration_duration, Duration::from_secs(60));
        assert!(config.debug);
    }

    #[test]
    fn test_fear_config_validation() {
        let mut config = FearConfig::default();
        assert!(config.validate().is_ok());

        config.model_path = String::new();
        assert!(config.validate().is_err());

        config.model_path = "test.onnx".to_string();
        config.calibration_duration = Duration::from_secs(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_terrain_config_default() {
        let config = TerrainConfig::default();
        assert_eq!(config.chunk_size, 16);
        assert_eq!(config.render_distance, 8);
        assert_eq!(config.base_height, 64.0);
        assert_eq!(config.fear_multiplier, 10.0);
        assert_eq!(config.noise_scale, 0.01);
    }
}
