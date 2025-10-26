//! ECS component types used by the game.
//! Includes identifiers, health, target positions, and unit descriptors shared between systems.
use bevy::prelude::*;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};

/// Stable identifier that mirrors the DBSP circuit's `DdlogId`.
///
/// Attach this component to Bevy entities that participate in the DBSP data
/// pipeline so that records can be joined across the engine/runtime boundary.
/// The wrapped integer is unique within the simulation.
#[derive(Component, Debug, Serialize)]
pub struct DdlogId(pub i64);

/// Hit point state mirrored between Bevy and the DBSP circuit.
///
/// `Health` is updated when the circuit emits aggregated deltas and is written
/// back to the ECS so gameplay systems observe the canonical totals.
///
/// # Fields
/// - `current`: Remaining hit points for the entity.
/// - `max`: Maximum hit points the entity can have.
///
/// # Examples
/// ```
/// use lille::components::Health;
/// let mut hp = Health { current: 45, max: 60 };
/// hp.current = hp.current.saturating_sub(10);
/// assert_eq!(hp.current, 35);
/// ```
#[derive(Component, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Health {
    /// Remaining hit points that drive gameplay reactions.
    pub current: u16,
    /// Ceiling used to clamp healing effects.
    pub max: u16,
}

/// Marker describing the behavioural archetype of an entity.
///
/// This determines AI defaults such as aggression and fear modulation.
///
/// # Examples
/// ```
/// use lille::components::UnitType;
/// let civilian = UnitType::Civvy { fraidiness: 0.8 };
/// match civilian {
///     UnitType::Civvy { fraidiness } => assert!(fraidiness > 0.0),
///     UnitType::Baddie { .. } => unreachable!(),
/// }
/// ```
#[derive(Component, Debug, Clone, Serialize)]
pub enum UnitType {
    /// Passive civilian unit parameterised by its fearfulness.
    Civvy {
        /// Likelihood of fleeing when danger is nearby.
        fraidiness: f32,
    },
    /// Hostile enemy unit parameterised by its aggression.
    Baddie {
        /// Aggression weighting used by combat behaviours.
        meanness: f32,
    },
}

/// Target location chosen by tactical AI.
///
/// Stores the desired destination as a 2D vector in world space. Systems feed
/// this into the DBSP circuit where it influences movement decisions.
///
/// # Examples
/// ```
/// use bevy::math::Vec2;
/// use lille::components::Target;
/// let target = Target(Vec2::new(4.0, -2.0));
/// assert_eq!(target.0.x, 4.0);
/// assert_eq!(target.0.y, -2.0);
/// ```
#[derive(Component, Debug, Deref, DerefMut, Serialize)]
pub struct Target(pub Vec2);

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use size_of::SizeOf;

/// World block sampled by DBSP terrain queries.
///
/// Each record represents a single terrain cube located at integer coordinates.
/// DBSP streams aggregate these blocks to derive highest-platform queries and
/// gradients that inform movement.
#[derive(
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
    Component,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Default,
    SizeOf,
)]
#[archive_attr(derive(Ord, PartialOrd, Eq, PartialEq, Hash))]
pub struct Block {
    /// Stable identifier emitted by the terrain importer.
    pub id: i64,
    /// Block position along the X axis in world units.
    pub x: i32,
    /// Block position along the Y axis in world units.
    pub y: i32,
    /// Block position along the Z axis in world units.
    pub z: i32,
}

/// Gradient metadata for sloped terrain.
///
/// DBSP uses these slopes to blend adjacent heights when approximating the
/// floor under a unit, enabling smooth stepped terrain.
#[derive(
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
    Component,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Default,
    SizeOf,
)]
#[archive_attr(derive(Ord, PartialOrd, Eq, PartialEq, Hash))]
pub struct BlockSlope {
    /// Identifier of the block that owns this slope information.
    pub block_id: i64,
    /// Partial derivative of the block surface along the X axis.
    pub grad_x: OrderedFloat<f64>,
    /// Partial derivative of the block surface along the Y axis.
    pub grad_y: OrderedFloat<f64>,
}
/// Linear velocity measured in metres per second.
///
/// Updated each tick from DBSP outputs and consumed by rendering and physics
/// systems that need directional velocity.
#[derive(Component, Debug, Clone, Default, Serialize)]
pub struct VelocityComp {
    /// X component of the velocity vector.
    pub vx: f32,
    /// Y component of the velocity vector.
    pub vy: f32,
    /// Z component of the velocity vector.
    pub vz: f32,
}

/// ECS component conveying an external force vector and optional mass for
/// `F = m·a`. Forces apply for a single tick and are cleared after the DBSP
/// circuit runs.
///
/// Units:
/// - `force_x`, `force_y`, `force_z` are forces in newtons (N) applied for the
///   current tick and cleared after each circuit step.
/// - `mass` is the entity's mass in kilograms (kg). Use `Some(m)` for a known
///   mass; use `None` to defer to the engine-defined default mass.
///
/// Invariants:
/// - `mass`, when provided, must be strictly positive; non-positive values are
///   ignored by the physics pipeline.
/// - Force components should be zero when no external force applies.
///
/// # Examples
///
/// Apply a 10 N upward force with an explicit 2 kg mass:
/// ```
/// use lille::components::ForceComp;
/// let f = ForceComp { force_x: 0.0, force_y: 0.0, force_z: 10.0, mass: Some(2.0) };
/// assert_eq!(f.mass, Some(2.0));
/// ```
#[derive(Component, Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ForceComp {
    /// Force along the X axis applied for the upcoming tick.
    #[serde(alias = "fx")]
    pub force_x: f64,
    /// Force along the Y axis applied for the upcoming tick.
    #[serde(alias = "fy")]
    pub force_y: f64,
    /// Force along the Z axis applied for the upcoming tick.
    #[serde(alias = "fz")]
    pub force_z: f64,
    /// Optional explicit mass overriding the global default.
    pub mass: Option<f64>,
}
