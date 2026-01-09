//! Presentation layer plugin owning camera setup and visual rendering systems.
//!
//! `PresentationPlugin` manages the camera lifecycle and will eventually host
//! systems for panning, zooming, and Y-sorting of isometric sprites. It remains
//! a passive observer of simulation state: the DBSP circuit is the sole source
//! of truth for inferred game behaviour.
//!
//! This module supersedes the temporary camera bootstrap in `LilleMapPlugin`.

use bevy::prelude::*;

use crate::apply_dbsp_outputs_system;

/// Marker component for the main presentation camera.
///
/// Entities with this component are controlled by the presentation layer's
/// camera systems. Currently there is exactly one such entity, spawned at
/// startup. The `camera_pan_system` queries for this marker to apply
/// keyboard-based panning.
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

/// Runtime configuration for camera panning behaviour.
///
/// Controls the speed at which the camera pans when using keyboard input
/// (WASD or arrow keys). The `max_delta_seconds` field prevents the camera
/// from teleporting during frame hitches by clamping the delta time.
///
/// # Examples
///
/// Customising camera speed:
///
/// ```ignore
/// use bevy::prelude::*;
/// use lille::presentation::CameraSettings;
///
/// let mut app = App::new();
/// app.insert_resource(CameraSettings {
///     pan_speed: 800.0,
///     max_delta_seconds: 0.1,
/// });
/// ```
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct CameraSettings {
    /// Camera pan speed in world units per second.
    ///
    /// A higher value makes the camera move faster across the map.
    /// Typical values range from 200.0 (slow) to 1000.0 (fast).
    pub pan_speed: f32,

    /// Maximum delta time to use for movement calculations.
    ///
    /// Clamps large frame hitches to prevent the camera jumping
    /// extreme distances during lag spikes.
    pub max_delta_seconds: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            pan_speed: 500.0,
            max_delta_seconds: 0.1,
        }
    }
}

/// Directional key states for camera panning.
///
/// Captures the pressed state of movement keys (WASD or arrow keys) to
/// compute a pan direction vector.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "This struct represents the pressed state of exactly four directional keys."
)]
pub struct PanInput {
    /// Whether an "up" key (W or `ArrowUp`) is pressed.
    pub up: bool,
    /// Whether a "down" key (S or `ArrowDown`) is pressed.
    pub down: bool,
    /// Whether a "left" key (A or `ArrowLeft`) is pressed.
    pub left: bool,
    /// Whether a "right" key (D or `ArrowRight`) is pressed.
    pub right: bool,
}

/// Computes a normalized pan direction from the given key states.
///
/// Returns `Vec2::ZERO` if no movement keys are pressed.
/// Diagonal movement is normalized to prevent faster movement when
/// multiple keys are held simultaneously.
///
/// # Examples
///
/// ```
/// use bevy::math::Vec2;
/// use lille::presentation::{compute_pan_direction, PanInput};
///
/// // Single key: unit vector
/// let input = PanInput { up: true, ..Default::default() };
/// let dir = compute_pan_direction(input);
/// assert!((dir.y - 1.0).abs() < f32::EPSILON);
///
/// // Diagonal: normalized
/// let input = PanInput { up: true, right: true, ..Default::default() };
/// let diag = compute_pan_direction(input);
/// assert!((diag.length() - 1.0).abs() < 0.001);
/// ```
#[must_use]
pub fn compute_pan_direction(input: PanInput) -> Vec2 {
    /// Maps a negative/positive key pair to an axis value.
    const fn axis(neg: bool, pos: bool) -> f32 {
        match (neg, pos) {
            (true, false) => -1.0,
            (false, true) => 1.0,
            _ => 0.0,
        }
    }

    let x = axis(input.left, input.right);
    let y = axis(input.down, input.up);
    let raw = Vec2::new(x, y);

    if raw == Vec2::ZERO {
        Vec2::ZERO
    } else {
        raw.normalize()
    }
}

/// Updates camera position based on keyboard input.
///
/// Reads WASD and arrow keys, computes a normalized direction, and moves
/// the camera at the configured speed scaled by `Time.delta_secs()`.
/// Large delta times are clamped to `CameraSettings.max_delta_seconds`
/// to prevent extreme jumps during frame hitches.
///
/// This system is ordered to run after `apply_dbsp_outputs_system` to
/// ensure the camera observes the latest entity positions before panning.
///
/// # Examples
///
/// The system is added automatically by `PresentationPlugin`. To add it
/// manually with custom ordering:
///
/// ```ignore
/// app.add_systems(Update, camera_pan_system.after(apply_dbsp_outputs_system));
/// ```
#[expect(
    clippy::needless_pass_by_value,
    reason = "Bevy systems require parameters by value, not by reference."
)]
pub fn camera_pan_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    settings: Res<CameraSettings>,
    mut camera_query: Query<&mut Transform, With<CameraController>>,
) {
    // Defensive: skip if no camera exists (e.g., during initialization).
    let Ok(mut transform) = camera_query.single_mut() else {
        return;
    };

    let input = PanInput {
        up: keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp),
        down: keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown),
        left: keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft),
        right: keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight),
    };

    let direction = compute_pan_direction(input);
    if direction == Vec2::ZERO {
        return;
    }

    // Clamp delta to prevent teleporting during frame hitches.
    // Guard against non-positive max_delta_seconds to avoid zero or reversed motion.
    let clamped_max = settings.max_delta_seconds.max(f32::EPSILON);
    let delta = time.delta_secs().min(clamped_max);
    let velocity = direction * settings.pan_speed * delta;

    transform.translation.x += velocity.x;
    transform.translation.y += velocity.y;
}

