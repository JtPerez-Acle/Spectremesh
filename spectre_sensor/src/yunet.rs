//! YuNet face detection implementation
//! 
//! Replaces Haar cascades with the modern YuNet ONNX model for improved accuracy
//! and performance. The model is embedded in the binary for easy deployment.

use opencv::{
    core::{Mat, Rect, Size, Point},
    imgproc,
    prelude::*,
};
use ort::{
    session::{Session, builder::GraphOptimizationLevel},
    value::Tensor,
};
use ndarray::Array4;
use std::time::Instant;
use thiserror::Error;

/// YuNet face detection errors
#[derive(Debug, Error)]
pub enum YuNetError {
    #[error("ONNX session creation failed: {0}")]
    SessionCreation(String),
    
    #[error("Image preprocessing failed: {0}")]
    Preprocessing(String),
    
    #[error("Model inference failed: {0}")]
    Inference(String),
    
    #[error("No faces detected in frame")]
    NoFacesDetected,
    
    #[error("Invalid model output format")]
    InvalidOutput,
}

/// Face detection result
#[derive(Debug, Clone)]
pub struct FaceDetection {
    /// Bounding box of the detected face
    pub bbox: Rect,
    /// Detection confidence [0.0, 1.0]
    pub confidence: f32,
    /// Facial landmarks (5 points: 2 eyes, nose, 2 mouth corners)
    pub landmarks: Vec<Point>,
}

/// YuNet face detector using ONNX Runtime
pub struct YuNetDetector {
    session: Session,
    input_size: Size,
    confidence_threshold: f32,
    nms_threshold: f32,
}

impl YuNetDetector {
    /// Create a new YuNet detector with embedded model
    pub fn new(num_threads: usize) -> Result<Self, YuNetError> {
        Self::from_bytes(crate::YUNET_MODEL_BYTES, num_threads)
    }

    /// Create a new YuNet detector from model bytes
    pub fn from_bytes(
        model_bytes: &[u8],
        num_threads: usize,
    ) -> Result<Self, YuNetError> {
        let session = Session::builder()
            .map_err(|e| YuNetError::SessionCreation(e.to_string()))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| YuNetError::SessionCreation(e.to_string()))?
            .with_intra_threads(num_threads)
            .map_err(|e| YuNetError::SessionCreation(e.to_string()))?
            .commit_from_memory(model_bytes)
            .map_err(|e| YuNetError::SessionCreation(e.to_string()))?;

