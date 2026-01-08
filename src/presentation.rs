//! Presentation layer plugin owning camera setup and visual rendering systems.
//!
//! `PresentationPlugin` manages the camera lifecycle and will eventually host
//! systems for panning, zooming, and Y-sorting of isometric sprites. It remains
//! a passive observer of simulation state: the DBSP circuit is the sole source
//! of truth for inferred game behaviour.
//!
//! This module supersedes the temporary camera bootstrap in `LilleMapPlugin`.

use bevy::prelude::*;

/// Marker component for the main presentation camera.
///
/// Entities with this component are controlled by the presentation layer's
/// camera systems. Currently there is exactly one such entity, spawned at
/// startup. Future tasks (2.1.2, 2.1.3) will add panning and zoom controls
/// that query for this marker.
///
/// # Examples
///
/// Querying for the presentation camera:
///
/// ```ignore
/// fn camera_controls(
///     mut query: Query<&mut Transform, With<CameraController>>,
/// ) {
///     for mut transform in &mut query {
///         // Adjust camera position
///     }
/// }
/// ```
#[derive(Component, Reflect, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[reflect(Component, Default)]
pub struct CameraController;

/// Plugin owning camera setup and presentation layer systems.
///
/// # Responsibilities
///
/// - Spawns the main `Camera2d` with `CameraController` marker at startup.
/// - Registers `CameraController` for reflection.
/// - Future: Hosts panning, zoom, and Y-sorting systems.
///
/// # Dependencies
///
/// This plugin has no hard dependencies but is typically added alongside
/// `DbspPlugin` and `LilleMapPlugin` in the main application.
///
/// # Examples
///
/// Adding the plugin to an application:
///
/// ```ignore
/// use bevy::prelude::*;
/// use lille::PresentationPlugin;
///
/// App::new()
///     .add_plugins(DefaultPlugins)
///     .add_plugins(PresentationPlugin)
///     .run();
/// ```
#[derive(Debug)]
pub struct PresentationPlugin;

impl Plugin for PresentationPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<CameraController>();
        app.add_systems(Startup, camera_setup);
    }
}

/// Spawns the presentation camera at startup.
///
/// Creates a `Camera2d` entity with:
/// - `CameraController` marker for presentation layer queries
/// - `Name` component for inspector visibility
///
/// Bevy's Required Components mechanism automatically inserts
/// `OrthographicProjection` and other camera infrastructure.
fn camera_setup(mut commands: Commands) {
    commands.spawn((Camera2d, CameraController, Name::new("PresentationCamera")));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_controller_is_eq() {
        // Verify Eq derive allows comparison.
        let a = CameraController;
        let b = CameraController;
        assert_eq!(a, b);
    }

    #[test]
    fn camera_controller_is_copy() {
        let controller = CameraController;
        let copied = controller;
        // Both variables are valid because CameraController is Copy.
        assert_eq!(controller, copied);
    }

    #[test]
    fn camera_controller_debug_format() {
        let controller = CameraController;
        let debug_str = format!("{controller:?}");
        assert_eq!(debug_str, "CameraController");
    }
}
