# Create PresentationPlugin and Camera Marker (Task 2.1.1)

This ExecPlan is a living document. The sections `Constraints`, `Tolerances`,
`Risks`, `Progress`, `Surprises & Discoveries`, `Decision Log`, and
`Outcomes & Retrospective` must be kept up to date as work proceeds.

Status: COMPLETE

This document must be maintained in accordance with
`docs/documentation-style-guide.md` and `AGENTS.md`.

## Purpose / Big Picture

Task 2.1.1 establishes the presentation layer as a dedicated plugin that owns
camera setup. After this change, the application will render through a camera
spawned by `PresentationPlugin` rather than the temporary bootstrap camera in
`LilleMapPlugin`. This separates rendering concerns from map loading, allowing
the presentation layer to evolve independently (panning, zooming, Y-sorting in
later tasks) whilst keeping the simulation authoritative via the DBSP circuit.

The user-visible outcome: launching the game produces the same visual output,
but the camera is now controlled by a dedicated presentation module that can be
extended for interactive controls.

## Constraints

Hard invariants that must hold throughout implementation:

- DBSP circuit must remain the sole source of truth for inferred behaviour;
  presentation systems must be passive observers of simulation state.
- No changes to `src/dbsp_circuit/` or `src/dbsp_sync/` modules.
- No modifications to map asset loading logic in `src/map/translate.rs` or
  spawn logic in `src/map/spawn.rs`.
- File size limit: no single code file may exceed 400 lines.
- Use en-GB-oxendict spelling in comments and documentation.
- Tests must use `rstest` for unit tests and `rust-rspec` for behavioural tests.

## Tolerances (Exception Triggers)

Thresholds that trigger escalation when breached:

- **Scope**: If implementation requires changes to more than 6 files (excluding
  test files), stop and escalate.
- **Interface**: If any public API signature in `src/dbsp_sync/` must change,
  stop and escalate.
- **Dependencies**: No new external crate dependencies are required; if one
  becomes necessary, stop and escalate.
- **Iterations**: If tests still fail after 3 attempts at fixing, stop and
  escalate.
- **Ambiguity**: If multiple valid interpretations exist for camera projection
  configuration, present options with trade-offs.

## Risks

- **Risk**: Removing `bootstrap_camera_if_missing()` may cause tests that depend
  on having a camera to fail.
  - Severity: medium
  - Likelihood: medium
  - Mitigation: Ensure `PresentationPlugin` is added to test apps that require
    rendering, or update `LilleMapSettings::should_bootstrap_camera` to default
    false and remove the system in a subsequent commit.

- **Risk**: Feature flag gating (`#[cfg(feature = "render")]`) may cause compile
  errors when features are toggled.
  - Severity: low
  - Likelihood: medium
  - Mitigation: Gate `PresentationPlugin` behind `render` feature like existing
    code; verify with `cargo check --all-features` and without.

## Progress

- [x] (2026-01-08) Stage A: Understand current camera bootstrap mechanism
- [x] (2026-01-08) Stage B: Create `src/presentation.rs` with
      `PresentationPlugin` skeleton
- [x] (2026-01-08) Stage C: Define `CameraController` marker component
- [x] (2026-01-08) Stage D: Implement `camera_setup` Startup system
- [x] (2026-01-08) Stage E: Wire module into `src/lib.rs` and `src/main.rs`
- [x] (2026-01-08) Stage F: Remove legacy camera bootstrap from `LilleMapPlugin`
- [x] (2026-01-08) Stage G: Add unit tests for component definitions
- [x] (2026-01-08) Stage H: Add behavioural test (rust-rspec) verifying camera
      spawns
- [x] (2026-01-08) Stage I: Run quality gates (fmt, lint, test)
- [x] (2026-01-08) Stage J: Update roadmap to mark task complete
- [x] (2026-01-08) Stage K: Commit with descriptive message

## Surprises & Discoveries

- **Observation**: Removing `should_bootstrap_camera` required updating 11 test
  files that referenced the now-removed field.
  - Evidence: Grep revealed tests explicitly setting `should_bootstrap_camera:
    false` in `LilleMapSettings` construction.
  - Impact: Minor additional work, but the removal cleaned up dead code and made
    the `LilleMapSettings` struct simpler.

