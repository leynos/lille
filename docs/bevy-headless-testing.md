# Headless Testing Strategies for Bevy in CI

## 1. Introduction

The Bevy Engine, a refreshingly simple data-driven game engine built in Rust,
leverages an Entity Component System (ECS) architecture to facilitate modular
and performant game development.1 In modern software development, particularly
within the context of game development where complexity can escalate rapidly,
automated testing is an indispensable practice. Integrating automated tests
into Continuous Integration/Continuous Deployment (CI/CD) pipelines ensures
that new changes do not introduce regressions and that the codebase remains
robust and maintainable over time.3

Testing game logic, however, often presents unique challenges, especially when
graphical output or windowing systems are involved. Running tests in a headless
environment—that is, without a visible window or direct GPU rendering
output—offers a way to circumvent these challenges, allowing for focused
testing of game systems and logic in automated CI environments. This report
provides a comprehensive guide to implementing various testing strategies for
Bevy applications in Rust, with a specific focus on leveraging headless mode
within a CI framework. It will cover the foundational setup for headless
testing, core techniques for testing systems and components, methods for
crafting effective test scenarios by manipulating states, inputs, and time,
advanced strategies such as snapshot testing and Test-Driven Development (TDD),
and best practices for integrating these tests into CI pipelines like GitHub
Actions.

## 2. Setting Up Bevy for Headless Testing in CI

Executing Bevy applications in a headless mode is essential for automated
testing in CI environments where graphical output is typically unavailable or
undesirable. Headless mode allows the Bevy application to run its core logic,
update systems, and manage its ECS world without attempting to create a window
or render to a screen.4 This is particularly useful for integration tests that
simulate application behavior without the overhead and flakiness associated
with UI rendering.

To configure a Bevy application for headless operation, modifications to the
`RenderPlugin` are necessary. The key is to instruct Bevy's rendering backend,
which relies on WGPU, to not initialize any specific presentation backends.
This is achieved by setting the `backends` field within `WgpuSettings` to
`None`. Additionally, ensuring synchronous pipeline compilation can make test
execution more deterministic. The following code illustrates the setup within a
Bevy `App`:

```rust
use bevy::prelude::*;
use bevy::render::{
    settings::{RenderCreation, WgpuSettings},
    RenderPlugin,
};

fn main() { // Or within a #[test] function for actual tests
    App::new()
       .add_plugins(DefaultPlugins.set(RenderPlugin {
            synchronous_pipeline_compilation: true, // Ensures shaders are compiled before proceeding
            render_creation: RenderCreation::Automatic(WgpuSettings {
                backends: None, // Crucial for headless operation
               ..default()
            }),
        }))
        //.add_systems(Update, my_system_to_test) // Example system
       .run(); // In a test, app.update() would be used iteratively
}

// fn my_system_to_test() { /*... */ } // Example system
```

This configuration, particularly `backends: None`, signals to WGPU (Bevy's
underlying graphics API abstraction) to operate without a presentation surface
or swapchain, which are typically tied to a window.4 While this effectively
bypasses the final display output, much of the rendering logic, such as shader
compilation and render graph setup, might still occur to some extent, as the
application will "load everything your game would normally have".4 This
distinction is important: headless tests configured this way are excellent for
verifying game logic and system interactions that may depend on render-related
data structures being present, but they may not capture issues related to
specific GPU vendor quirks or driver bugs that only manifest during actual
rendering and presentation to a display. For projects where high visual
fidelity and rendering correctness are paramount, these headless tests should
be complemented by some form of on-target visual testing, even if executed less
frequently. The community project `bevy_geppetto`, for instance, has noted a
long-term interest in capturing and comparing visual output, which aligns with
this need for broader testing coverage.5

The setting `synchronous_pipeline_compilation: true` is also beneficial for CI
environments.4 In a typical game loop, asynchronous shader compilation can
improve performance by not blocking the main thread. However, for tests,
determinism is paramount. Synchronous compilation ensures that all shader
processing is completed before the application proceeds, eliminating a
potential source of variability in test execution times or states, especially
if tests depend on render state being fully established.

When choosing which plugins to include, `DefaultPlugins` (modified as shown)
provides an environment closer to the actual game, including essential
non-rendering plugins and resources like `Time` or `AssetServer` (for
non-graphical assets) that systems under test might rely on. If the systems are
truly isolated and have no dependencies on such resources or any rendering
aspects, `MinimalPlugins` could be an alternative to potentially speed up test
initialization. However, for most integration-style tests, the modified
`DefaultPlugins` approach is generally more robust.

## 3. Foundations: Testing Bevy Systems and Components

The core of testing Bevy applications lies in verifying the behavior of its
systems and the state of its components within the ECS framework. Bevy's
architecture lends itself well to a structured testing approach where
individual pieces of logic can be instantiated and examined with relative ease.

