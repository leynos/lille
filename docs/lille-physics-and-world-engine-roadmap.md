# Lille Physics and World Engine Roadmap

## 1. Vision and Strategy

This document outlines the development roadmap for Lille's physics and
world-interaction engine. The strategy is to build a highly performant,
maintainable, and extensible simulation core by leveraging a declarative,
pure-Rust dataflow architecture powered by the DBSP library.

This roadmap supersedes all previous plans based on DDlog. The migration to
DBSP is a strategic decision to simplify the toolchain, improve type safety and
performance, and enable a more robust testing strategy. The core design is
predicated on the principles of incremental dataflow, with a clean separation
between the game's state (managed by Bevy ECS) and its logic (executed within a
DBSP circuit).

Our development will proceed in distinct, verifiable phases, moving from the
foundational migration to the implementation of advanced physics and agent
behaviours.

**Core Design Documents:**

- **Design:** `docs/lille-physics-engine-design.md`

- **Implementation:** `docs/declarative-world-inference-with-dbsp-and-rust.md`

- **Testing:** `docs/testing-declarative-game-logic-in-dbsp.md`

## Phase 1: Foundational Migration to DBSP

**Goal**: To completely remove the DDlog dependency and establish a working,
pure-Rust build where the core physics logic is handled by a minimal DBSP
circuit. This phase prioritises architectural simplification over feature
parity.

**Key Tasks**:

1. **Dependency Removal**:

   - [x] Remove the `ddlog` crate and all related FFI code (`lille_ddlog` stub).

   - [x] Delete the `build.rs` elements script responsible for DDlog
     compilation.

   - [x] Remove all `.dl` rule files from the repository.

   - [x] Purge DDlog-specific CI steps, targets and build scripts.

2. **Initial DBSP Circuit Implementation**:

   - [x] Create a new `dbsp_circuit` module to house the dataflow logic.

   - [x] Define the initial input streams (`Position`, `Block`) and output
     stream (`NewPosition`).

   - [x] Implement the most basic geometry rule: `HighestBlockAt` using
     `group_by` and `aggregate`.

   - [x] Implement the simplest physics rule: apply `GRAVITY_PULL` to all
     entities, ignoring floor support for this initial step.

3. **Bevy Integration**:

   - [x] Implement the Bevy systems responsible for marshalling data to and from
     the new DBSP circuit.

   - [x] Ensure the `ECS -> DBSP -> ECS` loop runs correctly each tick.

4. **Testing Foundation**:

   - [x] Write the first BDD tests to verify that the headless Bevy app
     correctly applies the simple gravity rule from the DBSP circuit.

   - [x] Write the first unit tests for the `HighestBlockAt` operator in
     isolation.

**Acceptance Criteria**:

- The project compiles and runs with `cargo build` and `cargo run` without any
  DDlog-related dependencies.

- A simple scene with a single entity demonstrates that the entity's position is
  updated by the DBSP circuit each frame (i.e., it falls downwards).

- Core BDD and unit tests pass, establishing the testing pattern for subsequent
  phases.

## Phase 2: Achieving Physics Parity

**Goal**: To re-implement the full set of existing physics and geometry rules
within the DBSP circuit, achieving parity with the capabilities of the final
DDlog-based version.

**Key Tasks**:

1. **Full Geometry Model**:

   - [x] Implement the `FloorHeightAt` calculation, including the `left_join`
     with `BlockSlope` data to handle sloped surfaces correctly.

   - [x] Implement the logic to join an entity's continuous `Position` with the
     discrete `FloorHeightAt` grid.

2. **Complete Motion Logic**:

   - [x] Implement the `Standing` vs. `Unsupported` filter based on `z_floor`.

   - [x] Implement the two branches of motion:

     - Apply gravity to `Unsupported` entities.

     - Implement basic AI-driven `(dx, dy)` movement for `Standing` entities,
       ensuring they snap to the new floor height.