- **Observation**: Bevy 0.17's Required Components mechanism means `Camera2d`
  automatically provides `Projection`, so explicit `OrthographicProjection`
  spawn is not required for basic camera functionality.
  - Evidence: Original plan called for spawning `OrthographicProjection`
    explicitly, but tests confirmed the camera works without it.
  - Impact: Simpler camera spawn code. Future zoom control (Task 2.1.3) can
    query/mutate the auto-inserted `Projection` component.

## Decision Log

- **Decision**: Gate `PresentationPlugin` behind `#[cfg(feature = "render")]`
  - Rationale: Matches existing pattern in `src/map/mod.rs` (line 199) and
    `src/main.rs` (line 1). Presentation is meaningless without rendering.
  - Date/Author: 2026-01-08 / Planning phase

- **Decision**: Use `Camera2d` with explicit `OrthographicProjection` component
  - Rationale: Design doc (`docs/lille-presentational-layer.md` lines 121-124)
    specifies spawning "Camera2d plus a 2D Projection". Explicit projection
    allows future zoom control (Task 2.1.3).
  - Date/Author: 2026-01-08 / Planning phase

- **Decision**: Remove `should_bootstrap_camera` setting from `LilleMapSettings`
  - Rationale: Setting becomes meaningless once `PresentationPlugin` owns camera
    setup. Keeping it creates dead code and confusion.
  - Date/Author: 2026-01-08 / Planning phase

## Outcomes & Retrospective

**Outcomes:**

- `src/presentation.rs` now defines `PresentationPlugin` with `CameraController`
  marker component and `camera_setup` Startup system.
- Legacy camera bootstrap (`MapBootstrapCamera`, `bootstrap_camera_if_missing`,
  `should_bootstrap_camera`) removed from `src/map/mod.rs`.
- Behavioural test (`tests/presentation_plugin_rspec.rs`) validates camera
  spawns with correct components and naming.
- All quality gates pass: `make check-fmt`, `make lint`, `make test`.

**Retrospective:**

- The implementation went smoothly, matching the plan closely.
- Removing the `should_bootstrap_camera` field required more test file updates
  than anticipated (11 files), but this was straightforward search-and-replace.
- The Bevy 0.17 Required Components model simplified the camera spawn since we
  don't need to explicitly add projection components.
- Next time: when removing a struct field, grep for it across test files early
  to scope the change accurately.

## Context and Orientation

### Current State

The codebase has a temporary camera bootstrap in `src/map/mod.rs`:

- **Component**: `MapBootstrapCamera` (marker, line 201)
- **System**: `bootstrap_camera_if_missing()` (lines 275-289)
- **Setting**: `LilleMapSettings::should_bootstrap_camera` (line 100)
- **Schedule**: `Startup` (line 524)

This was explicitly marked as temporary in the design docs, intended to be
superseded by `PresentationPlugin` in Task 2.1.1.

### Key Files

- `src/map/mod.rs` - Contains legacy camera bootstrap to remove
- `src/lib.rs` - Module declarations and re-exports (lines 1-73)
- `src/main.rs` - App plugin registration (lines 1-43)
- `src/components.rs` - Component definition patterns (lines 1-199)
- `docs/lille-presentational-layer.md` - Design specification
- `docs/lille-map-and-presentation-roadmap.md` - Task definition

### Test Infrastructure

- `tests/support/rspec_runner.rs` - `run_serial()` function for BDD tests
- `tests/support/map_fixture.rs` - `MapPluginFixtureBase` pattern
- `tests/support/thread_safe_app.rs` - `SharedApp` wrapper
- `tests/support/map_test_plugins.rs` - Headless test plugin helpers

### DBSP System Ordering (for Task 2.1.4 context)

DBSP systems run in `Update` schedule (or `PostUpdate` with feature flag):

    cache_state_for_dbsp_system → apply_dbsp_outputs_system

Presentation systems needing current transforms must use
`.after(apply_dbsp_outputs_system)`. This task (2.1.1) only creates the camera;
ordering will be addressed in Task 2.1.4.

## Plan of Work

### Stage A: Understand (read-only, no code changes)

Verify understanding of current camera mechanism by reading:

