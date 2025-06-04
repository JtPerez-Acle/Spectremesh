# SpectreMesh M0.2 Changelog

## 🚀 Major Improvements

**Face Detection Upgrade**: Replaced Haar cascades with YuNet ONNX model (345 KB, MIT-licensed) for superior accuracy and performance. Model embedded via `include_bytes!` with `--model-path` override support.

**Performance Optimization**: Implemented optimized ONNX Runtime with configurable threading (`SPECTRE_THREADS` env var, defaults to CPU count). Added performance validation requiring p95 inference ≤ 5ms on M1-class hardware.

**New Sensor Architecture**: Created dedicated `spectre_sensor` crate with gRPC streaming API. Supports Unix socket IPC by default with comprehensive `SensorEvent` protobuf schema for calibration progress, scores, and fault reporting.

**Back-pressure Handling**: Replaced unbounded channels with `bounded(2)` channels. Implements frame dropping on overflow to prevent memory bloat during processing spikes.

**Adaptive Calibration**: Continuous EMA-based calibration (α=0.05) with optional `--freeze-calibration` flag. Tracks baseline drift and provides real-time calibration progress.

**Monitoring & Metrics**: Prometheus metrics server on port 9090 exposing FPS, p95 latency, dropped frames, and calibration drift. Comprehensive tracing throughout the pipeline.

**Testing Infrastructure**: Added `sensor_fuzzer` binary for synthetic event generation and soak testing. Performance test suite validates latency requirements with detailed statistical analysis.

**Terrain Integration**: Fear scores mapped to shader uniform `distortion_intensity` (0-1). Mesh rebuilds triggered only on fear bucket transitions (low/medium/high) for optimal performance.

## 🔧 Technical Details

- YuNet model provides 5-point facial landmarks and confidence scores
- gRPC server supports event filtering and calibration control
- EMA calibrator runs continuously unless frozen
- Fear buckets: Low (0-0.33), Medium (0.33-0.66), High (0.66-1.0)
- Comprehensive error handling with graceful degradation