3. **Comprehensive Testing**:

   - [x] Port all existing BDD test cases (`physics_bdd.rs`) to verify the DBSP
     implementation. Scenarios must include:

     - Entity falling in empty space.

     - Entity standing on a flat block.

     - Entity standing on a sloped block.

     - Entity moving between blocks of different heights.

   - [x] Add unit tests for every key operator and sub-flow in the circuit
     (e.g., slope calculation, standing/unsupported filter).

**Acceptance Criteria**:

- All physics behaviours from the previous DDlog version are replicated.

- The BDD test suite provides full coverage for all core physics interactions.

- The project is considered stable and feature-complete with respect to its
  original design.

## Phase 3: Expanding Physics and Agent Dynamics

**Goal**: To move beyond the original feature set and introduce more dynamic
physical properties and agent behaviours.

**Key Tasks**:

1. **Velocity and Acceleration**:

   - [x] Add `Velocity` and `Force` as input streams to the DBSP circuit.

   - [x] Implement operators to calculate acceleration based on forces (`F=ma`).

   - [x] Integrate velocity into the `NewPosition` calculation
     (`p_new = p_old + v*dt`).

   - [x] Implement a `friction` operator that reduces velocity for `Standing`
     entities.

   - [x] Implement `terminal_velocity` clamping.

2. **Reactive Agent Behaviours**:

   - [x] Add `Target` and `Fear` as input streams.

   - [x] Implement `join` operations to generate movement vectors based on these
     inputs (e.g., move towards target, move away from fear source).

   - [x] Implement a simple priority system (e.g., fear overrides targeting).

3. **Health and Damage**:

   - [ ] Introduce a `Health` component and a corresponding `Damage` input
     stream.

     - [ ] Specify the ECS `Health` component fields (e.g., current and maximum
       hit points) and mirror the structure for a DBSP input collection.

     - [ ] Extend the DBSP circuit schema with health state and damage event
       streams so the circuit remains the canonical interpreter of health
       changes.

     - [ ] Update the Bevy → DBSP → Bevy marshalling layer to publish health
       snapshots into the circuit and apply circuit-emitted health deltas back
       onto ECS components.

     - [ ] Add data-driven tests—`rstest` fixtures and headless Bevy BDD
       scenarios—covering health synchronisation across the circuit boundary.

   - [ ] Implement a simple damage model (e.g., falling damage calculated from
     velocity upon landing).

     - [ ] Detect landing events inside the circuit by tracking transitions
       from `Unsupported` to `Standing` alongside vertical velocity.

     - [ ] Define a fall-damage operator that applies a safe-velocity threshold
       and scaling factor entirely within DBSP.

     - [ ] Emit derived damage events into the `Damage` stream and reduce
       entity health through the circuit's health accumulator.

     - [ ] Cover the damage flow with DBSP unit tests and headless Bevy
       simulations demonstrating falling damage.

**Acceptance Criteria**:

- Entities exhibit momentum and are affected by forces.

- Agents demonstrate simple, reactive seeking and fleeing behaviours.

- A basic health and damage system is operational.

## Phase 4: Advanced Features and Polish

**Goal**: To investigate and implement more complex features and ensure the
long-term stability and performance of the engine.

**Key Tasks**:

1. **Advanced World Interaction**:

   - [ ] Investigate models for multi-block entities (e.g., doors, moving
     platforms).

   - [ ] Design and implement a simple inventory or item-pickup system using the
     dataflow model.

2. **Complex AI Integration**:

   - [ ] Implement a dedicated A\* pathfinding system in imperative Rust.

   - [ ] Feed the *results* of the pathfinder (e.g., the next waypoint) into the
     DBSP circuit as a `PathGoal` input stream to drive agent movement.

3. **Performance and Optimisation**:

   - [ ] Conduct performance profiling of the DBSP circuit in complex scenes.

   - [ ] Investigate multi-worker DBSP circuits if performance bottlenecks are
     identified.

4. **Documentation and Refinement**:

   - [ ] Ensure all design and implementation documents are fully up-to-date.

   - [ ] Add comprehensive code comments and `rustdoc` throughout the
     `dbsp_circuit` module.