The fundamental workflow for testing systems or component interactions begins
with creating an instance of `App`: `let mut app = App::new();`.6 This `App`
instance serves as the container for the world, resources, systems, and
schedules that will be exercised during the test.

Managing Test Dependencies:

For a system to execute correctly, its dependencies must be available in the
App's World.

- **Resources:** Necessary resources are provided using
  `app.insert_resource(MyResource::default());`.6 This allows systems to query
  for and operate on shared data.
- **Events:** If systems produce or consume events, the event types must be
  registered: `app.add_event::<MyEvent>();`.6 Events can be sent for testing
  purposes either by directly accessing the world,
  `app.world_mut().send_event(MyEvent);`, or through an `EventWriter` in a
  dedicated setup system.
- **Systems:** The system(s) under test are added to a schedule, typically
  `Update`: `app.add_systems(Update, my_system_under_test);`. If multiple
  systems need to run in a specific order, they can be chained:
  `app.add_systems(Update, (system_a, system_b).chain());`.6

Entity and Component Setup:

Tests often require specific entities with particular components to exist in
the world. These are spawned using app.world_mut(): let entity_id =
app.world_mut().spawn(MyComponent { /\*… \*/ }).id();.6 For more complex or
common entity configurations, Bevy's Bundle trait can be used to group
components, simplifying setup.2

Executing Logic:

Once the App is configured with the necessary resources, entities, and systems,
the game logic is executed by calling app.update();. This method call runs one
cycle of the Update schedule and other relevant schedules (like FixedUpdate, if
configured and sufficient virtual time has passed). Multiple calls to
app.update() can simulate the passage of several frames or ticks.

Making Assertions:

After system execution, assertions are made to verify the outcomes:

- Component values can be checked by querying the `World`:

  ```rust
  assert_eq!(
      app.world().get::<MyComponent>(entity_id).unwrap().value,
      expected_value,
  );
  ```

- Entity despawning can be verified:

  ```rust
  assert!(app.world().get::<MyComponent>(entity_id).is_none());
  ```

- Event emission can be confirmed by reading events:

  ```rust
  let events = app.world().resource::<Events<MyEvent>>();
  let mut reader = events.get_reader();
  assert!(!reader.read(events).is_empty());
  ```

An illustrative example of testing a simple movement system:

```rust
#
struct Position { x: f32, y: f32 }
#
struct Velocity { dx: f32, dy: f32 }

fn movement_system(mut query: Query<(&mut Position, &Velocity)>) {
    for (mut pos, vel) in &mut query {
        pos.x += vel.dx;
        pos.y += vel.dy;
    }
}

#[test]
fn test_movement() {
    // Setup app
    let mut app = App::new();
    app.add_systems(Update, movement_system);

    // Setup test entities
    let entity = app.world_mut().spawn((
        Position { x: 0.0, y: 0.0 },
        Velocity { dx: 1.0, dy: 2.0 },
    )).id();

    // Run systems
    app.update();

    // Check resulting changes
    let position = app.world().get::<Position>(entity).unwrap();
    assert_eq!(*position, Position { x: 1.0, y: 2.0 });
}
```

This common pattern of `App::new() -> setup -> app.update() -> assert` 2
creates a highly controlled "micro-loop" for testing game logic. This
capability is a direct consequence of Bevy's architectural design, where the
`App` and `World` are central, programmatically manipulable constructs. The
explicit `app.update()` call drives registered systems through their schedules,
allowing tests to precisely control the execution flow and make assertions on a
well-defined world state after a specific number of "game ticks." This
fine-grained control contrasts with testing in some other game engines where
the main loop might be more opaque, or systems could be tightly coupled to
engine singletons, necessitating more complex mocking.

Consequently, developers can write very focused tests for individual systems or
small groups of interacting systems with minimal boilerplate code. This
encourages a testing approach akin to unit testing for game logic, which can
significantly improve code quality and reduce the likelihood of regressions.
The inherent modularity of Bevy's ECS, which allows for the easy addition and
removal of systems, resources, and entities on a per-test basis 2, is
fundamental to achieving this level of test isolation and control.

## 4. Crafting Effective Test Scenarios

Beyond testing individual systems in isolation, effective testing often
requires simulating more complex scenarios involving state changes, user
inputs, and the passage of time. Bevy provides mechanisms to control these
aspects programmatically, enabling robust scenario-based testing.

### 4.1. Leveraging Bevy States for Test Isolation

Bevy's `States` system is a powerful feature for managing the overall flow of a
game, such as transitions between a main menu, gameplay, and pause screens.9
This same system can be adeptly repurposed for orchestrating test scenarios,
providing a structured way to manage setup, execution, and teardown phases
within a test.

To use states for testing, one can define an `enum` specific to test phases
(e.g., `TestSetup`, `TestRunning`, `TestAssertions`). Systems can then be
scheduled to run during specific state transitions or while a particular state
is active.

