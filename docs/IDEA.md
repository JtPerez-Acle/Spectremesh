# SpectreMesh — Technical Specification v1.0
*Procedural horror terrain that literally feeds on the player's fear*

## 1. Project Overview

SpectreMesh is an innovative procedural horror game that creates a truly adaptive horror experience using real-time facial emotion recognition. The game dynamically modifies terrain, lighting, and triggers jumpscares based on the player's detected fear levels, creating a feedback loop where the game literally "feeds on" the player's emotional state.

### Core Innovation
- **Emotion-Driven Gameplay**: Real-time webcam analysis drives procedural content generation
- **Adaptive Horror**: Game intensity scales with player's actual fear response
- **Procedural Caves**: Marching cubes terrain that morphs based on emotional input
- **Privacy-First**: All processing happens locally, no data leaves the device

## 2. System Architecture

### High-Level Data Flow
```
[Webcam Stream] ──► [Fear Sensor Thread] ──► async_channel ──►
   Bevy App ▸ Resource<FearScore> ▸ Systems:
      • TerrainMutator (warps noise parameters)
      • JumpscareTrigger (stochastic events)
      • PostProcessFX (gloom/chromatic aberration)
      • AudioCrossfader (ambient layers)
```

### Threading Model
- **Main Thread**: Bevy ECS systems running at 60 FPS
- **Fear Sensor Thread**: tokio::task running webcam capture and inference at 20-30 FPS
- **Communication**: async_channel::Sender<FearScore> with non-blocking reads
- **Smoothing**: Exponential moving average (α ≈ 0.25) for stable fear values

## 3. Fear Detection System

### Emotion Recognition Model
| Component | Specification | Rationale |
|-----------|---------------|-----------|
| **Primary Model** | FaceONNX emotion head (MIT license, 2024 weights) | Modern ResNet architecture, 48×48 input, pre-optimized ONNX format |
| **Backup Options** | HSEmotionONNX (higher accuracy), Dyagnosys toolkit (Intel-optimized) | Performance A/B testing alternatives |
| **Input Processing** | 48×48 grayscale crops from 640×480 webcam stream | Optimized for <10ms inference on laptop GPU |
| **Licensing** | MIT-licensed models, commercial-friendly | No training required if accuracy sufficient |

### Calibration System
- **Baseline Establishment**: First 30 seconds compute per-player neutral mean + standard deviation
- **Normalization**: Convert raw logits → z-score → fear ∈ [0,1]
- **Adaptive Scaling**: Prevents stone-faced players from starving the feedback loop
- **Fallback Mode**: If camera unavailable, use configurable default fear level (0.3)

### Privacy & Consent
- **Explicit Consent**: First-boot dialogue for camera permission
- **No Storage**: Frames processed in memory only, never saved
- **Runtime Toggle**: Camera can be disabled anytime via settings
- **Transparency**: Clear "webcam disabled – ambient fear mode" indicator

## 4. Terrain Generation

### Chunk Architecture
| Parameter | Value | Notes |
|-----------|-------|-------|
| **Chunk Size** | 32³ voxels | ~35k triangles post-marching cubes |
| **Streaming Radius** | 3 chunks | ~192m cube around camera |
| **Active Chunks** | 9 total | ~315k triangles total |
| **Performance Target** | 60 FPS @ 1080p | GTX 1050 / Apple M1 baseline |

### Noise Generation Pipeline
1. **Base Layer**: fBM Perlin noise for cave structure
2. **Fear Warping**: 4D simplex noise seeded with current fear level
3. **Amplitude Scaling**: Fear multiplies noise amplitude (0.5× → 2.5×)
4. **Optional Spikes**: Ridged noise layer for dramatic fear-driven deformation

### Dynamic Terrain Response
- **Real-time Warping**: Noise parameters update each frame based on fear score
- **Smooth Transitions**: Interpolated changes prevent jarring terrain shifts
- **Performance Optimization**: GPU-driven rendering pipeline in Bevy 0.16

## 5. Development Environment

### Platform Support
| Platform | Status | Notes |
|----------|--------|-------|
| **Desktop** | Primary target | Windows, macOS, Linux |
| **Mobile** | Future consideration | Metal backend exists but may struggle with terrain + ONNX |
| **Minimum Hardware** | i5-8xxx + GTX 1050 or M1 Base | AVX2 helps for CPU inference fallback |

### Technology Stack
| Concern | Crate/Library | Version | Notes |
|---------|---------------|---------|-------|
| **Game Engine** | bevy | 0.16 | Stable release with GPU-driven render path |
| **Terrain** | fastnoise-lite, custom marching-cubes | latest | 32³ voxel chunks, 3-chunk streaming radius |
| **Webcam I/O** | opencv | 0.94 | 640×480 @ 20-30 FPS, mature bindings |
| **Vision Model** | FaceONNX | 2024 weights | 48×48 input, ResNet architecture, <10ms inference |
| **Async Plumbing** | tokio, async-channel | latest | Fear sensor runs outside Bevy's main thread |
| **Audio** | bevy_kira_audio | latest | Cross-fade ambient tracks based on fear level |
| **Shader Hot-reload** | bevy asset watch | built-in | Live GLSL changes in dev profile |

## 6. Game Design & User Experience

### Privacy & Safety Mechanisms
- **Explicit Consent**: First-boot camera permission dialogue
- **No Data Storage**: Frames processed in memory only, never saved to disk
- **Runtime Control**: Camera toggle available in settings at any time
- **Transparency**: Clear UI indicators when webcam is active/disabled

### Difficulty & Intensity Management
- **Sigmoid Curve Mapping**: Fear values mapped through sigmoid to prevent extremes
- **Daily Maxima Clamping**: Auto-lower intensity if player hovers near 1.0 for >30s
- **Individual Calibration**: Per-player baseline prevents stone-faced players from starving loop
- **Fallback Ambient Mode**: Default fear level (0.3) when camera unavailable

### Jumpscare System
- **Trigger Conditions**: fear_delta > 0.2 since last event
- **Cooldown Period**: 45 seconds + randomness jitter
- **Quality Focus**: Procedural spatial audio + shader flash vs stock screamer assets
- **Asset Constraints**: Jumpscare meshes ≤10k vertices with deforming shaders

