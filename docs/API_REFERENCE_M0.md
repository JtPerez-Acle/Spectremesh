# SpectreMesh M0 API Reference
**Quick reference for fear detection APIs and integration patterns**

## Core Types

### `FearScore`
```rust
pub struct FearScore {
    pub value: f32,           // Normalized fear level [0.0, 1.0]
    pub confidence: f32,      // Detection confidence [0.0, 1.0]
    pub calibrated: bool,     // Whether score is calibrated
    pub emotion_logits: [f32; 7], // Raw emotion class scores
    pub timestamp: std::time::Instant,
}

impl FearScore {
    pub fn extract_fear_logit(&self) -> f32 { /* ... */ }
}
```

### `FearConfig`
```rust
pub struct FearConfig {
    pub model_path: String,              // Path to ONNX model
    pub calibration_duration: f32,       // Calibration time in seconds
    pub camera: CameraConfig,            // Camera settings
}

impl Default for FearConfig {
    fn default() -> Self {
        Self {
            model_path: "assets/models/face_emotion.onnx".to_string(),
            calibration_duration: 30.0,
            camera: CameraConfig::default(),
        }
    }
}
```

## Fear Sensor Trait

### `FearSensor` (Async Trait)
```rust
#[async_trait]
pub trait FearSensor {
    // Initialize sensor with configuration
    async fn initialize(&mut self, config: &FearConfig) -> Result<(), FearError>;
    
    // Start fear detection, returns channel receiver
    async fn start(&mut self) -> Result<async_channel::Receiver<FearScore>, FearError>;
    
    // Stop fear detection
    async fn stop(&mut self) -> Result<(), FearError>;
    
    // Enumerate available cameras
    async fn enumerate_cameras(&self) -> Result<Vec<CameraDevice>, CameraError>;
    
    // Check calibration status
    fn is_calibrated(&self) -> bool;
    fn calibration_progress(&self) -> f32; // [0.0, 1.0]
}
```

## Implementations

### `MockFearSensor` (Development)
```rust
// Constant fear level
let sensor = MockFearSensor::new(vec![0.5]);

// Step pattern (low → high → low)
let sensor = MockFearSensor::step_pattern();

// Sine wave pattern
let sensor = MockFearSensor::sine_pattern(0.5, 0.3, 2.0); // center, amplitude, period

// Custom pattern
let sensor = MockFearSensor::new(vec![0.1, 0.3, 0.5, 0.7, 0.9]);
```

### `OnnxFearSensor` (Production)
```rust
// Real hardware sensor
let sensor = OnnxFearSensor::new();

// Requires model files:
// - assets/models/face_emotion.onnx
// - assets/models/haarcascade_frontalface_alt.xml (or system path)
```

## Usage Patterns

### Basic Usage
```rust
use spectremesh_fear_sensor::{FearSensor, MockFearSensor, FearConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut sensor = MockFearSensor::step_pattern();
    let config = FearConfig::default();
    
    // Initialize
    sensor.initialize(&config).await?;
    
    // Start detection
    let receiver = sensor.start().await?;
    
    // Process fear scores
    while let Ok(score) = receiver.recv().await {
        println!("Fear: {:.3}, Calibrated: {}", score.value, score.calibrated);
        
        // YOUR CODE: Use score.value to modify terrain/visuals
        if score.calibrated {
            update_terrain(score.value);
        }
    }
    
    // Stop when done
    sensor.stop().await?;
    Ok(())
}
```

