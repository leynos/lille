# Testing Declarative Game Logic in DBSP

The migration of our core physics and geometry logic from DDlog to a pure-Rust
DBSP circuit has profound, positive implications for our testing strategy.
Because the dataflow logic is now standard Rust code, we can leverage the full
power of Rust's native testing ecosystem, from fine-grained unit tests to
high-level behavioural scenarios. This eliminates the indirection and complexity
of testing a foreign language module via an FFI boundary.

This document outlines our two-tiered approach to ensuring the correctness and
robustness of the DBSP-based world inference engine.

## 1. Unit Testing: Verifying Individual Dataflow Operators

At the lowest level, we must verify that each logical step in our dataflow
circuit behaves as expected. A "unit" in this context is a small, self-contained
part of the circuit—typically a single operator (`map`, `join`, `filter`,
`aggregate`) or a small chain of them. The goal is to test the operator's logic
in isolation, providing it with controlled inputs and asserting on its outputs.

The move to DBSP makes this remarkably straightforward. A test for a dataflow
operator is simply a standard Rust test function marked with `#[test]`.

**The Testing Pattern:**

1. **Instantiate a Circuit**: Create a new `RootCircuit` within the test
   function.

2. **Define Inputs and Outputs**: Add the necessary input and output streams to
   the circuit. For a unit test, you will only need the streams relevant to the
   operator(s) under test. For example, to test the `FloorHeightAt` logic, you
   would need `Block` and `BlockSlope` inputs and a `FloorHeightAt` output.

3. **Wire the Operator**: Construct the specific dataflow operator or
   sub-circuit you wish to test, connecting it to the defined inputs and
   outputs.

4. **Provide Input Data**: Use the `InputHandle` for each input stream to push a
   curated set of test records. These records represent the specific conditions
   you want to test (e.g., a single block, a block with a slope, no blocks).

5. **Execute the Circuit**: Call `circuit.step()` to process the inputs and
   compute the result.

6. **Assert on the Output**: Read the resulting collection from the output
   handle and assert that its contents match your expected outcome.

**Example: Unit-testing the** `HighestBlockAt` **operator**

```rust
#[test]
fn test_highest_block_aggregation() {
    let (mut circuit, (block_input, highest_block_output)) =
        CircuitBuilder::new()
            .add_input_zset::<Block>("blocks")
            .add_output_zset::<HighestBlockAt>("highest_blocks")
            .build_with_handles(|circuit, (blocks, highest_blocks)| {
                // The operator under test: group by (x, y) and find max z.
                let highest_block_stream = blocks.group_by_key(|b| (b.x, b.y))
                                                 .aggregate(Aggregate::max(|b| b.z))
                                                 .map(|((x, y), z)| HighestBlockAt { x, y, z });

                highest_block_stream.output_to(highest_blocks);
            });

    // Given: Two blocks at the same (x, y), one higher than the other.
    block_input.insert(Block { id: 0, x: 10, y: 20, z: 5 });
    block_input.insert(Block { id: 1, x: 10, y: 20, z: 8 });
    block_input.insert(Block { id: 2, x: 15, y: 25, z: 3 });

    // When: The circuit is executed.
    circuit.step().unwrap();

    // Then: The output should contain only the highest block for each (x, y).
    let output = highest_block_output.consolidate();
    assert_eq!(output.len(), 2);
    assert!(output.contains(&HighestBlockAt { x: 10, y: 20, z: 8 }));
    assert!(output.contains(&HighestBlockAt { x: 15, y: 25, z: 3 }));
}

```

This approach allows for precise, fast, and isolated verification of every
logical component in our physics engine.

## 2. Behaviour-Driven Development (BDD): Testing Emergent Behaviours

While unit tests verify individual operators, they do not guarantee that the
fully integrated circuit produces the correct high-level game behaviour. For
this, we employ a Behaviour-Driven Development (BDD) strategy. These tests
verify the emergent outcomes of the entire ECS-to-DBSP-to-ECS data pipeline.

We write these tests from the perspective of an observer describing a scenario
in plain language, following the "Given-When-Then" structure.

- **Given**: A specific, well-defined initial state of the game world.

- **When**: The game simulation advances by one or more ticks.

- **Then**: The state of the world has changed in a predictable way.

**Implementation with Headless Bevy:**

We implement these BDD scenarios using a headless Bevy `App`. This allows us to
construct a complete game world in memory, run the simulation, and inspect the
results, all without the overhead of rendering or user input.

1. **Given (World Setup)**: We construct a new `App` and populate its `World`
   with the entities and components required for the scenario. This includes
   spawning entities with `Transform` and `Velocity` components, creating
   `Block` entities, etc. We can use the `rstest` crate to create fixtures that
   provide pre-configured `App` instances for common scenarios (e.g.,
   `app_with_falling_entity`, `app_with_entity_on_slope`).

2. When (Simulation Tick): We call app.update(). This executes the Bevy
   schedule, which includes our custom systems for:

   a. Querying the ECS and feeding the data into the DBSP circuit's input
   handles.

   b. Calling circuit.step().

   c. Reading the results from the circuit's output handles and applying them
   back to the ECS components.

3. **Then (State Assertion)**: After the update, we query the `World` again to
   verify the outcome. We check if components have been updated as expected. For
   example, we might assert that a falling entity's `Transform.translation.z`
   has decreased, or that a standing entity's position now matches the
   calculated floor height.

**Example: BDD test for gravity**

```rust
use rstest::rstest;
use test_utils::fixtures::app_with_single_unsupported_entity; // An rstest fixture

#[rstest]
fn test_gravity_on_unsupported_entity(
    mut app_with_single_unsupported_entity: App
) {
    // Given: An application with a single entity floating in space
    // (provided by the rstest fixture).
    let entity_id = app_with_single_unsupported_entity
        .world
        .query::<Entity>()
        .iter(&app_with_single_unsupported_entity.world)
        .next()
        .unwrap();

    let initial_z = app_with_single_unsupported_entity
        .world
        .get::<Transform>(entity_id)
        .unwrap()
        .translation.z;

    // When: The game simulation runs for one tick.
    app_with_single_unsupported_entity.update();

    // Then: The entity's z-coordinate should have decreased due to gravity.
    let final_z = app_with_single_unsupported_entity
        .world
        .get::<Transform>(entity_id)
        .unwrap()
        .translation.z;

    assert!(final_z < initial_z);
    assert_eq!(final_z, initial_z - GRAVITY_PULL); // Assuming GRAVITY_PULL is a known constant
}

```

## Tooling and Best Practices

- **Code Coverage**: Since our logic is now pure Rust, we can use standard tools
  like `cargo-tarpaulin` to measure test coverage and identify gaps in our test
  suites.

- `rstest` **Fixtures**: We will continue to use `rstest` extensively to create
  reusable, composable test setups. This reduces boilerplate and makes tests
  easier to read and maintain.

- **Modularity**: The DBSP circuit itself should be constructed in a modular
  way, allowing different parts to be instantiated independently for unit tests.

By combining fine-grained unit tests of our dataflow logic with high-level,
BDD-style tests of the integrated system, we can build a comprehensive and
robust testing suite that ensures the correctness and stability of Lille's
physics engine.
