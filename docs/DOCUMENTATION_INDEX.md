# SpectreMesh Documentation Index

**Complete documentation for the emotion-responsive procedural horror game**

## Quick Navigation

### 🚀 Getting Started
- **[README.md](../README.md)** - Project overview and quick start guide
- **[DEVELOPMENT_GUIDE.md](DEVELOPMENT_GUIDE.md)** - Complete setup and contribution guide
- **[DEPLOYMENT_GUIDE.md](DEPLOYMENT_GUIDE.md)** - Production deployment instructions

### 📖 Technical Documentation
- **[TECHNICAL_ARCHITECTURE.md](TECHNICAL_ARCHITECTURE.md)** - Deep dive into system design and architecture
- **[API_REFERENCE.md](API_REFERENCE.md)** - Complete API documentation with examples
- **[PROJECT_APPROACH.md](PROJECT_APPROACH.md)** - Technical strategy and design philosophy



## Documentation Structure

```
docs/
├── DOCUMENTATION_INDEX.md      # This file - master documentation index
├── TECHNICAL_ARCHITECTURE.md   # System architecture and design
├── DEVELOPMENT_GUIDE.md         # Setup and contribution guide
├── API_REFERENCE.md            # Complete API documentation
├── DEPLOYMENT_GUIDE.md         # Production deployment guide
└── PROJECT_APPROACH.md         # Technical philosophy and strategy
```

## Documentation Status

| Document | Status | Last Updated | Purpose |
|----------|--------|--------------|---------|
| **TECHNICAL_ARCHITECTURE.md** | ✅ **Current** | 2025-06 | **System design deep dive** |
| **DEVELOPMENT_GUIDE.md** | ✅ **Current** | 2025-06 | **Setup and contribution** |
| **API_REFERENCE.md** | ✅ **Current** | 2025-06 | **Complete API documentation** |
| **DEPLOYMENT_GUIDE.md** | ✅ **Current** | 2025-06 | **Production deployment** |
| **PROJECT_APPROACH.md** | ✅ **Current** | 2025-06 | **Technical philosophy** |
| README.md | ✅ Current | 2025-06 | Project overview |

## For Developers

### New Contributors
1. Start with **[README.md](../README.md)** for project overview
2. Read **[DEVELOPMENT_GUIDE.md](DEVELOPMENT_GUIDE.md)** for setup and contribution guidelines
3. Review **[TECHNICAL_ARCHITECTURE.md](TECHNICAL_ARCHITECTURE.md)** for system understanding
4. Use **[API_REFERENCE.md](API_REFERENCE.md)** for integration details

### Current Development Focus: M0.5
- **Goal**: Visual proof of concept with Bevy 3D terrain displacement
- **Status**: Ready to begin - all technical risks eliminated
- **Next Steps**: Implement Bevy scene with fear-responsive terrain

### API Integration
- **[API_REFERENCE.md](API_REFERENCE.md)** - Complete API documentation
- Focus on `FearSensor` trait and `FearScore` types
- Use `MockFearSensor` for development without hardware
- Cross-platform compatibility built-in

### Architecture Understanding
- **[TECHNICAL_ARCHITECTURE.md](TECHNICAL_ARCHITECTURE.md)** - Deep technical dive
- **[PROJECT_APPROACH.md](PROJECT_APPROACH.md)** - Design philosophy and strategy
- **[DEPLOYMENT_GUIDE.md](DEPLOYMENT_GUIDE.md)** - Production deployment

## Project Status

### ✅ Completed Achievements
- **Cross-Platform Compatibility**: Windows, macOS, and Linux deployment ready
- **Real Camera Integration**: OpenCV-based camera enumeration and capture
- **YuNet CNN Integration**: Modern face detection with embedded models
- **ONNX Runtime 2.0**: High-performance emotion recognition
- **Adaptive Calibration**: Personalized fear measurement system
- **Production Architecture**: Async, thread-safe, embedded models

### 🚧 Current Development: M0.5
- **Goal**: Visual proof of concept with Bevy 3D terrain
- **Target**: Fear-responsive terrain displacement at 60 FPS
- **Status**: Technical foundation complete, ready for visual development

## Quick Start Commands

### Verify System
```bash
# Test camera and fear detection
cargo run --bin spectreprobe

# Test with mock data (no camera required)
cargo run --bin spectreprobe --mock

# Run all tests
cargo test --workspace

# Build for production
cargo build --release
```

### Development Workflow
```bash
# Start development with mock sensor
let mut sensor = MockFearSensor::step_pattern();
sensor.initialize(&config).await?;
let receiver = sensor.start().await?;

# Process fear scores in game loop
while let Ok(fear_score) = receiver.recv().await {
    update_terrain(fear_score.value);
}
```

## Documentation Maintenance

This documentation is actively maintained and reflects the current state of the SpectreMesh project. All documents follow a consistent structure and are kept current with the codebase.

### Contributing to Documentation
- Follow the established documentation structure
- Include Mermaid diagrams for complex concepts
- Provide code examples for all APIs
- Update the documentation index when adding new files
- Maintain professional writing style throughout

### Documentation Standards
- **Technical Accuracy**: All code examples must be tested and working
- **Cross-Platform**: Consider Windows, macOS, and Linux in all guidance
- **Privacy Focus**: Emphasize local processing and data protection
- **Performance Awareness**: Include performance implications and optimizations
- **User-Friendly**: Write for both technical and non-technical audiences

For documentation issues or suggestions, please create an issue in the main repository.

---

**🎯 SpectreMesh is ready for cross-platform deployment and M0.5 visual development**
