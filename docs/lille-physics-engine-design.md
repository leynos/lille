# Lille Physics Engine Design

## 1. Guiding Principles

The physics and world-interaction engine for Lille is designed around a core
principle: **declarative, incremental dataflow**. We use the DBSP (Differential
Dataflow Stream Processing) library to define the complete set of rules
governing geometry, physics, and agent behaviour as a pure-Rust dataflow
circuit.

This approach provides several key advantages:

- **Clarity and Maintainability**: Game logic is expressed as a series of data
  transformations (`join`, `map`, `filter`, `aggregate`) rather than complex,
  imperative, and stateful code. This makes the rules easier to reason about,
  debug, and extend.

- **Performance**: DBSP provides incremental computation out of the box. At each
  tick, only the changes to the world state are processed, not the entire
  dataset. This ensures high performance even in complex scenes.

- **Robustness and Simplicity**: By implementing the entire engine in Rust, we
  eliminate the toolchain complexity, FFI overhead, and potential for
  type-system mismatches inherent in the previous DDlog-based approach.

The physics engine acts as the computational "brain" of the simulation. It is
driven by the Bevy Entity-Component-System (ECS), which remains the single
source of truth for the state of the world.

## 2. Architectural Overview

The architecture creates a clean separation between the *state* of the world
(in Bevy) and the *logic* that governs it (in DBSP).

The data flows in a continuous loop each simulation tick:

1. **ECS State Read**: Bevy systems query the `World` to gather the current
   state of all relevant entities—their positions, velocities, AI targets, etc.
   The state of the static world geometry (blocks and slopes) is also collected.

2. **Input to DBSP Circuit**: This collection of state information is fed as a
   batch of updates into the input streams of a single, comprehensive DBSP
   `circuit`.

3. **Incremental Computation**: The `circuit.step()` method is called. DBSP
   propagates the input changes through the entire dataflow graph,
   incrementally re-calculating all derived facts and producing new output
   streams.

4. **ECS State Write**: Bevy systems read the records from the circuit's output
   streams (e.g., `NewPosition`, `NewVelocity`) and apply these changes back to
   the corresponding components in the ECS.

In this model, the Bevy systems act as thin, stateless data marshals. All
substantive logic resides within the DBSP circuit.

> For a detailed breakdown of the circuit's construction, I/O streams, and the
> mechanics of its integration with Bevy, see:
>
> - `docs/declarative-world-inference-with-dbsp-and-rust.md`

## 3. Core Physics and Geometry

The simulation is built upon a foundation of relational data streams that
represent the physical world.

### 3.1. World Geometry and Floor Detection

The primary mechanism for entity interaction with the world is floor detection.
This is not a traditional collision check but a continuous calculation of the
"ground level" at any given point.

The dataflow is as follows:

1. **Highest Block Identification**: The input stream of `Block` data is
   processed to find the highest block at each `(x, y)` grid location. This is
   achieved with a `group_by((x, y)).aggregate(max(z))` operation in DBSP.

2. **Floor Height Calculation**: The resulting `HighestBlockAt` stream is joined
   with `BlockSlope` data. A `map` operator then calculates the precise
   `z_floor` coordinate. Slopes are joined using the block `id` and, for now,
   the height is evaluated at the constant `BLOCK_CENTRE_OFFSET` (currently
   `0.5`) because the entity-specific offset is not yet available. If no slope
   exists the floor is flat one unit above the block.

### 3.2. Entity State: Standing vs. Unsupported

An entity's physical state is derived by comparing its position to the
calculated floor height.

1. The `Position` stream is joined with the `FloorHeightAt` stream based on the
   entity's `(x, y)` coordinates. The continuous `x` and `y` values are floored
   to determine the grid cell. The join emits a `PositionFloor` record pairing
   the original position with the matched `z_floor` height.

2. A `filter` operator then partitions entities into two streams:

   - `Unsupported`: Entities where `position.z > z_floor + GRACE_DISTANCE`.

   - `Standing`: All other entities.

### 3.3. Motion Calculation

The two entity states flow into different branches of the circuit to determine
their new position.

- **Gravity on Unsupported Entities**: The `Unsupported` stream is passed
  through a simple `map` operator that subtracts the `GRAVITY_PULL` constant
  from the entity's `z` coordinate.

- **Movement for Standing Entities**: The `Standing` stream is joined with AI
  data (see below) to determine a desired movement vector `(dx, dy)`. The
  proposed new location `(x+dx, y+dy)` is then fed back into the
  floor-height-calculation sub-graph to find the correct `z` for the new
  position, ensuring entities stick to the ground as they move. Horizontal
  velocities double as AI intent; the circuit resets vertical velocity to zero
  and snaps the entity to the floor height at the new cell. Entities whose `z`
  coordinate is within `GRACE_DISTANCE` of the floor are treated as `Standing`.

## 4. Agent Behaviour (AI)

Simple, reactive agent behaviours can be expressed elegantly within the same
declarative dataflow model.

- **Seeking and Fleeing**: AI motivations like seeking a target or fleeing a
  source of fear are implemented as `join` operations. Joining the `Standing`
  entities with `Target` and `Fear` streams produces a movement vector for each
  entity.

- **State-driven Decisions**: More complex logic can be built by composing
  operators. For example, to make an agent flee when its health is low, we
  would `join` entities with their `Health` component, `filter` for those where
  `health < threshold`, and then `join` the result with the fleeing-behaviour
  sub-graph.

It is crucial to recognise the limitations of this approach. DBSP is not suited
for complex, stateful search algorithms. **A\* pathfinding**, for instance,
should remain implemented in imperative Rust. The *output* of such an algorithm
(e.g., the next waypoint in a path) can be fed as an input stream to the DBSP
circuit, which then uses it to generate movement.

## 5. Testing Strategy

The pure-Rust nature of the DBSP implementation allows for a powerful and
comprehensive two-tiered testing strategy, which is a significant improvement
over testing across an FFI boundary.

1. **Unit Testing**: Each logical component of the dataflow circuit (e.g., a
   single `map` or `join` operation) is tested in isolation using standard
   `#[test]` functions. We provide controlled inputs and assert on the direct
   output of the operator.

2. **Behaviour-Driven Development (BDD)**: The emergent, high-level behaviour of
   the fully integrated system is tested using headless Bevy applications. We
   set up a scenario (`Given`), run the simulation for a tick (`When`), and
   assert that the final state of the ECS matches the expected outcome (`Then`).

> For a complete overview of the testing methodology, including code examples
> and best practices, see:
>
> - `docs/testing-declarative-game-logic-in-dbsp.md`
