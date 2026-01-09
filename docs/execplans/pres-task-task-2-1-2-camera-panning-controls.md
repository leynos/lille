# Task 2.1.2: Implement Camera Panning Controls

This Execution Plan (ExecPlan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: COMPLETE

This document must be maintained in accordance with
`docs/documentation-style-guide.md` and the execplans skill.

## Purpose / Big Picture

Enable keyboard-based camera panning, so users can navigate the isometric map
using WASD or arrow keys. After this change, holding any of these keys pans the
camera smoothly across the map at a configurable speed, independent of frame
rate. This completes Task 2.1.2 of the Lille development roadmap.

Observable outcome: Running the game and holding W moves the camera up, S moves
down, A moves left, D moves right. Arrow keys provide equivalent behaviour.
Diagonal movement (e.g., W+D) moves at the same speed as cardinal movement.

## Constraints

- **Read-only presentation layer**: The camera system must not modify any
  gameplay or simulation components. It only reads input and modifies the
  camera's `Transform`.
- **Differential Dataflow-Based Stream Processing (DBSP) is source of truth**:
  No inferred game behaviour may originate from presentation code.
- **Feature gating**: All presentation code must be gated behind
  `#[cfg(feature = "render")]`.
- **File size limit**: `src/presentation.rs` must not exceed 400 lines.
- **No new dependencies**: Use only existing crate dependencies (Bevy 0.17.3).
- **System ordering**: Camera panning must run after `apply_dbsp_outputs_system`
  for semantic correctness.

## Tolerances (Exception Triggers)

- **Scope**: If implementation requires changes to more than 5 files, stop and
  escalate.
- **Lines of code**: If net additions exceed 200 lines (excluding tests), stop
  and escalate.
- **Test failures**: If tests fail after 3 fix attempts, stop and escalate.
- **API changes**: If public API of existing modules must change, stop and
  escalate.

## Risks

- **Risk**: `ButtonInput<KeyCode>` may not be initialized in headless test
  environment. Severity: medium Likelihood: low Mitigation: The
  `map_test_plugins::add_map_test_plugins` helper initializes `MinimalPlugins`
  which includes input. Verify during testing.

- **Risk**: `Time::delta_secs()` may return zero or minimal values during rapid
  test ticks. Severity: low Likelihood: medium Mitigation: Use
  `max_delta_seconds` in `CameraSettings` set to 1.0 in tests; accept that
  movement distances may be small but non-zero.

- **Risk**: Diagonal movement faster than cardinal if direction not normalized.
  Severity: medium Likelihood: high (if forgotten) Mitigation: Pure
  `compute_pan_direction` function always normalizes; unit tests verify
  diagonal length equals 1.0.

## Progress

- [x] Add `CameraSettings` resource with `pan_speed` and `max_delta_seconds`
- [x] Add `compute_pan_direction` pure function with unit tests
- [x] Add `camera_pan_system` with system ordering
- [x] Update `PresentationPlugin::build` to register resource and system
- [x] Update `src/lib.rs` exports for new public items
- [x] Create behavioural tests in `tests/camera_panning_rspec.rs`
- [x] Run quality gates: `make check-fmt && make lint && make test`
- [x] Update roadmap to mark Task 2.1.2 as done
- [x] Commit with descriptive message

## Surprises & Discoveries

No surprises encountered. The implementation proceeded as planned with all APIs
behaving as documented.

## Decision Log

- **Decision**: Use `PanInput` struct instead of four boolean parameters.
  Rationale: Improves readability and allows future extension (e.g., analog
  input). Date: Implementation phase.

- **Decision**: Guard against non-positive `max_delta_seconds` by clamping to
  `f32::EPSILON`. Rationale: Prevents division by zero or negative delta values
  that could cause unexpected camera behaviour. Date: PR review feedback.

- **Decision**: Use match-based `axis` helper function for direction
  calculation. Rationale: More explicit than arithmetic approach, easier to
  verify correctness. Date: PR review feedback.

## Outcomes & Retrospective

Implementation completed successfully. All progress items achieved:

- Camera panning works with WASD and arrow keys
- Diagonal movement normalized to prevent faster diagonal speed
- Frame-rate independent via delta time clamping
- Behavioural and unit tests provide coverage for edge cases

Lessons learned: The pure function approach (`compute_pan_direction`) made unit
testing straightforward and kept the system function focused on I/O.

## Context and Orientation

### Current State

The `PresentationPlugin` in `src/presentation.rs` (116 lines) currently:

- Defines `CameraController` marker component
- Spawns `Camera2d` with `CameraController` marker at startup
- Registers `CameraController` for reflection

No input handling exists yet. The camera is stationary after spawn.

### Key Files

Table: Files modified or referenced by this task

| File                                         | Purpose                                             |
| -------------------------------------------- | --------------------------------------------------- |
| `src/presentation.rs`                        | Core module to modify (add settings, system, tests) |
| `src/lib.rs`                                 | Update exports for new public items                 |
| `tests/presentation_plugin_rspec.rs`         | Reference for behavioural test structure            |
| `tests/support/map_fixture.rs`               | Test fixture infrastructure to reuse                |
| `docs/lille-map-and-presentation-roadmap.md` | Update completion status                            |

### Bevy 0.17.3 API Reference

- `Res<ButtonInput<KeyCode>>` for keyboard input
- `Res<Time>` with `time.delta_secs()` for frame-rate independence
- `Query<&mut Transform, With<CameraController>>` for camera access
- Key codes: `KeyCode::KeyW`, `KeyCode::KeyS`, `KeyCode::KeyA`, `KeyCode::KeyD`,
  `KeyCode::ArrowUp`, `KeyCode::ArrowDown`, `KeyCode::ArrowLeft`,
  `KeyCode::ArrowRight`

## Plan of Work

### Stage A: Add CameraSettings Resource

Add a `CameraSettings` resource following the `LilleMapSettings` pattern:

```rust
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct CameraSettings {
    /// Camera pan speed in world units per second.
    pub pan_speed: f32,
    /// Maximum delta time to prevent teleporting during frame hitches.
    pub max_delta_seconds: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            pan_speed: 500.0,
            max_delta_seconds: 0.1,
        }
    }
}
```

Location: `src/presentation.rs`, after `CameraController` definition.

### Stage B: Add Pure Direction Function

Add `PanInput` struct and `compute_pan_direction` as a pure, testable function:

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PanInput {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

#[must_use]
pub fn compute_pan_direction(input: PanInput) -> Vec2 {
    fn axis(neg: bool, pos: bool) -> f32 {
        match (neg, pos) {
            (true, false) => -1.0,
            (false, true) => 1.0,
            _ => 0.0,
        }
    }

    let x = axis(input.left, input.right);
    let y = axis(input.down, input.up);
    let raw = Vec2::new(x, y);

    if raw == Vec2::ZERO { Vec2::ZERO } else { raw.normalize() }
}
```

Location: `src/presentation.rs`, after `CameraSettings`.

Add unit tests using rstest parameterization:

- Cardinal directions return unit vectors
- Diagonal directions return normalized vectors (length ≈ 1.0)
- Opposing keys cancel out

### Stage C: Add Camera Pan System

Add `camera_pan_system`:

```rust
pub fn camera_pan_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    settings: Res<CameraSettings>,
    mut camera_query: Query<&mut Transform, With<CameraController>>,
) {
    let Ok(mut transform) = camera_query.single_mut() else { return };

    let input = PanInput {
        up: keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp),
        down: keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown),
        left: keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft),
        right: keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight),
    };

    let direction = compute_pan_direction(input);
    if direction == Vec2::ZERO { return }

    // Guard against non-positive max_delta_seconds.
    let clamped_max = settings.max_delta_seconds.max(f32::EPSILON);
    let delta = time.delta_secs().min(clamped_max);
    let velocity = direction * settings.pan_speed * delta;
    transform.translation.x += velocity.x;
    transform.translation.y += velocity.y;
}
```

Location: `src/presentation.rs`, after `compute_pan_direction`.

### Stage D: Update Plugin and Exports

Modify `PresentationPlugin::build`:

```rust
fn build(&self, app: &mut App) {
    app.register_type::<CameraController>();
    app.init_resource::<CameraSettings>();
    app.add_systems(Startup, camera_setup);
    app.add_systems(
        Update,
        camera_pan_system.after(apply_dbsp_outputs_system),
    );
}
```

Update `src/lib.rs` to export new items:

```rust
#[cfg(feature = "render")]
pub use presentation::{
    CameraController, CameraSettings, PresentationPlugin, compute_pan_direction
};
```

### Stage E: Add Behavioural Tests

Create `tests/camera_panning_rspec.rs` following the pattern in
`tests/presentation_plugin_rspec.rs`:

- Fixture extends `MapPluginFixtureBase`
- Helper methods: `press_key`, `release_key`, `camera_position`
- Test scenarios:
  - W key moves camera up (positive Y)
  - S key moves camera down (negative Y)
  - A key moves camera left (negative X)
  - D key moves camera right (positive X)
  - Arrow keys equivalent to WASD
  - Diagonal movement is normalized

### Stage F: Quality Gates and Finalization

1. Run `make check-fmt && make lint && make test`
2. Fix any issues
3. Update `docs/lille-map-and-presentation-roadmap.md` to mark Task 2.1.2 done
4. Commit with message describing the change

## Concrete Steps

All commands run from the repository root.

1. Edit `src/presentation.rs` to add `CameraSettings` resource
2. Edit `src/presentation.rs` to add `compute_pan_direction` function
3. Edit `src/presentation.rs` to add unit tests for direction function
4. Run `make test` to verify unit tests pass
5. Edit `src/presentation.rs` to add `camera_pan_system`
6. Edit `src/presentation.rs` to update `PresentationPlugin::build`
7. Edit `src/lib.rs` to export new items
8. Create `tests/camera_panning_rspec.rs` with behavioural tests
9. Run `make check-fmt && make lint && make test`
10. Edit `docs/lille-map-and-presentation-roadmap.md` to mark task done
11. Run `git add -A && git commit` with descriptive message

Expected output from `make test`:

```text
running X tests
test presentation::tests::… ok
…
test result: ok. X passed; 0 failed
```

## Validation and Acceptance

**Quality criteria:**

- Tests: All existing tests pass plus new unit and behavioural tests
- Lint: `make lint` passes with no warnings
- Format: `make check-fmt` passes

**Quality method:**

```sh
make check-fmt && make lint && make test
```

**Manual verification:**

Run the game with `cargo run --features render` and verify:

1. Camera starts at origin (0, 0)
2. Holding W pans the camera upward
3. Holding S pans the camera downward
4. Holding A pans the camera leftward
5. Holding D pans the camera rightward
6. Arrow keys work identically to WASD
7. Diagonal movement (W+D) moves at same speed as single key
8. Releasing keys stops camera movement

## Idempotence and Recovery

All steps are idempotent:

- Editing files can be repeated safely
- Tests can be run repeatedly
- Commits can be amended if not pushed

If a step fails, resolve the issue and retry from that step.

## Artifacts and Notes

### Unit Test Cases

```rust
#[rstest]
#[case::no_keys(PanInput::default(), Vec2::ZERO)]
#[case::up_only(PanInput { up: true, ..Default::default() }, Vec2::new(0.0, 1.0))]
#[case::down_only(PanInput { down: true, ..Default::default() }, Vec2::new(0.0, -1.0))]
#[case::left_only(PanInput { left: true, ..Default::default() }, Vec2::new(-1.0, 0.0))]
#[case::right_only(PanInput { right: true, ..Default::default() }, Vec2::new(1.0, 0.0))]
fn pan_direction_cardinal(…) { … }