- **Setup and Teardown:** The `OnEnter` and `OnExit` schedules are invaluable
  for test hygiene. For instance,
  `app.add_systems(OnEnter(MyTestState::Setup), my_test_setup_system);` can
  prepare the world with necessary entities and resources, while
  `app.add_systems(OnExit(MyTestState::TearDown), my_test_cleanup_system);` can
  ensure these are removed, preventing interference between tests.9

- **StateScoped Entities:** A best practice for managing entity lifecycles in
  conjunction with states is the use of `StateScoped(MyState::InGame)`
  components.10 Entities spawned with such a component are automatically
  despawned when the application exits the specified state. This greatly
  simplifies cleanup logic in tests. For example:
  `commands.spawn((Name::new("TestPlayer"), StateScoped(GameState::InGame), PlayerComponent));`
  .

- **Programmatic State Transitions:** Tests can trigger state changes by
  modifying the `NextState<MyTestState>` resource:

  ```rust
  app.world_mut()
      .resource_mut::<NextState<MyTestState>>()
      .set(MyTestState::Running);
  app.update();
  ```

  The `app.update()` call then processes the state transition and runs
  associated `OnEnter`/`OnExit` schedules.

- **Conditional System Execution:** Systems under test, or assertion systems,
  can be made to run only when the test scenario is in the appropriate state
  using the `in_state(MyState::MyVariant)` run condition.9

Employing `States` for test orchestration elevates Bevy testing from simple
system invocations to the management of mini state machines within the tests
themselves. This allows for the definition of more complex scenarios, such as a
setup phase, an action phase where inputs might be simulated over several
frames, an assertion phase, and a cleanup phase, all controlled by well-defined
state transitions. Such a structured approach enhances the readability and
maintainability of tests and reduces the likelihood of errors arising from
manually managed setup and teardown logic scattered across test functions.
Furthermore, it mirrors how the actual game might utilize states, making the
tests more representative of real application behavior. The `StateScoped`
component, in particular, directly addresses the common problem of resource
leakage or entity persistence between test runs, which is crucial for
maintaining the stability and reliability of CI pipelines.10

### 4.2. Simulating Inputs for Interactive Systems

Many game systems are designed to react to player inputs, such as keyboard
presses, mouse clicks, or gamepad movements. In an automated testing
environment, these inputs must be simulated programmatically. Bevy's input
handling, which relies on resources like `ButtonInput<KeyCode>`,
`ButtonInput<MouseButton>`, and `Axis<GamepadAxis>`, makes this simulation
straightforward.

To simulate input, tests can directly insert or modify the relevant input
resource before calling `app.update()`. For example, to simulate a spacebar
press:

```rust
// Within a test function:
let mut app = App::new();
// Add systems that respond to KeyCode::Space, etc.
// app.add_systems(Update, system_reacting_to_spacebar);

let mut key_input = ButtonInput::<KeyCode>::default();
key_input.press(KeyCode::Space); // Simulates the key being held down
// For a single-frame press event, use key_input.just_press(KeyCode::Space);
app.insert_resource(key_input);

app.update(); // Run systems that will read this input state

// To simulate release or clear for the next frame:
// let mut key_input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
// key_input.release(KeyCode::Space);
// key_input.clear(); // Important for just_pressed/just_released states
// app.update(); // Run another frame with the updated input
```

This pattern is demonstrated in the `spawn_enemy_using_input_resource` test
within Bevy's own examples.6 For testing sequences of inputs (e.g., press, hold
for several frames, release), one would modify the input resource and call
`app.update()` iteratively.

The resource-based nature of Bevy's input system is a significant advantage for
testability. Systems consuming input data do not need to be aware of whether
the `ButtonInput` resource was populated by actual hardware events or by test
code; they only interact with its current state. This design avoids the need
for complex mocking of hardware event loops or platform-specific input APIs.
Consequently, testing game controls, UI interactions, and any logic contingent
on discrete or continuous input states is greatly simplified, facilitating the
achievement of high test coverage for player-facing mechanics.

### 4.3. Controlling Time in Tests

Real-time progression is generally unsuitable for automated tests due to its
inherent unpredictability and the slowness it would impose on test execution.
Bevy distinguishes between `Time<Real>` (wall-clock time) and `Time<Virtual>`
(game simulation time).11 `Time<Virtual>` is the crucial resource for testing
time-dependent logic, as it can be paused, scaled, and advanced
programmatically. During the `Update` schedule, Bevy automatically uses
`Time<Virtual>` as the default `Time` resource.12

While tests *can* directly manipulate `Time<Virtual>` (e.g.,
`virtual_time.pause();`, `virtual_time.set_relative_speed(2.0);` 11), the
advancement of time is typically handled by `app.update()`. Each call to
`app.update()` advances virtual time by a delta, which in turn can trigger
`FixedUpdate` schedules if a `FixedTime` resource is configured (e.g.,
`app.insert_resource(Time::<Fixed>::from_hz(60.0));`). The
`how_to_test_systems.rs` example implicitly relies on `app.update()` advancing
time by one tick.6

