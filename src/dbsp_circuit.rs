//! DBSP-based world inference engine.
//!
//! This module defines [`DbspCircuit`], the authoritative dataflow program for
//! Lille's game world. Callers feed [`Position`] and [`Block`] records into the
//! circuit, call [`DbspCircuit::step`], then read [`NewPosition`] and
//! [`HighestBlockAt`] outputs. Input collections persist across steps—invoke
//! [`DbspCircuit::clear_inputs`] after each frame to prevent stale data from
//! affecting subsequent computations.

use dbsp::{
    operator::Max, typed_batch::OrdZSet, CircuitHandle, OutputHandle, RootCircuit, ZSetHandle,
};
use ordered_float::OrderedFloat;
use size_of::SizeOf;

use crate::components::Block;
use crate::GRAVITY_PULL;

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

pub struct DbspCircuit {
    circuit: CircuitHandle,
    position_in: ZSetHandle<Position>,
    velocity_in: ZSetHandle<Velocity>,
    block_in: ZSetHandle<Block>,
    new_position_out: OutputHandle<OrdZSet<NewPosition>>,
    new_velocity_out: OutputHandle<OrdZSet<NewVelocity>>,
    highest_block_out: OutputHandle<OrdZSet<HighestBlockAt>>,
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
    /// ```no_run
    /// use lille::DbspCircuit;
    /// let circuit = DbspCircuit::new().unwrap();
    /// ```
    pub fn new() -> Result<Self, dbsp::Error> {
        let (
            circuit,
            (
                position_in,
                velocity_in,
                block_in,
                new_position_out,
                new_velocity_out,
                highest_block_out,
            ),
        ) = RootCircuit::build(|circuit| {
            let (positions, position_in) = circuit.add_input_zset::<Position>();
            let (velocities, velocity_in) = circuit.add_input_zset::<Velocity>();
            let (blocks, block_in) = circuit.add_input_zset::<Block>();

            let highest =
                blocks
                    .map_index(|b| ((b.x, b.y), b.z))
                    .aggregate(Max)
                    .map(|((x, y), z)| HighestBlockAt {
                        x: *x,
                        y: *y,
                        z: *z,
                    });

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
                new_pos.output(),
                new_vel.output(),
                highest.output(),
            ))
        })?;

        Ok(Self {
            circuit,
            position_in,
            velocity_in,
            block_in,
            new_position_out,
            new_velocity_out,
            highest_block_out,
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
    /// ```no_run
    /// use lille::DbspCircuit;
    /// let circuit = DbspCircuit::new().unwrap();
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
    /// ```no_run
    /// use lille::DbspCircuit;
    /// let circuit = DbspCircuit::new().unwrap();
    /// let velocity_in = circuit.velocity_in();
    /// // Feed velocity data via `velocity_in`
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
    /// ```no_run
    /// use lille::DbspCircuit;
    /// use lille::components::Block;
    /// let circuit = DbspCircuit::new().unwrap();
    /// let block_handle = circuit.block_in();
    /// // Feed block data into the circuit using `block_handle`
    /// ```
    pub fn block_in(&self) -> &ZSetHandle<Block> {
        &self.block_in
    }

    /// Returns a reference to the output handle for newly computed entity positions.
    ///
    /// The output handle provides access to the set of updated positions after each circuit step.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use lille::DbspCircuit;
    /// let circuit = DbspCircuit::new().unwrap();
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
    /// ```no_run
    /// use lille::DbspCircuit;
    /// let circuit = DbspCircuit::new().unwrap();
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
    /// ```no_run
    /// use lille::DbspCircuit;
    /// let circuit = DbspCircuit::new().unwrap();
    /// let highest_block_handle = circuit.highest_block_out();
    /// // Use `highest_block_handle` to read aggregated highest block data.
    /// ```
    pub fn highest_block_out(&self) -> &OutputHandle<OrdZSet<HighestBlockAt>> {
        &self.highest_block_out
    }

    /// Clears all input collections to remove accumulated records.
    ///
    /// Input ZSets retain data across [`DbspCircuit::step`] calls. Invoke this method after
    /// processing outputs each frame to ensure that stale input data does not affect future
    /// computations.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use lille::DbspCircuit;
    /// let mut circuit = DbspCircuit::new().unwrap();
    /// circuit.clear_inputs();
    /// ```
    pub fn clear_inputs(&mut self) {
        self.position_in.clear_input();
        self.velocity_in.clear_input();
        self.block_in.clear_input();
    }
}
