//! Sensor fuzzer for soak testing and simulation
//! 
//! Generates synthetic SensorEvent streams with configurable patterns
//! and random faults for testing system resilience.

use spectre_sensor::{
    proto::{sensor_event, SensorEvent, Score, SensorFault, CalibrationProgress, FaultSeverity},
};
use clap::{Parser, Subcommand};
use rand::Rng;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, interval};
use tracing::{info, warn, error};

#[derive(Parser)]
#[command(name = "sensor_fuzzer")]
#[command(about = "Synthetic sensor event generator for testing")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate synthetic score events
    Scores {
        /// Number of events to generate (0 = infinite)
        #[arg(short, long, default_value = "0")]
        count: u64,
        
        /// Events per second
        #[arg(short, long, default_value = "30.0")]
        fps: f32,
        
        /// Fear pattern: sine, step, random, or constant
        #[arg(short, long, default_value = "sine")]
        pattern: String,
        
        /// Base fear level for patterns
        #[arg(short, long, default_value = "0.5")]
        base_fear: f32,
        
        /// Amplitude for sine/random patterns
        #[arg(short, long, default_value = "0.3")]
        amplitude: f32,
        
        /// Period in seconds for sine pattern
        #[arg(long, default_value = "10.0")]
        period: f32,
        
        /// Probability of inference errors (0.0-1.0)
        #[arg(long, default_value = "0.01")]
        error_rate: f32,
        
        /// Socket path for gRPC connection
        #[arg(long, default_value = "/tmp/spectre_sensor.sock")]
        socket: String,
    },
    
    /// Generate calibration events
    Calibration {
        /// Calibration duration in seconds
        #[arg(short, long, default_value = "30.0")]
        duration: f32,
        
        /// Updates per second during calibration
        #[arg(short, long, default_value = "10.0")]
        fps: f32,
        
        /// Socket path for gRPC connection
        #[arg(long, default_value = "/tmp/spectre_sensor.sock")]
        socket: String,
    },
    
    /// Generate fault events
    Faults {
        /// Number of faults to generate
        #[arg(short, long, default_value = "10")]
        count: u64,
        
        /// Interval between faults in seconds
        #[arg(short, long, default_value = "5.0")]
        interval: f32,
        
        /// Socket path for gRPC connection
        #[arg(long, default_value = "/tmp/spectre_sensor.sock")]
        socket: String,
    },
    
    /// Run comprehensive soak test
    SoakTest {
        /// Test duration in seconds
        #[arg(short, long, default_value = "300")]
        duration: u64,
        
        /// Socket path for gRPC connection
        #[arg(long, default_value = "/tmp/spectre_sensor.sock")]
        socket: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Scores { 
            count, fps, pattern, base_fear, amplitude, period, error_rate, socket 
        } => {
            generate_scores(count, fps, &pattern, base_fear, amplitude, period, error_rate, &socket).await?;
        },
        Commands::Calibration { duration, fps, socket } => {
            generate_calibration(duration, fps, &socket).await?;
        },
        Commands::Faults { count, interval, socket } => {
            generate_faults(count, interval, &socket).await?;
        },
        Commands::SoakTest { duration, socket } => {
            run_soak_test(duration, &socket).await?;
        },
    }
    
    Ok(())
}

/// Generate synthetic score events
async fn generate_scores(
    count: u64,
    fps: f32,
    pattern: &str,
    base_fear: f32,
    amplitude: f32,
    period: f32,
    error_rate: f32,
    _socket: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Generating scores: pattern={}, fps={}, count={}", pattern, fps, count);

    let frame_duration = Duration::from_secs_f32(1.0 / fps);
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::from_entropy();
    let mut generated = 0u64;
    let start_time = std::time::Instant::now();
    
    loop {
        if count > 0 && generated >= count {
            break;
        }
        
        let frame_start = std::time::Instant::now();
        
        // Generate fear score based on pattern
        let elapsed = start_time.elapsed().as_secs_f32();
        let fear_score = match pattern {
            "sine" => {
                base_fear + amplitude * (2.0 * std::f32::consts::PI * elapsed / period).sin()
            },
            "step" => {
                if (elapsed / period) as u32 % 2 == 0 {
                    base_fear - amplitude
                } else {
                    base_fear + amplitude
                }
            },
            "random" => {
                base_fear + amplitude * (rng.gen::<f32>() - 0.5) * 2.0
            },
            "constant" => base_fear,
            _ => {
                warn!("Unknown pattern '{}', using constant", pattern);
                base_fear
            }
        }.clamp(0.0, 1.0);
        
        // Generate synthetic emotion logits
        let mut emotion_logits = vec![0.1f32; 7];
        emotion_logits[2] = fear_score * 0.8 + 0.1; // Fear logit
        
        // Simulate inference error
        if rng.gen::<f32>() < error_rate {
            error!("Simulated inference error at frame {}", generated);
            continue;
        }
        
        // Create score event
        let event = SensorEvent {
            timestamp_us: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64,
            event: Some(sensor_event::Event::Score(Score {
                normalized_fear: fear_score,
                raw_fear_logit: emotion_logits[2],
                confidence: 0.8 + rng.gen::<f32>() * 0.2, // 0.8-1.0
                calibrated: generated > 30, // Calibrated after 30 frames
                emotion_logits,
                inference_latency_us: (3000 + rng.gen::<u64>() % 5000), // 3-8ms
            })),
        };
        
        // Print event (in real implementation, this would be sent via gRPC)
        println!("Score: fear={:.3}, confidence={:.3}, calibrated={}", 
                fear_score, 
                if let Some(sensor_event::Event::Score(ref score)) = event.event {
                    score.confidence
                } else { 0.0 },
                if let Some(sensor_event::Event::Score(ref score)) = event.event {
                    score.calibrated
                } else { false });
        
        generated += 1;
        
        // Maintain target FPS
        let elapsed = frame_start.elapsed();
        if elapsed < frame_duration {
            sleep(frame_duration - elapsed).await;
        }
    }
    
    info!("Generated {} score events", generated);
    Ok(())
}