For more precise control over time progression in tests, especially for
`FixedUpdate` systems or simulating specific durations:

1. Configure `Time<Fixed>` with the desired tick rate.
2. Call `app.update()` repeatedly. Each call will advance `Time<Virtual>`, and
   the `FixedUpdate` schedule will run when enough virtual time has accumulated
   according to `Time<Fixed>`. The `Time` resource also includes a `max_delta`
   setting, which prevents excessively large time jumps if the application
   freezes or updates infrequently.11 In tests, this might be set to a very
   large value if large simulated time steps are intended, or time progression
   can be managed through repeated `app.update()` calls.

The abstraction provided by `Time<Virtual>` is fundamental for creating
deterministic and efficient tests for time-sensitive game logic, such as
animations, cooldowns, or physics updates. It allows the test environment to
dictate the passage of "game time," irrespective of the actual execution speed
of the test machine or CI runner. This means gameplay mechanics that unfold
over several seconds or minutes in real gameplay, like a 10-second ability
cooldown, can be tested in milliseconds by controlling how `Time<Virtual>` is
advanced or by iterating `app.update()` an appropriate number of times.
Understanding the relationship between `Time<Virtual>`, `Time<Fixed>`, and the
`app.update()` mechanism is key to accurately simulating time for systems
operating in different Bevy schedules.

## 5. Advanced Testing Strategies for Bevy

Beyond foundational system and component testing, more advanced strategies can
provide deeper insights into application behavior and help catch a wider range
of regressions. These include snapshot testing, comprehensive integration
testing, and adopting Test-Driven Development.

### 5.1. Snapshot Testing

Snapshot testing involves capturing a "snapshot" of a known-good state of the
application—such as component data, a serialized representation of part of the
game world, or even a rendered image—and then comparing subsequent test runs
against this baseline. Deviations from the snapshot signal potential
regressions.

- **Data Snapshotting:** For Bevy, this typically means serializing relevant
  components, resources, or even entire entity hierarchies into a
  human-readable format like RON (Rusty Object Notation) or JSON. Crates like
  `insta` 13 are excellent for managing these snapshot files, automatically
  storing them and providing easy workflows for updating them when changes are
  intentional. A test might query for entities with specific components,
  collect their data, serialize it, and then assert its consistency using
  `insta::assert_ron_snapshot!`.
- `bevy_geppetto`**:** This proof-of-concept crate offers a specialized form of
  snapshot testing for Bevy, drawing inspiration from ideas discussed by Chad
  Nauseam.2 Its current focus is on capturing sequences of input events during
  a test run and serializing them (as RON files in a `snapshots/inputs`
  directory). These recorded inputs can then be replayed in subsequent test
  runs (`cargo test --test $TEST_NAME -- -r`) to ensure consistent behavior. It
  also supports capturing inputs at a specified frame rate
  (`cargo test --test $TEST_NAME -- -c $FRAMES_PER_SEC`). A notable
  characteristic of `bevy_geppetto` is its requirement to run tests on the main
  thread, often due to dependencies like `winit` that have main-thread
  constraints for windowing or event loop interactions, even if a window isn't
  visibly rendered.5 This has implications for CI setup. While visual output
  comparison was a long-term goal for `bevy_geppetto`, it is currently out of
  scope for the crate.5
- **Visual Snapshotting (Conceptual):** Although not directly supported by
  mainstream Bevy testing tools yet, the concept of visual snapshotting
  (comparing rendered images pixel by pixel or using perceptual difference
  algorithms) is a powerful technique. Bevy's own screenshot example 14
  demonstrates the basic capability of capturing images, which could form the
  basis for such a system. However, visual snapshots come with challenges,
  including large file sizes, sensitivity to minor rendering variations across
  different hardware or driver versions, and the complexity of managing updates.

The emergence of tools like `bevy_geppetto` 5 and the expressed interest in
visual snapshotting reflect a maturing testing ecosystem around Bevy.
Developers are seeking more comprehensive methods for regression testing that
go beyond simple value assertions, aiming for techniques common in other
domains like web development (e.g., Jest's snapshot testing, visual diffing
tools). Input recording and replaying, as facilitated by `bevy_geppetto`, is a
step towards testing emergent behaviors that arise from complex interactions
over simulated time. While powerful, snapshot tests, particularly visual ones,
demand careful management regarding the updating of snapshots and handling
minor, acceptable differences. Data snapshots are generally more robust and
easier to integrate into CI workflows. The main-thread requirement noted for
`bevy_geppetto` 5 is a practical constraint that CI configurations must
accommodate.

