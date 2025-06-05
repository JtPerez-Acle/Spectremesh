//! Integration tests for the compatibility layer
//! 
//! These tests validate that the YuNet-based implementation can be used as a drop-in
//! replacement for the legacy Haar cascade implementation.

use spectre_sensor::compat::{FearSensor, MockFearSensor, YuNetFearSensor};
use spectremesh_core::FearConfig;
use std::time::Duration;

#[tokio::test]
async fn test_mock_sensor_compatibility() {
    let mut sensor = MockFearSensor::step_pattern();
    let config = FearConfig::default();

    // Test the full workflow that existing code expects
    sensor.initialize(&config).await.expect("Failed to initialize mock sensor");
    
    let receiver = sensor.start().await.expect("Failed to start mock sensor");
    
    // Receive a few fear scores
    let mut scores = Vec::new();
    for _ in 0..3 {
        match tokio::time::timeout(Duration::from_millis(200), receiver.recv()).await {
            Ok(Ok(score)) => scores.push(score),
            Ok(Err(_)) => break,
            Err(_) => break, // Timeout
        }
    }
    
    // Validate we received scores
    assert!(!scores.is_empty(), "Should have received at least one fear score");
    
    // Validate score structure
    for score in &scores {
        assert!(score.value >= 0.0 && score.value <= 1.0, "Fear value should be normalized");
        assert!(score.confidence >= 0.0 && score.confidence <= 1.0, "Confidence should be normalized");
        assert_eq!(score.emotion_logits.len(), 7, "Should have 7 emotion classes");
        assert_eq!(score.extract_fear_logit(), score.emotion_logits[2], "Fear should be at index 2");
    }
    
    sensor.stop().await.expect("Failed to stop mock sensor");
}

#[tokio::test]
async fn test_yunet_sensor_compatibility() {
    let mut sensor = YuNetFearSensor::new();
    let config = FearConfig::default();

    // Test initialization (this will fail without proper models, but should handle gracefully)
    let init_result = sensor.initialize(&config).await;
    
    // In test environment, we expect this to fail due to missing model files
    // but the API should be compatible
    match init_result {
        Ok(_) => {
            // If initialization succeeds (unlikely in test env), test the full workflow
            let _receiver = sensor.start().await.expect("Failed to start YuNet sensor");
            sensor.stop().await.expect("Failed to stop YuNet sensor");
        },
        Err(_) => {
            // Expected in test environment - just validate the API is compatible
            println!("YuNet sensor initialization failed as expected in test environment");
        }
    }
    
    // Test camera enumeration (should work even without initialization)
    let cameras = sensor.enumerate_cameras().await.expect("Failed to enumerate cameras");
    assert!(!cameras.is_empty(), "Should return at least one camera device");
    
    // Test calibration status methods
    assert!(!sensor.is_calibrated(), "Should not be calibrated initially");
    assert_eq!(sensor.calibration_progress(), 0.0, "Should have zero progress initially");
}

#[test]
fn test_api_compatibility() {
    // Test that both sensor types implement the FearSensor trait
    fn accepts_fear_sensor<T: FearSensor>(_sensor: T) {}
    
    let mock_sensor = MockFearSensor::new(vec![0.5]);
    let yunet_sensor = YuNetFearSensor::new();
    
    accepts_fear_sensor(mock_sensor);
    accepts_fear_sensor(yunet_sensor);
}

#[tokio::test]
async fn test_fear_score_compatibility() {
    let mut sensor = MockFearSensor::new(vec![0.3, 0.7]);
    let config = FearConfig::default();
    
    sensor.initialize(&config).await.unwrap();
    let receiver = sensor.start().await.unwrap();
    
    // Get a fear score
    let score = tokio::time::timeout(Duration::from_millis(100), receiver.recv())
        .await
        .expect("Timeout waiting for score")
        .expect("Failed to receive score");
    
    // Test that FearScore has all expected methods and fields
    let _value: f32 = score.value;
    let _confidence: f32 = score.confidence;
    let _calibrated: bool = score.calibrated;
    let _emotion_logits: [f32; 7] = score.emotion_logits;
    let _timestamp = score.timestamp;
    
    // Test methods
    let fear_logit = score.extract_fear_logit();
    assert_eq!(fear_logit, score.emotion_logits[2]);
    
    sensor.stop().await.unwrap();
}

#[test]
fn test_config_compatibility() {
    // Test that FearConfig can be created and used as expected
    let config = FearConfig::default();
    
    assert!(!config.model_path.is_empty());
    assert!(config.calibration_duration.as_secs() > 0);
    assert_eq!(config.camera.device_id, 0);
    
    // Test builder pattern
    let custom_config = FearConfig::default()
        .with_model_path("custom_model.onnx")
        .with_camera_device(1)
        .with_calibration_duration(Duration::from_secs(60));
    
    assert_eq!(custom_config.model_path, "custom_model.onnx");
    assert_eq!(custom_config.camera.device_id, 1);
    assert_eq!(custom_config.calibration_duration, Duration::from_secs(60));
}

#[tokio::test]
async fn test_error_handling_compatibility() {
    let mut sensor = YuNetFearSensor::new();
    
    // Test that starting without initialization returns an error
    let start_result = sensor.start().await;
    assert!(start_result.is_err(), "Should fail to start without initialization");
    
    // Test that stop works even without starting
    let stop_result = sensor.stop().await;
    assert!(stop_result.is_ok(), "Stop should succeed even without starting");
}
