//! DBSP dataflow stream construction for spatial simulation.
//!
//! This module defines helper functions for building the dataflow streams used
//! by `DbspCircuit` to process the game world. These streams implement:
//!
//! - Block aggregation to track the highest block at each grid cell
//! - Floor height calculation with optional slopes
//! - Velocity updates applying gravity
//! - Position integration based on velocities
//! - Joining positions with floor height for collision queries
//!
//! Each function returns a new [`Stream`] that can be composed into the overall
//! circuit.

use dbsp::{operator::Max, typed_batch::OrdZSet, RootCircuit, Stream};
use ordered_float::OrderedFloat;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use size_of::SizeOf;

use crate::components::{Block, BlockSlope};
use crate::{
    applied_acceleration, apply_ground_friction, BLOCK_CENTRE_OFFSET, BLOCK_TOP_OFFSET,
    FEAR_THRESHOLD, GRAVITY_PULL, TERMINAL_VELOCITY,
};

use super::{
    FearLevel, FloorHeightAt, Force, HighestBlockAt, MovementDecision, Position, Target, Velocity,
};

/// Clamps a vertical velocity to the configured terminal speed.
fn clamp_terminal_velocity(vz: f64) -> OrderedFloat<f64> {
    // Prevent unbounded acceleration by enforcing a maximum fall speed.
    OrderedFloat(vz.max(-TERMINAL_VELOCITY))
}

/// Returns a stream pairing each grid cell with its highest block and id.
///
/// The function aggregates incoming [`Block`] records by `(x, y)` to find the
/// maximum `z` value at each coordinate. The output preserves the originating
/// block id so that subsequent joins can access slope information.
///
/// # Examples
///
/// ```rust,no_run
/// # fn main() -> Result<(), dbsp::Error> {
/// # use lille::prelude::*;
/// # use dbsp::{RootCircuit, typed_batch::OrdZSet};
/// # use lille::dbsp_circuit::highest_block_pair;
/// RootCircuit::build(|circuit| {
///     let (stream, _handle) = circuit.add_input_zset::<Block>();
///     let _highest = lille::dbsp_circuit::highest_block_pair(&stream);
///     Ok(())
/// })?;
/// # Ok::<(), dbsp::Error>(())
/// # }
/// ```
pub fn highest_block_pair(
    blocks: &Stream<RootCircuit, OrdZSet<Block>>,
) -> Stream<RootCircuit, OrdZSet<(HighestBlockAt, i64)>> {
    blocks
        .map_index(|b| ((b.x, b.y), (b.z, b.id)))
        .aggregate(Max)
        .map(|((x, y), (z, id))| {
            (
                HighestBlockAt {
                    x: *x,
                    y: *y,
                    z: *z,
                },
                *id,
            )
        })
}

/// Derives the floor height for each block, optionally applying slopes.
///
/// The stream joins the highest block id at a grid cell with any matching
/// [`BlockSlope`] record. When slope data is present the returned
/// [`FloorHeightAt`] accounts for the block's gradient, producing a smooth
/// surface. Missing slope data falls back to a flat top.
pub(super) fn floor_height_stream(
    highest_pair: &Stream<RootCircuit, OrdZSet<(HighestBlockAt, i64)>>,
    slopes: &Stream<RootCircuit, OrdZSet<BlockSlope>>,
) -> Stream<RootCircuit, OrdZSet<FloorHeightAt>> {
    highest_pair
        .map_index(|(hb, id)| (*id, (hb.x, hb.y, hb.z)))
        .outer_join(
            &slopes.map_index(|bs| (bs.block_id, (bs.grad_x, bs.grad_y))),
            |_, &(x, y, z), &(gx, gy)| {
                let base = z as f64 + BLOCK_TOP_OFFSET;
                let gradient = BLOCK_CENTRE_OFFSET * (gx.into_inner() + gy.into_inner());
                Some(FloorHeightAt {
                    x,
                    y,
                    z: OrderedFloat(base + gradient),
                })
            },
            |_, &(x, y, z)| {
                Some(FloorHeightAt {
                    x,
                    y,
                    z: OrderedFloat(z as f64 + BLOCK_TOP_OFFSET),
                })
            },
            |_, _| None,
        )
        // Convert `Option<FloorHeightAt>` from the outer join, discarding
        // unmatched slope records.
        .flat_map(|fh| (*fh).into_iter())
}