## 7. Asset Pipeline & Content

### Repository Structure
```
spectremesh/
├─ Cargo.toml              # workspace configuration
├─ crates/
│   ├─ core/               # shared types, error handling, utilities
│   ├─ terrain/            # marching cubes + FastNoise implementation
│   ├─ fear_sensor/        # webcam capture + ONNX runtime
│   └─ game/               # Bevy app entry point
└─ assets/
    ├─ shaders/            # GLSL with hot-reload in dev profile
    ├─ sounds/             # 48kHz Ogg Vorbis loops for ambient audio
    └─ models/             # jumpscare meshes ≤10k vertices
```

### Asset Specifications
| Asset Type | Format | Quality | Notes |
|------------|--------|---------|-------|
| **Shaders** | GLSL | Hot-reload enabled | Flickering lights, fog pulses, chromatic aberration |
| **Audio** | Ogg Vorbis | 48kHz | Cross-fade ambient layers via bevy_kira_audio |
| **3D Models** | glTF/GLB | ≤10k vertices | Jumpscare assets with deforming shader support |
| **Textures** | PNG/KTX2 | 1024×1024 max | Procedural generation preferred over static assets |

### Cargo Workspace Features
- **Feature Flags**: CPU vs GPU model switching for fear_sensor crate
- **Development Profile**: Asset hot-reloading, debug overlays
- **Release Profile**: Optimized builds, stripped symbols

## 8. Testing & Development Workflow

### Fear Simulation for Development
| Method | Implementation | Use Case |
|--------|----------------|----------|
| **Mock Fear CLI** | `--mock-fear wave` flag | Sine wave fear simulation for testing |
| **Manual Override** | Debug UI sliders | Real-time fear value adjustment |
| **Recorded Sessions** | JSON playback | Reproducible test scenarios |

### Unit Testing Strategy
- **Terrain Tests**: Property tests on density(x,y,z) continuity across chunk boundaries
- **Sensor Tests**: Snapshot testing of logits→score conversion on sample images
- **Integration Tests**: End-to-end fear pipeline with mock webcam input
- **Performance Tests**: Frame rate benchmarks with various fear levels

### Debug Tools
- **Bevy egui Overlay**: Real-time sliders for fear, post-FX toggles, performance metrics
- **Console Commands**: Runtime parameter adjustment without recompilation
- **Profiling Integration**: Tracy/puffin integration for performance analysis
- **Visual Debugging**: Wireframe terrain, noise visualization, fear heatmaps

## 9. Milestone Roadmap & Execution Strategy

### Development Phases (Risk-Kill Ordering)
| Milestone | Deliverable | Acceptance Criteria | Why First? |
|-----------|-------------|-------------------|------------|
| **M0 Sensor-Only** | Tiny CLI that prints fear every frame | Webcam permissions + inference performance proven | Proves webcam perms + inference perf day 1 |
| **M0.5 Shader-Warp** | Bevy plane with height-map noise driven by mock fear slider | Visual terrain warping responds to fear input | Visual dopamine in <1 hour, proves concept |
| **M1 Merge** | Combine above → live warping terrain | Real webcam fear drives visible terrain changes | **Core Hook Proven** - rest becomes "nice to have" |
| **M2 Caves** | Procedural cave chunks with marching cubes | 60 FPS voxel terrain streaming | Polish the terrain system |
| **M3 Jumpscare** | Polished jumpscare asset & trigger logic | fear-delta > 0.2 triggers procedural scare | Add horror elements |
| **M4 Polish** | Post-FX, ambient audio, build script | `cargo build --release` → playable demo | Release Ready |

### Execution Strategy (Risk-First)
1. **M0 Sensor-Only**: Minimal CLI proves camera access and model inference work
2. **M0.5 Shader-Warp**: Quick Bevy setup with simple height-map + mock fear slider
3. **M1 Merge**: Connect real fear sensor to terrain warping - **CORE CONCEPT PROVEN**
4. **M2+ Polish**: Once the hook works, marching cubes and jumpscares become incremental improvements

This approach de-risks the fundamental concept (emotion → terrain) before investing in complex voxel systems.

## 10. Technical Constraints & Project Scope

### Budget & Resources
| Resource | Constraint | Impact |
|----------|------------|--------|
| **Cloud Services** | $0 budget | All processing must be local/on-device |
| **Asset Budget** | <$30 total | May purchase itch.io sound pack if needed |
| **Development Time** | 4-6 weekends | Side project timeline, demo-focused scope |
| **Team Size** | Solo developer + AI assistant | Async collaboration model |

### System Requirements
- **Save System**: Session-only, no persistence between runs
- **User Settings**: JSON file for camera consent, graphics presets, fear sensitivity
- **Networking**: None - strictly single-player experience
- **Accessibility**: Basic keyboard/mouse only, no gamepad support initially

### Risk Mitigation Strategies
| Risk | Mitigation Strategy |
|------|-------------------|
| **Model Accuracy** | Start with coarse emotion classes; fine-tune later if needed |
| **Performance Issues** | 128×128 grayscale crops, 4-frame batching, GPU execution provider |
| **Camera Permissions** | CLI tool `spectreprobe` to test cameras before Bevy launch |
| **Cheap Jumpscares** | Focus on procedural spatial audio + shader effects vs static assets |

## 11. Success Metrics & Definition of Done

### Technical Success Criteria
- **Performance**: Stable 60 FPS at 1080p on GTX 1050/M1 baseline hardware
- **Responsiveness**: <10ms fear detection latency on target hardware
- **Stability**: No crashes during 30-minute play sessions
- **Privacy**: Zero data transmission, all processing local

### User Experience Goals
- **Immersion**: Seamless integration of fear detection with gameplay
- **Adaptability**: Noticeable terrain changes correlate with player emotional state
- **Comfort**: Respect player privacy and provide clear control mechanisms
- **Replayability**: Procedural content ensures different experiences each session

