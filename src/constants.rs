/// Game physics constants used across systems.
///
/// These values were previously loaded from `constants.toml` at build time but
/// are now hardcoded for simplicity.
pub const GRACE_DISTANCE: f64 = 0.1;
pub const GROUND_FRICTION: f64 = 0.1;
pub const AIR_FRICTION: f64 = 0.02;
pub const TERMINAL_VELOCITY: f64 = 2.0;
pub const GRAVITY_PULL: f64 = -1.0;
pub const DELTA_TIME: f64 = 1.0;
pub const DEFAULT_MASS: f64 = 70.0;
pub const FEAR_RADIUS_MULTIPLIER: f64 = 2.0;
pub const FEAR_THRESHOLD: f64 = 0.2;