- `src/map/mod.rs` lines 199-289, 524
- `src/lib.rs` module structure
- `src/main.rs` plugin registration

### Stage B: Create Presentation Module Skeleton

Create `src/presentation.rs` with:

- Module-level documentation comment (`//!`)
- Empty `PresentationPlugin` struct
- `impl Plugin for PresentationPlugin` with empty `build()`
- Feature gate: `#![cfg(feature = "render")]`

### Stage C: Define CameraController Component

In `src/presentation.rs`, define:

    #[derive(Component, Reflect, Default, Debug, Clone, Copy, PartialEq, Eq)]
    #[reflect(Component, Default)]
    pub struct CameraController;

This follows the pattern from `src/map/mod.rs` (e.g., `Collidable` line 114).

### Stage D: Implement camera_setup System

Create Startup system that:

1. Spawns entity with `Camera2d`
2. Adds `OrthographicProjection` (or relies on Required Components)
3. Tags with `CameraController` marker
4. Adds `Name::new("PresentationCamera")`

Example:

    fn camera_setup(mut commands: Commands) {
        commands.spawn((
            Camera2d,
            OrthographicProjection::default_2d(),
            CameraController,
            Name::new("PresentationCamera"),
        ));
    }

Register in plugin's `build()` method:

    app.register_type::<CameraController>();
    app.add_systems(Startup, camera_setup);

### Stage E: Wire Module into Crate

In `src/lib.rs`:

- Add `#[cfg(feature = "render")]` gated module declaration
- Add `#[cfg_attr(docsrs, doc(cfg(feature = "render")))]` for docs
- Re-export `PresentationPlugin` and `CameraController`

In `src/main.rs`:

- Add `app.add_plugins(PresentationPlugin);` after `DbspPlugin`

### Stage F: Remove Legacy Camera Bootstrap

In `src/map/mod.rs`:

1. Remove `MapBootstrapCamera` component (line 201)
2. Remove `bootstrap_camera_if_missing()` system (lines 275-289)
3. Remove `should_bootstrap_camera` field from `LilleMapSettings` (line 100)
4. Remove `add_systems(Startup, bootstrap_camera_if_missing)` (line 524)
5. Update `LilleMapSettings::default()` impl

### Stage G: Add Unit Tests

Create unit test in `src/presentation.rs` under `#[cfg(test)]` module:

- Test `CameraController` derives (Default, Clone, etc.)

### Stage H: Add Behavioural Test

Create `tests/presentation_plugin_rspec.rs`:

- Fixture: `PresentationPluginFixture` following `MapPluginFixtureBase` pattern
- Scenario: "PresentationPlugin spawns camera with controller marker"
- Assertions:
  - Exactly one `Camera2d` entity exists after first tick
  - That entity has `CameraController` component
  - That entity has `OrthographicProjection` component

### Stage I: Run Quality Gates

Execute in order:

    make check-fmt 2>&1 | tee /tmp/fmt.log
    make lint 2>&1 | tee /tmp/lint.log
    make test 2>&1 | tee /tmp/test.log

Fix any failures before proceeding.

### Stage J: Update Roadmap

In `docs/lille-map-and-presentation-roadmap.md`, change line 113:

    - [ ] Task 2.1.1

To:

    - [x] Task 2.1.1

### Stage K: Commit

Create atomic commit with message:

    Add PresentationPlugin with CameraController marker

    Introduce src/presentation.rs defining PresentationPlugin, which spawns
    a Camera2d with CameraController marker component. This supersedes the
    temporary bootstrap camera in LilleMapPlugin.

    - Add CameraController component with Reflect derives
    - Add camera_setup Startup system spawning presentation camera
    - Remove MapBootstrapCamera and bootstrap_camera_if_missing from map module
    - Remove should_bootstrap_camera setting from LilleMapSettings
    - Add behavioural test verifying camera spawns with marker

    Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>

## Concrete Steps

All commands run from `/data/leynos/Projects/lille`.

### 1. Create presentation module

Write `src/presentation.rs` with full implementation.

### 2. Update lib.rs

Add module declaration and re-exports.

### 3. Update main.rs

Add plugin registration.

### 4. Modify map/mod.rs

Remove legacy camera bootstrap code.

### 5. Create behavioural test

