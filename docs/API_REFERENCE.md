# SpectreMesh API Reference

**Complete API documentation for emotion-responsive procedural horror**

## Table of Contents

- [Core Types](#core-types)
- [Fear Detection API](#fear-detection-api)
- [Configuration System](#configuration-system)
- [Error Handling](#error-handling)
- [Platform Compatibility](#platform-compatibility)
- [Usage Examples](#usage-examples)

## Core Types

### FearScore

The primary data structure for normalized fear measurements.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FearScore {
    /// Normalized fear level [0.0, 1.0]
    pub value: f32,
    
    /// Discrete fear classification
    pub bucket: FearBucket,
    
    /// Calibration completion percentage [0.0, 1.0]
    pub calibration_progress: f32,
    
    /// Additional metadata
    pub metadata: FearMetadata,
}

impl FearScore {
    /// Create a new fear score with validation
    pub fn new(value: f32, calibration_progress: f32) -> Self;
    
    /// Check if the score is from a calibrated sensor
    pub fn is_calibrated(&self) -> bool;
    
    /// Get the distortion intensity for terrain modification
    pub fn distortion_intensity(&self) -> f32;
}
```

### FearBucket

Discrete fear level classification for game logic.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FearBucket {
    Calm,       // [0.0, 0.2) - Baseline state
    Uneasy,     // [0.2, 0.4) - Slight tension
    Nervous,    // [0.4, 0.6) - Moderate anxiety
    Scared,     // [0.6, 0.8) - High fear response
    Terrified,  // [0.8, 1.0] - Maximum fear
}

impl FearBucket {
    /// Convert fear value to bucket
    pub fn from_fear_value(value: f32) -> Self;
    
    /// Get the distortion multiplier for this bucket
    pub fn distortion_multiplier(&self) -> f32;
    
    /// Get the color representation for UI
    pub fn color(&self) -> (u8, u8, u8);
}
```

### CameraDevice

Represents an available camera device.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraDevice {
    /// Camera device ID
    pub id: u32,
    
    /// Human-readable camera name
    pub name: String,
    
    /// Camera resolution (width, height)
    pub resolution: (u32, u32),
}

impl CameraDevice {
    /// Create a new camera device
    pub fn new(id: u32, name: String, resolution: (u32, u32)) -> Self;
    
    /// Check if this is a mock camera
    pub fn is_mock(&self) -> bool;
}
```

## Fear Detection API

### FearSensor Trait

Unified interface for both mock and real fear detection implementations.

```rust
#[async_trait]
pub trait FearSensor: Send + Sync {
    /// Initialize the sensor with configuration
    async fn initialize(&mut self, config: &FearConfig) -> Result<(), FearError>;
    
    /// Start fear detection and return receiver for scores
    async fn start(&mut self) -> Result<Receiver<FearScore>, FearError>;
    
    /// Stop fear detection
    async fn stop(&mut self) -> Result<(), FearError>;
    
    /// Enumerate available cameras
    async fn enumerate_cameras(&self) -> Result<Vec<CameraDevice>, CameraError>;
    
    /// Get current calibration progress [0.0, 1.0]
    fn calibration_progress(&self) -> f32;
    
    /// Check if sensor is currently running
    fn is_running(&self) -> bool;
}
```

### MockFearSensor

Development implementation that generates synthetic fear data.

```rust
pub struct MockFearSensor {
    pattern: Vec<f32>,
    current_index: usize,
    is_running: bool,
}

impl MockFearSensor {
    /// Create sensor with custom fear pattern
    pub fn new(pattern: Vec<f32>) -> Self;
    
    /// Create sensor with step pattern (0.1, 0.2, 0.3, ...)
    pub fn step_pattern() -> Self;
    
    /// Create sensor with sine wave pattern
    pub fn sine_pattern(amplitude: f32, frequency: f32) -> Self;
    
    /// Create sensor with random pattern
    pub fn random_pattern(seed: u64) -> Self;
}

// Usage example:
let mut sensor = MockFearSensor::step_pattern();
sensor.initialize(&config).await?;
let receiver = sensor.start().await?;
```

### YuNetFearSensor

Production implementation using real camera and YuNet CNN.

```rust
pub struct YuNetFearSensor {
    emotion_sensor: Option<EmotionSensor>,
    state: Arc<Mutex<SensorState>>,
}

impl YuNetFearSensor {
    /// Create a new YuNet-based fear sensor
    pub fn new() -> Self;
    
    /// Reset calibration data
    pub fn reset_calibration(&mut self) -> Result<(), SensorError>;
    
    /// Get detailed sensor state
    pub fn get_state(&self) -> SensorState;
}

// Usage example:
let mut sensor = YuNetFearSensor::new();
sensor.initialize(&config).await?;
let receiver = sensor.start().await?;
```

## Configuration System

### FearConfig

Main configuration structure for fear detection.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FearConfig {
    /// Camera configuration
    pub camera: CameraConfig,
    
    /// Model file paths (optional, uses embedded models by default)
    pub model_path: String,
    
    /// Processing configuration
    pub processing: ProcessingConfig,
    
    /// Calibration settings
    pub calibration: CalibrationConfig,
}

impl FearConfig {
    /// Create default configuration
    pub fn default() -> Self;
    
    /// Create configuration for specific camera
    pub fn for_camera(camera_id: u32) -> Self;
    
    /// Validate configuration
    pub fn validate(&self) -> Result<(), ConfigError>;
}
```

### CameraConfig

Camera-specific configuration options.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraConfig {
    /// Camera device ID
    pub device_id: u32,
    
    /// Target frames per second
    pub fps: u32,
    
    /// Camera resolution
    pub resolution: (u32, u32),
    
    /// Auto-exposure settings
    pub auto_exposure: bool,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            device_id: 0,
            fps: 30,
            resolution: (640, 480),
            auto_exposure: true,
        }
    }
}
```

### SensorConfig

Internal sensor configuration (platform-specific).

```rust
#[derive(Debug, Clone)]
pub struct SensorConfig {
    /// ONNX model path (None for embedded)
    pub emotion_model_path: Option<String>,
    
    /// Number of ONNX threads
    pub onnx_threads: usize,
    
    /// Freeze calibration after completion
    pub freeze_calibration: bool,
    
    /// Camera device ID
    pub camera_id: u32,
    
    /// Target FPS
    pub target_fps: f32,
    
    /// Channel buffer size
    pub channel_buffer_size: usize,
    
    /// Metrics server port
    pub metrics_port: u16,
    
    /// Platform-specific socket path
    pub grpc_socket_path: String,
}

impl SensorConfig {
    /// Validate configuration
    pub fn validate(&self) -> Result<(), ConfigError>;
    
    /// Get platform-specific default socket path
    fn default_socket_path() -> String;
}
```

## Error Handling

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum FearError {
    #[error("Camera error: {0}")]
    Camera(#[from] CameraError),
    
    #[error("Sensor error: {0}")]
    Sensor(#[from] SensorError),
    
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    
    #[error("Calibration incomplete: {progress}% complete")]
    CalibrationIncomplete { progress: f32 },
}

#[derive(Debug, thiserror::Error)]
pub enum CameraError {
    #[error("No cameras available on system")]
    NoCamerasAvailable,
    
    #[error("Camera {id} not found")]
    CameraNotFound { id: u32 },
    
    #[error("Camera access denied: {reason}")]
    AccessDenied { reason: String },
    
    #[error("Camera initialization failed: {details}")]
    InitializationFailed { details: String },
}

#[derive(Debug, thiserror::Error)]
pub enum SensorError {
    #[error("Camera initialization failed: {0}")]
    CameraInit(String),
    
    #[error("Model loading failed: {0}")]
    ModelLoad(String),
    
    #[error("Inference failed: {0}")]
    Inference(String),
    
    #[error("No face detected in frame")]
    NoFaceDetected,
    
    #[error("Platform not supported: {platform}")]
    UnsupportedPlatform { platform: String },
}
```

### Error Handling Patterns

```rust
// Graceful error handling with fallback
match sensor.start().await {
    Ok(receiver) => {
        // Use real sensor
        process_fear_scores(receiver).await;
    },
    Err(FearError::Camera(CameraError::NoCamerasAvailable)) => {
        // Fallback to mock sensor
        let mut mock_sensor = MockFearSensor::step_pattern();
        let receiver = mock_sensor.start().await?;
        process_fear_scores(receiver).await;
    },
    Err(e) => {
        eprintln!("Fatal error: {}", e);
        return Err(e);
    }
}
```

## Platform Compatibility

### Platform-Specific Features

```rust
// Platform detection
#[cfg(target_os = "windows")]
const PLATFORM: &str = "Windows";

#[cfg(target_os = "macos")]
const PLATFORM: &str = "macOS";

#[cfg(target_os = "linux")]
const PLATFORM: &str = "Linux";

// Platform-specific camera backends
pub fn get_camera_backend() -> &'static str {
    #[cfg(target_os = "windows")]
    return "DirectShow";
    
    #[cfg(target_os = "macos")]
    return "AVFoundation";
    
    #[cfg(target_os = "linux")]
    return "V4L2";
}

// Platform-specific socket paths
pub fn default_socket_path() -> String {
    #[cfg(target_os = "windows")]
    return r"\\.\pipe\spectre_sensor".to_string();
    
    #[cfg(target_os = "macos")]
    return format!("/tmp/spectre_sensor_{}.sock", std::process::id());
    
    #[cfg(target_os = "linux")]
    return "/tmp/spectre_sensor.sock".to_string();
}
```

### Cross-Platform Testing

```rust
#[tokio::test]
async fn test_platform_compatibility() {
    let config = SensorConfig::default();
    
    // Test platform-specific paths
    #[cfg(target_os = "windows")]
    assert!(config.grpc_socket_path.contains(r"\\.\pipe\"));
    
    #[cfg(target_os = "macos")]
    assert!(config.grpc_socket_path.contains(&std::process::id().to_string()));
    
    #[cfg(target_os = "linux")]
    assert_eq!(config.grpc_socket_path, "/tmp/spectre_sensor.sock");
}
```

## Usage Examples

### Basic Fear Detection

```rust
use spectre_sensor::compat::{FearSensor, MockFearSensor};
use spectremesh_core::{FearConfig, FearScore};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create and configure sensor
    let mut sensor = MockFearSensor::step_pattern();
    let config = FearConfig::default();
    
    // Initialize and start
    sensor.initialize(&config).await?;
    let receiver = sensor.start().await?;
    
    // Process fear scores
    for _ in 0..10 {
        let fear_score = receiver.recv().await?;
        println!("Fear: {:.2}, Bucket: {:?}", 
                 fear_score.value, fear_score.bucket);
    }
    
    // Clean shutdown
    sensor.stop().await?;
    Ok(())
}
```

### Bevy Integration

```rust
use bevy::prelude::*;
use spectre_sensor::compat::{FearSensor, MockFearSensor};
use spectremesh_core::FearScore;

#[derive(Resource)]
struct FearState {
    receiver: Receiver<FearScore>,
    current_fear: f32,
}

fn setup_fear_detection(mut commands: Commands) {
    let mut sensor = MockFearSensor::step_pattern();
    let config = FearConfig::default();
    
    // Initialize sensor (in real app, handle errors properly)
    let receiver = sensor.start().await.unwrap();
    
    commands.insert_resource(FearState {
        receiver,
        current_fear: 0.0,
    });
}

fn update_fear_system(mut fear_state: ResMut<FearState>) {
    if let Ok(fear_score) = fear_state.receiver.try_recv() {
        fear_state.current_fear = fear_score.value;
        
        // Trigger terrain updates based on fear bucket
        match fear_score.bucket {
            FearBucket::Calm => { /* Smooth terrain */ },
            FearBucket::Terrified => { /* Chaotic displacement */ },
            _ => { /* Intermediate effects */ },
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_fear_detection)
        .add_systems(Update, update_fear_system)
        .run();
}
```

### Camera Enumeration

```rust
use spectre_sensor::compat::YuNetFearSensor;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sensor = YuNetFearSensor::new();
    
    match sensor.enumerate_cameras().await {
        Ok(cameras) => {
            println!("Found {} camera(s):", cameras.len());
            for camera in cameras {
                println!("  - ID: {}, Name: '{}', Resolution: {}x{}", 
                         camera.id, camera.name, 
                         camera.resolution.0, camera.resolution.1);
            }
        },
        Err(e) => {
            eprintln!("Camera enumeration failed: {}", e);
        }
    }
    
    Ok(())
}
```

### Error Handling with Fallback

```rust
use spectre_sensor::compat::{FearSensor, MockFearSensor, YuNetFearSensor};
use spectremesh_core::{FearError, CameraError};

async fn create_sensor() -> Result<Box<dyn FearSensor>, FearError> {
    // Try real sensor first
    let mut real_sensor = YuNetFearSensor::new();
    match real_sensor.enumerate_cameras().await {
        Ok(cameras) if !cameras.is_empty() => {
            println!("Using real camera: {}", cameras[0].name);
            Ok(Box::new(real_sensor))
        },
        Err(CameraError::NoCamerasAvailable) => {
            println!("No cameras found, using mock sensor");
            Ok(Box::new(MockFearSensor::step_pattern()))
        },
        Err(e) => Err(FearError::Camera(e)),
    }
}
```

This API reference provides complete documentation for integrating SpectreMesh's fear detection system into game applications while maintaining cross-platform compatibility and robust error handling.
