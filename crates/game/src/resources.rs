//! ECS Resources for SpectreMesh

use bevy::prelude::*;
use spectremesh_core::types::{FearScore, FearFrame, FearBucket};
use async_channel::Receiver;
use std::time::Instant;

/// Resource for managing fear sensor state and integration
#[derive(Resource)]
pub struct FearState {
    /// Current normalized fear level [0.0, 1.0]
    pub current_fear: f32,
    /// Current fear bucket for terrain updates
    pub current_bucket: FearBucket,
    /// Previous fear bucket (to detect changes)
    pub previous_bucket: FearBucket,
    /// Whether the sensor is calibrated
    pub calibrated: bool,
    /// Receiver for fear frames from sensor
    pub receiver: Option<Receiver<FearFrame>>,
    /// Last update timestamp
    pub last_update: Instant,
    /// Distortion intensity for shader uniforms
    pub distortion_intensity: f32,
    /// Whether terrain needs rebuilding
    pub terrain_needs_rebuild: bool,
}

impl Default for FearState {
    fn default() -> Self {
        Self {
            current_fear: 0.3, // Neutral fear level
            current_bucket: FearBucket::Low,
            previous_bucket: FearBucket::Low,
            calibrated: false,
            receiver: None,
            last_update: Instant::now(),
            distortion_intensity: 0.1, // Low distortion by default
            terrain_needs_rebuild: false,
        }
    }
}

impl FearState {
    /// Update fear state from a new frame
    pub fn update_from_frame(&mut self, frame: FearFrame) {
        self.current_fear = frame.fear_score;
        self.calibrated = frame.calibrated;
        self.last_update = Instant::now();

        // Update fear bucket and check for changes
        self.previous_bucket = self.current_bucket;
        self.current_bucket = FearBucket::from_score(frame.fear_score);

        // Update distortion intensity for shaders
        self.distortion_intensity = self.current_bucket.distortion_intensity();

        // Mark terrain for rebuild if bucket changed
        if self.current_bucket != self.previous_bucket {
            self.terrain_needs_rebuild = true;
            tracing::info!(
                "Fear bucket changed: {:?} -> {:?}, marking terrain for rebuild",
                self.previous_bucket,
                self.current_bucket
            );
        }
    }

    /// Update fear state from legacy FearScore
    pub fn update_from_score(&mut self, score: FearScore) {
        self.current_fear = score.value;
        self.calibrated = score.calibrated;
        self.last_update = Instant::now();

        // Update fear bucket and check for changes
        self.previous_bucket = self.current_bucket;
        self.current_bucket = FearBucket::from_score(score.value);

        // Update distortion intensity for shaders
        self.distortion_intensity = self.current_bucket.distortion_intensity();

        // Mark terrain for rebuild if bucket changed
        if self.current_bucket != self.previous_bucket {
            self.terrain_needs_rebuild = true;
            tracing::info!(
                "Fear bucket changed: {:?} -> {:?}, marking terrain for rebuild",
                self.previous_bucket,
                self.current_bucket
            );
        }
    }

    /// Mark terrain rebuild as complete
    pub fn terrain_rebuilt(&mut self) {
        self.terrain_needs_rebuild = false;
    }

    /// Check if terrain needs rebuilding
    pub fn needs_terrain_rebuild(&self) -> bool {
        self.terrain_needs_rebuild
    }

    /// Get current distortion intensity for shader uniforms
    pub fn get_distortion_intensity(&self) -> f32 {
        self.distortion_intensity
    }
}
