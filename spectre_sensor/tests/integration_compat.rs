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
    match sensor.enumerate_cameras().await {
        Ok(cameras) => {
            assert!(!cameras.is_empty(), "Should return at least one camera device");
        }
        Err(spectremesh_core::CameraError::NoCamerasAvailable) => {
            println!("No cameras available - acceptable in CI environment");
        }
        Err(e) => {
            panic!("Unexpected camera enumeration error: {}", e);
        }
    }
    
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

#[tokio::test]
async fn test_cross_platform_camera_enumeration() {
    let sensor = YuNetFearSensor::new();

    match sensor.enumerate_cameras().await {
        Ok(cameras) => {
            println!("Found {} camera(s):", cameras.len());

            for camera in &cameras {
                println!("  - ID: {}, Name: '{}', Resolution: {}x{}",
                    camera.id, camera.name, camera.resolution.0, camera.resolution.1);

                // Validate camera properties
                assert!(!camera.name.contains("Default Camera"), "Should not be mock camera");
                assert!(camera.id < 10, "Camera ID should be reasonable");
                assert!(camera.resolution.0 > 0 && camera.resolution.1 > 0, "Resolution should be valid");

                // Platform-specific validations
                #[cfg(target_os = "windows")]
                assert!(camera.name.contains("DirectShow"), "Windows should use DirectShow");

                #[cfg(target_os = "macos")]
                assert!(camera.name.contains("AVFoundation"), "macOS should use AVFoundation");

                #[cfg(target_os = "linux")]
                assert!(camera.name.contains("V4L2"), "Linux should use V4L2");
            }
        },
        Err(spectremesh_core::CameraError::NoCamerasAvailable) => {
            println!("No cameras available - acceptable in CI environment");
        },
        Err(e) => {
            panic!("Unexpected camera enumeration error: {}", e);
        }
    }
}

#[test]
fn test_platform_specific_paths() {
    use spectre_sensor::config::SensorConfig;

    let config = SensorConfig::default();

    #[cfg(target_os = "windows")]
    {
        assert!(config.grpc_socket_path.contains(r"\\.\pipe\"), "Windows should use named pipes");
        assert!(!config.grpc_socket_path.contains("/tmp/"), "Windows should not use /tmp/");
    }

    #[cfg(target_os = "macos")]
    {
        assert!(config.grpc_socket_path.starts_with("/tmp/"), "macOS should use /tmp/");
        assert!(config.grpc_socket_path.contains(&std::process::id().to_string()), "macOS should include process ID");
    }

    #[cfg(target_os = "linux")]
    {
        assert_eq!(config.grpc_socket_path, "/tmp/spectre_sensor.sock", "Linux should use standard path");
    }
}

#[tokio::test]
async fn test_mock_vs_real_camera_enumeration() {
    // Test mock sensor
    let mock_sensor = MockFearSensor::new(vec![0.5]);
    let mock_cameras = mock_sensor.enumerate_cameras().await.unwrap();

    assert_eq!(mock_cameras.len(), 1);
    assert_eq!(mock_cameras[0].name, "Mock Camera");

    // Test real sensor
    let real_sensor = YuNetFearSensor::new();
    let real_cameras_result = real_sensor.enumerate_cameras().await;

    match real_cameras_result {
        Ok(real_cameras) => {
            // If real cameras are found, they should be different from mock
            for camera in &real_cameras {
                assert_ne!(camera.name, "Mock Camera", "Real cameras should not be named 'Mock Camera'");
                assert_ne!(camera.name, "Default Camera", "Real cameras should not be named 'Default Camera'");
            }
        },
        Err(spectremesh_core::CameraError::NoCamerasAvailable) => {
            // Acceptable in CI environments
            println!("No real cameras available for testing");
        },
        Err(e) => {
            panic!("Unexpected error in real camera enumeration: {}", e);
        }
    }
}