### 5.2. Integration Testing Complex Interactions

Integration tests verify how multiple systems, components, and events interact
to produce a desired outcome or emergent game behavior. The focus is on the
"contracts" between different parts of the application: if System A produces an
event or modifies a component, does System B (and subsequent systems) react
correctly?

Designing effective integration tests involves:

1. Setting up entities with a diverse set of components that will be processed
   by the multiple systems under scrutiny.
2. Triggering an initial event, input, or state change that kicks off the
   interaction.
3. Running `app.update()` one or more times to allow the sequence of
   interactions to unfold across several frames or game ticks.
4. Asserting on the final state of relevant components, resources, or the
   emission of expected events.

For example, testing a complete combat interaction might involve a
`PlayerAttackSystem` generating a `DamageEvent`, which is then processed by a
`HealthSystem` on an enemy entity. This could lead to the enemy's health
dropping, potentially triggering a `DeathEvent` if health reaches zero, which
in turn might be handled by a `LootDropSystem` and an `EnemyCleanupSystem`. The
integration test would set up the player and enemy entities, simulate the
attack, and then verify the enemy's health reduction, the correct emission of
death and loot events, and the eventual despawning of the enemy entity. Bevy's
`States` system 9 can also be effectively used here to manage different phases
of a complex integration test, such as "CombatInProgress" or
"PostCombatResolution."

Bevy's ECS architecture inherently facilitates integration testing.2 Systems
explicitly declare their data dependencies through queries, and inter-system
communication often occurs via well-defined Events or shared Resources. This
explicitness at the "seams" between systems makes it easier to understand and
test the flow of data and control, mitigating the "spaghetti-code problem" that
can make integration testing notoriously difficult in other programming
paradigms.2 While unit tests are crucial for verifying the internal logic of
individual systems, integration tests provide confidence that these systems
compose correctly to produce the intended gameplay behaviors. In game
development, where complex emergent behavior is a hallmark, robust integration
tests are particularly valuable. The balance between unit and integration tests
can vary; some codebases, especially those with many interacting parts, may
benefit significantly from a strong emphasis on integration testing.3

### 5.3. Test-Driven Development (TDD) with Bevy

Test-Driven Development (TDD) is a software development methodology where tests
are written *before* the actual application code is implemented.16 The typical
TDD cycle is "Red-Green-Refactor":

1. **Red:** Write a test that defines a desired improvement or new function.
   This test will initially fail because the code doesn't exist or isn't
   correct yet.
2. **Green:** Write the minimal amount of code necessary to make the test pass.
3. **Refactor:** Clean up the newly written code (and the test code itself) for
   clarity, efficiency, and good design, ensuring all tests still pass.

Applying TDD to Bevy development involves 16:

- Identifying a specific unit of behavior to implement, such as a new game
  system, a complex interaction between components, or a piece of resource
  management logic.
- Writing a test function that sets up a Bevy `App`, spawns any necessary
  entities with their components, inserts required resources, adds the
  (yet-to-be-written) system, and then makes assertions about the expected
  outcome after `app.update()` is called.
- Implementing the system or component logic.
- Running the test repeatedly until it passes.
- Refactoring the implementation and test code.

The benefits of TDD include fostering better, more modular designs, enabling
early detection of bugs, increasing developer confidence when making changes,
and reducing long-term technical debt.16 For instance, when developing a
character animation system, a TDD approach might start with a test asserting
that a newly spawned character entity defaults to an "idle" animation state
after the animation system runs for the first time.16

Adopting TDD in a Bevy context encourages developers to think critically about
system interactions and data transformations from the perspective of
testability and desired behavior *before* writing any implementation code. This
process naturally leads to clearer, more decoupled designs because, to write a
test for a system, its inputs (queries, resources, events) and expected outputs
(component changes, new events) must be precisely defined. This forced clarity
on a system's responsibilities and its API often results in more focused and
less coupled code. While TDD can sometimes feel slower during rapid prototyping
phases, which are common in game development, its benefits in terms of
long-term maintainability and stability, especially for core game systems, can
be substantial. The idea of "playing your tests," as mentioned in relation to
creating interactive test scenes 2, can be seen as a game development-friendly
adaptation of the TDD feedback loop, allowing for both automated verification
and interactive debugging of the feature under development.

## 6. Automating Bevy Tests with CI (GitHub Actions)

Automating the execution of Bevy tests within a Continuous Integration (CI)
pipeline is crucial for maintaining code quality and ensuring that regressions
are caught early. GitHub Actions is a popular platform for CI and provides
robust support for Rust projects.17

A basic GitHub Actions workflow for running Rust tests typically involves the
following steps:

1. **Setting up the Rust toolchain:** Using an action like
   `actions-rust-lang/setup-rust-toolchain` to install the desired Rust version.
2. **Checking out the code:** Using `actions/checkout@v4` to get the repository
   content.
