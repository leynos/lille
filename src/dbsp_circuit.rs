//! DBSP-based world inference engine.
//!
//! This module defines [`DbspCircuit`], the authoritative dataflow program for
//! Lille's game world. Callers feed [`Position`] and [`Block`] records into the
//! circuit, call [`DbspCircuit::step`], then read [`NewPosition`] and
//! [`HighestBlockAt`] outputs. Input collections persist across steps—invoke
//! [`DbspCircuit::clear_inputs`] after each frame to prevent stale data from
//! affecting subsequent computations.
//!
//! # Doctest prelude
//!
//! Examples in this module use the following imports:
//!
//! ```rust,no_run
//! # use lille::prelude::*;
//! ```

use dbsp::{
    operator::Max, typed_batch::OrdZSet, CircuitHandle, OutputHandle, RootCircuit, ZSetHandle,
};
use ordered_float::OrderedFloat;
use size_of::SizeOf;

use crate::components::{Block, BlockSlope};
use crate::{BLOCK_CENTRE_OFFSET, GRAVITY_PULL};

use rkyv::{Archive, Deserialize, Serialize};

#[derive(
    Archive,
    Serialize,
    Deserialize,
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
pub struct Position {
    pub entity: i64,
    pub x: OrderedFloat<f64>,
    pub y: OrderedFloat<f64>,
    pub z: OrderedFloat<f64>,
}

pub type NewPosition = Position;

#[derive(
    Archive,
    Serialize,
    Deserialize,
    Clone,
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

pub type NewVelocity = Velocity;

#[derive(
    Archive,
    Serialize,
    Deserialize,
    Clone,
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

pub struct DbspCircuit {
    circuit: CircuitHandle,
    position_in: ZSetHandle<Position>,
    velocity_in: ZSetHandle<Velocity>,
    block_in: ZSetHandle<Block>,
    block_slope_in: ZSetHandle<BlockSlope>,
    new_position_out: OutputHandle<OrdZSet<NewPosition>>,
    new_velocity_out: OutputHandle<OrdZSet<NewVelocity>>,
    highest_block_out: OutputHandle<OrdZSet<HighestBlockAt>>,
    floor_height_out: OutputHandle<OrdZSet<FloorHeightAt>>,
}

