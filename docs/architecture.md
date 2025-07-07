# Lille Architecture

## 1. Architectural Philosophy

The architecture of Lille is founded upon a strict **separation of concerns**
between the *state* of the simulation and the *logic* that governs it. This
principle manifests in the choice of our two core technologies:

1. **The Bevy Engine**: Serves as the application framework and the **state
   container**. Its primary responsibilities are rendering, user input, asset
   management, and acting as the definitive source of truth for all world data
   via its Entity-Component-System (ECS).

2. **The DBSP Dataflow Circuit**: Serves as the declarative **logic engine**. It
   contains the complete set of rules for physics, geometry, and agent
   behaviour, expressed as a pure-Rust, incremental dataflow graph.

This bifurcation allows us to manage complexity effectively. The imperative,
event-driven world of Bevy is kept clean and focused on presentation and
orchestration, while the complex, relational rules of the simulation are
isolated within a maintainable, testable, and highly performant declarative
model.

## 2. Core Components and Data Flow

The engine operates in a continuous, unidirectional data-flow loop that executes
on every simulation tick.

```text
+--------------------------------+      +----------------------------------+
|        Bevy ECS World          |      |         DBSP Circuit             |
| (Source of Truth for State)    |      | (Source of Truth for Logic)      |
|                                |      |                                  |
|  [ Components: Transform,    ] |----->| [ Input Streams: Position,     ] |
|  [   Velocity, Target, etc.  ] | 1.   | [   Block, Velocity, etc.      ] |
|                                |      |                                  |
|                                |      |         (circuit.step())         |
|                                |      |                |                 |
|                                |      |                v                 |
|                                |      |                                  |
|  [ Components are updated   ]  |<-----| [ Output Streams: NewPosition, ] |
|  [   from circuit outputs.  ]  | 3.   | [   NewVelocity, etc.          ] |
+--------------------------------+      +----------------------------------+
```

### The Tick Cycle

1. **Data Extraction (ECS → DBSP)**: A set of dedicated Bevy "input systems" run
   at the beginning of the frame. They query the ECS for all components relevant
   to the simulation logic (e.g., `Transform`, `Velocity`, `Block`, `Target`).
   This data is collected and fed as a batch of updates into the corresponding
   `InputHandle`s of the DBSP circuit.

2. **Declarative Computation (DBSP)**: A central system calls `circuit.step()`
   exactly once. This single function call triggers DBSP to process the entire
   batch of input changes. The engine incrementally propagates these changes
   through the dataflow graph, re-computing only what is necessary. This step is
   where all physics calculations, geometry checks, and AI decisions occur,
   resulting in a new set of output streams.

3. **State Synchronisation (DBSP → ECS)**: A final set of Bevy "output systems"
   run. They read the records from the circuit's output handles (e.g.,
   `NewPosition`, `NewVelocity`). For each output record, the system finds the
   corresponding entity in the ECS and updates its components with the new,
   authoritative values calculated by the circuit.

After this cycle, the Bevy ECS holds the new, consistent state of the world,
ready for the next frame's rendering and the start of the next simulation tick.

## 3. Logic Placement and Boundaries

The separation of concerns dictates clear boundaries for where different types
of logic should be implemented.

### Logic within the DBSP Circuit

The DBSP circuit is the ideal place for any logic that is **relational** and
**data-centric**. It excels at transforming collections of data.

- **Physics and Geometry**: Collision detection (via floor height calculation),
  gravity, friction, and force application.

- **Simple, Reactive AI**: Agent behaviours that can be expressed as direct
  reactions to the current state, such as fleeing from a source of fear, seeking
  a target, or responding to damage.

- **Game Rules**: Any rule that can be modelled as a data transformation, such
  as "if an entity is standing on a 'lava' block, create a `Damage` event".

### Logic within Bevy Systems (Imperative Rust)

Bevy systems handle everything else. This code is typically imperative and
event-driven.

- **Orchestration**: The systems that manage the data flow to and from the DBSP
  circuit.

- **Rendering and I/O**: All interaction with the user, graphics card, file
  system, and network.

- **Complex, Stateful Algorithms**: Logic that does not fit the relational
  dataflow model. The canonical example is **A\* pathfinding**, which requires
  maintaining stateful data structures (open/closed sets, priority queues) and
  performing iterative, heuristic searches. Such algorithms are implemented in
  standard Rust, and their *results* (e.g., the next waypoint on a path) are fed
  into the DBSP circuit as just another input stream.

## 4. Advantages of this Architecture

This design represents a significant evolution from the previous DDlog-based
architecture.

- **Simplified Toolchain**: The entire engine is a standard Cargo project. There
  is no need for external compilers, `build.rs` code generation, or FFI
  bindings.

- **End-to-End Type Safety**: Data flows between Bevy and DBSP using native Rust
  types, eliminating a major class of potential integration errors.

- **Enhanced Testability**: The logic circuit can be tested in isolation (unit
  tests) and the entire data pipeline can be tested via headless Bevy apps (BDD
  tests), as detailed in `docs/testing-declarative-game-logic-in-dbsp.md`.

- **Performance and Clarity**: We retain the high performance of incremental
  computation while expressing complex rules in a clear, declarative style, as
  outlined in `docs/lille-physics-engine-design.md`.

This architecture provides a robust and scalable foundation for building the
complex, emergent world of Lille.
