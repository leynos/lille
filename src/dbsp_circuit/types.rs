//! Public data types used by the DBSP circuit.
//!
//! These types:
//! - Provide a stable total ordering of floating-point values via `OrderedFloat`,
//!   which DBSP requires for keys, joins, and aggregations.
//! - Support zero-copy archiving with `rkyv` for efficient interchange across circuit
//!   boundaries and test fixtures.
//! - Derive `SizeOf` to aid memory accounting.
//!
//! Avoid introducing `NaN` values into these types. While `OrderedFloat` defines a
//! total order that includes `NaN`, the resulting ordering can be surprising.
use ordered_float::OrderedFloat;
use rkyv::{Archive, Deserialize, Serialize};
use size_of::SizeOf;

#[derive(
    Archive,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    SizeOf,
)]
#[archive_attr(derive(Ord, PartialOrd, Eq, PartialEq, Hash))]
pub struct Position {
    pub entity: i64,
    pub x: OrderedFloat<f64>,
    pub y: OrderedFloat<f64>,
    pub z: OrderedFloat<f64>,
}

/// Newly computed position emitted by the circuit in the current step.
pub type NewPosition = Position;

#[derive(
    Archive,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    SizeOf,
)]
#[archive_attr(derive(Ord, PartialOrd, Eq, PartialEq, Hash))]
pub struct Velocity {
    pub entity: i64,
    pub vx: OrderedFloat<f64>,
    pub vy: OrderedFloat<f64>,
    pub vz: OrderedFloat<f64>,
}

/// Newly computed velocity emitted by the circuit in the current step.
pub type NewVelocity = Velocity;

#[derive(
    Archive,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    SizeOf,
)]
#[archive_attr(derive(Ord, PartialOrd, Eq, PartialEq, Hash))]
/// Force applied to an entity.
///
/// Units:
/// - `fx`, `fy`, `fz` are Newtons (N).
/// - `mass` is kilograms (kg). When `mass` is `None`, a default mass is used downstream.
/// - When `mass` is present but non-positive, the force is ignored.
///
/// # Examples
/// ```rust,no_run
/// # use lille::prelude::*;
/// use ordered_float::OrderedFloat;
/// let f = Force {
///     entity: 42,
///     fx: OrderedFloat(5.0),
///     fy: OrderedFloat(0.0),
///     fz: OrderedFloat(0.0),
///     mass: Some(OrderedFloat(5.0)),
/// };
/// assert_eq!(f.entity, 42);
/// ```
pub struct Force {
    pub entity: i64,
    pub fx: OrderedFloat<f64>,
    pub fy: OrderedFloat<f64>,
    pub fz: OrderedFloat<f64>,
    pub mass: Option<OrderedFloat<f64>>,
}

#[derive(
    Archive,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    SizeOf,
)]
#[archive_attr(derive(Ord, PartialOrd, Eq, PartialEq, Hash))]
pub struct HighestBlockAt {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(
    Archive,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    SizeOf,
)]
#[archive_attr(derive(Ord, PartialOrd, Eq, PartialEq, Hash))]
pub struct FloorHeightAt {
    pub x: i32,
    pub y: i32,
    pub z: OrderedFloat<f64>,
}