/// Applies gravity and a single external force to each velocity record
/// (dt = 1).
///
/// Each entity may supply at most one [`Force`] record per tick. Forces with
/// invalid masses are ignored with a log warning.
pub(super) fn new_velocity_stream(
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
                    force.mass.map(|m| m.into_inner()),
                );
                if accel.is_none() {
                    log::warn!(
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
pub(super) fn new_position_stream(
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
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    size_of::SizeOf,
)]
#[archive_attr(derive(Ord, PartialOrd, Eq, PartialEq, Hash))]
/// Pairs an entity's position with the floor height at its grid location.
///
/// This struct is the result of joining the continuous [`Position`] stream with
/// the discrete [`FloorHeightAt`] grid. It is primarily consumed by higher-level
/// physics logic for tasks such as collision detection or standing vs. falling
/// checks.
///
/// * `position` - The entity's current position in world coordinates
/// * `z_floor` - The computed floor height at the position's grid cell
pub struct PositionFloor {
    pub position: Position,
    pub z_floor: OrderedFloat<f64>,
}

/// Joins each `Position` with the corresponding floor height.
///
/// Positions are discretised to grid coordinates by flooring their `x` and `y`
/// values. Those indices look up a [`FloorHeightAt`] record to produce a
/// [`PositionFloor`] stream suitable for higher-level physics logic.
pub(super) fn position_floor_stream(
    positions: &Stream<RootCircuit, OrdZSet<Position>>,
    floor_height: &Stream<RootCircuit, OrdZSet<FloorHeightAt>>,
) -> Stream<RootCircuit, OrdZSet<PositionFloor>> {
    positions
        .map_index(|p| {
            (
                (
                    p.x.into_inner().floor() as i32,
                    p.y.into_inner().floor() as i32,
                ),
                *p,
            )
        })
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
/// # Parameters
///
/// - `standing` - Entities currently deemed to be on the floor with their
///   positions.
/// - `floor_height` - Discrete floor heights indexed by grid coordinates.
/// - `velocities` - Current velocity vectors for each entity.
///
/// # Returns
///
/// A tuple containing the updated position and velocity streams for all
/// standing entities.
///
/// # Examples
///
/// ```ignore
/// # use dbsp::{typed_batch::OrdZSet, RootCircuit};
/// # use lille::dbsp_circuit::{standing_motion_stream, PositionFloor, FloorHeightAt, Velocity};
/// RootCircuit::build(|circuit| {
///     let (standing, _sh) = circuit.add_input_zset::<PositionFloor>();
///     let (floor, _fh) = circuit.add_input_zset::<FloorHeightAt>();
///     let (vel, _vh) = circuit.add_input_zset::<Velocity>();
///     let (_pos, _vel) = standing_motion_stream(&standing, &floor, &vel);
///     Ok(())
/// })?;
/// # Ok::<(), dbsp::Error>(())
/// ```
pub(super) fn standing_motion_stream(
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
            (x.into_inner().floor() as i32, y.into_inner().floor() as i32),
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

/// Produces a zero fear level for each entity.
///
/// This is a placeholder pending a full AI implementation.
pub(super) fn fear_level_stream(
    positions: &Stream<RootCircuit, OrdZSet<Position>>,
) -> Stream<RootCircuit, OrdZSet<FearLevel>> {
    positions.map(|p| FearLevel {
        entity: p.entity,
        level: OrderedFloat(0.0),
    })
}

/// Intermediate structure pairing entity positions with their target.
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
struct PositionTarget {
    entity: i64,
    px: OrderedFloat<f64>,
    py: OrderedFloat<f64>,
    tx: OrderedFloat<f64>,
    ty: OrderedFloat<f64>,
}

fn decide_movement(level: OrderedFloat<f64>, pt: &PositionTarget) -> MovementDecision {
    let (dx, dy) = if level.into_inner() <= FEAR_THRESHOLD {
        (
            (pt.tx.into_inner() - pt.px.into_inner()).signum().into(),
            (pt.ty.into_inner() - pt.py.into_inner()).signum().into(),
        )
    } else {
        (0.0.into(), 0.0.into())
    };
    MovementDecision {
        entity: pt.entity,
        dx,
        dy,
    }
}

/// Converts fear levels and targets into simple movement decisions.
///
/// Entities with a target move one unit towards it when their fear is below
/// [`FEAR_THRESHOLD`]. Higher fear currently results in no movement; fleeing is
/// not yet implemented.
pub(super) fn movement_decision_stream(
    fear: &Stream<RootCircuit, OrdZSet<FearLevel>>,
    targets: &Stream<RootCircuit, OrdZSet<Target>>,
    positions: &Stream<RootCircuit, OrdZSet<Position>>,
) -> Stream<RootCircuit, OrdZSet<MovementDecision>> {
    let pos_target = positions
        .map_index(|p| (p.entity, *p))
        .join(&targets.map_index(|t| (t.entity, *t)), |_entity, p, t| {
            PositionTarget {
                entity: p.entity,
                px: p.x,
                py: p.y,
                tx: t.x,
                ty: t.y,
            }
        })
        .map_index(|pt| (pt.entity, pt.clone()));

    fear.map_index(|f| (f.entity, f.level))
        .join(&pos_target, |_entity, &level, pt| {
            decide_movement(level, pt)
        })
}

/// Applies movement decisions to base positions.
pub(super) fn apply_movement(
    base: &Stream<RootCircuit, OrdZSet<Position>>,
    movement: &Stream<RootCircuit, OrdZSet<MovementDecision>>,
) -> Stream<RootCircuit, OrdZSet<Position>> {
    let base_idx = base.map_index(|p| (p.entity, *p));
    let mv = movement.map_index(|m| (m.entity, (m.dx, m.dy)));

    let moved = base_idx.join(&mv, |_, p, &(dx, dy)| Position {
        entity: p.entity,
        x: OrderedFloat(p.x.into_inner() + dx.into_inner()),
        y: OrderedFloat(p.y.into_inner() + dy.into_inner()),
        z: p.z,
    });

    let unmoved = base_idx.antijoin(&mv).map(|(_, p)| *p);

    moved.plus(&unmoved)
}
