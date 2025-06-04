//! gRPC server implementation for sensor streaming

use crate::{
    proto::{
        sensor_service_server::{SensorService, SensorServiceServer},
        *,
    },
    types::FearFrame,
    sensor::EmotionSensor,
};
use async_channel::Receiver;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::{wrappers::ReceiverStream, Stream};
use tonic::{transport::Server, Request, Response, Status, Code};
use std::pin::Pin;

/// gRPC service implementation
pub struct SensorServiceImpl {
    sensor: Arc<Mutex<EmotionSensor>>,
}

impl SensorServiceImpl {
    /// Create new service implementation
    pub fn new(sensor: EmotionSensor) -> Self {
        Self {
            sensor: Arc::new(Mutex::new(sensor)),
        }
    }
}

#[tonic::async_trait]
impl SensorService for SensorServiceImpl {
    type StreamEventsStream = Pin<Box<dyn Stream<Item = Result<SensorEvent, Status>> + Send>>;

    /// Stream sensor events
    async fn stream_events(
        &self,
        request: Request<StreamRequest>,
    ) -> Result<Response<Self::StreamEventsStream>, Status> {
        let req = request.into_inner();
        let event_types = req.event_types;
        
        tracing::info!("Starting sensor event stream with filters: {:?}", event_types);
        
        // Start the sensor
        let receiver = {
            let mut sensor = self.sensor.lock().await;
            sensor.start().await.map_err(|e| {
                Status::new(Code::Internal, format!("Failed to start sensor: {}", e))
            })?
        };
        
        // Create event stream
        let stream = create_event_stream(receiver, event_types);
        
        Ok(Response::new(Box::pin(stream)))
    }

    /// Get current sensor status
    async fn get_status(
        &self,
        _request: Request<StatusRequest>,
    ) -> Result<Response<StatusResponse>, Status> {
        let sensor = self.sensor.lock().await;
        let state = sensor.get_state();
        
        let response = StatusResponse {
            running: state.running,
            calibration: Some(CalibrationProgress {
                progress: state.calibration_progress,
                completed: state.calibrated,
                baseline: if state.calibrated {
                    Some(BaselineStats {
                        mean: 0.0, // Would need to get from calibrator
                        std_dev: 1.0,
                        sample_count: 0,
                    })
                } else {
                    None
                },
            }),
            last_error: state.last_error.map(|msg| SensorFault {
                severity: FaultSeverity::Error as i32,
                message: msg,
                error_code: "SENSOR_ERROR".to_string(),
                recoverable: true,
            }),
            metrics: Some(crate::proto::PerformanceMetrics {
                current_fps: state.metrics.current_fps,
                p95_inference_latency_us: state.metrics.p95_inference_latency.as_micros() as u64,
                dropped_frames: state.metrics.dropped_frames,
                calibration_drift: state.metrics.calibration_drift,
            }),
        };
        
        Ok(Response::new(response))
    }

    /// Control calibration
    async fn control_calibration(
        &self,
        request: Request<CalibrationControl>,
    ) -> Result<Response<CalibrationResponse>, Status> {
        let req = request.into_inner();
        let mut sensor = self.sensor.lock().await;
        
        let result = match req.action {
            Some(calibration_control::Action::StartCalibration(_)) => {
                sensor.reset_calibration()
            },
            Some(calibration_control::Action::FreezeCalibration(freeze)) => {
                sensor.control_calibration(freeze)
            },
            Some(calibration_control::Action::ResetCalibration(_)) => {
                sensor.reset_calibration()
            },
            None => {
                return Ok(Response::new(CalibrationResponse {
                    success: false,
                    error_message: Some("No action specified".to_string()),
                }));
            }
        };
        
        let response = match result {
            Ok(_) => CalibrationResponse {
                success: true,
                error_message: None,
            },
            Err(e) => CalibrationResponse {
                success: false,
                error_message: Some(e.to_string()),
            },
        };
        
        Ok(Response::new(response))
    }
}

