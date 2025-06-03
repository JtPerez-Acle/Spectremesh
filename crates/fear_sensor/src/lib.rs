//! Fear detection and emotion recognition for SpectreMesh

pub mod sensor;
pub mod calibration;
pub mod mock_sensor;
pub mod onnx_sensor;

// Re-export main types
pub use sensor::*;
pub use calibration::*;

// Always export both implementations
pub use mock_sensor::MockFearSensor;
pub use onnx_sensor::OnnxFearSensor;

// Default sensor type based on features
#[cfg(feature = "mock")]
pub type DefaultFearSensor = MockFearSensor;

#[cfg(not(feature = "mock"))]
pub type DefaultFearSensor = OnnxFearSensor;