Write `tests/presentation_plugin_rspec.rs`.

### 6. Run validation

    make check-fmt 2>&1 | tee /tmp/fmt.log
    make lint 2>&1 | tee /tmp/lint.log
    make test 2>&1 | tee /tmp/test.log

Expected: All pass with no warnings.

### 7. Update roadmap

Mark Task 2.1.1 as complete in roadmap.

### 8. Commit

    git add src/presentation.rs src/lib.rs src/main.rs src/map/mod.rs \
        tests/presentation_plugin_rspec.rs docs/lille-map-and-presentation-roadmap.md
    git commit -m "$(cat <<'EOF'
    Add PresentationPlugin with CameraController marker

    Introduce src/presentation.rs defining PresentationPlugin, which spawns
    a Camera2d with CameraController marker component. This supersedes the
    temporary bootstrap camera in LilleMapPlugin.

    - Add CameraController component with Reflect derives
    - Add camera_setup Startup system spawning presentation camera
    - Remove MapBootstrapCamera and bootstrap_camera_if_missing from map module
    - Remove should_bootstrap_camera setting from LilleMapSettings
    - Add behavioural test verifying camera spawns with marker

    Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
    EOF
    )"

## Validation and Acceptance

### Quality Criteria

- **Tests**: `make test` passes; new test
  `presentation_plugin_rspec::presentation_plugin_spawns_camera_with_marker`
  passes
- **Lint**: `make lint` passes with zero warnings
- **Format**: `make check-fmt` passes

### Quality Method

    make check-fmt && make lint && make test

### Acceptance Behaviour

After implementation:

1. Run `cargo run --features "render map"` - game window appears with camera
   rendering the isometric map
2. Run `cargo test --features "render map" presentation_plugin` - behavioural
   test passes
3. Query in test: `world.query::<&CameraController>()` returns exactly one
   entity
4. That entity also has `Camera2d` and `OrthographicProjection` components

## Idempotence and Recovery

- If any step fails, fix the issue and re-run from Stage I (quality gates)
- Module creation is idempotent (overwrites existing file)
- Git commit is safe to amend if not yet pushed

## Artifacts and Notes

### Expected File Structure After Implementation

    src/
      presentation.rs    # NEW - ~80 lines
      lib.rs             # MODIFIED - add module + exports
      main.rs            # MODIFIED - add plugin
      map/
        mod.rs           # MODIFIED - remove bootstrap camera
    tests/
      presentation_plugin_rspec.rs  # NEW - ~80 lines

### Component Definition Pattern (from src/map/mod.rs)

    #[derive(Component, Reflect, Default, Debug, Clone, Copy, PartialEq, Eq)]
    #[reflect(Component, Default)]
    pub struct MarkerName;

### Test Fixture Pattern (from tests/support/map_fixture.rs)

    #[derive(Debug, Clone)]
    struct PresentationPluginFixture {
        base: MapPluginFixtureBase,
    }

    impl PresentationPluginFixture {
        fn bootstrap() -> Self {
            let mut app = App::new();
            map_test_plugins::add_map_test_plugins(&mut app);
            app.add_plugins(PresentationPlugin);
            Self { base: MapPluginFixtureBase::new(app) }
        }
    }

## Interfaces and Dependencies

### New Public API

In `src/presentation.rs`:

    /// Marker component for the main presentation camera.
    #[derive(Component, Reflect, Default, Debug, Clone, Copy, PartialEq, Eq)]
    #[reflect(Component, Default)]
    pub struct CameraController;

    /// Plugin owning camera setup and presentation layer systems.
    pub struct PresentationPlugin;

    impl Plugin for PresentationPlugin {
        fn build(&self, app: &mut App) { … }
    }

### Re-exports in `src/lib.rs`

    #[cfg(feature = "render")]
    #[cfg_attr(docsrs, doc(cfg(feature = "render")))]
    pub mod presentation;

    #[cfg(feature = "render")]
    #[cfg_attr(docsrs, doc(cfg(feature = "render")))]
    pub use presentation::{PresentationPlugin, CameraController};

### Dependencies

- `bevy::prelude::*` (existing)
- No new crate dependencies

______________________________________________________________________

## Revision Notes

- 2026-01-08: Initial draft created during planning phase.
