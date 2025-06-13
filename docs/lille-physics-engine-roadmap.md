### **Physics Engine Implementation Roadmap**

This roadmap outlines the steps required to implement the physics engine described in `docs/lille-physics-engine-design.md`. Each phase includes behavioural testing tasks guided by `docs/bdd-strategies-for-differential-datalog-rulesets.md`.

#### **Phase 1: Kinematic Foundations**

- **Goal:** Realise the gravity and floor-height model.
- **Key Tasks:**
  - [ ] Define DDlog types and relations for positions, blocks and slopes as detailed in the design document.
  - [ ] Implement rules calculating `FloorHeightAt` and entity state (`IsUnsupported`/`IsStanding`).
  - [ ] Write Rust integration code to feed world data into DDlog and apply `NewPosition` outputs.
  - [ ] **Behavioural Tests:** Use the BDD approach to verify that entities correctly transition between standing and falling when terrain heights change. Snapshot the resulting DDlog deltas for regression tests.

#### **Phase 2: Dynamics (Forces and Friction)**

- **Goal:** Extend the engine with velocity, forces and friction.
  - **Key Tasks:**
    - [ ] Add new DDlog relations `Velocity`, `Force` and `NewVelocity` and implement the rules for accelerations and friction.
    - [ ] Expose helper functions from Rust for vector math as specified in the design.
    - [ ] Update the Bevy systems to read `NewVelocity` and produce final positions.
    - [ ] **Behavioural Tests:** Following the BDD strategy, craft scenarios covering force application and friction behaviour. Snapshot the expected velocity and position changes.

#### **Phase 3: Integration and Polish**

- **Goal:** Hook the physics systems into the game loop and ensure determinism.
  - **Key Tasks:**
    - [ ] Integrate physics updates into the existing DDlog/Bevy synchronisation pipeline.
    - [ ] Profile performance and tune constants such as `TERMINAL_VELOCITY`.
    - [ ] Finalise documentation and examples.
    - [ ] **Behavioural Tests:** Continue expanding BDD test coverage, ensuring incremental updates match snapshots across multiple ticks.

