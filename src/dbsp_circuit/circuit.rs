//! Core DBSP circuit construction and handle accessors.

use dbsp::circuit::Circuit;
use dbsp::{
    operator::Generator, typed_batch::OrdZSet, CircuitHandle, OutputHandle, RootCircuit, ZSetHandle,
};

use crate::components::{Block, BlockSlope};

use super::helpers::{advance_tick, within_grace};
use super::streams::{
    apply_movement, fall_damage_stream, fear_level_stream, floor_height_stream,
    health_delta_stream, highest_block_pair, movement_decision_stream, new_position_stream,
    new_velocity_stream, position_floor_stream, standing_motion_stream,
};
use super::types::{
    DamageEvent, FearLevel, FloorHeightAt, Force, HealthDelta, HealthState, HighestBlockAt,
    NewPosition, NewVelocity, Position, PositionFloor, Target, Tick, Velocity,
};

/// Authoritative DBSP dataflow for Lille's world simulation.
///
/// `DbspCircuit` owns the underlying [`RootCircuit`] and exposes typed
/// handles for feeding entity state and environment records into the
/// dataflow. After updating the inputs, advance the circuit with
/// [`DbspCircuit::step`] to derive new positions, velocities, and terrain
/// queries.
///
/// # Examples
///
/// ```rust,no_run
/// # use lille::prelude::*;
/// # use lille::dbsp_circuit::step as _;
/// let mut circuit = DbspCircuit::new().expect("circuit construction failed");
///
/// // 1) Feed inputs for this frame.
/// // circuit.position_in().push(Position { /* ... */ }, 1);
/// // circuit.velocity_in().push(Velocity { /* ... */ }, 1);
/// // circuit.force_in().push(Force { /* ... */ }, 1);
/// // circuit.fear_in().push(FearLevel { /* ... */ }, 1);
/// // circuit.target_in().push(Target { /* ... */ }, 1);
/// // circuit.block_in().push(Block { /* ... */ }, 1);
/// // circuit.block_slope_in().push(BlockSlope { /* ... */ }, 1);
///
/// // 2) Advance the circuit.
/// lille::dbsp_circuit::step(&mut circuit);
///
/// // 3) Read outputs via the getters.
/// // let _ = circuit.new_position_out();
///
/// // 4) Clear inputs before the next frame.
/// circuit.clear_inputs();
/// ```
pub struct DbspCircuit {
    pub(crate) circuit: CircuitHandle,
    position_in: ZSetHandle<Position>,
    velocity_in: ZSetHandle<Velocity>,
    force_in: ZSetHandle<Force>,
    fear_in: ZSetHandle<FearLevel>,
    target_in: ZSetHandle<Target>,
    health_state_in: ZSetHandle<HealthState>,
    damage_in: ZSetHandle<DamageEvent>,
    block_in: ZSetHandle<Block>,
    block_slope_in: ZSetHandle<BlockSlope>,
    new_position_out: OutputHandle<OrdZSet<NewPosition>>,
    new_velocity_out: OutputHandle<OrdZSet<NewVelocity>>,
    highest_block_out: OutputHandle<OrdZSet<HighestBlockAt>>,
    floor_height_out: OutputHandle<OrdZSet<FloorHeightAt>>,
    position_floor_out: OutputHandle<OrdZSet<PositionFloor>>,
    health_delta_out: OutputHandle<OrdZSet<HealthDelta>>,
}

