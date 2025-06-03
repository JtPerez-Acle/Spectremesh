# M0 → M0.5 Developer Handoff Guide
**From**: Sensor-Only (COMPLETED) → **To**: Shader Warp (Visual Proof of Concept)

## What Was Accomplished in M0 ✅

### 🎯 **PRIMARY ACHIEVEMENT: REAL HARDWARE INTEGRATION VALIDATED**
- **Risk-Kill Success**: Core fear detection technology proven with actual hardware
- **Production Ready**: Real OpenCV camera capture + ONNX Runtime emotion recognition
- **Development Ready**: Mock implementation for CI/testing without hardware dependencies

### Technical Foundation Built
1. **Real Fear Detection Pipeline**: OpenCV → Face Detection → ONNX → Calibration
2. **Thread-Safe Async Architecture**: tokio spawning with proper Send/Sync handling
3. **Comprehensive Error Handling**: Graceful degradation for all failure modes
4. **Dual Implementation Strategy**: Mock for development, Real for production
5. **Complete Test Coverage**: 24 passing tests including real hardware validation

## What You're Inheriting 🎁

### Solid Foundation
```rust
// Fear detection is DONE and WORKING
use spectremesh_fear_sensor::{FearSensor, MockFearSensor, OnnxFearSensor};

// This works RIGHT NOW:
let mut sensor = MockFearSensor::new(vec![0.3, 0.5, 0.7]); // or OnnxFearSensor::new()
sensor.initialize(&config).await?;
let receiver = sensor.start().await?;

// Real-time fear scores at 30 FPS
while let Ok(fear_score) = receiver.recv().await {
    println!("Fear: {:.3}, Calibrated: {}", fear_score.value, fear_score.calibrated);
    // YOUR JOB: Use this to modify terrain!
}
```