#[rstest]
#[case::up_right(PanInput { up: true, right: true, ..Default::default() })]
#[case::up_left(PanInput { up: true, left: true, ..Default::default() })]
fn pan_direction_diagonal_is_normalized(…) { … }
```

### Edge Cases Handled

Table: Edge cases and their handling in the camera pan system

| Edge Case           | Handling                                       |
| ------------------- | ---------------------------------------------- |
| No camera entity    | `get_single_mut()` returns `Err`, early return |
| No keys pressed     | `direction == Vec2::ZERO`, early return        |
| Opposing keys (W+S) | Cancel to zero on that axis                    |
| Diagonal movement   | `normalize()` prevents faster speed            |
| Frame hitch         | Clamped to `max_delta_seconds`                 |

## Interfaces and Dependencies

### New Public Items in `lille::presentation`

```rust
pub struct CameraSettings {
    pub pan_speed: f32,
    pub max_delta_seconds: f32,
}

pub struct PanInput {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

pub fn compute_pan_direction(input: PanInput) -> Vec2

pub fn camera_pan_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    settings: Res<CameraSettings>,
    mut camera_query: Query<&mut Transform, With<CameraController>>,
)
```

### Dependencies

- `bevy::prelude::*` (existing)
- `bevy::input::ButtonInput` (existing via prelude)
- `crate::apply_dbsp_outputs_system` (for system ordering)