3. **Caching Cargo dependencies:** Employing `actions/cache@v4` to cache
   directories like `~/.cargo` and the project's `target/` directory. This is
   critical for Rust projects due to potentially long compilation times;
   effective caching significantly speeds up subsequent CI runs.18
4. **Running tests:** Executing `cargo test`, potentially with flags like
   `--all-features` or targeting specific test suites.

When incorporating headless Bevy tests into this workflow, a few additional
considerations arise:

- **System Dependencies:** The CI runner environment must have any necessary
  libraries for headless operation. While the `backends: None` WGPU setting
  aims to minimize external graphics dependencies, if WGPU (or underlying
  graphics drivers) still attempts to initialize certain low-level graphics
  components, a minimal set of graphics libraries (e.g., Vulkan SDK components
  or X server libraries like Xvfb on Linux for a virtual display) might be
  needed. Bevy's own CI often uses `ubuntu-latest` runners, which generally
  have good support for these.18

- **Main Thread Requirement:** If tests utilize crates like `bevy_geppetto` 5 or
  otherwise have components (often related to `winit` or event loops) that
  require execution on the main thread, the standard `cargo test` invocation
  might not suffice. Such tests often need to be structured as separate
  integration test binaries defined in `Cargo.toml` using the `[[test]]`
  syntax, for example:

  ```toml
  [[test]]
  name = "my_e2e_test"
  path = "tests/e2e/my_e2e_test.rs"

  ```

  These are then run using `cargo test --test my_e2e_test`. This ensures each
  such test runs in its own process where it can control the main thread.5

Bevy's own CI workflow, typically found at `.github/workflows/ci.yml` in the
Bevy repository, serves as an excellent reference.18 Key features and
configurations observed in such workflows include:

- **Triggers:** CI runs are often triggered on `merge_group` events,
  `pull_request` events, and pushes to specific branches like `release-*`.
- **Environment Variables:**
  - `CARGO_TERM_COLOR: always`: For enhanced readability of logs.
  - `CARGO_INCREMENTAL: 0`: Often disabled in CI to ensure cleaner, more
    reproducible builds, though it can increase build times compared to local
    incremental builds.
  - `CARGO_PROFILE_TEST_DEBUG: 0`, `CARGO_PROFILE_DEV_DEBUG: 0`: To control the
    inclusion of debug information in test and development profiles,
    potentially reducing binary sizes and compile times for CI artifacts.
  - Specialized flags like `RUSTFLAGS` and `MIRIFLAGS` for enabling tools like
    Miri.
- **Jobs:** A comprehensive CI setup often includes multiple jobs:
  - Basic compilation checks (`check-compiles`).
  - Extensive test runs across a matrix of configurations
    (`test-various-configs`), covering different feature flag combinations,
    operating systems (e.g., `windows-latest`, `macos-latest`,
    `ubuntu-latest`), and Rust toolchain versions (stable, beta, nightly). This
    matrix testing is a best practice for ensuring broad compatibility.
  - Execution with Miri (`miri`) to detect undefined behavior.
  - Static analysis and linting (`clippy`).
  - Code formatting checks (`rustfmt`).
  - Spell checking in documentation and comments (`typos`).
- **Caching Strategy:** The caching configuration in Bevy's CI is typically
  fine-tuned, specifying paths such as `~/.cargo/bin/`,
  `~/.cargo/registry/index/`, `~/.cargo/registry/cache/`, `~/.cargo/git/db/`,
  and the project-local `target/` directory. The cache key is usually composed
  of the operating system, Rust toolchain version, the hash of `Cargo.lock`,
  and potentially other custom strings to ensure cache validity.18

Bevy's own CI configuration 18 provides a battle-tested template that
demonstrates mature practices for testing a complex Rust project. Its use of
matrix builds to cover diverse environments, integration of Miri for memory
safety checks, and sophisticated caching strategies are indicative of a serious
approach to maintaining code quality and build efficiency. Developers setting
up CI for their own Bevy projects can save significant time and avoid common
pitfalls by referencing this existing configuration. The choice of
`CARGO_INCREMENTAL: 0` is a common CI trade-off: it ensures more consistent
builds at the cost of speed; projects experiencing very long CI times might
experiment with enabling it, carefully monitoring for any introduced flakiness.
The platform or dependency constraints (like `winit` requiring the main thread
for some operations) that necessitate specific test structures 5 are practical
realities that the CI testing strategy must accommodate.

## 7. Best Practices for Writing Testable Bevy Code

The ease and effectiveness of testing a Bevy application are significantly
influenced by how the application code itself is structured. Adhering to
certain design principles not only leads to better overall code quality but
also makes writing unit, integration, and headless tests considerably simpler.

