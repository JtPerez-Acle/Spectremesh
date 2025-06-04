//! Prometheus metrics server for sensor monitoring

use prometheus::{
    Counter, Gauge, Histogram, Registry, TextEncoder,
    HistogramOpts, Opts,
};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use crate::types::PerformanceMetrics;

/// Prometheus metrics collection
#[derive(Clone)]
pub struct SensorMetrics {
    registry: Registry,
    
    // Counters
    frames_processed: Counter,
    frames_dropped: Counter,
    inference_errors: Counter,
    calibration_resets: Counter,
    
    // Gauges
    current_fps: Gauge,
    calibration_progress: Gauge,
    calibration_drift: Gauge,
    
    // Histograms
    inference_latency: Histogram,
}

impl SensorMetrics {
    /// Create new metrics collection
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();
        
        // Create metrics
        let frames_processed = Counter::with_opts(Opts::new(
            "spectre_frames_processed_total",
            "Total number of frames processed by the sensor"
        ))?;
        
        let frames_dropped = Counter::with_opts(Opts::new(
            "spectre_frames_dropped_total", 
            "Total number of frames dropped due to back-pressure"
        ))?;
        
        let inference_errors = Counter::with_opts(Opts::new(
            "spectre_inference_errors_total",
            "Total number of inference errors"
        ))?;
        
        let calibration_resets = Counter::with_opts(Opts::new(
            "spectre_calibration_resets_total",
            "Total number of calibration resets"
        ))?;
        
        let current_fps = Gauge::with_opts(Opts::new(
            "spectre_current_fps",
            "Current frames per second"
        ))?;
        
        let calibration_progress = Gauge::with_opts(Opts::new(
            "spectre_calibration_progress",
            "Current calibration progress [0.0, 1.0]"
        ))?;
        
        let calibration_drift = Gauge::with_opts(Opts::new(
            "spectre_calibration_drift",
            "Calibration drift (change in baseline mean)"
        ))?;
        
        let inference_latency = Histogram::with_opts(HistogramOpts::new(
            "spectre_inference_latency_seconds",
            "Inference latency in seconds"
        ).buckets(vec![
            0.001, 0.002, 0.005, 0.010, 0.020, 0.050, 0.100, 0.200, 0.500, 1.0
        ]))?;
        
        // Register metrics
        registry.register(Box::new(frames_processed.clone()))?;
        registry.register(Box::new(frames_dropped.clone()))?;
        registry.register(Box::new(inference_errors.clone()))?;
        registry.register(Box::new(calibration_resets.clone()))?;
        registry.register(Box::new(current_fps.clone()))?;
        registry.register(Box::new(calibration_progress.clone()))?;
        registry.register(Box::new(calibration_drift.clone()))?;
        registry.register(Box::new(inference_latency.clone()))?;
        
        Ok(Self {
            registry,
            frames_processed,
            frames_dropped,
            inference_errors,
            calibration_resets,
            current_fps,
            calibration_progress,
            calibration_drift,
            inference_latency,
        })
    }
    
    /// Record a processed frame
    pub fn record_frame_processed(&self) {
        self.frames_processed.inc();
    }
    
    /// Record a dropped frame
    pub fn record_frame_dropped(&self) {
        self.frames_dropped.inc();
    }
    
    /// Record an inference error
    pub fn record_inference_error(&self) {
        self.inference_errors.inc();
    }
    
    /// Record a calibration reset
    pub fn record_calibration_reset(&self) {
        self.calibration_resets.inc();
    }
    
    /// Update current FPS
    pub fn update_fps(&self, fps: f32) {
        self.current_fps.set(fps as f64);
    }
    
    /// Update calibration progress
    pub fn update_calibration_progress(&self, progress: f32) {
        self.calibration_progress.set(progress as f64);
    }
    
    /// Update calibration drift
    pub fn update_calibration_drift(&self, drift: f32) {
        self.calibration_drift.set(drift as f64);
    }
    
    /// Record inference latency
    pub fn record_inference_latency(&self, latency_seconds: f64) {
        self.inference_latency.observe(latency_seconds);
    }
    
    /// Update all metrics from performance metrics
    pub fn update_from_performance_metrics(&self, metrics: &PerformanceMetrics) {
        self.update_fps(metrics.current_fps);
        self.update_calibration_drift(metrics.calibration_drift);
        self.record_inference_latency(metrics.p95_inference_latency.as_secs_f64());
    }
    
    /// Get metrics as Prometheus text format
    pub fn gather(&self) -> Result<String, prometheus::Error> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder.encode_to_string(&metric_families)
    }
}