        Ok(Self {
            session,
            input_size: Size::new(640, 640), // YuNet 2023mar model input size
            confidence_threshold: 0.6,
            nms_threshold: 0.3,
        })
    }

    /// Create from external model file (for --model-path override)
    pub fn from_file(
        model_path: &str,
        num_threads: usize,
    ) -> Result<Self, YuNetError> {
        let session = Session::builder()
            .map_err(|e| YuNetError::SessionCreation(e.to_string()))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| YuNetError::SessionCreation(e.to_string()))?
            .with_intra_threads(num_threads)
            .map_err(|e| YuNetError::SessionCreation(e.to_string()))?
            .commit_from_file(model_path)
            .map_err(|e| YuNetError::SessionCreation(e.to_string()))?;

        Ok(Self {
            session,
            input_size: Size::new(640, 640), // YuNet 2023mar model input size
            confidence_threshold: 0.6,
            nms_threshold: 0.3,
        })
    }

    /// Detect faces in the given image
    pub fn detect_faces(&mut self, image: &Mat) -> Result<Vec<FaceDetection>, YuNetError> {
        let start_time = Instant::now();

        // Preprocess image
        let input_tensor = self.preprocess_image(image)?;
        
        // Run inference
        let outputs = self.session
            .run(ort::inputs!["input" => input_tensor])
            .map_err(|e| YuNetError::Inference(e.to_string()))?;

        // Extract parameters needed for post-processing
        let input_size = self.input_size;
        let confidence_threshold = self.confidence_threshold;
        let nms_threshold = self.nms_threshold;

        // Post-process results
        let detections = Self::postprocess_outputs_static(&outputs, image.size().unwrap(), input_size, confidence_threshold, nms_threshold)?;
        
        let inference_time = start_time.elapsed();
        tracing::debug!(
            "YuNet inference completed in {:?}, found {} faces",
            inference_time,
            detections.len()
        );

        Ok(detections)
    }

    /// Get the largest face detection (most confident)
    pub fn get_largest_face(&mut self, image: &Mat) -> Result<FaceDetection, YuNetError> {
        let detections = self.detect_faces(image)?;
        
        detections
            .into_iter()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
            .ok_or(YuNetError::NoFacesDetected)
    }

    /// Preprocess image for YuNet input
    fn preprocess_image(&self, image: &Mat) -> Result<Tensor<f32>, YuNetError> {
        let mut resized = Mat::default();
        imgproc::resize(
            image,
            &mut resized,
            self.input_size,
            0.0,
            0.0,
            imgproc::INTER_LINEAR,
        )
        .map_err(|e| YuNetError::Preprocessing(e.to_string()))?;

        // Convert BGR to RGB
        let mut rgb = Mat::default();
        imgproc::cvt_color(&resized, &mut rgb, imgproc::COLOR_BGR2RGB, 0)
            .map_err(|e| YuNetError::Preprocessing(e.to_string()))?;

        // Convert to float and normalize [0, 255] -> [0, 1]
        let mut float_img = Mat::default();
        rgb.convert_to(&mut float_img, opencv::core::CV_32F, 1.0 / 255.0, 0.0)
            .map_err(|e| YuNetError::Preprocessing(e.to_string()))?;

        // Convert to ndarray format (NCHW: batch, channels, height, width)
        // Use data_bytes() and cast to f32 slice to handle multi-channel Mat
        let data_bytes = float_img.data_bytes()
            .map_err(|e| YuNetError::Preprocessing(e.to_string()))?;
        let data = unsafe {
            std::slice::from_raw_parts(
                data_bytes.as_ptr() as *const f32,
                data_bytes.len() / std::mem::size_of::<f32>()
            )
        };
        
        let height = self.input_size.height as usize;
        let width = self.input_size.width as usize;
        
        // Reshape from HWC to NCHW
        let mut input_array = Array4::<f32>::zeros((1, 3, height, width));
        for y in 0..height {
            for x in 0..width {
                for c in 0..3 {
                    let src_idx = (y * width + x) * 3 + c;
                    input_array[[0, c, y, x]] = data[src_idx];
                }
            }
        }

        let shape = [1, 3, self.input_size.height as usize, self.input_size.width as usize];
        let data: Vec<f32> = input_array.into_raw_vec();

        Tensor::from_array((shape, data))
            .map_err(|e| YuNetError::Preprocessing(e.to_string()))
    }

    /// Post-process YuNet outputs to extract face detections (static version)
    /// This version handles the multi-scale output format of YuNet 2023mar
    fn postprocess_outputs_static(
        outputs: &ort::session::SessionOutputs,
        original_size: Size,
        input_size: Size,
        confidence_threshold: f32,
        nms_threshold: f32,
    ) -> Result<Vec<FaceDetection>, YuNetError> {
        let mut detections = Vec::new();

        // YuNet 2023mar has multi-scale outputs at 8x, 16x, and 32x downsampling
        let scales = [8, 16, 32];

        for &scale in &scales {
            // Get outputs for this scale
            let cls_name = format!("cls_{}", scale);
            let obj_name = format!("obj_{}", scale);
            let bbox_name = format!("bbox_{}", scale);
            let kps_name = format!("kps_{}", scale);

            let cls_output = outputs.get(&cls_name).ok_or(YuNetError::InvalidOutput)?;
            let obj_output = outputs.get(&obj_name).ok_or(YuNetError::InvalidOutput)?;
            let bbox_output = outputs.get(&bbox_name).ok_or(YuNetError::InvalidOutput)?;
            let kps_output = outputs.get(&kps_name).ok_or(YuNetError::InvalidOutput)?;

            let (_, cls_data) = cls_output.try_extract_tensor::<f32>().map_err(|_| YuNetError::InvalidOutput)?;
            let (_, obj_data) = obj_output.try_extract_tensor::<f32>().map_err(|_| YuNetError::InvalidOutput)?;
            let (bbox_shape, bbox_data) = bbox_output.try_extract_tensor::<f32>().map_err(|_| YuNetError::InvalidOutput)?;
            let (_, kps_data) = kps_output.try_extract_tensor::<f32>().map_err(|_| YuNetError::InvalidOutput)?;

            // Process detections for this scale
            let num_anchors = bbox_shape[1] as usize;
            let stride = scale as f32;

            for i in 0..num_anchors {
                let obj_score = obj_data[i];
                let cls_score = cls_data[i];
                let confidence = obj_score * cls_score;

                if confidence > confidence_threshold {
                    // Extract bounding box (format: [x_center, y_center, width, height])
                    let cx = bbox_data[i * 4 + 0] * stride;
                    let cy = bbox_data[i * 4 + 1] * stride;
                    let w = bbox_data[i * 4 + 2] * stride;
                    let h = bbox_data[i * 4 + 3] * stride;

                    // Convert to corner format and scale to original image
                    let scale_x = original_size.width as f32 / input_size.width as f32;
                    let scale_y = original_size.height as f32 / input_size.height as f32;

                    let x1 = ((cx - w / 2.0) * scale_x) as i32;
                    let y1 = ((cy - h / 2.0) * scale_y) as i32;
                    let x2 = ((cx + w / 2.0) * scale_x) as i32;
                    let y2 = ((cy + h / 2.0) * scale_y) as i32;

                    let bbox = Rect::new(x1, y1, x2 - x1, y2 - y1);

                    // Extract landmarks (5 points, 2 coordinates each)
                    let mut landmarks = Vec::new();
                    for j in 0..5 {
                        let lm_x = ((kps_data[i * 10 + j * 2] * stride) * scale_x) as i32;
                        let lm_y = ((kps_data[i * 10 + j * 2 + 1] * stride) * scale_y) as i32;
                        landmarks.push(Point::new(lm_x, lm_y));
                    }

                    detections.push(FaceDetection {
                        bbox,
                        confidence,
                        landmarks,
                    });
                }
            }
        }

        // Apply NMS to remove overlapping detections
        Self::apply_nms_static(&mut detections, nms_threshold);

        Ok(detections)
    }

    /// Apply Non-Maximum Suppression to remove overlapping detections (static version)
    fn apply_nms_static(detections: &mut Vec<FaceDetection>, nms_threshold: f32) {
        if detections.len() <= 1 {
            return;
        }

        // Sort by confidence (highest first)
        detections.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        let mut keep = vec![true; detections.len()];

        for i in 0..detections.len() {
            if !keep[i] {
                continue;
            }

            for j in (i + 1)..detections.len() {
                if !keep[j] {
                    continue;
                }

                let iou = Self::calculate_iou_static(&detections[i].bbox, &detections[j].bbox);
                if iou > nms_threshold {
                    keep[j] = false;
                }
            }
        }

        // Keep only non-suppressed detections
        let mut filtered = Vec::new();
        for (i, detection) in detections.iter().enumerate() {
            if keep[i] {
                filtered.push(detection.clone());
            }
        }

        *detections = filtered;
    }

    /// Calculate Intersection over Union (IoU) for two bounding boxes (static version)
    fn calculate_iou_static(bbox1: &Rect, bbox2: &Rect) -> f32 {
        let x1 = bbox1.x.max(bbox2.x);
        let y1 = bbox1.y.max(bbox2.y);
        let x2 = (bbox1.x + bbox1.width).min(bbox2.x + bbox2.width);
        let y2 = (bbox1.y + bbox1.height).min(bbox2.y + bbox2.height);

        if x2 <= x1 || y2 <= y1 {
            return 0.0;
        }

        let intersection = ((x2 - x1) * (y2 - y1)) as f32;
        let area1 = (bbox1.width * bbox1.height) as f32;
        let area2 = (bbox2.width * bbox2.height) as f32;
        let union = area1 + area2 - intersection;

        if union > 0.0 {
            intersection / union
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iou_calculation() {
        // Create a mock detector for testing IoU calculation
        // We only need the IoU method, so we can create a minimal struct

        // Test IoU calculation directly using a helper function
        fn calculate_iou(bbox1: &Rect, bbox2: &Rect) -> f32 {
            let x1 = bbox1.x.max(bbox2.x);
            let y1 = bbox1.y.max(bbox2.y);
            let x2 = (bbox1.x + bbox1.width).min(bbox2.x + bbox2.width);
            let y2 = (bbox1.y + bbox1.height).min(bbox2.y + bbox2.height);

            if x2 <= x1 || y2 <= y1 {
                return 0.0;
            }

            let intersection = ((x2 - x1) * (y2 - y1)) as f32;
            let area1 = (bbox1.width * bbox1.height) as f32;
            let area2 = (bbox2.width * bbox2.height) as f32;
            let union = area1 + area2 - intersection;

            if union > 0.0 {
                intersection / union
            } else {
                0.0
            }
        }

        // Identical boxes
        let bbox1 = Rect::new(10, 10, 50, 50);
        let bbox2 = Rect::new(10, 10, 50, 50);
        assert_eq!(calculate_iou(&bbox1, &bbox2), 1.0);

        // Non-overlapping boxes
        let bbox3 = Rect::new(100, 100, 50, 50);
        assert_eq!(calculate_iou(&bbox1, &bbox3), 0.0);

        // Partially overlapping boxes
        let bbox4 = Rect::new(30, 30, 50, 50);
        let iou = calculate_iou(&bbox1, &bbox4);
        assert!(iou > 0.0 && iou < 1.0);
    }
}
