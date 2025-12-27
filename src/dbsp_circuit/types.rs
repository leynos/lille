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

/// Stable identifier shared between Bevy and the DBSP circuit.
pub type EntityId = u64;
/// Authoritative simulation tick counter propagated with health deltas.
pub type Tick = u64;

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
/// Classification of damage event origins.
#[non_exhaustive]
pub enum DamageSource {
    /// Damage originating from gameplay systems outside the circuit.
    #[default]
    External,
    /// Fall damage inferred by the physics pipeline.
    Fall,
    /// Script-driven healing or scripted damage applied upstream.
    Script,
    /// Placeholder for bespoke downstream discriminators.
    Other {
        /// User-defined discriminator.
        code: u16,
    },
}

crate::dbsp_copy_record! {
    /// Snapshot of an entity's health at the start of a tick.
    pub struct HealthState {
        /// Entity identifier of the snapshot subject.
        pub entity: EntityId,
        /// Hit points recorded at tick ingress.
        pub current: u16,
        /// Maximum permissible hit points for the entity.
        pub max: u16,
    }
}

crate::dbsp_copy_record! {
    /// Damage or healing event entering the circuit.
    pub struct DamageEvent {
        /// Entity receiving the damage or healing.
        pub entity: EntityId,
        /// Magnitude of the delta in hit points.
        pub amount: u16,
        /// Origin of the change, used for deduplication and analytics.
        pub source: DamageSource,
        /// Tick at which the event occurred.
        pub at_tick: Tick,
        /// Optional sequence identifier for ordering across replicas.
        pub seq: Option<u32>,
    }
}

crate::dbsp_copy_record! {
    /// Authoritative health delta emitted by the circuit for a tick.
    pub struct HealthDelta {
        /// Entity receiving the aggregated delta.
        pub entity: EntityId,
        /// Tick at which the delta applies.
        pub at_tick: Tick,
        /// Highest sequence number encountered amongst contributing events.
        pub seq: Option<u32>,
        /// Net change to apply to the entity's health.
        pub delta: i32,
        /// Whether the entity transitioned to zero health during this tick.
        pub death: bool,
    }
}

crate::dbsp_copy_record! {
    /// Public data type for entity positions.
    pub struct Position {
        /// Entity associated with the position.
        pub entity: i64,
        /// World-space X coordinate in blocks.
        pub x: OrderedFloat<f64>,
        /// World-space Y coordinate in blocks.
        pub y: OrderedFloat<f64>,
        /// World-space Z coordinate in blocks.
        pub z: OrderedFloat<f64>,
    }
}

/// Newly computed position emitted by the circuit in the current step.
pub type NewPosition = Position;

