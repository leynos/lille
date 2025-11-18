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

## Bevy Upgrade Checklist

- [x] Phase 1 — Upgrade Bevy crates from 0.12 to 0.13 (completed 18 November
  2025 with artefacts stored under `artifacts/bevy-0-17-upgrade/phase-1/` and
  regression coverage in `tests/physics_bdd/dbsp_authority.rs`).
- [ ] Phase 2 — Upgrade Bevy crates from 0.13 to 0.14.
- [ ] Phase 3 — Upgrade Bevy crates from 0.14 to 0.15.
- [ ] Phase 4 — Upgrade Bevy crates from 0.15 to 0.16.
- [ ] Phase 5 — Upgrade Bevy crates from 0.16 to 0.17.3.

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

3. **Health and Damage** *(done)*:

    - [x] Introduce a `Health` component and a corresponding `Damage` input
      stream.

      - [x] Specify the ECS `Health` component fields (e.g., current and maximum
        hit points) and mirror the structure for a DBSP input collection.

        - [x] Field types: `entity: EntityId`, `current: u16`, `max: u16`.
          Enforce `0 ≤ current ≤ max` at all times.

        - [x] Type aliases: use the
          [canonical type definitions][health-type-defs]
          (`type EntityId = u64; type Tick = u64`). `Tick` counts simulation
          ticks and advances monotonically.

        - [x] Arithmetic: apply saturating add/sub inside the circuit so health
          never underflows below `0` or overflows above `max`.

        - [x] Serialisation: mirror the component layout into a `HealthState`
          input collection for the circuit.

        - [x] Rounding: if non-integer sources emerge, round damage down and
          round healing down before applying deltas.

      - [x] Extend the DBSP circuit schema with health state and damage event
        streams so the circuit remains the canonical interpreter of health
        changes.

        - Define:
          - `DamageSource = { External, Fall, Script, Other(u16) }`.
          - `Tick = u64` (simulation ticks; monotonic).
          - `DamageEvent { entity: EntityId, amount: u16, source: DamageSource,
              at_tick: Tick, seq: u32 }` (`seq` optional but recommended).
          - `HealthDelta { entity: EntityId, delta: i32, death: bool }`.

        - [x] Ordering: reduce multiple `DamageEvent`s for an entity
          deterministically within a tick.

      - [x] Update the Bevy → DBSP → Bevy marshalling layer to publish health
        snapshots into the circuit and apply circuit-emitted health deltas back
        onto ECS components.

        - [x] Snapshot cadence: publish the full `HealthState` at the start of
          each tick and apply `HealthDelta` after `circuit.step()`.

        - [x] Authority: treat DBSP as the single writer and prohibit
          out-of-band ECS health mutation.

        - [x] Idempotency: apply each `HealthDelta` at most once per
          `(entity, at_tick, seq)` triple; ignore duplicates and log at debug
          with a counter. On missing deltas, carry forward last applied state
          (no-op) and emit a resynchronisation metric.

      - [x] Add data-driven tests—`rstest` fixtures and headless Bevy BDD
        scenarios—covering health synchronisation across the circuit boundary.

        - [x] Acceptance: achieve tick-bounded convergence where ECS and circuit
          health match within one tick.

        - [x] Edge cases: cover max-health clamps, zero-health death flags,
          large burst damage, and concurrent external plus derived damage.

    - [x] Implement a simple damage model (e.g., falling damage calculated from
     velocity upon landing).

      - [x] Detect landing events inside the circuit by tracking transitions
        from `Unsupported` to `Standing` alongside vertical velocity.

        - Edge detection: fire once on the boolean edge
          `Unsupported_prev && Standing_now`.

        - Debounce: add a per-entity cooldown of `LANDING_COOLDOWN_TICKS: u32`
          (default: 6 ticks). Document how ticks map to wall time in the engine.

### Tick timing

The fixed-step simulation runs at 1 Hz, as configured by `DELTA_TIME = 1.0` in
`src/constants.rs`. Each tick therefore spans 1,000 milliseconds of wall time.
With `LANDING_COOLDOWN_TICKS` defaulting to six ticks, landing suppression
lasts for 6,000 milliseconds before another fall-damage event may trigger.

        - Hysteresis: reuse the motion system's `z_floor` grace band to avoid
          chatter-induced re-triggers.

      - [x] Define a fall-damage operator that applies a safe-velocity threshold
        and scaling factor entirely within DBSP.

        - Units: velocity in world units per second; derive impact speed as
          `abs(vz_before_contact)` sampled from the last `Unsupported` frame
          immediately before contact detection.

        - Constants (defaults): `SAFE_LANDING_SPEED = 6.0`,
          `FALL_DAMAGE_SCALE = 4.0`. State that tuning may change but tests pin
          current values.

        - Clamp: never emit negative damage; apply `.floor()` before casting to
          `u16`. Clamp `vz_before_contact` by `TERMINAL_VELOCITY` before
          computing impact.

      - [x] Emit derived damage events into the `Damage` stream and reduce
        entity health through the circuit's health accumulator.

        - Determinism: ensure a stable per-tick ordering for multiple derived
          `DamageEvent`s targeting one entity.

      - [x] Cover the damage flow with DBSP unit tests and headless Bevy
        simulations demonstrating falling damage.

        - Tests: cover stair-step jitter (single hit), terminal-velocity caps,
          cooldown enforcement, and mixed external plus fall damage.

[health-type-defs]: ./lille-physics-engine-design.md#canonical-type-definitions

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