impl DbspCircuit {
    /// Constructs a new `DbspCircuit` for simulating game world physics and environment state.
    ///
    /// Sets up a DBSP dataflow circuit with input handles for entity positions, velocities, and blocks.
    /// The circuit computes updated velocities by applying gravity, joins them with positions to
    /// produce new positions, and aggregates block data to determine the highest block at each
    /// `(x, y)` coordinate. Returns input and output handles for external interaction.
    ///
    /// # Returns
    ///
    /// A new `DbspCircuit` instance on success, or a DBSP error if circuit construction fails.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use lille::prelude::*;
    /// let circuit = DbspCircuit::new().expect("circuit construction failed");
    /// ```
    pub fn new() -> Result<Self, dbsp::Error> {
        let (
            circuit,
            (
                position_in,
                velocity_in,
                block_in,
                block_slope_in,
                new_position_out,
                new_velocity_out,
                highest_block_out,
                floor_height_out,
            ),
        ) = RootCircuit::build(|circuit| {
            let (positions, position_in) = circuit.add_input_zset::<Position>();
            let (velocities, velocity_in) = circuit.add_input_zset::<Velocity>();
            let (blocks, block_in) = circuit.add_input_zset::<Block>();
            let (slopes, block_slope_in) = circuit.add_input_zset::<BlockSlope>();

            let highest_pair = blocks
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
                });

            let highest = highest_pair.map(|(hb, _id)| hb.clone());

            let floor_height = highest_pair
                .map_index(|(hb, id)| (*id, (hb.x, hb.y, hb.z)))
                .outer_join(
                    &slopes.map_index(|bs| (bs.block_id, (bs.grad_x, bs.grad_y))),
                    |_, &(x, y, z), &(grad_x, grad_y)| {
                        Some(FloorHeightAt {
                            x,
                            y,
                            z: OrderedFloat(
                                z as f64
                                    + 1.0
                                    + BLOCK_CENTRE_OFFSET * grad_x.into_inner()
                                    + BLOCK_CENTRE_OFFSET * grad_y.into_inner(),
                            ),
                        })
                    },
                    |_, &(x, y, z)| {
                        Some(FloorHeightAt {
                            x,
                            y,
                            z: OrderedFloat(z as f64 + 1.0),
                        })
                    },
                    |_, _| None,
                )
                .filter(|fh| fh.is_some())
                .map(|fh| fh.clone().unwrap());

            let new_vel = velocities.map(|v| Velocity {
                entity: v.entity,
                vx: v.vx,
                vy: v.vy,
                vz: OrderedFloat(v.vz.into_inner() + GRAVITY_PULL),
            });

            let joined = positions.map_index(|p| (p.entity, p.clone())).join(
                &new_vel.map_index(|v| (v.entity, v.clone())),
                |_, pos, vel| (pos.clone(), vel.clone()),
            );

            let new_pos = joined.map(|(p, v)| Position {
                entity: p.entity,
                x: OrderedFloat(p.x.into_inner() + v.vx.into_inner()),
                y: OrderedFloat(p.y.into_inner() + v.vy.into_inner()),
                z: OrderedFloat(p.z.into_inner() + v.vz.into_inner()),
            });

            Ok((
                position_in,
                velocity_in,
                block_in,
                block_slope_in,
                new_pos.output(),
                new_vel.output(),
                highest.output(),
                floor_height.output(),
            ))
        })?;

        Ok(Self {
            circuit,
            position_in,
            velocity_in,
            block_in,
            block_slope_in,
            new_position_out,
            new_velocity_out,
            highest_block_out,
            floor_height_out,
        })
    }

    pub fn step(&mut self) -> Result<(), dbsp::Error> {
        self.circuit.step()
    }

    /// Returns a reference to the input handle for feeding position records into the circuit.
    ///
    /// Use this handle to provide entity position data for processing by the dataflow circuit.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use lille::prelude::*;
    /// let circuit = DbspCircuit::new().expect("circuit construction failed");
    /// let position_handle = circuit.position_in();
    /// // Feed positions into the circuit using `position_handle`
    /// ```
    pub fn position_in(&self) -> &ZSetHandle<Position> {
        &self.position_in
    }

    /// Returns a reference to the input handle for feeding velocity records into the circuit.
    ///
    /// Use this handle to provide entity velocity data for each simulation step.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use lille::prelude::*;
    /// let circuit = DbspCircuit::new().expect("circuit construction failed");
    /// let velocity_in = circuit.velocity_in();
    /// velocity_in.push(
    ///     Velocity {
    ///         entity: 1,
    ///         vx: OrderedFloat(0.0),
    ///         vy: OrderedFloat(0.0),
    ///         vz: OrderedFloat(0.0),
    ///     },
    ///     1,
    /// );
    /// ```
    pub fn velocity_in(&self) -> &ZSetHandle<Velocity> {
        &self.velocity_in
    }

    /// Returns a reference to the input handle for feeding block records into the circuit.
    ///
    /// Use this handle to provide block data as input for each computation step.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use lille::prelude::*;
    /// let circuit = DbspCircuit::new().expect("circuit construction failed");
    /// let block_handle = circuit.block_in();
    /// // Feed block data into the circuit using `block_handle`
    /// ```
    pub fn block_in(&self) -> &ZSetHandle<Block> {
        &self.block_in
    }

    /// Returns a reference to the input handle for feeding block slope records into the circuit.
    ///
    /// Use this handle to supply slope gradient data for blocks, enabling
    /// slope-aware floor height calculations.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use lille::prelude::*;
    /// let circuit = DbspCircuit::new().expect("circuit construction failed");
    /// let slope_handle = circuit.block_slope_in();
    /// // Feed block slope data into the circuit using `slope_handle`
    /// ```
    pub fn block_slope_in(&self) -> &ZSetHandle<BlockSlope> {
        &self.block_slope_in
    }

    /// Returns a reference to the output handle for newly computed entity positions.
    ///
    /// The output handle provides access to the set of updated positions after each circuit step.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use lille::prelude::*;
    /// let circuit = DbspCircuit::new().expect("circuit construction failed");
    /// let new_positions = circuit.new_position_out();
    /// // Read new positions from the output handle
    /// ```
    pub fn new_position_out(&self) -> &OutputHandle<OrdZSet<NewPosition>> {
        &self.new_position_out
    }

    /// Returns a reference to the output handle for newly computed velocities.
    ///
    /// The output contains updated velocity records for all entities after applying
    /// the circuit's physics computations, such as gravity.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use lille::prelude::*;
    /// let circuit = DbspCircuit::new().expect("circuit construction failed");
    /// let velocities = circuit.new_velocity_out();
    /// // Use `velocities` to read updated velocity data.
    /// ```
    pub fn new_velocity_out(&self) -> &OutputHandle<OrdZSet<NewVelocity>> {
        &self.new_velocity_out
    }

    /// Returns a reference to the output handle for the highest block at each (x, y) coordinate.
    ///
    /// The output contains `HighestBlockAt` records representing the maximum `z` value for each `(x, y)`
    /// position in the input block data.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use lille::prelude::*;
    /// let circuit = DbspCircuit::new().expect("circuit construction failed");
    /// let highest_block_handle = circuit.highest_block_out();
    /// // Use `highest_block_handle` to read aggregated highest block data.
    /// ```
    pub fn highest_block_out(&self) -> &OutputHandle<OrdZSet<HighestBlockAt>> {
        &self.highest_block_out
    }

    /// Returns a reference to the output handle for calculated floor heights.
    ///
    /// The output contains `FloorHeightAt` records representing the computed
    /// floor height at each `(x, y)` position, incorporating block heights and
    /// optional slope gradients.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use lille::prelude::*;
    /// let circuit = DbspCircuit::new().expect("circuit construction failed");
    /// let floor_heights = circuit.floor_height_out();
    /// // Read computed floor heights from the output handle
    /// ```
    pub fn floor_height_out(&self) -> &OutputHandle<OrdZSet<FloorHeightAt>> {
        &self.floor_height_out
    }

    /// Clears all input collections to remove accumulated records.
    ///
    /// Input ZSets retain data across [`DbspCircuit::step`] calls. Invoke this method after
    /// processing outputs each frame to ensure that stale input data does not affect future
    /// computations.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use lille::prelude::*;
    /// let mut circuit = DbspCircuit::new().expect("circuit construction failed");
    /// circuit.clear_inputs();
    /// ```
    pub fn clear_inputs(&mut self) {
        self.position_in.clear_input();
        self.velocity_in.clear_input();
        self.block_in.clear_input();
        self.block_slope_in.clear_input();
    }
}
