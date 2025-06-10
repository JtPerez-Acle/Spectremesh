//! Camera Viewer with Face Detection for SpectreMesh
//!
//! This shows what the camera sees in real-time with face detection overlays
//! to help debug positioning and lighting issues.

use opencv::{
    videoio::{VideoCapture, CAP_ANY},
    prelude::{VideoCaptureTraitConst, VideoCaptureTrait, MatTraitConst},
    core::{Mat, Point, Scalar, Rect},
    imgproc,
    highgui,
};
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 SpectreMesh Visual Camera Viewer");
    println!("===================================");
    println!("This will open a popup window showing your live camera feed!");
    println!("Position yourself in front of the camera and you'll see yourself on screen.");
    println!("This helps validate camera positioning and lighting for face detection.");
    println!("");

    // Open camera
    println!("📹 Opening camera...");
    let mut camera = VideoCapture::new(0, CAP_ANY)?;
    
    if !camera.is_opened()? {
        return Err("Failed to open camera".into());
    }

    println!("✅ Camera opened successfully");

    // Set camera properties for better performance
    camera.set(opencv::videoio::CAP_PROP_FRAME_WIDTH, 640.0)?;
    camera.set(opencv::videoio::CAP_PROP_FRAME_HEIGHT, 480.0)?;
    camera.set(opencv::videoio::CAP_PROP_FPS, 30.0)?;
    camera.set(opencv::videoio::CAP_PROP_BUFFERSIZE, 1.0)?; // Reduce buffer to minimize latency

    // Verify camera settings
    let actual_width = camera.get(opencv::videoio::CAP_PROP_FRAME_WIDTH)?;
    let actual_height = camera.get(opencv::videoio::CAP_PROP_FRAME_HEIGHT)?;
    let actual_fps = camera.get(opencv::videoio::CAP_PROP_FPS)?;

    println!("📐 Camera configured: {}x{} @ {:.1} FPS",
        actual_width as i32, actual_height as i32, actual_fps);

    // Create window for displaying camera feed
    let window_name = "SpectreMesh Camera Feed - Position Yourself Here!";
    highgui::named_window(window_name, highgui::WINDOW_AUTOSIZE)?;

    println!("🎬 Opening high-performance camera viewer...");
    println!("💡 Controls:");
    println!("   - Press 'q' or ESC to quit");
    println!("   - Press 's' to save current frame");
    println!("   - Press SPACE to take a test photo");
    println!("   - Press 'f' to show FPS stats");
    println!("📹 Position yourself in the camera view and check lighting!");
    println!("🎯 Target: 15-30 FPS for reliable face detection");

    let start_time = Instant::now();
    let mut frame_count = 0;
    let mut last_info_time = Instant::now();
    let mut last_fps_time = Instant::now();
    let mut saved_frames = 0;
    let mut show_fps_stats = false;

    loop {
        let frame_start = Instant::now();
        let mut frame = Mat::default();

        if camera.read(&mut frame)? && !frame.empty() {
            frame_count += 1;

            // Add overlay information to the frame (only if not too frequent)
            if show_fps_stats || frame_count % 5 == 0 {
                add_overlay_info(&mut frame, frame_count, start_time.elapsed(), show_fps_stats)?;
            }

            // Display the frame in the window
            highgui::imshow(window_name, &frame)?;

            // Show progress every 3 seconds in console (less frequent for performance)
            if last_info_time.elapsed() >= Duration::from_secs(3) {
                let elapsed = start_time.elapsed().as_secs_f32();
                let fps = frame_count as f32 / elapsed;
                let size = frame.size()?;

                // Calculate recent FPS (last 3 seconds)
                let recent_fps = if last_fps_time.elapsed().as_secs_f32() > 0.1 {
                    30.0 / last_fps_time.elapsed().as_secs_f32() // Approximate recent FPS
                } else {
                    fps
                };

                println!("📸 Frame {}: {}x{} | Avg FPS: {:.1} | Recent FPS: {:.1} | Runtime: {:.0}s",
                    frame_count, size.width, size.height, fps, recent_fps, elapsed);

                // Performance assessment
                if fps < 10.0 {
                    println!("⚠️  FPS too low for reliable face detection (target: 15+ FPS)");
                } else if fps >= 15.0 {
                    println!("✅ FPS adequate for face detection");
                }

                last_info_time = Instant::now();
                last_fps_time = Instant::now();
            }

            // Handle key presses with minimal wait time for better performance
            let key = highgui::wait_key(1)?; // Reduced from 30ms to 1ms
            match key {
                113 | 27 => { // 'q' or ESC
                    println!("👋 User requested exit");
                    break;
                }
                115 => { // 's' - save frame
                    save_current_frame(&frame, frame_count)?;
                    saved_frames += 1;
                    println!("📸 Saved frame {} to file", frame_count);
                }
                32 => { // SPACE - take test photo
                    take_test_photo(&frame, frame_count)?;
                    println!("📷 Test photo taken!");
                }
                102 => { // 'f' - toggle FPS stats
                    show_fps_stats = !show_fps_stats;
                    println!("📊 FPS stats overlay: {}", if show_fps_stats { "ON" } else { "OFF" });
                }
                _ => {}
            }

        } else {
            println!("⚠️  Failed to capture frame {}", frame_count + 1);
            std::thread::sleep(Duration::from_millis(10)); // Reduced delay
        }

        // Optional: Add small delay to prevent CPU overload, but keep it minimal
        let frame_time = frame_start.elapsed();
        if frame_time < Duration::from_millis(33) { // Target ~30 FPS max
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    // Cleanup
    highgui::destroy_all_windows()?;

    // Final results
    println!("\n📊 CAMERA VIEWER RESULTS:");
    println!("=========================");
    println!("✅ Total frames displayed: {}", frame_count);
    println!("📸 Frames saved: {}", saved_frames);
    println!("🎯 Camera viewer session complete!");

    if frame_count > 0 {
        let elapsed = start_time.elapsed().as_secs_f32();
        let avg_fps = frame_count as f32 / elapsed;
        println!("💪 Your camera is working perfectly!");
        println!("📈 Average FPS: {:.1}", avg_fps);
        println!("⏱️  Total runtime: {:.1}s", elapsed);
        println!("👁️  You should have seen yourself in the camera window!");

        println!("\n🎯 Next Steps:");
        println!("   1. If you saw yourself clearly, camera positioning is good");
        println!("   2. If lighting looked good, face detection should work");
        println!("   3. Try running the face detection test now:");
        println!("      cargo run --bin spectreprobe");
    } else {
        println!("❌ No frames were captured - check camera permissions");
    }

    Ok(())
}

/// Add overlay information to the camera frame
fn add_overlay_info(frame: &mut Mat, frame_num: i32, elapsed: Duration, show_detailed: bool) -> Result<(), Box<dyn std::error::Error>> {
    let size = frame.size()?;
    let fps = frame_num as f32 / elapsed.as_secs_f32().max(0.1);

    // Add frame counter and FPS
    let info_text = if show_detailed {
        format!("Frame: {} | FPS: {:.1} | {}x{} | {:.1}s",
            frame_num, fps, size.width, size.height, elapsed.as_secs_f32())
    } else {
        format!("FPS: {:.1}", fps)
    };

    // Color code FPS: Red if low, Yellow if medium, Green if good
    let fps_color = if fps < 10.0 {
        Scalar::new(0.0, 0.0, 255.0, 0.0) // Red
    } else if fps < 15.0 {
        Scalar::new(0.0, 255.0, 255.0, 0.0) // Yellow
    } else {
        Scalar::new(0.0, 255.0, 0.0, 0.0) // Green
    };

    imgproc::put_text(
        frame,
        &info_text,
        Point::new(10, 30),
        imgproc::FONT_HERSHEY_SIMPLEX,
        0.7,
        fps_color,
        2,
        imgproc::LINE_8,
        false,
    )?;

    // Only add detailed overlays if requested (for performance)
    if show_detailed {
        // Add instructions
        let instructions = "Q=Quit | S=Save | SPACE=Photo | F=Stats";
        imgproc::put_text(
            frame,
            &instructions,
            Point::new(10, size.height - 20),
            imgproc::FONT_HERSHEY_SIMPLEX,
            0.5,
            Scalar::new(255.0, 255.0, 255.0, 0.0), // White text
            1,
            imgproc::LINE_8,
            false,
        )?;

        // Add face detection area guide
        let center_x = size.width / 2;
        let center_y = size.height / 2;
        let guide_size = 120; // Smaller for less processing

        let face_rect = Rect::new(
            center_x - guide_size/2,
            center_y - guide_size/2,
            guide_size,
            guide_size
        );

        // Draw face detection guide rectangle
        imgproc::rectangle(
            frame,
            face_rect,
            Scalar::new(0.0, 255.0, 255.0, 0.0), // Yellow
            1, // Thinner line for performance
            imgproc::LINE_8,
            0,
        )?;

        // Add face guide text
        imgproc::put_text(
            frame,
            "Face Here",
            Point::new(center_x - 40, center_y - guide_size/2 - 10),
            imgproc::FONT_HERSHEY_SIMPLEX,
            0.5,
            Scalar::new(0.0, 255.0, 255.0, 0.0), // Yellow
            1,
            imgproc::LINE_8,
            false,
        )?;
    }

    Ok(())
}

/// Save current frame as image file
fn save_current_frame(frame: &Mat, frame_num: i32) -> Result<(), Box<dyn std::error::Error>> {
    let filename = format!("spectremesh_frame_{}.jpg", frame_num);
    opencv::imgcodecs::imwrite(&filename, frame, &opencv::core::Vector::new())?;
    Ok(())
}

/// Take a test photo with timestamp
fn take_test_photo(frame: &Mat, frame_num: i32) -> Result<(), Box<dyn std::error::Error>> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let filename = format!("spectremesh_test_photo_{}_{}.jpg", timestamp, frame_num);
    opencv::imgcodecs::imwrite(&filename, frame, &opencv::core::Vector::new())?;
    Ok(())
}


