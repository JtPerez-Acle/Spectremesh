//! gRPC client helpers for connecting to the sensor service

use crate::proto::{
    sensor_service_client::SensorServiceClient,
    *,
};
use tonic::{transport::Channel, Request, Status};
use futures::StreamExt;
use std::time::Duration;

/// Client wrapper for sensor service
pub struct SensorClient {
    client: SensorServiceClient<Channel>,
}

impl SensorClient {
    /// Connect to sensor service via Unix socket (simplified to TCP for now)
    pub async fn connect_unix(socket_path: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // For now, use TCP instead of Unix socket due to tonic compatibility issues
        // In production, this would use proper Unix socket support
        let _ = socket_path; // Suppress unused warning
        let channel = Channel::from_static("http://127.0.0.1:50051")
            .connect()
            .await?;

        let client = SensorServiceClient::new(channel);

        Ok(Self { client })
    }
    
    /// Connect to sensor service via TCP
    pub async fn connect_tcp(address: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let channel = Channel::from_shared(format!("http://{}", address))?
            .connect()
            .await?;
        
        let client = SensorServiceClient::new(channel);
        
        Ok(Self { client })
    }
    
    /// Stream all sensor events
    pub async fn stream_events(&mut self) -> Result<impl StreamExt<Item = Result<SensorEvent, Status>>, Status> {
        let request = Request::new(StreamRequest {
            event_types: vec![], // No filters, get all events
        });
        
        let response = self.client.stream_events(request).await?;
        Ok(response.into_inner())
    }
    
    /// Stream only score events
    pub async fn stream_scores(&mut self) -> Result<impl StreamExt<Item = Result<SensorEvent, Status>>, Status> {
        let request = Request::new(StreamRequest {
            event_types: vec![EventType::Score as i32],
        });
        
        let response = self.client.stream_events(request).await?;
        Ok(response.into_inner())
    }
    
    /// Stream only calibration progress events
    pub async fn stream_calibration(&mut self) -> Result<impl StreamExt<Item = Result<SensorEvent, Status>>, Status> {
        let request = Request::new(StreamRequest {
            event_types: vec![EventType::CalibrationProgress as i32],
        });
        
        let response = self.client.stream_events(request).await?;
        Ok(response.into_inner())
    }
    
    /// Get current sensor status
    pub async fn get_status(&mut self) -> Result<StatusResponse, Status> {
        let request = Request::new(StatusRequest {});
        let response = self.client.get_status(request).await?;
        Ok(response.into_inner())
    }
    
    /// Start calibration
    pub async fn start_calibration(&mut self) -> Result<CalibrationResponse, Status> {
        let request = Request::new(CalibrationControl {
            action: Some(calibration_control::Action::StartCalibration(true)),
        });
        
        let response = self.client.control_calibration(request).await?;
        Ok(response.into_inner())
    }
    
    /// Freeze calibration
    pub async fn freeze_calibration(&mut self) -> Result<CalibrationResponse, Status> {
        let request = Request::new(CalibrationControl {
            action: Some(calibration_control::Action::FreezeCalibration(true)),
        });
        
        let response = self.client.control_calibration(request).await?;
        Ok(response.into_inner())
    }
    
    /// Unfreeze calibration
    pub async fn unfreeze_calibration(&mut self) -> Result<CalibrationResponse, Status> {
        let request = Request::new(CalibrationControl {
            action: Some(calibration_control::Action::FreezeCalibration(false)),
        });
        
        let response = self.client.control_calibration(request).await?;
        Ok(response.into_inner())
    }
    
    /// Reset calibration
    pub async fn reset_calibration(&mut self) -> Result<CalibrationResponse, Status> {
        let request = Request::new(CalibrationControl {
            action: Some(calibration_control::Action::ResetCalibration(true)),
        });
        
        let response = self.client.control_calibration(request).await?;
        Ok(response.into_inner())
    }
    
    /// Wait for calibration to complete
    pub async fn wait_for_calibration(&mut self, timeout: Duration) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let start = std::time::Instant::now();
        