### Bevy Integration Pattern
```rust
use bevy::prelude::*;
use spectremesh_fear_sensor::{FearSensor, MockFearSensor, FearScore};

#[derive(Resource)]
struct FearState {
    current_fear: f32,
    receiver: Option<async_channel::Receiver<FearScore>>,
    calibrated: bool,
}

fn setup_fear_sensor(mut commands: Commands) {
    // Initialize in separate task
    let (sender, receiver) = async_channel::unbounded();
    
    tokio::spawn(async move {
        let mut sensor = MockFearSensor::step_pattern();
        let config = FearConfig::default();
        sensor.initialize(&config).await.unwrap();
        let fear_receiver = sensor.start().await.unwrap();
        
        while let Ok(score) = fear_receiver.recv().await {
            let _ = sender.send(score).await;
        }
    });
    
    commands.insert_resource(FearState {
        current_fear: 0.0,
        receiver: Some(receiver),
        calibrated: false,
    });
}

fn update_fear_system(mut fear_state: ResMut<FearState>) {
    if let Some(receiver) = &fear_state.receiver {
        // Non-blocking receive
        while let Ok(score) = receiver.try_recv() {
            fear_state.current_fear = score.value;
            fear_state.calibrated = score.calibrated;
        }
    }
}

fn update_terrain_system(
    fear_state: Res<FearState>,
    mut terrain_query: Query<&mut TerrainComponent>,
) {
    if fear_state.calibrated {
        for mut terrain in terrain_query.iter_mut() {
            terrain.fear_level = fear_state.current_fear;
            // Update terrain mesh based on fear level
        }
    }
}
```

## Error Handling

### `FearError`
```rust
pub enum FearError {
    ModelNotFound { path: String },
    OnnxRuntime { message: String },
    NoFaceDetected,
    InvalidLogits { reason: String },
    CalibrationFailed { reason: String },
    CameraError(CameraError),
}
```

### `CameraError`
```rust
pub enum CameraError {
    NoCamerasFound,
    PermissionDenied,
    DeviceInUse,
    InvalidDevice { id: u32 },
    InitializationFailed { message: String },
}
```

## Testing Utilities

### Test Helpers
```rust
// Test with controlled fear pattern
#[tokio::test]
async fn test_fear_integration() {
    let mut sensor = MockFearSensor::new(vec![0.0, 0.5, 1.0]);
    let config = FearConfig::default();
    
    sensor.initialize(&config).await.unwrap();
    let receiver = sensor.start().await.unwrap();
    
    // Verify expected fear scores
    assert_eq!(receiver.recv().await.unwrap().value, 0.0);
    assert_eq!(receiver.recv().await.unwrap().value, 0.5);
    assert_eq!(receiver.recv().await.unwrap().value, 1.0);
}
```

### Spectreprobe CLI
```bash
# Test mock implementation
cargo run -p spectremesh --bin spectreprobe -- --mock

# Test real hardware
cargo run -p spectremesh --bin spectreprobe

# Test both side-by-side
cargo run -p spectremesh --bin spectreprobe -- --test-both
```

## Performance Notes

### Timing Expectations
- **Mock Sensor**: ~50ms per frame (configurable)
- **Real Sensor**: ~33ms per frame (30 FPS target)
- **ONNX Inference**: <10ms target on GTX 1050/M1
- **Calibration**: 30 seconds default duration

### Memory Usage
- **Mock Sensor**: Minimal (pattern array)
- **Real Sensor**: ~100MB (OpenCV + ONNX Runtime)
- **Model Files**: ~5MB (emotion model + face detector)

## Feature Flags

### Cargo Features
```toml
# Default: Real ONNX implementation
spectremesh-fear-sensor = { path = "../fear_sensor" }

# Mock implementation for testing
spectremesh-fear-sensor = { path = "../fear_sensor", features = ["mock"] }
```

### Type Aliases
```rust
// Automatically selects implementation based on features
use spectremesh_fear_sensor::DefaultFearSensor;

#[cfg(feature = "mock")]
type DefaultFearSensor = MockFearSensor;

#[cfg(not(feature = "mock"))]
type DefaultFearSensor = OnnxFearSensor;
```

## Integration Checklist

### For M0.5 Development
- [ ] Add `spectremesh-fear-sensor` dependency to game crate
- [ ] Use `MockFearSensor` for initial development
- [ ] Implement Bevy resource for fear state
- [ ] Create system to poll fear scores
- [ ] Connect fear level to terrain generation
- [ ] Add debug UI for manual fear control
- [ ] Test with `spectreprobe` utility

### For M1 Production
- [ ] Switch to `OnnxFearSensor` for real camera input
- [ ] Handle calibration period in UI
- [ ] Add camera permission handling
- [ ] Bundle model files with application
- [ ] Test end-to-end pipeline
- [ ] Performance optimization and profiling
