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
/// Minimum squared distance added to fear calculations to avoid division by
/// zero when threats coincide with the actor.
pub const FEAR_DISTANCE_EPSILON: f64 = 0.001;
/// The normalised offset used to sample slopes within a block.
///
/// The `FloorHeightAt` calculation currently evaluates the slope at the
/// centre of each block because entity-specific offsets are not yet
/// available.
pub const BLOCK_CENTRE_OFFSET: f64 = 0.5;
/// Offset from a block's base to its top face.
pub const BLOCK_TOP_OFFSET: f64 = 1.0;
