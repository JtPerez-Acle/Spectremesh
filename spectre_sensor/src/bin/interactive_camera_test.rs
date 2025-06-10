//! Interactive Camera Test for SpectreMesh
//! 
//! This test runs for 30 seconds and continuously tries to detect faces,
//! giving real-time feedback about what the camera sees.

use spectremesh_core::FearConfig;
use spectre_sensor::compat::{FearSensor, YuNetFearSensor};
use spectre_sensor::types::FearBucket;
use std::time::{Duration, Instant};
use tokio::time::timeout;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🎯 SpectreMesh Interactive Camera Test");
    println!("======================================");
    println!("This test will run for 30 seconds and continuously try to detect your face.");
    println!("Position yourself in front of the camera and ensure good lighting!");
    println!("");

    // Test camera enumeration first
    println!("🔍 Step 1: Checking camera availability...");
    let sensor = YuNetFearSensor::new();
    
    match sensor.enumerate_cameras().await {
        Ok(cameras) => {
            println!("✅ Found {} camera(s):", cameras.len());
            for camera in cameras {
                println!("   - ID: {}, Name: '{}', Resolution: {}x{}",
                    camera.id, camera.name, camera.resolution.0, camera.resolution.1);
            }
        }
        Err(e) => {
            println!("❌ No cameras found: {}", e);
            return Err(e.into());
        }
    }

    println!("\n🧠 Step 2: Starting face detection...");
    println!("📹 Camera is now active - position yourself in front of the camera!");
    println!("💡 Tips:");
    println!("   - Face the camera directly");
    println!("   - Ensure good lighting");
    println!("   - Stay within 1-3 feet of the camera");
    println!("   - The test will run for 30 seconds");
    println!("");

    // Try to run a basic camera test first
    println!("🎬 Testing basic camera access...");
    match test_basic_camera_access().await {
        Ok(_) => println!("✅ Basic camera access working!"),
        Err(e) => {
            println!("❌ Basic camera test failed: {}", e);
            println!("💡 This might be a camera permission or hardware issue");
            return Err(e);
        }
    }

    println!("\n🧠 Now testing with YuNet face detection...");
    println!("⚠️  Note: YuNet requires a face to be visible during initialization");
    println!("📹 Please position yourself in front of the camera NOW!");

    // Wait a moment for user to position themselves
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Initialize sensor
    let mut sensor = YuNetFearSensor::new();
    let config = FearConfig {
        calibration_duration: Duration::from_secs(10), // 10 second calibration
        camera: spectremesh_core::CameraConfig {
            fps: 10, // Lower FPS for better feedback
            ..Default::default()
        },
        ..FearConfig::default()
    };

    print!("Initializing YuNet sensor (requires face detection)... ");
    match sensor.initialize(&config).await {
        Ok(_) => println!("✅"),
        Err(e) => {
            println!("❌");
            println!("Error: {}", e);
            println!("💡 This means no face was detected during initialization.");
            println!("   Try running the test again with better lighting and positioning.");
            return Err(e.into());
        }
    }

    // Start sensor
    print!("Starting camera feed... ");
    let receiver = match sensor.start().await {
        Ok(receiver) => {
            println!("✅");
            receiver
        }
        Err(e) => {
            println!("❌");
            println!("Error: {}", e);
            return Err(e.into());
        }
    };

    println!("\n🎬 CAMERA IS NOW ACTIVE - LOOK AT THE CAMERA!");
    println!("================================================");

    // Run for 30 seconds
    let start_time = Instant::now();
    let test_duration = Duration::from_secs(30);
    let mut frame_count = 0;
    let mut face_detected_count = 0;
    let mut last_status_time = Instant::now();

    while start_time.elapsed() < test_duration {
        let remaining = test_duration - start_time.elapsed();
        
        // Show status every 2 seconds
        if last_status_time.elapsed() >= Duration::from_secs(2) {
            println!("\n⏱️  Time remaining: {:.0}s | Frames processed: {} | Faces detected: {}", 
                remaining.as_secs(), frame_count, face_detected_count);
            
            // Show calibration progress
            let progress = sensor.calibration_progress();
            let is_calibrated = sensor.is_calibrated();
            println!("📊 Calibration: {:.1}% complete | Calibrated: {}", 
                progress * 100.0, is_calibrated);
            
            last_status_time = Instant::now();
        }

        // Try to receive a frame
        match timeout(Duration::from_millis(500), receiver.recv()).await {
            Ok(Ok(score)) => {
                frame_count += 1;
                face_detected_count += 1;
                
                println!("😊 FACE DETECTED! Fear={:.3}, Confidence={:.3}, Calibrated={}", 
                    score.value, score.confidence, score.calibrated);
                
                // Show fear bucket if calibrated
                if score.calibrated {
                    let bucket = FearBucket::from_score(score.value);
                    println!("   📈 Fear Bucket: {:?} | Raw Fear Logit: {:.3}",
                        bucket, score.extract_fear_logit());
                }
            }
            Ok(Err(_)) => {
                println!("❌ Channel closed unexpectedly");
                break;
            }
            Err(_) => {
                // Timeout - no face detected
                if frame_count == 0 {
                    print!(".");
                    std::io::Write::flush(&mut std::io::stdout()).unwrap();
                }
            }
        }
    }

    // Stop sensor
    println!("\n\n🛑 Stopping camera...");
    sensor.stop().await?;

    // Show final results
    println!("\n📊 TEST RESULTS:");
    println!("================");
    println!("✅ Total frames processed: {}", frame_count);
    println!("😊 Faces detected: {}", face_detected_count);
    println!("📈 Final calibration: {:.1}%", sensor.calibration_progress() * 100.0);
    println!("🎯 Calibrated: {}", sensor.is_calibrated());

    if face_detected_count > 0 {
        println!("\n🎉 SUCCESS! Your camera is working and face detection is functional!");
        println!("💪 SpectreMesh is ready for real-time fear detection!");
    } else {
        println!("\n⚠️  No faces were detected during the test.");
        println!("💡 Troubleshooting tips:");
        println!("   - Check camera permissions");
        println!("   - Ensure good lighting");
        println!("   - Position yourself directly in front of the camera");
        println!("   - Try adjusting the camera angle");
        println!("   - Make sure no other applications are using the camera");
    }

    Ok(())
}

/// Test basic camera access without face detection
async fn test_basic_camera_access() -> Result<(), Box<dyn std::error::Error>> {
    use opencv::{
        videoio::{VideoCapture, CAP_ANY},
        prelude::{VideoCaptureTraitConst, VideoCaptureTrait, MatTraitConst},
        core::Mat as CoreMat,
    };

    println!("   Opening camera device 0...");
    let mut camera = VideoCapture::new(0, CAP_ANY)?;

    if !camera.is_opened()? {
        return Err("Failed to open camera".into());
    }

    println!("   ✅ Camera opened successfully");
    println!("   📹 Capturing 5 test frames...");

    for i in 1..=5 {
        let mut frame = CoreMat::default();
        if camera.read(&mut frame)? && !frame.empty() {
            let size = frame.size()?;
            println!("   📸 Frame {}: {}x{} pixels", i, size.width, size.height);
        } else {
            return Err(format!("Failed to capture frame {}", i).into());
        }

        // Small delay between frames
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    println!("   ✅ Successfully captured 5 frames");
    println!("   🎯 Camera hardware is working correctly!");

    Ok(())
}