crate::dbsp_copy_record! {
    /// Entity velocity vector.
    pub struct Velocity {
        /// Entity whose velocity is measured.
        pub entity: i64,
        /// Velocity along the X axis, blocks per tick.
        pub vx: OrderedFloat<f64>,
        /// Velocity along the Y axis, blocks per tick.
        pub vy: OrderedFloat<f64>,
        /// Velocity along the Z axis, blocks per tick.
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
///
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
    /// Entity receiving the impulse.
    pub entity: i64,
    /// Applied force along the X axis.
    pub fx: OrderedFloat<f64>,
    /// Applied force along the Y axis.
    pub fy: OrderedFloat<f64>,
    /// Applied force along the Z axis.
    pub fz: OrderedFloat<f64>,
    /// Optional explicit mass overriding the global default in kilograms.
    pub mass: Option<OrderedFloat<f64>>,
}

crate::dbsp_copy_record! {
    /// Discrete highest block at a grid cell.
    pub struct HighestBlockAt {
        /// Grid X coordinate of the sample.
        pub x: i32,
        /// Grid Y coordinate of the sample.
        pub y: i32,
        /// Highest block height at the cell.
        pub z: i32,
    }
}

crate::dbsp_copy_record! {
    /// Floor height at a grid cell, accounting for slopes.
    pub struct FloorHeightAt {
        /// Grid X coordinate of the evaluated floor.
        pub x: i32,
        /// Grid Y coordinate of the evaluated floor.
        pub y: i32,
        /// Floor height including slope adjustments.
        pub z: OrderedFloat<f64>,
    }
}

crate::dbsp_copy_record! {
    /// Target position for an entity.
    ///
    /// Units:
    /// - `x`, `y` are world coordinates in blocks (1.0 == one block).
    ///
    /// Invariants:
    /// - One active `Target` per `entity` per tick is expected upstream.
    pub struct Target {
        /// Entity identifier to steer.
        pub entity: i64,
        /// Desired X coordinate for the entity.
        pub x: OrderedFloat<f64>,
        /// Desired Y coordinate for the entity.
        pub y: OrderedFloat<f64>,
    }
}

// `FearLevel` must remain non-`Copy` to avoid implicit duplication and to
// permit future non-`Copy` fields. A compile-time test asserts this type never
// gains `Copy` accidentally.
crate::dbsp_record! {
    /// Fear level computed for an entity.
    ///
    /// This type intentionally omits `Copy`; clone it explicitly when duplication
    /// is required.
    ///
    /// # Examples
    /// ```rust
    /// use ordered_float::OrderedFloat;
    ///
    /// use lille::dbsp_circuit::FearLevel;
    ///
    /// let fear = FearLevel { entity: 1, level: OrderedFloat(0.5) };
    /// let clone = fear.clone();
    /// assert_eq!(clone.level, OrderedFloat(0.5));
    /// ```
    ///
    /// Units:
    /// - `level` ∈ [0.0, 1.0] where higher implies greater fear.
    pub struct FearLevel {
        /// Entity whose fear is being measured.
        pub entity: i64,
        /// Fear intensity scaled between zero and one.
        pub level: OrderedFloat<f64>,
    }
}

crate::dbsp_copy_record! {
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
        /// Entity to move.
        pub entity: i64,
        /// Unit vector X component of the intended movement.
        pub dx: OrderedFloat<f64>,
        /// Unit vector Y component of the intended movement.
        pub dy: OrderedFloat<f64>,
    }
}

crate::dbsp_copy_record! {
    /// Player spawn location for the DBSP circuit.
    ///
    /// Contains world-space coordinates of the spawn point derived from the
    /// entity's `Transform` component. The circuit can use this to query floor
    /// heights at spawn locations or to trigger spawn logic.
    ///
    /// # Examples
    /// ```rust
    /// use ordered_float::OrderedFloat;
    ///
    /// use lille::dbsp_circuit::PlayerSpawnLocation;
    ///
    /// let spawn = PlayerSpawnLocation {
    ///     id: 42,
    ///     x: OrderedFloat(10.5),
    ///     y: OrderedFloat(20.5),
    ///     z: OrderedFloat(0.0),
    /// };
    /// assert_eq!(spawn.id, 42);
    /// ```
    pub struct PlayerSpawnLocation {
        /// Unique identifier derived from Bevy Entity bits.
        pub id: i64,
        /// World-space X coordinate.
        pub x: OrderedFloat<f64>,
        /// World-space Y coordinate.
        pub y: OrderedFloat<f64>,
        /// World-space Z coordinate.
        pub z: OrderedFloat<f64>,
    }
}

crate::dbsp_copy_record! {
    /// NPC spawn point for the DBSP circuit.
    ///
    /// Contains world-space coordinates and spawn configuration. The circuit
    /// uses this to query floor heights at spawn locations and to provide spawn
    /// metadata for entity creation systems.
    ///
    /// # Examples
    /// ```rust
    /// use ordered_float::OrderedFloat;
    ///
    /// use lille::dbsp_circuit::SpawnPointRecord;
    ///
    /// let spawn = SpawnPointRecord {
    ///     id: 1,
    ///     x: OrderedFloat(5.0),
    ///     y: OrderedFloat(10.0),
    ///     z: OrderedFloat(0.0),
    ///     enemy_type: 3,
    ///     respawn: true,
    /// };
    /// assert!(spawn.respawn);
    /// assert_eq!(spawn.enemy_type, 3);
    /// ```
    pub struct SpawnPointRecord {
        /// Unique identifier derived from Bevy Entity bits.
        pub id: i64,
        /// World-space X coordinate.
        pub x: OrderedFloat<f64>,
        /// World-space Y coordinate.
        pub y: OrderedFloat<f64>,
        /// World-space Z coordinate.
        pub z: OrderedFloat<f64>,
        /// Enemy type identifier for archetype lookup.
        pub enemy_type: u32,
        /// Whether this spawn point respawns after use.
        pub respawn: bool,
    }
}
