//! Game physics constants shared across systems.
//!
//! Distances are measured in block units, time in simulation ticks, and mass
//! in kilograms. Values use `f64` to align with the Database Stream Processor
//! (DBSP) circuit's numeric type and minimize rounding error.
/// Distance from the floor considered standing, in block units.
pub const GRACE_DISTANCE: f64 = 0.1;
/// Coefficient of ground friction, unitless.
pub const GROUND_FRICTION: f64 = 0.1;
/// Coefficient of air friction, unitless.
pub const AIR_FRICTION: f64 = 0.02;
/// Maximum downward speed in block units per tick.
pub const TERMINAL_VELOCITY: f64 = 12.0;
/// Downward acceleration in block units per tick squared.
pub const GRAVITY_PULL: f64 = -1.0;
/// Safe landing speed in block units per tick.
pub const SAFE_LANDING_SPEED: f64 = 6.0;
/// Damage scaling applied to speed beyond the safe landing threshold, in
/// health points per block per tick.
pub const FALL_DAMAGE_SCALE: f64 = 4.0;
/// Minimum interval between fall damage applications, in ticks. One tick equals
/// `DELTA_TIME` seconds.
pub const LANDING_COOLDOWN_TICKS: u32 = 6;
/// Simulation time step in seconds.
pub const DELTA_TIME: f64 = 1.0;
/// Default entity mass in kilograms.
pub const DEFAULT_MASS: f64 = 70.0;
/// Multiplier applied to an entity's radius when calculating fear distance.
pub const FEAR_RADIUS_MULTIPLIER: f64 = 2.0;
/// Threshold above which an entity is considered afraid, unitless.
pub const FEAR_THRESHOLD: f64 = 0.2;
/// Minimum squared distance added to fear calculations to avoid division by
/// zero when threats coincide with the actor, in block units.
pub const FEAR_DISTANCE_EPSILON: f64 = 0.001;
/// Normalised offset used to sample slopes within a block, unitless.
///
/// The `FloorHeightAt` calculation currently evaluates the slope at the
/// centre of each block because entity-specific offsets are not yet
/// available.
pub const BLOCK_CENTRE_OFFSET: f64 = 0.5;
/// Offset from a block's base to its top face, in block units.
pub const BLOCK_TOP_OFFSET: f64 = 1.0;
