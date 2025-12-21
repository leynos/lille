# Register Tiled Custom Property Types

This ExecPlan is a living document. The sections `Progress`,
`Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must
be kept up to date as work proceeds.

No `PLANS.md` file exists in the repository root, so this ExecPlan is the sole
plan-of-record for the change.

## Purpose / Big Picture

Enable Lille's Tiled integration to read typed custom properties and surface
them as ECS components, so map-authored metadata flows into the game world
without bespoke parsing. Success is visible when loading a map with typed
properties produces entities with populated components, and the DBSP circuit
remains the only source of inferred behaviour.

## Progress

- [x] (2025-12-21 00:00Z) Review roadmap and design context for Task 1.1.3.
- [x] (2025-12-21 00:10Z) Enable `bevy_ecs_tiled` user properties and register
  map property types.
- [x] (2025-12-21 00:15Z) Add a dedicated Tiled map asset containing typed
  custom properties.
- [x] (2025-12-21 00:25Z) Add rstest unit coverage for type registration.
- [x] (2025-12-21 00:35Z) Add rust-rspec behavioural coverage for property
  hydration and failures.
- [x] (2025-12-21 00:40Z) Update design documentation and roadmap to record
  decisions and status.
- [x] (2025-12-21 00:55Z) Run validation commands and capture evidence.

## Surprises & Discoveries

- Observation: `reflect_auto_register` pre-registers reflectable component
  types, so the type registry contains the map property types even before the
  plugin is added. Evidence: Initial unit test expecting unregistered types
  failed during `make test`.

- Observation: `LilleMapPlugin` assumes asset infrastructure is present when
  added, so unit tests must install the map test plugins to avoid missing
  resources. Evidence: `bevy_asset` resource panic during the first registry
  test run.

- Observation: `make nixie` failed on a Mermaid class diagram that used braces
  in attribute text, which Mermaid parses as struct delimiters. Evidence: parse
  error in `docs/lille-isometric-tiled-maps-design.md` during validation.

## Decision Log

- Decision: Use `Collidable`, `SlopeProperties`, `PlayerSpawn`, and
  `SpawnPoint` as the initial set of Tiled custom property components.
  Rationale: These names align with the design doc and roadmap tasks, and they
  map directly to upcoming collision and spawn workflows without introducing
  gameplay inference outside DBSP. Date/Author: 2025-12-21 / Codex.

- Decision: Add a dedicated `assets/maps/` test map that exercises typed
  properties, including one invalid property type to cover unhappy paths.
  Rationale: Integration tests can validate both successful hydration and
  ignored unknown types without mutating the production map. Date/Author:
  2025-12-21 / Codex.

- Decision: Use fully qualified type paths (for example,
  `lille::map::Collidable`) in the test map's `propertytype` fields. Rationale:
  `bevy_ecs_tiled` resolves user properties by type path, so the test map must
  match the reflection registry names. Date/Author: 2025-12-21 / Codex.

## Outcomes & Retrospective

Custom property hydration is now enabled and validated. The map plugin
registers the new types, the test map loads with hydrated components, and both
rstest and rust-rspec coverage exercise success and failure cases. Validation
completed via `make check-fmt`, `make lint`, and `make test`.

## Context and Orientation

The map integration lives in `src/map.rs` and is responsible only for asset
loading and wiring in `bevy_ecs_tiled`. Custom properties require enabling the
`user_properties` feature on the `bevy_ecs_tiled` dependency and registering
the relevant component types with Bevy's type registry using
`App::register_type`. The new property components must be reflectable and
derive `Component` so they can be hydrated by `bevy_ecs_tiled` when parsing
Tiled maps.

The current tests for `LilleMapPlugin` live under `tests/`, with shared helpers
in `tests/support/`. Behavioural tests use `rust-rspec` and must run in their
own file because ticking the Bevy app initialises renderer state. The roadmap
for this work is in `docs/lille-map-and-presentation-roadmap.md`, and the
design rationale is in `docs/lille-isometric-tiled-maps-design.md`.

## Plan of Work

Update `Cargo.toml` to enable the `user_properties` feature for
`bevy_ecs_tiled`. Define the custom property components in `src/map.rs` with
`Reflect`, `Component`, and `Default` derives, then register each type inside
`LilleMapPlugin::build`. Add a new Tiled map asset under `assets/maps/` that
includes tiles and objects annotated with the new custom property types plus
one intentionally unknown type. Add rstest unit tests that assert the type
registry contains the expected registrations once the plugin is installed. Add
a rust-rspec behavioural test that loads the new map and asserts both
successful component hydration and the ignored unknown property. Finally,
update the design doc and roadmap, and capture the validation results.

## Concrete Steps

1. Edit `Cargo.toml` to add the `user_properties` feature to
   `bevy_ecs_tiled`.
2. Add `Collidable`, `SlopeProperties`, `PlayerSpawn`, and `SpawnPoint` to
   `src/map.rs`, and register them in `LilleMapPlugin::build`.
3. Create `assets/maps/primary-isometric-custom-properties.tmx` that includes:
   - a tileset tile with `Collidable` and `SlopeProperties` properties,
   - a `PlayerSpawn` object,
   - a `SpawnPoint` object with fields set, and
   - a third object using an unknown property type for the unhappy path.
4. Add rstest unit coverage in a new test file that checks type registry
   registration.
5. Add a rust-rspec behavioural test file that loads the new map and asserts
   components plus the ignored unknown property.
6. Update `docs/lille-isometric-tiled-maps-design.md` with the registration
   decision and mark Task 1.1.3 done in
   `docs/lille-map-and-presentation-roadmap.md`.
7. Run validation commands:

   - `timeout 300 bash -lc 'set -o pipefail && make check-fmt 2>&1 | tee /tmp/lille-check-fmt.log'`
   - `timeout 300 bash -lc 'set -o pipefail && make lint 2>&1 | tee /tmp/lille-lint.log'`
   - `timeout 300 bash -lc 'set -o pipefail && make test 2>&1 | tee /tmp/lille-test.log'`

## Validation and Acceptance

- The new rstest unit test fails before the change because the map property
  types do not exist in the registry, and passes after the plugin registers
  them.
- The rust-rspec behavioural test fails before the change because the custom
  properties are not hydrated, and passes after registration.
- Running `make check-fmt`, `make lint`, and `make test` completes with zero
  failures (logs captured in `/tmp/lille-*.log`).
- The roadmap marks Task 1.1.3 as done, and the design doc records the
  registration decision.

## Idempotence and Recovery

All edits are additive and can be re-applied safely. If tests fail, re-run the
failing test binary after fixing the reported error. If map loading fails,
remove the new test map and reintroduce it after validating the XML structure.

## Artifacts and Notes

- Expected evidence includes a rust-rspec scenario showing the custom
  properties load, plus the captured test logs in `/tmp/lille-*.log`.

## Interfaces and Dependencies

Expose the following public components in `lille::map`:

    pub struct Collidable;
    pub struct SlopeProperties { pub grad_x: f32, pub grad_y: f32 }
    pub struct PlayerSpawn;
    pub struct SpawnPoint { pub enemy_type: u32, pub respawn: bool }

Ensure `LilleMapPlugin::build` calls:

    app.register_type::<Collidable>()
       .register_type::<SlopeProperties>()
       .register_type::<PlayerSpawn>()
       .register_type::<SpawnPoint>();

The `bevy_ecs_tiled` dependency must include the `user_properties` feature.

## Revision note (required when editing an ExecPlan)

Updated progress, decisions, and outcomes after implementing the feature,
fixing Mermaid syntax, and running validation to reflect completed work and the
observed registry behaviour.