/// Create event stream from fear frame receiver
fn create_event_stream(
    receiver: Receiver<FearFrame>,
    event_filters: Vec<i32>,
) -> impl Stream<Item = Result<SensorEvent, Status>> {
    let (tx, rx) = tokio::sync::mpsc::channel(100);
    
    // Convert event type filters
    let filters: Vec<EventType> = event_filters
        .into_iter()
        .filter_map(|i| EventType::try_from(i).ok())
        .collect();
    
    // Spawn task to convert fear frames to sensor events
    tokio::spawn(async move {
        while let Ok(fear_frame) = receiver.recv().await {
            let event = SensorEvent {
                timestamp_us: fear_frame.timestamp_us(),
                event: Some(sensor_event::Event::Score(Score {
                    normalized_fear: fear_frame.fear_score,
                    raw_fear_logit: fear_frame.extract_fear_logit(),
                    confidence: fear_frame.confidence,
                    calibrated: fear_frame.calibrated,
                    emotion_logits: fear_frame.emotion_logits.to_vec(),
                    inference_latency_us: fear_frame.inference_latency.as_micros() as u64,
                })),
            };
            
            // Apply filters
            if should_send_event(&event, &filters) {
                if tx.try_send(Ok(event)).is_err() {
                    break; // Receiver dropped or channel full
                }
            }
        }
    });
    
    ReceiverStream::new(rx)
}

/// Check if event should be sent based on filters
fn should_send_event(event: &SensorEvent, filters: &[EventType]) -> bool {
    if filters.is_empty() {
        return true; // No filters, send all events
    }
    
    match &event.event {
        Some(sensor_event::Event::CalibrationProgress(_)) => {
            filters.contains(&EventType::CalibrationProgress)
        },
        Some(sensor_event::Event::Score(_)) => {
            filters.contains(&EventType::Score)
        },
        Some(sensor_event::Event::SensorFault(_)) => {
            filters.contains(&EventType::SensorFault)
        },
        None => false,
    }
}

/// Start gRPC server on TCP (simplified for compatibility)
pub async fn start_grpc_server(
    _socket_path: &str,
    sensor: EmotionSensor,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let service = SensorServiceImpl::new(sensor);
    let server = SensorServiceServer::new(service);

    let addr = "127.0.0.1:50051".parse()?;

    tracing::info!("gRPC server listening on TCP: {}", addr);

    Server::builder()
        .add_service(server)
        .serve(addr)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SensorConfig;

    #[test]
    fn test_service_creation() {
        let config = SensorConfig::default();
        let sensor = EmotionSensor::new(config);
        let _service = SensorServiceImpl::new(sensor);
    }

    #[test]
    fn test_event_filtering() {
        let event = SensorEvent {
            timestamp_us: 0,
            event: Some(sensor_event::Event::Score(Score {
                normalized_fear: 0.5,
                raw_fear_logit: 0.3,
                confidence: 0.9,
                calibrated: true,
                emotion_logits: vec![0.1; 7],
                inference_latency_us: 5000,
            })),
        };
        
        // No filters - should send
        assert!(should_send_event(&event, &[]));
        
        // Matching filter - should send
        assert!(should_send_event(&event, &[EventType::Score]));
        
        // Non-matching filter - should not send
        assert!(!should_send_event(&event, &[EventType::CalibrationProgress]));
        
        // Multiple filters with match - should send
        assert!(should_send_event(&event, &[EventType::CalibrationProgress, EventType::Score]));
    }

    #[tokio::test]
    async fn test_status_response() {
        let config = SensorConfig::default();
        let sensor = EmotionSensor::new(config);
        let service = SensorServiceImpl::new(sensor);
        
        let request = Request::new(StatusRequest {});
        let response = service.get_status(request).await.unwrap();
        
        let status = response.into_inner();
        assert!(!status.running); // Should not be running initially
        assert!(status.calibration.is_some());
        assert!(status.metrics.is_some());
    }
}
