# YuNet Face Detection Model

This directory contains the YuNet ONNX model for face detection.

## Model Information

- **File**: `face_detection_yunet.onnx`
- **Size**: 345 KB (353,312 bytes)
- **License**: MIT
- **Source**: OpenCV DNN Face Detection
- **Input**: 320x240 RGB image
- **Output**: Face detections with landmarks

## Download

The model should be downloaded from the official OpenCV repository:

```bash
wget -O face_detection_yunet.onnx \
  https://github.com/opencv/opencv_zoo/raw/master/models/face_detection_yunet/face_detection_yunet_2023mar.onnx
```

## Usage

The model is embedded in the binary using `include_bytes!` for easy deployment.
It can be overridden using the `--model-path` flag if needed.

## Model Architecture

YuNet is a lightweight face detection model that provides:
- High accuracy face detection
- 5-point facial landmarks
- Real-time performance
- Small model size (345 KB)

This replaces the previous Haar cascade approach with a modern CNN-based detector.