struct BuildHandles {
    position_in: ZSetHandle<Position>,
    velocity_in: ZSetHandle<Velocity>,
    force_in: ZSetHandle<Force>,
    fear_in: ZSetHandle<FearLevel>,
    target_in: ZSetHandle<Target>,
    health_state_in: ZSetHandle<HealthState>,
    damage_in: ZSetHandle<DamageEvent>,
    block_in: ZSetHandle<Block>,
    block_slope_in: ZSetHandle<BlockSlope>,
    new_position_out: OutputHandle<OrdZSet<NewPosition>>,
    new_velocity_out: OutputHandle<OrdZSet<NewVelocity>>,
    highest_block_out: OutputHandle<OrdZSet<HighestBlockAt>>,
    floor_height_out: OutputHandle<OrdZSet<FloorHeightAt>>,
    position_floor_out: OutputHandle<OrdZSet<PositionFloor>>,
    health_delta_out: OutputHandle<OrdZSet<HealthDelta>>,
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
    /// # Errors
    /// Returns a DBSP error if the underlying circuit fails to build.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use lille::prelude::*;
    /// # use lille::dbsp_circuit::step as _;
    /// let circuit = DbspCircuit::new().expect("circuit construction failed");
    /// ```
    pub fn new() -> Result<Self, dbsp::Error> {
        let (circuit, handles) = RootCircuit::build(Self::build_streams)?;

        Ok(Self {
            circuit,
            position_in: handles.position_in,
            velocity_in: handles.velocity_in,
            force_in: handles.force_in,
            fear_in: handles.fear_in,
            target_in: handles.target_in,
            health_state_in: handles.health_state_in,
            damage_in: handles.damage_in,
            block_in: handles.block_in,
            block_slope_in: handles.block_slope_in,
            new_position_out: handles.new_position_out,
            new_velocity_out: handles.new_velocity_out,
            highest_block_out: handles.highest_block_out,
            floor_height_out: handles.floor_height_out,
            position_floor_out: handles.position_floor_out,
            health_delta_out: handles.health_delta_out,
        })
    }

    /// Advances the DBSP circuit by one tick.
    ///
    /// Call this once per frame after pushing all input records. The evaluation
    /// propagates changes through the dataflow and atomically refreshes the output
    /// handles with derived positions, velocities, and terrain queries for this
    /// tick. This method does not clear inputs; input collections persist across
    /// steps. Invoke [`DbspCircuit::clear_inputs`] after processing outputs to avoid
    /// stale state carrying into the next frame.
    ///
    /// # Errors
    ///
    /// Propagates any error reported by the underlying DBSP circuit.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use lille::prelude::*;
    /// let mut circuit = DbspCircuit::new().expect("circuit construction failed");
    /// circuit.step().expect("circuit evaluation failed");
    /// ```
    pub fn step(&mut self) -> Result<(), dbsp::Error> {
        self.circuit.step()
    }