### Key Files You'll Work With
- **`spectremesh_fear_sensor`**: Complete fear detection (don't touch, it works)
- **`crates/game/src/main.rs`**: Your Bevy application entry point
- **`crates/game/src/systems/`**: Where you'll add terrain systems
- **`assets/shaders/`**: Where you'll create terrain displacement shaders

## Your Mission: M0.5 (Shader Warp) 🎯

### Objective
Create **immediate visual feedback** showing fear-responsive terrain using Bevy 0.16.

### What You Need to Build
1. **Basic Bevy App**: 3D scene with camera controls
2. **Simple Terrain**: Height-map based mesh (not full voxels yet)
3. **Debug UI**: egui slider for manual fear control
4. **Terrain Shaders**: Vertex displacement based on fear level
5. **Fear Integration**: Connect mock sensor to terrain updates

### Success Criteria (Copy from DEVELOPMENT_PLAN.md)
- [ ] Bevy app launches and displays 3D terrain mesh
- [ ] Fear slider in debug UI immediately affects terrain height/shape
- [ ] Terrain updates smoothly without frame rate drops (60 FPS)
- [ ] Camera controls allow inspection of terrain from multiple angles
- [ ] Visual changes are clearly noticeable when fear level changes

## Technical Integration Strategy

### Phase 1: Basic Bevy Setup (Week 1)
```rust
// Start here - basic Bevy app
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_scene)
        .add_systems(Update, camera_controls)
        .run();
}

fn setup_scene(mut commands: Commands) {
    // Add camera, lighting, basic terrain mesh
}
```

### Phase 2: Terrain Generation (Week 1-2)
```rust
// Simple height-map terrain
use fastnoise_lite::{FastNoiseLite, NoiseType};

#[derive(Component)]
struct TerrainMesh {
    size: u32,
    height_scale: f32,
}

fn generate_terrain_mesh(fear_level: f32) -> Mesh {
    // Use noise + fear_level to generate height-map
    // Convert to Bevy mesh with vertices/indices
}
```

### Phase 3: Fear Integration (Week 2)
```rust
// Connect fear sensor to terrain
use spectremesh_fear_sensor::MockFearSensor;

#[derive(Resource)]
struct FearState {
    current_fear: f32,
    receiver: async_channel::Receiver<FearScore>,
}

fn update_terrain_from_fear(
    mut fear_state: ResMut<FearState>,
    mut terrain_query: Query<&mut TerrainMesh>,
) {
    // Poll receiver for new fear scores
    // Update terrain mesh based on fear level
}
```

### Phase 4: Debug UI (Week 2)
```rust
// egui debug overlay
use bevy_egui::{egui, EguiContexts};

fn debug_ui_system(
    mut contexts: EguiContexts,
    mut fear_state: ResMut<FearState>,
) {
    egui::Window::new("Fear Control").show(contexts.ctx_mut(), |ui| {
        ui.add(egui::Slider::new(&mut fear_state.current_fear, 0.0..=1.0).text("Fear Level"));
    });
}
```

## Development Workflow

### Daily Development
```bash
# Start with mock sensor (no hardware needed)
cargo run -p spectremesh --bin spectreprobe -- --mock  # Verify fear detection works
cargo run -p spectremesh                               # Your Bevy app

# Test integration
cargo test -p spectremesh-fear-sensor                  # Fear detection tests
cargo test -p spectremesh                              # Your terrain tests
```

### Key Dependencies You'll Add
```toml
# Add to crates/game/Cargo.toml
[dependencies]
bevy = { workspace = true, features = ["default"] }
bevy_egui = "0.29"  # For debug UI
fastnoise-lite = { workspace = true }  # For terrain noise
spectremesh-fear-sensor = { path = "../fear_sensor" }  # Fear detection (already works!)
```

## Important Notes

### DO NOT MODIFY
- `crates/fear_sensor/` - This is DONE and WORKING, don't break it
- `crates/core/` - Core types are stable
- Fear detection pipeline - It's production ready

### FOCUS ON
- Bevy rendering pipeline
- Terrain mesh generation
- Shader implementation
- Visual feedback systems
- Debug UI for testing

### Testing Strategy
1. **Start with Mock**: Use `MockFearSensor` for all development
2. **Visual Testing**: Manual verification with debug slider
3. **Performance Testing**: Ensure 60 FPS with terrain updates
4. **Integration Testing**: Verify fear → terrain pipeline works

## Resources & References

### Bevy 0.16 Documentation
- [Bevy Book](https://bevyengine.org/learn/book/)
- [Bevy Examples](https://github.com/bevyengine/bevy/tree/main/examples)
- [Mesh Generation](https://github.com/bevyengine/bevy/blob/main/examples/3d/generate_custom_mesh.rs)

### Terrain Generation
- [FastNoise Lite](https://github.com/Auburn/FastNoiseLite)
- [Height-map to Mesh](https://docs.rs/bevy/latest/bevy/render/mesh/struct.Mesh.html)

### Shader Resources
- [Bevy Shader Examples](https://github.com/bevyengine/bevy/tree/main/examples/shader)
- [WGSL Documentation](https://www.w3.org/TR/WGSL/)

## Success Metrics

### Technical
- 60 FPS with real-time terrain updates
- Smooth fear → terrain response (no stuttering)
- Clear visual correlation between fear level and terrain shape

### User Experience
- Obvious terrain changes when fear slider moves
- Intuitive camera controls for terrain inspection
- Responsive debug UI

## When You're Done

### M0.5 Completion Criteria
- Demo video showing fear slider → terrain changes
- All acceptance criteria met (see DEVELOPMENT_PLAN.md)
- Code ready for M1 integration with real camera

### Handoff to M1
- Document Bevy integration patterns
- Ensure real fear sensor can replace mock sensor easily
- Performance benchmarks for terrain rendering

## Questions? Check These First

1. **Fear detection not working?** → Run `./demo_m0.sh` to verify M0 foundation
2. **Bevy build issues?** → Check Bevy 0.16 compatibility and feature flags
3. **Performance problems?** → Profile terrain mesh generation and update frequency
4. **Integration confusion?** → Study `spectreprobe.rs` for fear sensor usage patterns

**Remember**: The hard part (fear detection) is DONE. Focus on making it look awesome! 🎨
