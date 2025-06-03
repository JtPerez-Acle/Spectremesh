# SpectreMesh Model Assets

This directory contains the ONNX models and other assets required for SpectreMesh.

## Required Files for Real Hardware Testing

### FaceONNX Emotion Recognition Model
- **File**: `face_emotion.onnx`
- **Description**: Pre-trained emotion recognition model (7 classes)
- **Input**: 48x48 grayscale face images
- **Output**: 7-class emotion logits [angry, disgust, fear, happy, sad, surprise, neutral]
- **License**: MIT (compatible with project)
- **Download**: Available from FaceONNX repository or similar emotion recognition models

### Haar Cascade Face Detector
- **File**: `haarcascade_frontalface_alt.xml`
- **Description**: OpenCV Haar cascade for face detection
- **Source**: OpenCV distribution or download from OpenCV GitHub
- **License**: BSD (compatible with project)

## For Development/Testing

During development, the mock sensor can be used instead of real hardware:

```bash
# Test with mock sensor (no model files required)
cargo run --bin spectreprobe --features mock-fear -- --mock

# Test with real ONNX sensor (requires model files)
cargo run --bin spectreprobe

# Test both implementations
cargo run --bin spectreprobe -- --test-both
```

## Model Integration Notes

1. **Model Path Configuration**: Update `FearConfig::model_path` to point to the correct ONNX file
2. **Face Detector Path**: The Haar cascade path is currently hardcoded in `OnnxFearSensor::load_face_detector()`
3. **Performance**: Target <10ms inference time on GTX 1050/M1 baseline hardware
4. **Fallback**: System gracefully degrades to neutral emotions when face detection or inference fails

## Future Enhancements

- Bundle model files with the application binary
- Support multiple emotion recognition models
- Add GPU acceleration detection and fallback
- Implement model caching and optimization