        while start.elapsed() < timeout {
            let status = self.get_status().await?;
            
            if let Some(calibration) = status.calibration {
                if calibration.completed {
                    return Ok(true);
                }
            }
            
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        Ok(false) // Timeout
    }
    
    /// Get the next fear score
    pub async fn get_next_score(&mut self) -> Result<Option<Score>, Box<dyn std::error::Error + Send + Sync>> {
        let mut stream = self.stream_scores().await?;
        
        while let Some(event_result) = stream.next().await {
            let event = event_result?;
            
            if let Some(sensor_event::Event::Score(score)) = event.event {
                return Ok(Some(score));
            }
        }
        
        Ok(None)
    }
}

/// Helper function to extract scores from event stream
pub fn extract_scores(events: impl StreamExt<Item = Result<SensorEvent, Status>>) -> impl StreamExt<Item = Result<Score, Status>> {
    events
        .filter_map(|event_result| async move {
            match event_result {
                Ok(event) => {
                    if let Some(sensor_event::Event::Score(score)) = event.event {
                        Some(Ok(score))
                    } else {
                        None
                    }
                },
                Err(e) => Some(Err(e)),
            }
        })
}

/// Helper function to extract calibration progress from event stream
pub fn extract_calibration_progress(
    events: impl StreamExt<Item = Result<SensorEvent, Status>>
) -> impl StreamExt<Item = Result<CalibrationProgress, Status>> {
    events
        .filter_map(|event_result| async move {
            match event_result {
                Ok(event) => {
                    if let Some(sensor_event::Event::CalibrationProgress(progress)) = event.event {
                        Some(Ok(progress))
                    } else {
                        None
                    }
                },
                Err(e) => Some(Err(e)),
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        // We can't easily test the actual connection without a running server
        // but we can test that the client structure is correct
        
        // Test that we can create requests
        let stream_request = StreamRequest {
            event_types: vec![EventType::Score as i32],
        };
        assert_eq!(stream_request.event_types.len(), 1);
        
        let status_request = StatusRequest {};
        assert_eq!(std::mem::size_of_val(&status_request), 0); // Empty struct
        
        let calibration_request = CalibrationControl {
            action: Some(calibration_control::Action::StartCalibration(true)),
        };
        assert!(calibration_request.action.is_some());
    }

    #[tokio::test]
    async fn test_stream_filtering() {
        // Create a mock stream of events
        let events = vec![
            Ok(SensorEvent {
                timestamp_us: 1000,
                event: Some(sensor_event::Event::Score(Score {
                    normalized_fear: 0.5,
                    raw_fear_logit: 0.3,
                    confidence: 0.9,
                    calibrated: true,
                    emotion_logits: vec![0.1; 7],
                    inference_latency_us: 5000,
                })),
            }),
            Ok(SensorEvent {
                timestamp_us: 2000,
                event: Some(sensor_event::Event::CalibrationProgress(CalibrationProgress {
                    progress: 0.5,
                    completed: false,
                    baseline: None,
                })),
            }),
            Ok(SensorEvent {
                timestamp_us: 3000,
                event: Some(sensor_event::Event::Score(Score {
                    normalized_fear: 0.7,
                    raw_fear_logit: 0.5,
                    confidence: 0.8,
                    calibrated: true,
                    emotion_logits: vec![0.2; 7],
                    inference_latency_us: 4000,
                })),
            }),
        ];
        
        let stream = tokio_stream::iter(events);
        let score_stream = extract_scores(stream);
        let mut score_stream = std::pin::pin!(score_stream);

        // Should get 2 scores
        let score1 = score_stream.next().await.unwrap().unwrap();
        assert_eq!(score1.normalized_fear, 0.5);

        let score2 = score_stream.next().await.unwrap().unwrap();
        assert_eq!(score2.normalized_fear, 0.7);

        // Should be no more scores
        assert!(score_stream.next().await.is_none());
    }
}