- **Designing Clear and Focused Components:** Components are the fundamental
  data containers in Bevy's ECS. They should ideally be plain Rust structs or
  enums that derive the `Component` trait.8

  - **Granularity:** Keep components small and focused on a single, cohesive
    piece of data or concern. As a guideline, "all of the data on a component
    should generally be accessed at once".19 This avoids overly large
    components where only a fraction of the data is relevant to most systems,
    improving clarity and potentially performance.
  - **Behavior vs. Data:** Components should primarily hold data. Logic
    operating on this data resides in systems. Avoid writing "anemic tests"
    that merely check simple getters or setters on components if those
    components encapsulate no inherent logic 20; instead, test the behavior of
    systems that interact with these components.
  - **Specialized Components:** Utilize marker components (empty structs
    deriving `Component`) for tagging entities or filtering queries
    effectively.8 Newtype wrappers around simple types (e.g.,
    `struct PlayerXp(u32);`) can create distinct component types, preventing
    ambiguity and enhancing type safety.8

- **Effective Use of Events for Decoupled Communication:** Bevy's event system
  is a powerful tool for enabling communication between systems without
  creating tight coupling.10

  - **Decoupling:** Prefer events for signaling occurrences or passing data that
    may affect multiple, disparate parts of the game. Systems can subscribe to
    events (`EventReader`) or send them (`EventWriter`) without needing direct
    knowledge of each other.
  - **Ordering:** When an `EventReader` and its corresponding `EventWriter`
    operate within the same frame (e.g., both in the `Update` schedule), ensure
    explicit ordering (e.g., using `.before()`, `.chain()`, or `SystemSet`
    ordering) to prevent events from being missed due to systems running in an
    unintended sequence.
  - **Run Conditions:** Systems that are designed to react only when specific
    events occur should often include the presence of those events as part of
    their run criteria.

- **Structuring Systems for Testability:** Systems contain the application's
  logic.

  - **Single Responsibility:** Each system should have a clear, single
    responsibility.19 This makes them easier to understand, test, and maintain.
  - **Well-Defined Interfaces:** The inputs to a system (its Queries, accessed
    Resources, consumed Events) and its outputs (changes to Components or
    Resources, sent Events via Commands or EventWriters) should be well-defined.
  - **Isolation:** When testing, aim to test systems in isolation or in small,
    controlled groups. As suggested, one can "create a standalone stage with
    the systems you want to run, and manually run it on the World".7 This
    aligns with the idea that "ideally every test just adds the bare-minimum
    components it needs… This is the anti-spaghetti property that makes me
    really love ECS".2

- **Managing Entity Lifecycles and IDs:** Proper management of entity lifecycles
  is crucial for both game correctness and test hygiene.10

  - **Naming Entities:** Spawning top-level or significant entities with a
    `Name` component (e.g., `Name::new("Player")`) greatly aids in debugging,
    as it allows for easier identification of entities in logs or inspectors.
  - **Automated Cleanup with** `StateScoped`**:** The preferred method for
    ensuring entities are cleaned up when they are no longer relevant (e.g.,
    when exiting a game state or a test scenario) is to spawn them with a
    `StateScoped(MyState::Variant)` component. This automatically despawns the
    entity when the application transitions out of `MyState::Variant`. This is
    a significant improvement for test hygiene, preventing state leakage
    between tests.
  - **Strong IDs:** For entities that need to persist across game sessions
    (e.g., through saving and loading) or be reliably referenced over a
    network, do not rely solely on Bevy's `Entity` ID, which is ephemeral and
    can be reused. Instead, implement custom "strong ID" types (e.g., a
    `struct QuestId(u32);`) that provide stable, unique identification.