### Demo Deliverable
- **Playable Build**: `cargo build --release` produces standalone executable
- **Documentation**: Updated README with setup instructions and system requirements
- **Video Demo**: Screen recording showing fear-responsive terrain in action
- **Code Quality**: Comprehensive test coverage and clean architecture for future development

## 12. Core Type System & Inter-Crate APIs

### 12.1 Shared Types (crates/core/src/types.rs)

```rust
use std::time::Instant;
use serde::{Deserialize, Serialize};

/// Core fear measurement with metadata for calibration and delta tracking
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FearScore {
    /// Normalized fear value [0.0, 1.0] after calibration
    pub value: f32,
    /// Raw model confidence [0.0, 1.0]
    pub confidence: f32,
    /// When this measurement was taken
    pub timestamp: Instant,
    /// Whether this score has been calibrated to player baseline
    pub calibrated: bool,
    /// Raw logits from emotion model (pre-normalization)
    pub raw_logits: [f32; 7], // 7 emotion classes
}

/// 3D coordinate for terrain chunks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// Individual voxel data within a chunk
#[derive(Debug, Clone, Copy, Default)]
pub struct VoxelData {
    /// Density value for marching cubes [0.0, 1.0]
    pub density: f32,
    /// Material type identifier
    pub material_id: u8,
}

/// Camera device information for enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraDevice {
    pub id: u32,
    pub name: String,
    pub resolution: (u32, u32),
    pub available: bool,
}

/// Noise generation parameters that can be modified by fear
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseParams {
    /// Base Perlin noise frequency
    pub base_frequency: f32,
    /// Fear-driven amplitude multiplier [0.5, 2.5]
    pub amplitude_multiplier: f32,
    /// 4D simplex noise seed (includes fear component)
    pub warp_seed: u64,
    /// Ridged noise contribution [0.0, 1.0]
    pub ridge_strength: f32,
}
```

### 12.2 Error Taxonomy & Handling

```rust
/// Top-level error type for the entire application
#[derive(Debug, thiserror::Error)]
pub enum SpectreError {
    #[error("Camera error: {0}")]
    Camera(#[from] CameraError),

    #[error("Fear detection error: {0}")]
    FearDetection(#[from] FearError),

    #[error("Terrain generation error: {0}")]
    Terrain(#[from] TerrainError),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Camera-specific errors (recoverable vs fatal)
#[derive(Debug, thiserror::Error)]
pub enum CameraError {
    #[error("No cameras found on system")]
    NoCamerasFound,

    #[error("Camera {id} permission denied")]
    PermissionDenied { id: u32 },

    #[error("Camera {id} disconnected during operation")]
    Disconnected { id: u32 },

    #[error("Unsupported resolution {width}x{height}")]
    UnsupportedResolution { width: u32, height: u32 },

    #[error("OpenCV error: {message}")]
    OpenCvError { message: String },
}

/// Fear detection and model inference errors
#[derive(Debug, thiserror::Error)]
pub enum FearError {
    #[error("ONNX model not found at path: {path}")]
    ModelNotFound { path: String },

    #[error("ONNX runtime error: {message}")]
    OnnxRuntime { message: String },

    #[error("Face detection failed - no faces found")]
    NoFaceDetected,

    #[error("Calibration incomplete - need {needed} more samples")]
    CalibrationIncomplete { needed: usize },

    #[error("Invalid emotion logits: {reason}")]
    InvalidLogits { reason: String },
}

/// Terrain generation and chunk management errors
#[derive(Debug, thiserror::Error)]
pub enum TerrainError {
    #[error("Chunk generation failed at {coord:?}: {reason}")]
    ChunkGenerationFailed { coord: ChunkCoord, reason: String },

    #[error("Marching cubes failed: {reason}")]
    MarchingCubesFailed { reason: String },

    #[error("Noise generation error: {reason}")]
    NoiseError { reason: String },
}

/// Configuration loading and validation errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Invalid configuration value for {key}: {reason}")]
    InvalidValue { key: String, reason: String },

    #[error("Missing required configuration: {key}")]
    MissingRequired { key: String },

    #[error("Configuration file not found: {path}")]
    FileNotFound { path: String },
}
```

### 12.3 Configuration Schema (settings.toml)

```toml
# SpectreMesh Configuration File
# Auto-generated on first run, user-editable

[camera]
# Camera device selection (0 = default, -1 = auto-detect)
device_id = 0
# Capture resolution [width, height]
resolution = [640, 480]
# Target FPS for webcam capture
fps = 30
# Enable camera on startup (requires user consent)
enabled = true

[fear_detection]
# Path to ONNX emotion model (relative to executable)
model_path = "assets/models/face_emotion.onnx"
# Calibration period in seconds
calibration_duration = 30.0
# Default fear level when camera disabled [0.0, 1.0]
default_fear = 0.3
# Confidence threshold for face detection [0.0, 1.0]
confidence_threshold = 0.7
# EMA smoothing factor [0.0, 1.0]
smoothing_alpha = 0.25

[terrain]
# Voxel chunk size (must be power of 2)
chunk_size = 32
# Number of chunks to stream around player
streaming_radius = 3
# Base noise frequency
base_frequency = 0.01
# Fear amplitude range [min, max]
fear_amplitude_range = [0.5, 2.5]

[graphics]
# Target resolution [width, height]
resolution = [1920, 1080]
# Target FPS
target_fps = 60
# VSync enabled
vsync = true
# MSAA samples (0, 2, 4, 8)
msaa_samples = 4

[audio]
# Master volume [0.0, 1.0]
master_volume = 0.8
# Number of ambient audio layers
ambient_layers = 3
# Audio crossfade duration in seconds
crossfade_duration = 2.0

[debug]
# Show debug overlay
show_overlay = false
# Enable wireframe rendering
wireframe = false
# Show chunk boundaries
show_chunks = false
# Log level (error, warn, info, debug, trace)
log_level = "info"

[privacy]
# User has consented to camera usage
camera_consent = false
# Timestamp of consent (ISO 8601)
consent_timestamp = ""
```

### 12.4 Inter-Crate Trait Contracts

