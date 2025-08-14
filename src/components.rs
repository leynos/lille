//! ECS component types used by the game.
//! Includes identifiers, health, target positions, and unit descriptors shared between systems.
use bevy::prelude::*;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};

#[derive(Component, Debug, Serialize)]
pub struct DdlogId(pub i64);

#[derive(Component, Default, Serialize)]
pub struct Health(pub i32);

#[derive(Component, Debug, Clone, Serialize)]
pub enum UnitType {
    Civvy { fraidiness: f32 },
    Baddie { meanness: f32 },
}

#[derive(Component, Debug, Deref, DerefMut, Serialize)]
pub struct Target(pub Vec2);

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use size_of::SizeOf;

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
    pub id: i64,
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

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
    pub block_id: i64,
    pub grad_x: OrderedFloat<f64>,
    pub grad_y: OrderedFloat<f64>,
}
#[derive(Component, Debug, Clone, Default, Serialize)]
pub struct VelocityComp {
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
}

/// ECS component conveying an external force vector and optional mass for
/// `F = m·a`.
///
/// Units:
/// - `force_x`, `force_y`, `force_z` are forces in newtons applied for the
///   current tick.
/// - `mass` is the entity's mass in kilograms. Use `Some(m)` for a known
///   mass; use `None` to defer to the default mass.
///
/// Invariants:
/// - `mass`, when provided, must be strictly positive.
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
    #[serde(alias = "fx")]
    pub force_x: f64,
    #[serde(alias = "fy")]
    pub force_y: f64,
    #[serde(alias = "fz")]
    pub force_z: f64,
    pub mass: Option<f64>,
}