- **Co-locating** `OnEnter`**/**`OnExit` **Systems for State Transitions:** When
  defining systems that handle the setup (`OnEnter(MyState)`) and teardown
  (`OnExit(MyState)`) logic for a particular application state, it's good
  practice to register these systems together in the code.10 This improves
  readability and makes it less likely that cleanup logic will be forgotten or
  mismatched with setup logic.

- **System Scheduling Boundaries:** Systems added to the main `Update` schedule
  should generally be bounded by run conditions based on `State` and/or
  organized into `SystemSet`s.10 This provides better control over when systems
  execute, improves predictability, and can optimize performance by preventing
  unnecessary system runs.

Adherence to these architectural patterns—such as small, focused components,
single-responsibility systems, and event-driven communication—fosters
modularity. Modularity, in turn, is a prerequisite for effective unit and
integration testing, as it allows different parts of the system to be tested in
isolation or in controlled combinations. Features like `StateScoped` entities
directly address common testing pain points like state leakage and manual
cleanup. Similarly, the use of strong IDs enables more reliable testing of
persistence and networking logic. Therefore, designing for testability from the
outset by adopting these practices makes the process of writing comprehensive
tests significantly easier and the tests themselves more robust and
maintainable. Ignoring these principles can lead to tightly coupled code that
is difficult to test effectively, especially in automated CI environments.2 The
evolution of Bevy, including features like `StateScoped` entities 10,
demonstrates a trend towards providing engine-level support for patterns that
enhance both application structure and testability.

## 8. Helpful Crates for Bevy Testing

While Bevy's core engine and its ECS architecture provide a strong foundation
for testing, the broader Rust ecosystem offers a wealth of crates that can
further enhance testing capabilities, streamline test writing, and enable more
sophisticated verification strategies. Since Bevy tests are fundamentally Rust
tests (typically using the `#[test]` attribute and `cargo test` runner), many
general-purpose Rust testing utilities can be seamlessly integrated into a Bevy
project's testing workflow.

The following table highlights some particularly useful crates from the Rust
testing ecosystem 13 and their potential applications in Bevy testing:

| Crate Name  | Key Feature(s)                                           | Use Case in Bevy Testing                                                                                                                                                   |
| ----------- | -------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| rstest      | Fixture-based testing, parameterized test cases.         | Reduce boilerplate for tests with common setup logic (fixtures). Run the same test logic with multiple different input values or configurations.                           |
| insta       | Snapshot testing library.                                | Capture and verify complex component states, resource data, or serialized entity hierarchies (e.g., in RON/JSON format). Manages snapshot file updates.                    |
| mockall     | Powerful mock object library for traits.                 | Mock external dependencies or trait implementations if they are difficult to control directly in a test environment. Less frequently needed in Bevy due to ECS data focus. |
| proptest    | Property-based testing (Hypothesis-like).                | Define properties that should hold true for a range of inputs; proptest generates diverse test cases to find edge cases in system logic.                                   |
| serial_test | Allows tests to be run serially rather than in parallel. | Useful for tests that interact with shared, mutable global state or external resources not managed by Bevy's World (e.g., file system access).                             |
| test-log    | Initializes logging/tracing infrastructure for tests.    | Automatically sets up tracing or log subscribers before each test, making it easier to inspect detailed log output when debugging failing tests.                           |
| assert_cmd  | Utilities for testing command-line applications.         | Potentially useful if the Bevy application includes CLI tools, or if test helper scripts are invoked as part of the testing process.                                       |

The applicability of these general-purpose Rust testing crates to Bevy projects
underscores the strength of Rust's ecosystem and Bevy's successful integration
within it. Developers are not confined to the testing utilities provided
directly by Bevy but can draw upon a rich set of mature, widely-used tools.
This allows for the construction of more sophisticated, robust, and
maintainable test suites, tailored to the specific needs of the Bevy
application being developed. For example, `proptest` can be invaluable for
uncovering subtle bugs in complex game logic by exploring a wide input space,
while `insta` can simplify the verification of intricate game states that would
otherwise require numerous individual assertions.

## 9. Conclusion: Building Quality Bevy Applications with Confidence

This guide has detailed a comprehensive array of strategies for testing Bevy
applications in Rust, with a particular emphasis on leveraging headless mode
for effective automation within Continuous Integration environments. Key
approaches include the foundational setup for headless execution, techniques
for testing individual systems and components by manipulating the `App` and
`World`, methods for crafting effective test scenarios through the control of
Bevy `States`, simulated inputs, and virtual time, and advanced strategies such
as data-driven snapshot testing and the application of Test-Driven Development
principles. Furthermore, the integration of these tests into CI pipelines,
drawing from Bevy's own robust CI practices, and adherence to best practices
for writing testable Bevy code have been explored.

The core message is that a disciplined and comprehensive approach to testing,
one that fully utilizes the strengths of Bevy's Entity Component System and its
capability for headless operation, is not merely an adjunct to development but
a critical component for building high-quality, maintainable, and robust Bevy
applications. The modularity inherent in ECS, combined with Bevy's programmatic
control over its execution loop, resources, and state, provides a fertile
ground for a wide spectrum of testing methodologies.

Automated testing, especially when integrated into CI, should be viewed as an
enabler rather than a chore. It facilitates faster iteration cycles by
providing rapid feedback on changes, allows for safer refactoring by guarding
against regressions, and ultimately instills greater confidence in the
development process. The patterns and tools discussed—from `StateScoped`
entities simplifying test cleanup to the `Time<Virtual>` resource enabling
deterministic testing of time-dependent logic—are available to Bevy developers.

The comprehensive nature of the testing strategies applicable to Bevy, ranging
from low-level unit tests of systems to sophisticated CI automation workflows,
indicates the engine's maturation and its suitability for serious, long-term
projects where reliability and maintainability are paramount. As the Bevy
community continues to expand and more complex applications are developed, the
widespread knowledge and adoption of these testing strategies will be a key
determinant in the overall quality and success of projects within the Bevy
ecosystem. Encouraging a "testing culture," where testing is an integral part
of the development lifecycle, will empower developers to build with Bevy more
confidently and effectively.