/// Generate calibration progress events
async fn generate_calibration(
    duration: f32,
    fps: f32,
    _socket: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Generating calibration events: duration={}s, fps={}", duration, fps);
    
    let total_frames = (duration * fps) as u64;
    let frame_duration = Duration::from_secs_f32(1.0 / fps);
    
    for frame in 0..total_frames {
        let progress = frame as f32 / total_frames as f32;
        let completed = progress >= 1.0;
        
        let _event = SensorEvent {
            timestamp_us: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64,
            event: Some(sensor_event::Event::CalibrationProgress(CalibrationProgress {
                progress,
                completed,
                baseline: if completed {
                    Some(spectre_sensor::proto::BaselineStats {
                        mean: 0.3,
                        std_dev: 0.15,
                        sample_count: total_frames as u32,
                    })
                } else {
                    None
                },
            })),
        };
        
        println!("Calibration: progress={:.1}%, completed={}", progress * 100.0, completed);
        
        sleep(frame_duration).await;
    }
    
    info!("Calibration simulation complete");
    Ok(())
}

/// Generate fault events
async fn generate_faults(
    count: u64,
    interval: f32,
    _socket: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Generating {} fault events with {}s interval", count, interval);
    
    let fault_types = [
        ("CAMERA_DISCONNECTED", FaultSeverity::Error, true),
        ("MODEL_INFERENCE_TIMEOUT", FaultSeverity::Warning, true),
        ("FACE_DETECTION_FAILED", FaultSeverity::Info, true),
        ("CALIBRATION_DRIFT", FaultSeverity::Warning, false),
        ("SYSTEM_OVERLOAD", FaultSeverity::Critical, false),
    ];
    
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::from_entropy();
    
    for i in 0..count {
        let (error_code, severity, recoverable) = fault_types[rng.gen_range(0..fault_types.len())];
        
        let _event = SensorEvent {
            timestamp_us: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64,
            event: Some(sensor_event::Event::SensorFault(SensorFault {
                severity: severity as i32,
                message: format!("Simulated fault #{}: {}", i + 1, error_code),
                error_code: error_code.to_string(),
                recoverable,
            })),
        };
        
        println!("Fault: {} - {} (recoverable: {})", 
                error_code, 
                format!("Simulated fault #{}", i + 1), 
                recoverable);
        
        if i < count - 1 {
            sleep(Duration::from_secs_f32(interval)).await;
        }
    }
    
    info!("Generated {} fault events", count);
    Ok(())
}

/// Run comprehensive soak test
async fn run_soak_test(
    duration: u64,
    socket: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting soak test for {} seconds", duration);
    
    let end_time = std::time::Instant::now() + Duration::from_secs(duration);
    
    // Spawn concurrent tasks for different event types
    let score_task = tokio::spawn({
        let socket = socket.to_string();
        async move {
            let _ = generate_scores(0, 30.0, "sine", 0.5, 0.3, 20.0, 0.02, &socket).await;
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        }
    });
    
    let calibration_task = tokio::spawn({
        let socket = socket.to_string();
        async move {
            // Recalibrate every 60 seconds
            let mut interval = interval(Duration::from_secs(60));
            while std::time::Instant::now() < end_time {
                interval.tick().await;
                let _ = generate_calibration(30.0, 10.0, &socket).await;
            }
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        }
    });
    
    let fault_task = tokio::spawn({
        let socket = socket.to_string();
        async move {
            // Generate random faults
            use rand::SeedableRng;
            let mut rng = rand::rngs::StdRng::from_entropy();
            while std::time::Instant::now() < end_time {
                let wait_time = Duration::from_secs(rng.gen_range(10..60));
                sleep(wait_time).await;
                let _ = generate_faults(1, 0.0, &socket).await;
            }
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        }
    });
    
    // Wait for test duration
    sleep(Duration::from_secs(duration)).await;
    
    // Cancel tasks
    score_task.abort();
    calibration_task.abort();
    fault_task.abort();
    
    info!("Soak test completed");
    Ok(())
}