/// Plugin owning camera setup and presentation layer systems.
///
/// # Responsibilities
///
/// - Spawns the main `Camera2d` with `CameraController` marker at startup.
/// - Registers `CameraController` for reflection.
/// - Hosts the `camera_pan_system` for keyboard-based camera panning.
/// - Future: Hosts zoom and Y-sorting systems.
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
        app.init_resource::<CameraSettings>();
        app.add_systems(Startup, camera_setup);
        app.add_systems(Update, camera_pan_system.after(apply_dbsp_outputs_system));
    }
}

/// Spawns the presentation camera at startup if no camera exists.
///
/// Creates a `Camera2d` entity with:
/// - `CameraController` marker for presentation layer queries
/// - `Name` component for inspector visibility
///
/// If a `Camera2d` already exists (e.g. spawned by the host application), this
/// system does nothing to avoid creating duplicate cameras.
///
/// Bevy's Required Components mechanism automatically inserts
/// `OrthographicProjection` and other camera infrastructure.
fn camera_setup(mut commands: Commands, cameras: Query<&Camera2d>) {
    if cameras.is_empty() {
        commands.spawn((Camera2d, CameraController, Name::new("PresentationCamera")));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

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

    // --- CameraSettings tests ---

    #[test]
    fn camera_settings_default_values_are_sensible() {
        let settings = CameraSettings::default();
        assert!(settings.pan_speed > 0.0, "pan_speed should be positive");
        assert!(
            settings.max_delta_seconds > 0.0,
            "max_delta_seconds should be positive"
        );
        assert!(
            settings.max_delta_seconds <= 0.5,
            "max_delta_seconds should be reasonable"
        );
    }

    #[test]
    fn camera_settings_is_clone() {
        let settings = CameraSettings::default();
        let cloned = settings.clone();
        assert_eq!(settings, cloned);
    }

    // --- compute_pan_direction tests ---

    #[rstest]
    #[case::no_keys(PanInput::default(), Vec2::ZERO)]
    #[case::up_only(PanInput { up: true, ..Default::default() }, Vec2::new(0.0, 1.0))]
    #[case::down_only(PanInput { down: true, ..Default::default() }, Vec2::new(0.0, -1.0))]
    #[case::left_only(PanInput { left: true, ..Default::default() }, Vec2::new(-1.0, 0.0))]
    #[case::right_only(PanInput { right: true, ..Default::default() }, Vec2::new(1.0, 0.0))]
    fn pan_direction_cardinal(#[case] input: PanInput, #[case] expected: Vec2) {
        let actual = compute_pan_direction(input);
        assert!(
            (actual - expected).length() < 0.001,
            "expected {expected:?}, got {actual:?}"
        );
    }

    #[rstest]
    #[case::up_right(PanInput { up: true, right: true, ..Default::default() })]
    #[case::up_left(PanInput { up: true, left: true, ..Default::default() })]
    #[case::down_right(PanInput { down: true, right: true, ..Default::default() })]
    #[case::down_left(PanInput { down: true, left: true, ..Default::default() })]
    fn pan_direction_diagonal_is_normalized(#[case] input: PanInput) {
        let dir = compute_pan_direction(input);
        assert!(
            (dir.length() - 1.0).abs() < 0.001,
            "diagonal should be normalized, got length {}",
            dir.length()
        );
    }

    #[rstest]
    #[case::up_and_down(PanInput { up: true, down: true, ..Default::default() }, Vec2::ZERO)]
    #[case::left_and_right(PanInput { left: true, right: true, ..Default::default() }, Vec2::ZERO)]
    #[case::all_keys(PanInput { up: true, down: true, left: true, right: true }, Vec2::ZERO)]
    fn pan_direction_opposing_keys_cancel(#[case] input: PanInput, #[case] expected: Vec2) {
        let dir = compute_pan_direction(input);
        assert_eq!(dir, expected, "opposing keys should cancel to zero");
    }

    #[rstest]
    #[case::up_left_and_right(PanInput { up: true, left: true, right: true, ..Default::default() })]
    #[case::down_left_and_right(PanInput { down: true, left: true, right: true, ..Default::default() })]
    fn pan_direction_partial_cancellation(#[case] input: PanInput) {
        let dir = compute_pan_direction(input);
        // When left and right cancel, only vertical movement remains.
        assert!(
            dir.x.abs() < 0.001,
            "horizontal should cancel, got x={}",
            dir.x
        );
        assert!(
            dir.y.abs() > 0.001,
            "vertical should remain, got y={}",
            dir.y
        );
    }
}
