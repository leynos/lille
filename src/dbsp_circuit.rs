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

use anyhow::Error as AnyError;
use dbsp::{typed_batch::OrdZSet, CircuitHandle, OutputHandle, RootCircuit, ZSetHandle};

use crate::components::{Block, BlockSlope};
use crate::GRACE_DISTANCE;

mod streams;
mod types;
use streams::{
    apply_movement, fear_level_stream, floor_height_stream, movement_decision_stream,
    new_position_stream, new_velocity_stream, position_floor_stream, standing_motion_stream,
};
pub use streams::{highest_block_pair, PositionFloor};
pub use types::{
    FearLevel, FloorHeightAt, Force, HighestBlockAt, MovementDecision, NewPosition, NewVelocity,
    Position, Target, Velocity,
};

pub struct DbspCircuit {
    circuit: CircuitHandle,
    position_in: ZSetHandle<Position>,
    velocity_in: ZSetHandle<Velocity>,
    force_in: ZSetHandle<Force>,
    target_in: ZSetHandle<Target>,
    block_in: ZSetHandle<Block>,
    block_slope_in: ZSetHandle<BlockSlope>,
    new_position_out: OutputHandle<OrdZSet<NewPosition>>,
    new_velocity_out: OutputHandle<OrdZSet<NewVelocity>>,
    highest_block_out: OutputHandle<OrdZSet<HighestBlockAt>>,
    floor_height_out: OutputHandle<OrdZSet<FloorHeightAt>>,
    position_floor_out: OutputHandle<OrdZSet<PositionFloor>>,
}

struct BuildHandles {
    position_in: ZSetHandle<Position>,
    velocity_in: ZSetHandle<Velocity>,
    force_in: ZSetHandle<Force>,
    target_in: ZSetHandle<Target>,
    block_in: ZSetHandle<Block>,
    block_slope_in: ZSetHandle<BlockSlope>,
    new_position_out: OutputHandle<OrdZSet<NewPosition>>,
    new_velocity_out: OutputHandle<OrdZSet<NewVelocity>>,
    highest_block_out: OutputHandle<OrdZSet<HighestBlockAt>>,
    floor_height_out: OutputHandle<OrdZSet<FloorHeightAt>>,
    position_floor_out: OutputHandle<OrdZSet<PositionFloor>>,
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
        let (circuit, handles) = RootCircuit::build(Self::build_streams)?;

        Ok(Self {
            circuit,
            position_in: handles.position_in,
            velocity_in: handles.velocity_in,
            force_in: handles.force_in,
            target_in: handles.target_in,
            block_in: handles.block_in,
            block_slope_in: handles.block_slope_in,
            new_position_out: handles.new_position_out,
            new_velocity_out: handles.new_velocity_out,
            highest_block_out: handles.highest_block_out,
            floor_height_out: handles.floor_height_out,
            position_floor_out: handles.position_floor_out,
        })
    }

    pub fn step(&mut self) -> Result<(), dbsp::Error> {
        self.circuit.step()
    }

    fn build_streams(circuit: &mut RootCircuit) -> Result<BuildHandles, AnyError> {
        let (positions, position_in) = circuit.add_input_zset::<Position>();
        let (velocities, velocity_in) = circuit.add_input_zset::<Velocity>();
        let (forces, force_in) = circuit.add_input_zset::<Force>();
        let (targets, target_in) = circuit.add_input_zset::<Target>();
        let (blocks, block_in) = circuit.add_input_zset::<Block>();
        let (slopes, block_slope_in) = circuit.add_input_zset::<BlockSlope>();

        let highest_pair = highest_block_pair(&blocks);
        let highest = highest_pair.map(|(hb, _)| *hb);
        let floor_height = floor_height_stream(&highest_pair, &slopes);

        let pos_floor = position_floor_stream(&positions, &floor_height);

        fn within_grace(pf: &PositionFloor) -> bool {
            pf.position.z.into_inner() <= pf.z_floor.into_inner() + GRACE_DISTANCE
        }
        let unsupported = pos_floor.filter(|pf| !within_grace(pf));
        let standing = pos_floor.filter(within_grace);

        let unsupported_positions = unsupported.map(|pf| pf.position);
        let all_new_vel = new_velocity_stream(&velocities, &forces);
        let unsupported_velocities = all_new_vel.map_index(|v| (v.entity, *v)).join(
            &unsupported.map_index(|pf| (pf.position.entity, ())),
            |_, vel, _| *vel,
        );
        let new_pos_unsupported =
            new_position_stream(&unsupported_positions, &unsupported_velocities);

        let (new_pos_standing, new_vel_standing) =
            standing_motion_stream(&standing, &floor_height, &all_new_vel);

        let base_pos = new_pos_unsupported.plus(&new_pos_standing);
        let new_vel = unsupported_velocities.plus(&new_vel_standing);

        let fear = fear_level_stream(&positions);
        let decisions = movement_decision_stream(&fear, &targets, &positions);

        let moved_pos = apply_movement(&base_pos, &decisions);

        Ok(BuildHandles {
            position_in,
            velocity_in,
            force_in,
            target_in,
            block_in,
            block_slope_in,
            new_position_out: moved_pos.output(),
            new_velocity_out: new_vel.output(),
            highest_block_out: highest.output(),
            floor_height_out: floor_height.output(),
            position_floor_out: pos_floor.output(),
        })
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

    /// Returns a reference to the input handle for feeding force records into the circuit.
    ///
    /// Use this handle to supply external forces acting on entities. If a
    /// force is omitted for an entity, only gravity is applied.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use lille::prelude::*;
    /// # use ordered_float::OrderedFloat;
    /// let circuit = DbspCircuit::new().expect("circuit construction failed");
    /// let force_in = circuit.force_in();
    /// force_in.push(
    ///     Force {
    ///         entity: 1,
    ///         fx: OrderedFloat(5.0),
    ///         fy: OrderedFloat(0.0),
    ///         fz: OrderedFloat(0.0),
    ///         mass: Some(OrderedFloat(5.0)),
    ///     },
    ///     1,
    /// );
    /// ```
    pub fn force_in(&self) -> &ZSetHandle<Force> {
        &self.force_in
    }

    /// Returns a reference to the input handle for entity targets.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use ordered_float::OrderedFloat;
    /// # use lille::dbsp_circuit::{DbspCircuit, Target as DbspTarget};
    /// let circuit = DbspCircuit::new().expect("circuit construction failed");
    /// let handle = circuit.target_in();
    /// handle.push(
    ///     DbspTarget { entity: 1, x: OrderedFloat(10.0), y: OrderedFloat(5.0) },
    ///     1,
    /// );
    /// ```
    pub fn target_in(&self) -> &ZSetHandle<Target> {
        &self.target_in
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

    /// Returns a reference to the output handle for entity positions joined with
    /// floor height.
    ///
    /// The output contains [`PositionFloor`] records that pair each entity's
    /// [`Position`] with the discrete [`FloorHeightAt`] value at its grid cell.
    /// Use this handle to read the results of the position-to-floor join after
    /// each call to [`DbspCircuit::step`].
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use lille::prelude::*;
    /// let mut circuit = DbspCircuit::new().expect("circuit construction failed");
    /// let joined = circuit.position_floor_out();
    /// // Read joined records from `joined`
    /// ```
    pub fn position_floor_out(&self) -> &OutputHandle<OrdZSet<PositionFloor>> {
        &self.position_floor_out
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
        self.force_in.clear_input();
        self.target_in.clear_input();
        self.block_in.clear_input();
        self.block_slope_in.clear_input();
    }
}

#[cfg(test)]
mod tests;
