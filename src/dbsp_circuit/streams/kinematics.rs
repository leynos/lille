//! Velocity integration and ground interaction streams.
//!
//! These functions update entity velocities based on forces, integrate new
//! positions, and handle motion relative to the floor surface.

use dbsp::{typed_batch::OrdZSet, RootCircuit, Stream};
use log::warn;
use ordered_float::OrderedFloat;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use size_of::SizeOf;

use crate::{applied_acceleration, apply_ground_friction, GRAVITY_PULL, TERMINAL_VELOCITY};

use crate::dbsp_circuit::{FloorHeightAt, Force, Position, Velocity};

/// Clamps a vertical velocity to the configured terminal speed.
fn clamp_terminal_velocity(vz: f64) -> OrderedFloat<f64> {
    // Prevent unbounded acceleration by enforcing a maximum fall speed.
    OrderedFloat(vz.max(-TERMINAL_VELOCITY))
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "Value is clamped into the `i32` domain before conversion."
)]
fn floor_to_i32(value: OrderedFloat<f64>) -> i32 {
    let floored = value.into_inner().floor();
    let clamped = floored.clamp(f64::from(i32::MIN), f64::from(i32::MAX));
    clamped as i32
}

/// Applies gravity and a single external force to each velocity record (dt = 1).
///
/// Each entity may supply at most one [`Force`] record per tick. Forces with
/// invalid masses are ignored with a log warning.
#[must_use]
pub fn new_velocity_stream(
    velocities: &Stream<RootCircuit, OrdZSet<Velocity>>,
    forces: &Stream<RootCircuit, OrdZSet<Force>>,
) -> Stream<RootCircuit, OrdZSet<Velocity>> {
    velocities
        .map_index(|v| (v.entity, *v))
        .outer_join(
            &forces.map_index(|f| (f.entity, *f)),
            |_, vel, force| {
                let accel = applied_acceleration(
                    (
                        force.fx.into_inner(),
                        force.fy.into_inner(),
                        force.fz.into_inner(),
                    ),
                    force.mass.map(OrderedFloat::into_inner),
                );
                if accel.is_none() {
                    warn!(
                        "force with invalid mass for entity {} ignored",
                        force.entity
                    );
                }
                let (ax, ay, az) = accel.unwrap_or((0.0, 0.0, 0.0));
                Some(Velocity {
                    entity: vel.entity,
                    vx: OrderedFloat(vel.vx.into_inner() + ax),
                    vy: OrderedFloat(vel.vy.into_inner() + ay),
                    vz: clamp_terminal_velocity(vel.vz.into_inner() + az + GRAVITY_PULL),
                })
            },
            |_, vel| {
                Some(Velocity {
                    entity: vel.entity,
                    vx: vel.vx,
                    vy: vel.vy,
                    vz: clamp_terminal_velocity(vel.vz.into_inner() + GRAVITY_PULL),
                })
            },
            |_, _| None,
        )
        .flat_map(|v| *v)
}

/// Integrates positions with updated velocities.
///
/// The input streams are joined by `entity`, producing a new [`Position`]
/// translated by the entity's velocity components. The function performs a
/// simple Euler integration suitable for the small time step used in the game
/// loop.
#[must_use]
pub fn new_position_stream(
    positions: &Stream<RootCircuit, OrdZSet<Position>>,
    new_vel: &Stream<RootCircuit, OrdZSet<Velocity>>,
) -> Stream<RootCircuit, OrdZSet<Position>> {
    positions.map_index(|p| (p.entity, *p)).join(
        &new_vel.map_index(|v| (v.entity, *v)),
        |_, p, v| Position {
            entity: p.entity,
            x: OrderedFloat(p.x.into_inner() + v.vx.into_inner()),
            y: OrderedFloat(p.y.into_inner() + v.vy.into_inner()),
            z: OrderedFloat(p.z.into_inner() + v.vz.into_inner()),
        },
    )
}

#[derive(
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
    Clone,
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
/// Pairs an entity's position with the floor height at its grid location.
///
/// This struct is the result of joining the continuous [`Position`] stream with
/// the discrete [`FloorHeightAt`] grid. It is primarily consumed by
/// higher-level physics logic for tasks such as collision detection or standing
/// checks.
pub struct PositionFloor {
    /// Continuous position of the entity.
    pub position: Position,
    /// Floor height beneath the entity at the sampled grid cell.
    pub z_floor: OrderedFloat<f64>,
}

/// Joins each `Position` with the corresponding floor height.
///
/// Positions are discretised to grid coordinates by flooring their `x` and `y`
/// values. Those indices look up a [`FloorHeightAt`] record to produce a
/// [`PositionFloor`] stream suitable for higher-level physics logic.
#[must_use]
pub fn position_floor_stream(
    positions: &Stream<RootCircuit, OrdZSet<Position>>,
    floor_height: &Stream<RootCircuit, OrdZSet<FloorHeightAt>>,
) -> Stream<RootCircuit, OrdZSet<PositionFloor>> {
    positions
        .map_index(|p| ((floor_to_i32(p.x), floor_to_i32(p.y)), *p))
        .join(
            &floor_height.map_index(|fh| ((fh.x, fh.y), fh.z)),
            |_idx, pos, &z_floor| PositionFloor {
                position: *pos,
                z_floor,
            },
        )
}

/// Computes new positions and velocities for entities standing on the ground.
///
/// Standing entities move according to their horizontal velocity components and
/// snap to the floor height at their new `(x, y)` coordinates. The vertical
/// velocity is reset to zero to keep entities grounded.
///
/// # Returns
///
/// A tuple containing the updated position and velocity streams for all
/// standing entities.
#[must_use]
pub fn standing_motion_stream(
    standing: &Stream<RootCircuit, OrdZSet<PositionFloor>>,
    floor_height: &Stream<RootCircuit, OrdZSet<FloorHeightAt>>,
    velocities: &Stream<RootCircuit, OrdZSet<Velocity>>,
) -> (
    Stream<RootCircuit, OrdZSet<Position>>,
    Stream<RootCircuit, OrdZSet<Velocity>>,
) {
    // `dbsp` 0.98 lacks a `join_map` combinator. We explicitly compose
    // `map_index` and `join` operators to achieve equivalent behaviour.
    let moved = standing
        .map_index(|pf| (pf.position.entity, pf.position))
        .join(&velocities.map_index(|v| (v.entity, *v)), |_, pos, vel| {
            let fx = apply_ground_friction(vel.vx.into_inner());
            let fy = apply_ground_friction(vel.vy.into_inner());
            let new_x = OrderedFloat(pos.x.into_inner() + fx);
            let new_y = OrderedFloat(pos.y.into_inner() + fy);
            (new_x, new_y, pos.entity, OrderedFloat(fx), OrderedFloat(fy))
        });

    let indexed = moved.map_index(|(x, y, entity, vx, vy)| {
        (
            (floor_to_i32(*x), floor_to_i32(*y)),
            (*entity, *x, *y, *vx, *vy),
        )
    });

    let with_floor = indexed.join(
        &floor_height.map_index(|fh| ((fh.x, fh.y), fh.z)),
        |_idx, &(entity, x, y, vx, vy), &z_floor| {
            (
                Position {
                    entity,
                    x,
                    y,
                    z: z_floor,
                },
                Velocity {
                    entity,
                    vx,
                    vy,
                    vz: OrderedFloat(0.0),
                },
            )
        },
    );

    let new_pos = with_floor.map(|(p, _)| *p);
    let new_vel = with_floor.map(|(_, v)| *v);
    (new_pos, new_vel)
}

#[cfg(test)]
mod tests;