```rust
/// Core trait for fear detection implementations
#[async_trait::async_trait]
pub trait FearSensor: Send + Sync {
    /// Initialize the sensor with given configuration
    async fn initialize(&mut self, config: &FearConfig) -> Result<(), FearError>;

    /// Start continuous fear detection
    async fn start(&mut self) -> Result<async_channel::Receiver<FearScore>, FearError>;

    /// Stop detection and cleanup resources
    async fn stop(&mut self) -> Result<(), FearError>;

    /// Get available camera devices
    async fn enumerate_cameras(&self) -> Result<Vec<CameraDevice>, CameraError>;

    /// Check if sensor is currently calibrated
    fn is_calibrated(&self) -> bool;

    /// Get current calibration progress [0.0, 1.0]
    fn calibration_progress(&self) -> f32;
}

/// Trait for terrain chunk generation
pub trait TerrainGenerator: Send + Sync {
    /// Generate a single chunk at the given coordinate
    fn generate_chunk(&self, coord: ChunkCoord, noise_params: &NoiseParams) -> Result<TerrainChunk, TerrainError>;

    /// Update noise parameters (called when fear changes)
    fn update_noise_params(&mut self, fear_score: FearScore) -> NoiseParams;

    /// Get the current noise parameters
    fn current_noise_params(&self) -> &NoiseParams;
}

/// Trait for noise generation implementations
pub trait NoiseProvider: Send + Sync {
    /// Sample noise at given 3D coordinate
    fn sample_3d(&self, x: f32, y: f32, z: f32) -> f32;

    /// Sample 4D noise (includes time/fear dimension)
    fn sample_4d(&self, x: f32, y: f32, z: f32, w: f32) -> f32;

    /// Update parameters based on fear level
    fn update_fear_influence(&mut self, fear: f32);
}

/// Configuration structures for each crate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FearConfig {
    pub model_path: String,
    pub calibration_duration: f32,
    pub default_fear: f32,
    pub confidence_threshold: f32,
    pub smoothing_alpha: f32,
    pub camera: CameraConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraConfig {
    pub device_id: i32,
    pub resolution: (u32, u32),
    pub fps: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainConfig {
    pub chunk_size: u32,
    pub streaming_radius: u32,
    pub base_frequency: f32,
    pub fear_amplitude_range: (f32, f32),
}
```

## 13. Algorithm Implementation Details

### 13.1 ONNX Integration Pipeline

```rust
/// Complete ONNX integration implementation for M0
pub struct FaceOnnxSensor {
    session: Option<ort::Session>,
    calibrator: FearCalibrator,
    camera: opencv::videoio::VideoCapture,
    face_detector: opencv::objdetect::CascadeClassifier,
}

impl FaceOnnxSensor {
    /// Step 1: Load ONNX model with appropriate execution provider
    pub fn load_model(model_path: &str) -> Result<ort::Session, FearError> {
        let session = ort::Session::builder()?
            .with_optimization_level(ort::GraphOptimizationLevel::All)?
            .with_intra_threads(1)?
            // Try GPU first, fallback to CPU
            .with_execution_providers([
                ort::ExecutionProvider::CUDA(Default::default()),
                ort::ExecutionProvider::CPU(Default::default()),
            ])?
            .commit_from_file(model_path)?;
        Ok(session)
    }

    /// Step 2: OpenCV frame preprocessing for ONNX input
    pub fn preprocess_frame(frame: &opencv::core::Mat) -> Result<ndarray::Array4<f32>, FearError> {
        // 1. Convert BGR to RGB
        let mut rgb_frame = opencv::core::Mat::default();
        opencv::imgproc::cvt_color(frame, &mut rgb_frame, opencv::imgproc::COLOR_BGR2RGB, 0)?;

        // 2. Detect face using Haar cascade
        let mut faces = opencv::core::Vector::<opencv::core::Rect>::new();
        self.face_detector.detect_multi_scale(
            &rgb_frame,
            &mut faces,
            1.1,  // scale factor
            3,    // min neighbors
            0,    // flags
            opencv::core::Size::new(30, 30), // min size
            opencv::core::Size::new(300, 300), // max size
        )?;

        if faces.is_empty() {
            return Err(FearError::NoFaceDetected);
        }

        // 3. Crop to largest face
        let face_rect = faces.get(0)?;
        let face_roi = opencv::core::Mat::roi(&rgb_frame, face_rect)?;

        // 4. Resize to 48x48 for FaceONNX
        let mut resized = opencv::core::Mat::default();
        opencv::imgproc::resize(
            &face_roi,
            &mut resized,
            opencv::core::Size::new(48, 48),
            0.0, 0.0,
            opencv::imgproc::INTER_LINEAR,
        )?;

        // 5. Convert to grayscale
        let mut gray = opencv::core::Mat::default();
        opencv::imgproc::cvt_color(&resized, &mut gray, opencv::imgproc::COLOR_RGB2GRAY, 0)?;

        // 6. Normalize to [0.0, 1.0] and convert to NCHW format
        let data: Vec<u8> = gray.data_bytes()?.to_vec();
        let normalized: Vec<f32> = data.iter().map(|&x| x as f32 / 255.0).collect();

        // Reshape to [1, 1, 48, 48] (batch, channels, height, width)
        let array = ndarray::Array4::from_shape_vec((1, 1, 48, 48), normalized)
            .map_err(|e| FearError::InvalidLogits { reason: e.to_string() })?;

        Ok(array)
    }

    /// Step 3: Run ONNX inference and extract emotion logits
    pub fn run_inference(&self, input: ndarray::Array4<f32>) -> Result<[f32; 7], FearError> {
        let session = self.session.as_ref().ok_or(FearError::OnnxRuntime {
            message: "Session not initialized".to_string()
        })?;

        // Create input tensor
        let input_tensor = ort::Value::from_array(session.allocator(), &input)?;

        // Run inference
        let outputs = session.run(vec![input_tensor])?;

        // Extract emotion logits (assuming single output)
        let output_tensor = &outputs[0];
        let logits: &[f32] = output_tensor.try_extract()?;

        if logits.len() != 7 {
            return Err(FearError::InvalidLogits {
                reason: format!("Expected 7 emotion classes, got {}", logits.len())
            });
        }

        let mut result = [0.0; 7];
        result.copy_from_slice(&logits[0..7]);
        Ok(result)
    }
}
```

