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
//!
//! Grace-distance comparisons in the circuit treat equality as supported (`<=`),
//! and any `NaN` involved in a comparison yields `false`.

use ordered_float::OrderedFloat;

use crate::dbsp_record;

dbsp_record! {
    /// Public data type for entity positions.
    pub struct Position {
        pub entity: i64,
        pub x: OrderedFloat<f64>,
        pub y: OrderedFloat<f64>,
        pub z: OrderedFloat<f64>,
    }
}

/// Newly computed position emitted by the circuit in the current step.
pub type NewPosition = Position;

dbsp_record! {
    /// Entity velocity vector.
    pub struct Velocity {
        pub entity: i64,
        pub vx: OrderedFloat<f64>,
        pub vy: OrderedFloat<f64>,
        pub vz: OrderedFloat<f64>,
    }
}

/// Newly computed velocity emitted by the circuit in the current step.
pub type NewVelocity = Velocity;

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
#[derive(
    ::rkyv::Archive,
    ::rkyv::Serialize,
    ::rkyv::Deserialize,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    ::size_of::SizeOf,
)]
#[archive_attr(derive(Ord, PartialOrd, Eq, PartialEq, Hash))]
pub struct Force {
    pub entity: i64,
    pub fx: OrderedFloat<f64>,
    pub fy: OrderedFloat<f64>,
    pub fz: OrderedFloat<f64>,
    pub mass: Option<OrderedFloat<f64>>,
}

dbsp_record! {
    /// Discrete highest block at a grid cell.
    pub struct HighestBlockAt {
        pub x: i32,
        pub y: i32,
        pub z: i32,
    }
}

dbsp_record! {
    /// Floor height at a grid cell, accounting for slopes.
    pub struct FloorHeightAt {
        pub x: i32,
        pub y: i32,
        pub z: OrderedFloat<f64>,
    }
}

dbsp_record! {
    /// Target position for an entity.
    ///
    /// Units:
    /// - `x`, `y` are world coordinates in blocks (1.0 == one block).
    ///
    /// Invariants:
    /// - One active `Target` per `entity` per tick is expected upstream.
    pub struct Target {
        pub entity: i64,
        pub x: OrderedFloat<f64>,
        pub y: OrderedFloat<f64>,
    }
}

dbsp_record! {
    /// Fear level computed for an entity.
    ///
    /// Units:
    /// - `level` ∈ [0.0, 1.0] where higher implies greater fear.
    pub struct FearLevel {
        pub entity: i64,
        pub level: OrderedFloat<f64>,
    }
}

dbsp_record! {
    /// Decided unit movement vector for an entity.
    ///
    /// Units:
    /// - `dx`, `dy` are world-units per tick.
    ///
    /// Semantics:
    /// - The vector is normalised with a maximum magnitude of one; diagonal
    ///   movement is not faster than axis-aligned movement.
    ///
    /// Invariants:
    /// - At most one `MovementDecision` per `entity` per tick is expected
    ///   upstream.
    pub struct MovementDecision {
        pub entity: i64,
        pub dx: OrderedFloat<f64>,
        pub dy: OrderedFloat<f64>,
    }
}