    #[expect(
        clippy::unnecessary_wraps,
        reason = "RootCircuit::build expects constructors that return Result."
    )]
    fn build_streams(circuit: &mut RootCircuit) -> Result<BuildHandles, dbsp::Error> {
        let (positions, position_in) = circuit.add_input_zset::<Position>();
        let (velocities, velocity_in) = circuit.add_input_zset::<Velocity>();
        let (forces, force_in) = circuit.add_input_zset::<Force>();
        let (fears, fear_in) = circuit.add_input_zset::<FearLevel>();
        let (targets, target_in) = circuit.add_input_zset::<Target>();
        let (health_states, health_state_in) = circuit.add_input_zset::<HealthState>();
        let (damage_events, damage_in) = circuit.add_input_zset::<DamageEvent>();
        let (blocks, block_in) = circuit.add_input_zset::<Block>();
        let (slopes, block_slope_in) = circuit.add_input_zset::<BlockSlope>();

        let tick_source = circuit.add_source(Generator::new({
            let mut tick: Tick = 0;
            move || advance_tick(&mut tick)
        }));
        let current_tick = tick_source;

        let highest_pair = highest_block_pair(&blocks);
        let highest = highest_pair.map(|(hb, _)| *hb);
        let floor_height = floor_height_stream(&highest_pair, &slopes);

        let pos_floor = position_floor_stream(&positions, &floor_height);

        let unsupported = pos_floor.filter(|pf| !within_grace(pf));
        let standing = pos_floor.filter(within_grace);

        let unsupported_positions = unsupported.map(|pf| pf.position);
        let all_new_vel = new_velocity_stream(&velocities, &forces);
        let unsupported_velocities = all_new_vel.map_index(|v| (v.entity, *v)).join(
            &unsupported.map_index(|pf| (pf.position.entity, ())),
            |_, vel, ()| *vel,
        );
        let new_pos_unsupported =
            new_position_stream(&unsupported_positions, &unsupported_velocities);

        let (new_pos_standing, new_vel_standing) =
            standing_motion_stream(&standing, &floor_height, &all_new_vel);

        let fall_damage = fall_damage_stream(
            &standing,
            &unsupported,
            &unsupported_velocities,
            &current_tick,
        );
        let damage_with_fall = damage_events.plus(&fall_damage);

        let base_pos = new_pos_unsupported.plus(&new_pos_standing);
        let new_vel = unsupported_velocities.plus(&new_vel_standing);

        let fear = fear_level_stream(&positions, &fears);
        let decisions = movement_decision_stream(&fear, &targets, &positions);

        let moved_pos = apply_movement(&base_pos, &decisions);

        let health_deltas = health_delta_stream(&health_states, &damage_with_fall);

        Ok(BuildHandles {
            position_in,
            velocity_in,
            force_in,
            fear_in,
            target_in,
            health_state_in,
            damage_in,
            block_in,
            block_slope_in,
            new_position_out: moved_pos.output(),
            new_velocity_out: new_vel.output(),
            highest_block_out: highest.output(),
            floor_height_out: floor_height.output(),
            position_floor_out: pos_floor.output(),
            health_delta_out: health_deltas.output(),
        })
    }

    /// Returns a reference to the input handle for feeding position records into the circuit.
    pub const fn position_in(&self) -> &ZSetHandle<Position> {
        &self.position_in
    }

    /// Returns a reference to the input handle for feeding velocity records into the circuit.
    pub const fn velocity_in(&self) -> &ZSetHandle<Velocity> {
        &self.velocity_in
    }

    /// Returns a reference to the input handle for feeding force records into the circuit.
    pub const fn force_in(&self) -> &ZSetHandle<Force> {
        &self.force_in
    }

    /// Returns a reference to the input handle for entity fear levels.
    pub const fn fear_in(&self) -> &ZSetHandle<FearLevel> {
        &self.fear_in
    }

    /// Returns a reference to the input handle for entity targets.
    pub const fn target_in(&self) -> &ZSetHandle<Target> {
        &self.target_in
    }

    /// Returns a reference to the input handle for entity health snapshots.
    pub const fn health_state_in(&self) -> &ZSetHandle<HealthState> {
        &self.health_state_in
    }

    /// Returns a reference to the damage/healing input stream.
    pub const fn damage_in(&self) -> &ZSetHandle<DamageEvent> {
        &self.damage_in
    }

    /// Returns a reference to the input handle for feeding block records into the circuit.
    pub const fn block_in(&self) -> &ZSetHandle<Block> {
        &self.block_in
    }

    /// Returns a reference to the input handle for feeding block slope records into the circuit.
    pub const fn block_slope_in(&self) -> &ZSetHandle<BlockSlope> {
        &self.block_slope_in
    }

    /// Returns a reference to the output handle for newly computed entity positions.
    pub const fn new_position_out(&self) -> &OutputHandle<OrdZSet<NewPosition>> {
        &self.new_position_out
    }

    /// Returns a reference to the output handle for newly computed velocities.
    pub const fn new_velocity_out(&self) -> &OutputHandle<OrdZSet<NewVelocity>> {
        &self.new_velocity_out
    }

    /// Returns a reference to the output handle for the highest block at each (x, y) coordinate.
    pub const fn highest_block_out(&self) -> &OutputHandle<OrdZSet<HighestBlockAt>> {
        &self.highest_block_out
    }

    /// Returns a reference to the output handle for calculated floor heights.
    pub const fn floor_height_out(&self) -> &OutputHandle<OrdZSet<FloorHeightAt>> {
        &self.floor_height_out
    }

    /// Returns a reference to the output handle pairing positions with floor heights.
    pub const fn position_floor_out(&self) -> &OutputHandle<OrdZSet<PositionFloor>> {
        &self.position_floor_out
    }

    /// Returns a reference to the health delta output handle.
    pub const fn health_delta_out(&self) -> &OutputHandle<OrdZSet<HealthDelta>> {
        &self.health_delta_out
    }

    /// Clears all input collections to remove accumulated records.
    pub fn clear_inputs(&mut self) {
        self.position_in.clear_input();
        self.velocity_in.clear_input();
        self.force_in.clear_input();
        self.fear_in.clear_input();
        self.target_in.clear_input();
        self.health_state_in.clear_input();
        self.damage_in.clear_input();
        self.block_in.clear_input();
        self.block_slope_in.clear_input();
    }
}