### 13.2 Fear Calibration Mathematics

```rust
/// Implements the z-score normalization for fear calibration
pub struct FearCalibrator {
    baseline_samples: Vec<f32>,
    target_samples: usize,
    mean: f32,
    std_dev: f32,
    calibrated: bool,
}

impl FearCalibrator {
    pub fn new(calibration_duration: f32, fps: f32) -> Self {
        let target_samples = (calibration_duration * fps) as usize;
        Self {
            baseline_samples: Vec::with_capacity(target_samples),
            target_samples,
            mean: 0.0,
            std_dev: 1.0,
            calibrated: false,
        }
    }

    /// Add a raw emotion logit sample during calibration period
    pub fn add_sample(&mut self, raw_fear_logit: f32) -> Result<(), FearError> {
        if self.calibrated {
            return Ok(()); // Already calibrated, ignore
        }

        self.baseline_samples.push(raw_fear_logit);

        if self.baseline_samples.len() >= self.target_samples {
            self.compute_baseline()?;
        }

        Ok(())
    }

    /// Compute mean and standard deviation from baseline samples
    fn compute_baseline(&mut self) -> Result<(), FearError> {
        if self.baseline_samples.is_empty() {
            return Err(FearError::CalibrationIncomplete { needed: self.target_samples });
        }

        // Calculate mean
        self.mean = self.baseline_samples.iter().sum::<f32>() / self.baseline_samples.len() as f32;

        // Calculate standard deviation
        let variance = self.baseline_samples
            .iter()
            .map(|x| (x - self.mean).powi(2))
            .sum::<f32>() / self.baseline_samples.len() as f32;

        self.std_dev = variance.sqrt();

        // Handle edge case: if std_dev is too small, use default
        if self.std_dev < 0.01 {
            self.std_dev = 1.0;
        }

        self.calibrated = true;
        Ok(())
    }

    /// Convert raw fear logit to normalized fear score [0.0, 1.0]
    pub fn normalize_fear(&self, raw_fear_logit: f32) -> f32 {
        if !self.calibrated {
            // During calibration, return neutral fear
            return 0.3;
        }

        // Z-score normalization
        let z_score = (raw_fear_logit - self.mean) / self.std_dev;

        // Convert z-score to [0, 1] using sigmoid function
        // This maps z-scores: -3 -> ~0.05, 0 -> 0.5, +3 -> ~0.95
        let sigmoid = 1.0 / (1.0 + (-z_score).exp());

        // Clamp to [0, 1] for safety
        sigmoid.clamp(0.0, 1.0)
    }

    /// Extract fear component from 7-class emotion logits
    /// Assumes: [angry, disgust, fear, happy, sad, surprise, neutral]
    pub fn extract_fear_logit(emotion_logits: &[f32; 7]) -> f32 {
        // FaceONNX emotion classes (typical order)
        const FEAR_INDEX: usize = 2;
        emotion_logits[FEAR_INDEX]
    }

    pub fn is_calibrated(&self) -> bool {
        self.calibrated
    }

    pub fn calibration_progress(&self) -> f32 {
        (self.baseline_samples.len() as f32 / self.target_samples as f32).min(1.0)
    }
}
```

### 13.3 Noise Generation Formulas

