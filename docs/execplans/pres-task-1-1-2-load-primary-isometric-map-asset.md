# Load primary isometric map asset (Task 1.1.2)

This ExecPlan is a living document. The sections `Progress`,
`Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must
be kept up to date as work proceeds.

This work implements **Task 1.1.2** from
`docs/lille-map-and-presentation-roadmap.md`.

## Purpose / Big Picture

After this change, running the Lille game loads a baseline isometric Tiled map
from `assets/maps` using the `bevy_ecs_tiled` asset pipeline. The map loads
into Bevy’s ECS as a hierarchy of entities (map root → layers → tilemaps /
objects), and base tile layers render without panics.

This is the first “data-driven world” milestone: we stop hardcoding the world
geometry and start consuming authored content. Importantly, **this task does
not infer gameplay or physics**. The DBSP circuit remains the sole authority
for any inferred behaviour (movement, collisions, etc.). This task only loads
and renders the map asset.

## Progress

- [x] (2025-12-17 21:40Z) Add baseline map assets under `assets/maps/`.
- [x] (2025-12-17 21:40Z) Extend `src/map.rs` to spawn the primary `TiledMap`.
- [x] (2025-12-17 21:40Z) Add unit tests (`rstest`) for spawn and failures.
- [x] (2025-12-17 21:40Z) Add behavioural tests (`rspec`) for lifecycle.
- [x] (2025-12-17 21:40Z) Record decisions in
      `docs/lille-isometric-tiled-maps-design.md`.
- [x] (2025-12-17 21:40Z) Mark Task 1.1.2 as done in
      `docs/lille-map-and-presentation-roadmap.md`.
- [x] (2025-12-17 22:26Z) Run quality gates (`make check-fmt`, `make lint`,
  `make test`, `make markdownlint`) and fix failures.

## Surprises & Discoveries

- Observation: The Rust test harness runs tests on worker threads, and Bevy’s
  `WinitPlugin` panics if its event loop is initialized off the main thread.
  Evidence: unit tests panic with the standard “event loop must be on the main
  thread” failure if `WinitPlugin` is left enabled.

- Observation: `bevy_ecs_tiled` may despawn the root map entity when recursive
  dependency loading fails, so “does the root map entity still exist?” is not a
  reliable failure signal. Evidence: a missing dependency can result in no
  `TiledMap` entity being queryable even though a load failure occurred.

- Observation: Enabling `bevy_ecs_tiled` rendering in `test-support` builds
  makes the Bevy test harness brittle (render initialization and platform
  threading assumptions), even when no window is created. Evidence: tests are
  substantially more reliable when `map` does not imply `render`, and when
  tests explicitly configure the `RenderPlugin` while disabling
  `bevy::winit::WinitPlugin`.

## Decision Log

- Decision: Add a minimal camera spawn that only runs if no `Camera2d` exists,
  and only compiles in `render` builds. Rationale: Task 1.1.2 requires base
  tile layers to render when launching the game. Lille currently spawns no
  camera, and the future `PresentationPlugin` (Task 2.1.1) is not implemented
  yet. A conditional “only if missing” camera keeps the map visible now while
  remaining safe to replace later, while the `render` gate ensures headless
  builds stay minimal. Date/Author: 2025-12-17 (Codex CLI)

- Decision: Keep `LilleMapPlugin` idempotent and safe to add multiple times by
  installing its systems only once. Rationale: Existing tests (and the design
  doc) already assert this behaviour for `TiledPlugin` registration. Extending
  this idempotence to map spawning prevents accidental double-spawns.
  Date/Author: 2025-12-17 (Codex CLI)

- Decision: Enable `bevy_ecs_tiled`’s `render` feature (in addition to `png`)
  only when the `render` feature is enabled, so base tile layers render in the
  game binary without forcing render initialization in tests. Rationale:
  `bevy_ecs_tiled`’s defaults include rendering, but Lille opts out of
  dependency defaults. To satisfy the task completion criteria (“renders base
  tile layers”), Lille must explicitly opt into rendering, but keeping that
  opt-in behind `render` avoids brittle headless test configurations.
  Date/Author: 2025-12-17 (Codex CLI)

- Decision: Use a `PrimaryMapAssetTracking` resource to hold the strong
  `Handle<TiledMapAsset>` and track recursive dependency load state. Rationale:
  This makes load failures observable even if the map entity is despawned by
  `bevy_ecs_tiled`, and it avoids false negatives in unhappy-path tests.
  Date/Author: 2025-12-17 (Codex CLI)

- Decision: Use headless `DefaultPlugins` (with `WinitPlugin` disabled) for
  tests that rely on asset loading. Rationale: This keeps the asset pipeline
  behaviour close to runtime while remaining safe under the Rust test harness’
  threading model. Date/Author: 2025-12-17 (Codex CLI)

## Outcomes & Retrospective

`LilleMapPlugin` now provides a single entry point for loading a primary
isometric `.tmx` asset and letting `bevy_ecs_tiled` spawn its ECS hierarchy.
The plugin reports invalid configuration and load failures as structured
`LilleMapError` events, and it avoids mutating simulation state: DBSP remains
the authority for inferred behaviour.

The implementation includes both `rstest` unit tests and `rspec` behavioural
tests, covering both the happy path (map layers appear) and unhappy paths
(invalid path / missing map).

## Context and Orientation

Key files and what they do:

- `src/map.rs`: Home of `LilleMapPlugin`. Today it only registers
  `bevy_ecs_tiled::TiledPlugin`.
- `src/main.rs`: The example game binary (behind `render` + `map` features).
- `docs/lille-isometric-tiled-maps-design.md`: Design proposal for using
  `bevy_ecs_tiled` and flowing map data into ECS and DBSP.
- `docs/lille-map-and-presentation-roadmap.md`: Roadmap containing Task 1.1.2.

Key concepts (defined briefly):

- **Tiled**: A level editor that exports `.tmx` map files.
- **Tiled map hierarchy**: When `bevy_ecs_tiled` loads a `.tmx`, it spawns a
  root map entity (with a `TiledMap` component) and child entities for layers,
  tilemaps, and objects.
- **DBSP circuit**: Lille’s simulation logic. It must remain authoritative for
  derived/inferred behaviour. Loading a map must not “invent” physics state
  outside DBSP.

## Plan of Work

1. Add a minimal baseline isometric Tiled map under `assets/maps/`.

   - Create `assets/maps/primary-isometric.tmx`.
   - Include a tiny tileset image (a simple `.png`) so a tile layer renders.
   - Keep the asset intentionally small to reduce repo size and test runtime.

2. Extend `src/map.rs` to support “spawn the active map”.

   - Add a small configuration resource (defaulting to
     `maps/primary-isometric.tmx`) so the selected map is explicit and
     testable.
   - Add a `Startup` system that:
     - does nothing if a `TiledMap` already exists (single-map assumption),
     - otherwise spawns a root entity with `TiledMap(asset_server.load(...))`.

3. Add minimal camera bootstrapping (conditional).

   - Spawn `Camera2d` only if the world has no camera yet.
   - This is intentionally minimal and will be superseded by the later
     `PresentationPlugin`.

4. Add tests covering happy and unhappy paths.

   - Unit tests (`rstest`) validate:
     - the plugin spawns exactly one `TiledMap` root entity by default,
     - it does not spawn a second map if one already exists.
   - Behavioural tests (`rspec`) validate:
     - Given the primary map assets exist, after some ticks we observe at least
       one Tiled layer entity spawned (proves hierarchy load),
     - Given a non-existent map path, the plugin reports a structured failure
       (and does not panic).

5. Update documentation.

   - Mark Task 1.1.2 as done in the roadmap.
   - Record the camera bootstrapping decision (and why it is temporary) in
     `docs/lille-isometric-tiled-maps-design.md`.

## Concrete Steps

Run these commands from the repo root: `/mnt/home/leynos/Projects/lille`.

When running long commands (tests/lints), always capture output to a log file
so failures are reviewable even when CLI output is truncated:

    set -o pipefail
    make test 2>&1 | tee /tmp/lille-make-test.log

Implementation workflow:

1. Create assets under `assets/maps/`:

    - Create directories `assets/maps` and `assets/maps/tiles`.
    - Add `assets/maps/primary-isometric.tmx`.
    - Generate a tiny `assets/maps/tiles/iso-tile.png` (a small placeholder
      tile image).

2. Implement map spawning in `src/map.rs`.

3. Add/update tests under `tests/`:

    - Update `tests/map_plugin.rs` for unit checks around spawning.
    - Update `tests/map_plugin_rspec.rs` to assert hierarchy load and failure
      reporting.

4. Run the quality gates:

    set -o pipefail
    make check-fmt 2>&1 | tee /tmp/lille-check-fmt.log
    make lint 2>&1 | tee /tmp/lille-lint.log
    make test 2>&1 | tee /tmp/lille-make-test.log

If documentation changed, also run:

    set -o pipefail
    make markdownlint 2>&1 | tee /tmp/lille-markdownlint.log
    make fmt 2>&1 | tee /tmp/lille-fmt.log

1. Manual runtime check (render):

    cargo run --features "render map"

Expected behaviour: the window opens and the baseline isometric tile layer is
visible (even if primitive).

## Validation and Acceptance

Acceptance criteria for Task 1.1.2 is met when all of the following hold:

1. `make test` passes and includes new tests covering map spawning and map load
   outcomes (happy + unhappy).

2. Running the game with `cargo run --features "render map"` loads the baseline
   map without panics and shows base tile layers on screen.

3. The map load does not create inferred gameplay entities or apply physics
   logic outside the DBSP circuit (this task is purely asset/hierarchy load).

## Idempotence and Recovery

- `LilleMapPlugin` must be safe to add multiple times without spawning multiple
  maps or duplicating systems.
- The primary map spawn must be safe to re-run (it should early-return if a
  `TiledMap` already exists).
- If asset generation fails, delete `assets/maps` and re-run the generation
  step; no other state should be affected.

## Artifacts and Notes

Quality gate logs captured during implementation:

- `make check-fmt`: `/tmp/lille-check-fmt.log`
- `make lint`: `/tmp/lille-lint.log`
- `make test`: `/tmp/lille-test.log`
- `make markdownlint`: `/tmp/lille-markdownlint.log`
- `cargo build --features "render map"`: `/tmp/lille-build-render-map.log`

## Interfaces and Dependencies

This task relies on:

- `bevy_ecs_tiled::prelude::TiledMap` as the root component that triggers
  loading of a `.tmx` map into ECS.
- Bevy’s `AssetServer` for loading the `.tmx` and its dependencies.
- `rstest` for unit tests and `rspec` for behavioural tests.

At the end of this task, the following interfaces must exist:

- In `src/map.rs`, `LilleMapPlugin` registers `TiledPlugin` and adds a startup
  system that spawns the primary `TiledMap` entity.

## Revision note (required when editing an ExecPlan)

2025-12-17 (Codex CLI): Marked the quality gates as complete and recorded the
log locations. Updated decision notes to reflect the final feature split (`map`
is independent of `render`) and the `render`-gated camera bootstrap.