/// Metrics server state
#[derive(Clone)]
pub struct MetricsState {
    pub metrics: Arc<SensorMetrics>,
}

/// Start metrics server on specified port
pub async fn start_metrics_server(
    port: u16,
    metrics: Arc<SensorMetrics>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state = MetricsState { metrics };
    
    let app = Router::new()
        .route("/metrics", get(metrics_handler))
        .route("/health", get(health_handler))
        .with_state(state)
        .layer(ServiceBuilder::new());
    
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    
    tracing::info!("Metrics server listening on {}", addr);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

/// Metrics endpoint handler
async fn metrics_handler(State(state): State<MetricsState>) -> Response {
    match state.metrics.gather() {
        Ok(metrics_text) => {
            (
                StatusCode::OK,
                [("content-type", "text/plain; version=0.0.4")],
                metrics_text,
            ).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to gather metrics: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to gather metrics: {}", e),
            ).into_response()
        }
    }
}

/// Health check endpoint
async fn health_handler() -> Response {
    (StatusCode::OK, "OK").into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_metrics_creation() {
        let metrics = SensorMetrics::new().unwrap();
        
        // Test that we can gather metrics without error
        let gathered = metrics.gather().unwrap();
        assert!(!gathered.is_empty());
    }

    #[test]
    fn test_metrics_recording() {
        let metrics = SensorMetrics::new().unwrap();
        
        // Record some metrics
        metrics.record_frame_processed();
        metrics.record_frame_dropped();
        metrics.record_inference_error();
        metrics.record_calibration_reset();
        metrics.update_fps(30.0);
        metrics.update_calibration_progress(0.5);
        metrics.update_calibration_drift(0.1);
        metrics.record_inference_latency(0.005);
        
        // Gather metrics and check they contain our data
        let gathered = metrics.gather().unwrap();
        assert!(gathered.contains("spectre_frames_processed_total"));
        assert!(gathered.contains("spectre_frames_dropped_total"));
        assert!(gathered.contains("spectre_inference_errors_total"));
        assert!(gathered.contains("spectre_calibration_resets_total"));
        assert!(gathered.contains("spectre_current_fps"));
        assert!(gathered.contains("spectre_calibration_progress"));
        assert!(gathered.contains("spectre_calibration_drift"));
        assert!(gathered.contains("spectre_inference_latency_seconds"));
    }

    #[test]
    fn test_performance_metrics_update() {
        let metrics = SensorMetrics::new().unwrap();
        
        let perf_metrics = PerformanceMetrics {
            current_fps: 25.5,
            p95_inference_latency: Duration::from_millis(8),
            dropped_frames: 5,
            calibration_drift: 0.15,
            last_update: std::time::Instant::now(),
        };
        
        metrics.update_from_performance_metrics(&perf_metrics);
        
        let gathered = metrics.gather().unwrap();
        assert!(gathered.contains("25.5")); // FPS
        assert!(gathered.contains("0.008")); // Latency in seconds
        assert!(gathered.contains("0.15")); // Drift
    }

    #[tokio::test]
    async fn test_metrics_server_creation() {
        let metrics = Arc::new(SensorMetrics::new().unwrap());
        
        // Test that we can create the router without error
        let state = MetricsState { 
            metrics: metrics.clone() 
        };
        
        let _app: axum::Router<MetricsState> = Router::new()
            .route("/metrics", get(metrics_handler))
            .route("/health", get(health_handler))
            .with_state(state);
        
        // We can't easily test the full server without binding to a port
        // but we can test the handler functions
    }
}