```rust
/// Implements the exact noise pipeline: fBM Perlin + 4D simplex warp + fear scaling
pub struct FearDrivenNoise {
    perlin: fastnoise_lite::FastNoiseLite,
    simplex: fastnoise_lite::FastNoiseLite,
    current_fear: f32,
    base_frequency: f32,
    fear_amplitude_range: (f32, f32),
}

impl FearDrivenNoise {
    pub fn new(base_frequency: f32, fear_amplitude_range: (f32, f32)) -> Self {
        let mut perlin = fastnoise_lite::FastNoiseLite::new();
        perlin.set_noise_type(Some(fastnoise_lite::NoiseType::Perlin));
        perlin.set_frequency(Some(base_frequency));
        perlin.set_fractal_type(Some(fastnoise_lite::FractalType::FBm));
        perlin.set_fractal_octaves(Some(4));
        perlin.set_fractal_lacunarity(Some(2.0));
        perlin.set_fractal_gain(Some(0.5));

        let mut simplex = fastnoise_lite::FastNoiseLite::new();
        simplex.set_noise_type(Some(fastnoise_lite::NoiseType::OpenSimplex2));
        simplex.set_frequency(Some(base_frequency * 0.5)); // Lower frequency for warping

        Self {
            perlin,
            simplex,
            current_fear: 0.3,
            base_frequency,
            fear_amplitude_range,
        }
    }

    /// Core noise sampling function - this is the exact formula used for terrain
    pub fn sample_density(&self, x: f32, y: f32, z: f32) -> f32 {
        // Step 1: Generate base fBM Perlin noise
        let base_noise = self.perlin.get_noise_3d(x, y, z);

        // Step 2: Generate 4D simplex warp (4th dimension is fear-driven)
        let fear_time = self.current_fear * 100.0; // Scale fear to reasonable 4D coordinate
        let warp_x = self.simplex.get_noise_3d(x * 0.1, y * 0.1, z * 0.1) * 10.0;
        let warp_y = self.simplex.get_noise_3d(x * 0.1 + 100.0, y * 0.1, z * 0.1) * 10.0;
        let warp_z = self.simplex.get_noise_3d(x * 0.1, y * 0.1 + 100.0, z * 0.1) * 10.0;

        // Step 3: Sample warped Perlin noise
        let warped_noise = self.perlin.get_noise_3d(
            x + warp_x,
            y + warp_y,
            z + warp_z,
        );

        // Step 4: Apply fear-driven amplitude scaling
        let fear_multiplier = self.fear_to_amplitude_multiplier(self.current_fear);
        let scaled_noise = warped_noise * fear_multiplier;

        // Step 5: Combine base and warped noise
        let combined = (base_noise * 0.6) + (scaled_noise * 0.4);

        // Step 6: Convert to density [0, 1] for marching cubes
        // Positive values = solid, negative = air
        (combined + 1.0) * 0.5 // Map [-1, 1] to [0, 1]
    }

    /// Convert fear level [0, 1] to amplitude multiplier [0.5, 2.5]
    fn fear_to_amplitude_multiplier(&self, fear: f32) -> f32 {
        let (min_amp, max_amp) = self.fear_amplitude_range;
        min_amp + (fear * (max_amp - min_amp))
    }

    /// Update noise parameters when fear changes
    pub fn update_fear(&mut self, new_fear: f32) {
        self.current_fear = new_fear.clamp(0.0, 1.0);

        // Optionally update noise seeds for more dramatic changes
        if (new_fear - self.current_fear).abs() > 0.1 {
            let new_seed = (new_fear * 1000.0) as i32;
            self.simplex.set_seed(Some(new_seed));
        }
    }
}

/// Terrain chunk data structure for marching cubes
#[derive(Debug, Clone)]
pub struct TerrainChunk {
    pub coord: ChunkCoord,
    pub voxels: Vec<VoxelData>, // 32³ = 32,768 voxels
    pub mesh: Option<TerrainMesh>,
    pub dirty: bool, // Needs regeneration
}

impl TerrainChunk {
    pub const SIZE: usize = 32;
    pub const TOTAL_VOXELS: usize = Self::SIZE * Self::SIZE * Self::SIZE;

    pub fn new(coord: ChunkCoord) -> Self {
        Self {
            coord,
            voxels: vec![VoxelData::default(); Self::TOTAL_VOXELS],
            mesh: None,
            dirty: true,
        }
    }

    /// Convert 3D voxel coordinate to 1D array index
    pub fn voxel_index(x: usize, y: usize, z: usize) -> usize {
        x + (y * Self::SIZE) + (z * Self::SIZE * Self::SIZE)
    }

    /// Generate voxel densities using noise
    pub fn generate_voxels(&mut self, noise: &FearDrivenNoise) {
        let chunk_world_pos = (
            self.coord.x as f32 * Self::SIZE as f32,
            self.coord.y as f32 * Self::SIZE as f32,
            self.coord.z as f32 * Self::SIZE as f32,
        );

        for z in 0..Self::SIZE {
            for y in 0..Self::SIZE {
                for x in 0..Self::SIZE {
                    let world_x = chunk_world_pos.0 + x as f32;
                    let world_y = chunk_world_pos.1 + y as f32;
                    let world_z = chunk_world_pos.2 + z as f32;

                    let density = noise.sample_density(world_x, world_y, world_z);
                    let index = Self::voxel_index(x, y, z);

                    self.voxels[index] = VoxelData {
                        density,
                        material_id: if density > 0.5 { 1 } else { 0 }, // Simple air/stone
                    };
                }
            }
        }

        self.dirty = true;
    }
}

/// Mesh data for rendering
#[derive(Debug, Clone)]
pub struct TerrainMesh {
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub triangle_count: usize,
}
```

## 14. Bevy ECS Design

### 14.1 Component Definitions

```rust
use bevy::prelude::*;

/// Component for terrain chunks in the world
#[derive(Component, Debug)]
pub struct TerrainChunkComponent {
    pub coord: ChunkCoord,
    pub chunk_data: TerrainChunk,
    pub needs_mesh_update: bool,
}

/// Component for the player camera/controller
#[derive(Component, Debug)]
pub struct Player {
    pub move_speed: f32,
    pub look_sensitivity: f32,
}

/// Component for entities that respond to fear (lighting, effects, etc.)
#[derive(Component, Debug)]
pub struct FearResponsive {
    pub base_intensity: f32,
    pub fear_multiplier: f32,
    pub response_type: FearResponseType,
}

#[derive(Debug, Clone)]
pub enum FearResponseType {
    LightFlicker { frequency_range: (f32, f32) },
    ChromaticAberration { strength_range: (f32, f32) },
    AudioVolume { volume_range: (f32, f32) },
}

/// Marker component for jumpscares
#[derive(Component, Debug)]
pub struct JumpscareEntity {
    pub trigger_threshold: f32,
    pub cooldown_remaining: f32,
}
```

### 14.2 Resource Structures

```rust
/// Global fear state resource
#[derive(Resource, Debug)]
pub struct FearState {
    pub current_score: FearScore,
    pub previous_score: FearScore,
    pub fear_history: VecDeque<FearScore>, // For delta calculations
    pub calibration_complete: bool,
    pub receiver: Option<async_channel::Receiver<FearScore>>,
}

impl FearState {
    pub fn fear_delta(&self) -> f32 {
        self.current_score.value - self.previous_score.value
    }

    pub fn is_fear_rising(&self) -> bool {
        self.fear_delta() > 0.05 // Threshold for "rising"
    }
}

/// Terrain management resource
#[derive(Resource, Debug)]
pub struct TerrainManager {
    pub noise_generator: FearDrivenNoise,
    pub active_chunks: HashMap<ChunkCoord, Entity>,
    pub player_chunk: ChunkCoord,
    pub streaming_radius: i32,
    pub chunk_generation_queue: VecDeque<ChunkCoord>,
}

/// Configuration resource
#[derive(Resource, Debug, Clone)]
pub struct GameConfig {
    pub fear: FearConfig,
    pub terrain: TerrainConfig,
    pub graphics: GraphicsConfig,
    pub audio: AudioConfig,
    pub debug: DebugConfig,
}

/// Debug overlay resource
#[derive(Resource, Debug)]
pub struct DebugOverlay {
    pub show_overlay: bool,
    pub fear_slider_value: f32, // For M0.5 mock fear
    pub wireframe_enabled: bool,
    pub show_chunk_boundaries: bool,
}
```

### 14.3 System Organization & Scheduling

