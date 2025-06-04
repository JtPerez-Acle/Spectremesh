//! ECS Systems for SpectreMesh

use bevy::prelude::*;
use crate::resources::FearState;
#[allow(unused_imports)] // Used in update_from_frame method parameter
use spectremesh_core::types::FearFrame;

/// System to update fear state from sensor input
pub fn update_fear_system(mut fear_state: ResMut<FearState>) {
    // Collect frames first to avoid borrow conflicts
    let mut frames = Vec::new();
    if let Some(receiver) = &fear_state.receiver {
        // Process all available fear frames (non-blocking)
        while let Ok(frame) = receiver.try_recv() {
            frames.push(frame);
        }
    }

    // Update state with collected frames
    for frame in frames {
        fear_state.update_from_frame(frame);
    }
}

/// System to update terrain based on fear level changes
pub fn update_terrain_system(
    mut fear_state: ResMut<FearState>,
    // TODO: Add terrain query when terrain system is implemented
    // mut terrain_query: Query<&mut TerrainComponent>,
) {
    if fear_state.needs_terrain_rebuild() {
        // TODO: Implement terrain mesh rebuilding based on fear bucket
        // For now, just log the change and mark as complete
        tracing::info!(
            "Terrain update triggered: fear={:.3}, bucket={:?}, distortion={:.3}",
            fear_state.current_fear,
            fear_state.current_bucket,
            fear_state.get_distortion_intensity()
        );

        // YOUR CODE: Use fear_state.get_distortion_intensity() to modify terrain/visuals
        // This is the integration point mentioned in the API documentation

        fear_state.terrain_rebuilt();
    }
}

/// System to update shader uniforms based on fear level
pub fn update_shader_uniforms_system(
    fear_state: Res<FearState>,
    // TODO: Add material query when shader system is implemented
    // mut materials: ResMut<Assets<TerrainMaterial>>,
) {
    // Update distortion intensity uniform every frame for smooth transitions
    let distortion_intensity = fear_state.get_distortion_intensity();

    // TODO: Update actual shader uniforms
    // For now, just trace the value for debugging
    if fear_state.calibrated {
        tracing::trace!("Shader uniform update: distortion_intensity={:.3}", distortion_intensity);
    }
}
