//! SpectreMesh game library

pub mod components;
pub mod resources;
pub mod systems;

use bevy::prelude::*;
use resources::FearState;
use systems::{update_fear_system, update_terrain_system, update_shader_uniforms_system};

/// SpectreMesh game plugin
pub struct SpectreMeshPlugin;

impl Plugin for SpectreMeshPlugin {
    fn build(&self, app: &mut App) {
        app
            // Add resources
            .init_resource::<FearState>()

            // Add systems
            .add_systems(Update, (
                update_fear_system,
                update_terrain_system.after(update_fear_system),
                update_shader_uniforms_system.after(update_fear_system),
            ));
    }
}

/// Create a basic SpectreMesh app for M0.5 development
pub fn create_spectremesh_app() -> App {
    let mut app = App::new();

    app
        .add_plugins(DefaultPlugins)
        .add_plugins(SpectreMeshPlugin)
        .insert_resource(ClearColor(Color::srgb(0.1, 0.1, 0.15)));

    app
}