```rust
/// System sets for proper ordering
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum GameSystemSet {
    // Input and fear detection (runs first)
    Input,
    FearProcessing,

    // Core game logic
    TerrainGeneration,
    PlayerMovement,

    // Rendering and effects (runs last)
    Rendering,
    PostProcessing,
    Audio,

    // Debug systems
    Debug,
}

/// Main system scheduling configuration
pub fn configure_systems(app: &mut App) {
    app
        // Configure system sets
        .configure_sets(Update, (
            GameSystemSet::Input,
            GameSystemSet::FearProcessing,
            GameSystemSet::TerrainGeneration,
            GameSystemSet::PlayerMovement,
            GameSystemSet::Rendering,
            GameSystemSet::PostProcessing,
            GameSystemSet::Audio,
            GameSystemSet::Debug,
        ).chain())

        // Input systems
        .add_systems(Update, (
            handle_keyboard_input,
            handle_mouse_input,
        ).in_set(GameSystemSet::Input))

        // Fear processing systems
        .add_systems(Update, (
            receive_fear_scores,
            update_fear_state,
            apply_fear_to_noise,
        ).chain().in_set(GameSystemSet::FearProcessing))

        // Terrain systems
        .add_systems(Update, (
            update_player_chunk,
            queue_chunk_generation,
            generate_terrain_chunks,
            update_chunk_meshes,
            despawn_distant_chunks,
        ).chain().in_set(GameSystemSet::TerrainGeneration))

        // Player systems
        .add_systems(Update, (
            move_player,
            update_camera,
        ).in_set(GameSystemSet::PlayerMovement))

        // Rendering systems
        .add_systems(Update, (
            update_fear_responsive_lighting,
            trigger_jumpscares,
        ).in_set(GameSystemSet::Rendering))

        // Debug systems
        .add_systems(Update, (
            update_debug_overlay,
            handle_debug_input,
        ).in_set(GameSystemSet::Debug));
}
```

### 14.4 Event Types & Flow

```rust
/// Events for fear-driven gameplay
#[derive(Event, Debug)]
pub struct FearUpdatedEvent {
    pub new_score: FearScore,
    pub delta: f32,
}

#[derive(Event, Debug)]
pub struct TerrainChunkGeneratedEvent {
    pub coord: ChunkCoord,
    pub entity: Entity,
}

#[derive(Event, Debug)]
pub struct JumpscareTriggeredEvent {
    pub fear_level: f32,
    pub scare_type: JumpscareType,
}

#[derive(Debug, Clone)]
pub enum JumpscareType {
    AudioSting,
    VisualFlash,
    TerrainShift,
    Combined,
}

/// Event handling systems
pub fn handle_fear_events(
    mut fear_events: EventReader<FearUpdatedEvent>,
    mut terrain_manager: ResMut<TerrainManager>,
) {
    for event in fear_events.read() {
        // Update noise generator with new fear level
        terrain_manager.noise_generator.update_fear(event.new_score.value);

        // Mark nearby chunks as dirty for regeneration
        if event.delta.abs() > 0.1 {
            // Significant fear change - regenerate nearby terrain
            // Implementation details...
        }
    }
}
```

## 15. Development Infrastructure

### 15.1 Build System Configuration

#### Root Cargo.toml (Workspace)
```toml
[workspace]
members = [
    "crates/core",
    "crates/fear_sensor",
    "crates/terrain",
    "crates/game",
]
resolver = "2"

[workspace.dependencies]
# Core dependencies with exact versions
bevy = { version = "0.16", default-features = false }
tokio = { version = "1.0", features = ["rt-multi-thread", "macros"] }
async-channel = "2.0"
serde = { version = "1.0", features = ["derive"] }
thiserror = "1.0"
tracing = "0.1"

# Computer vision and ML
opencv = { version = "0.94", default-features = false, features = ["opencv-4"] }
ort = { version = "2.0", features = ["cuda", "tensorrt"] }
ndarray = "0.15"

# Noise and terrain
fastnoise-lite = "1.0"

# Configuration and serialization
toml = "0.8"
dirs = "5.0"

# Development dependencies
async-trait = "0.1"

[workspace.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "docsrs"]

# Profile configurations
[profile.dev]
opt-level = 1
debug = true
split-debuginfo = "unpacked"

[profile.release]
opt-level = 3
debug = false
lto = "thin"
codegen-units = 1
panic = "abort"

[profile.dev.package."*"]
opt-level = 2  # Optimize dependencies in dev mode for better performance
```

#### crates/game/Cargo.toml (Main executable)
```toml
[package]
name = "spectremesh"
version = "0.1.0"
edition = "2021"
authors = ["SpectreMesh Team"]
description = "Procedural horror terrain that feeds on player fear"
license = "MIT"
repository = "https://github.com/user/spectremesh"

[[bin]]
name = "spectremesh"
path = "src/main.rs"

[[bin]]
name = "spectreprobe"
path = "src/bin/spectreprobe.rs"

[dependencies]
# Workspace crates
spectremesh-core = { path = "../core" }
spectremesh-fear-sensor = { path = "../fear_sensor" }
spectremesh-terrain = { path = "../terrain" }

# Bevy with required features
bevy = { workspace = true, features = [
    "bevy_winit",
    "bevy_render",
    "bevy_core_pipeline",
    "bevy_pbr",
    "bevy_asset",
    "bevy_scene",
    "bevy_gltf",
    "x11",  # Linux
    "wayland",  # Linux
    "file_watcher",  # Asset hot-reloading
    "png",
    "hdr",
    "ktx2",
    "zstd",
] }

# Audio
bevy_kira_audio = "0.20"

# UI and debug
bevy_egui = "0.29"

# Async runtime
tokio = { workspace = true }
async-channel = { workspace = true }

# Configuration
serde = { workspace = true }
toml = { workspace = true }
dirs = { workspace = true }

# Logging
tracing = { workspace = true }
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Error handling
thiserror = { workspace = true }
anyhow = "1.0"

[features]
default = ["cpu-inference"]
cpu-inference = ["spectremesh-fear-sensor/cpu"]
gpu-inference = ["spectremesh-fear-sensor/gpu"]
mock-fear = []  # For testing without camera
debug-overlay = []  # Always show debug UI
```

#### crates/fear_sensor/Cargo.toml
```toml
[package]
name = "spectremesh-fear-sensor"
version = "0.1.0"
edition = "2021"

[dependencies]
spectremesh-core = { path = "../core" }

# Computer vision
opencv = { workspace = true, features = ["imgproc", "objdetect", "videoio"] }

# ONNX runtime with conditional features
ort = { workspace = true, optional = true }

# Math and arrays
ndarray = { workspace = true }

# Async
tokio = { workspace = true }
async-channel = { workspace = true }
async-trait = { workspace = true }

# Utilities
serde = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }

[features]
default = ["cpu"]
cpu = ["ort/cpu"]
gpu = ["ort/cuda", "ort/tensorrt"]
mock = []  # Mock implementation for testing
```

### 15.2 Testing Framework & Test Data

#### Test Configuration (tests/common/mod.rs)
```rust
use spectremesh_core::*;
use std::path::PathBuf;

/// Test data directory structure
pub struct TestData {
    pub models_dir: PathBuf,
    pub sample_images_dir: PathBuf,
    pub config_dir: PathBuf,
}

impl TestData {
    pub fn new() -> Self {
        let test_data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data");
        Self {
            models_dir: test_data_root.join("models"),
            sample_images_dir: test_data_root.join("images"),
            config_dir: test_data_root.join("configs"),
        }
    }

    /// Get path to test ONNX model
    pub fn test_model_path(&self) -> PathBuf {
        self.models_dir.join("test_emotion_model.onnx")
    }

    /// Get sample face images for testing
    pub fn sample_face_images(&self) -> Vec<PathBuf> {
        std::fs::read_dir(&self.sample_images_dir)
            .unwrap()
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                if path.extension()? == "png" {
                    Some(path)
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Mock fear sensor for testing
pub struct MockFearSensor {
    pub fear_sequence: Vec<f32>,
    pub current_index: usize,
}

#[async_trait::async_trait]
impl FearSensor for MockFearSensor {
    async fn initialize(&mut self, _config: &FearConfig) -> Result<(), FearError> {
        Ok(())
    }

    async fn start(&mut self) -> Result<async_channel::Receiver<FearScore>, FearError> {
        let (sender, receiver) = async_channel::unbounded();

        // Spawn task to send mock fear scores
        let sequence = self.fear_sequence.clone();
        tokio::spawn(async move {
            for fear_value in sequence.iter().cycle() {
                let score = FearScore {
                    value: *fear_value,
                    confidence: 0.9,
                    timestamp: std::time::Instant::now(),
                    calibrated: true,
                    raw_logits: [0.1, 0.1, *fear_value, 0.1, 0.1, 0.1, 0.5],
                };

                if sender.send(score).await.is_err() {
                    break;
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(33)).await; // ~30 FPS
            }
        });

        Ok(receiver)
    }

    async fn stop(&mut self) -> Result<(), FearError> {
        Ok(())
    }

    async fn enumerate_cameras(&self) -> Result<Vec<CameraDevice>, CameraError> {
        Ok(vec![CameraDevice {
            id: 999,
            name: "Mock Camera".to_string(),
            resolution: (640, 480),
            available: true,
        }])
    }

    fn is_calibrated(&self) -> bool { true }
    fn calibration_progress(&self) -> f32 { 1.0 }
}
```

#### Integration Test Example (tests/integration_test.rs)
```rust
use spectremesh_core::*;
use spectremesh_fear_sensor::*;
use spectremesh_terrain::*;

mod common;

#[tokio::test]
async fn test_fear_to_terrain_pipeline() {
    // Setup mock fear sensor with known sequence
    let mut mock_sensor = common::MockFearSensor {
        fear_sequence: vec![0.1, 0.3, 0.7, 0.9, 0.5], // Low to high to medium fear
        current_index: 0,
    };

    // Initialize sensor
    let config = FearConfig::default();
    mock_sensor.initialize(&config).await.unwrap();

    // Start fear detection
    let fear_receiver = mock_sensor.start().await.unwrap();

    // Setup terrain generator
    let mut terrain_gen = TerrainGenerator::new(TerrainConfig::default());

    // Test that fear changes affect terrain generation
    let mut previous_noise_params = None;

    for _ in 0..5 {
        // Receive fear score
        let fear_score = fear_receiver.recv().await.unwrap();

        // Update terrain noise
        let noise_params = terrain_gen.update_noise_params(fear_score);

        // Verify noise parameters change with fear
        if let Some(prev) = previous_noise_params {
            if (fear_score.value - 0.5).abs() > 0.1 {
                // Significant fear change should affect amplitude
                assert_ne!(noise_params.amplitude_multiplier, prev);
            }
        }

        previous_noise_params = Some(noise_params.amplitude_multiplier);

        // Generate a test chunk
        let chunk_coord = ChunkCoord { x: 0, y: 0, z: 0 };
        let chunk = terrain_gen.generate_chunk(chunk_coord, &noise_params).unwrap();

        // Verify chunk was generated
        assert_eq!(chunk.coord, chunk_coord);
        assert_eq!(chunk.voxels.len(), TerrainChunk::TOTAL_VOXELS);
    }
}

#[test]
fn test_fear_calibration_mathematics() {
    let mut calibrator = FearCalibrator::new(30.0, 1.0); // 30 samples at 1 FPS

    // Add baseline samples (neutral emotion)
    let baseline_samples = vec![0.2, 0.25, 0.18, 0.22, 0.19, 0.21, 0.23, 0.20, 0.24, 0.17];
    for sample in baseline_samples {
        calibrator.add_sample(sample).unwrap();
    }

    // Should not be calibrated yet (need 30 samples)
    assert!(!calibrator.is_calibrated());

    // Add remaining samples
    for _ in 0..20 {
        calibrator.add_sample(0.2).unwrap();
    }

    // Should now be calibrated
    assert!(calibrator.is_calibrated());

    // Test normalization
    let neutral_fear = calibrator.normalize_fear(0.2); // Baseline value
    assert!((neutral_fear - 0.5).abs() < 0.1); // Should be near 0.5

    let high_fear = calibrator.normalize_fear(0.8); // High fear
    assert!(high_fear > 0.7); // Should be high

    let low_fear = calibrator.normalize_fear(0.05); // Low fear
    assert!(low_fear < 0.3); // Should be low
}
```