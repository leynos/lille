# Translate Map Data into Engine State (Task 1.2.1)

This Execution Plan (ExecPlan) is a living document. The sections `Progress`,
`Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must
be kept up to date as work proceeds.

## Purpose / Big Picture

Translate Tiled collision annotations into Differential Dataflow-Based Stream
Processing (DBSP)-compatible physics components so that static map geometry
participates in the physics simulation. When complete, tiles marked
`Collidable` in Tiled will carry `Block` components that DBSP consumes for
floor detection and collision. This bridges authored map data with the
declarative physics circuit, keeping DBSP as the sole source of inferred
behaviour.

## Progress

- [x] Review roadmap and design context for Task 1.2.1.
- [x] Create `src/map/translate.rs` with block attachment system.
- [x] Register the system in `LilleMapPlugin`.
- [x] Add rstest (parameterized testing crate) unit tests for block attachment
  logic.
- [x] Add rust-rspec (RSpec-style behaviour-driven testing) behavioural tests
  for property hydration.
- [x] Update roadmap to mark Task 1.2.1 complete.
- [x] Run validation commands and capture evidence.

## Surprises & Discoveries

- `bevy_ecs_tiled` 0.10 still uses the deprecated `EventReader` API (renamed to
  `MessageReader` in Bevy 0.17). Added `#[expect(deprecated)]` to suppress the
  warning until upstream updates.
- `TilePos` uses `u32` for grid coordinates, requiring explicit cast to `i32`
  for `Block`. Added lint suppression with rationale.
- The rspec test structure requires careful nesting management to avoid
  `excessive_nesting` and `too_many_lines` clippy lints. Refactored assertions
  into helper methods on the fixture.

## Decision Log

- Decision: Attach `Block` directly to `Collidable` tiles instead of an
  intermediate `Wall` marker component. Rationale: Reduces indirection since
  DBSP consumes `Block` directly; the roadmap's "Wall" term is satisfied by
  `Block` serving as the collision geometry marker. Date/Author: 2025-12-23 /
  Codex.

- Decision: Use `TiledEvent<MapCreated>` as the trigger rather than
  `Added<Collidable>`. Rationale: Ensures all tiles are spawned before
  processing, simpler than tracking per-tile events, and aligns with design doc
  recommendation. Date/Author: 2025-12-23 / Codex.

- Decision: Use `Local<i64>` counter for block IDs. Rationale: Simple
  implementation, deterministic within a session. IDs need only be unique per
  map load, not across sessions. Coordinate hashing adds complexity without
  benefit. Date/Author: 2025-12-23 / Codex.

- Decision: Set `z = 0` for all blocks. Rationale: Single-level map scope per
  design doc. Multi-level vertical stacking is explicitly out of scope for
  Phase 1. Date/Author: 2025-12-23 / Codex.

## Outcomes & Retrospective

**Outcome:** Task 1.2.1 is complete. Tiles marked `Collidable` in Tiled now
receive `Block` components when the map loads. The DBSP physics circuit can
consume these blocks for floor height calculations and collision detection.

**Evidence:**

- All 7 rstest unit tests pass, covering block attachment, coordinate mapping,
  idempotency, and unique ID generation.
- All 7 rust-rspec behavioural assertions pass, confirming the completion
  criteria: "Querying loaded maps for `Block` returns every collidable tile and
  no unrelated tiles."
- `make check-fmt`, `make lint`, and `make test` all pass with zero failures.

**Notes for future work:**

- Task 1.2.2 ("Attach physics blocks via Tiled events") is now largely
  superseded by this implementation. Consider updating the roadmap to reflect
  that the event-based trigger (`TiledEvent<MapCreated>`) is already in place.
- Task 1.2.3 ("Feed static geometry to DBSP") may already be satisfied, since
  the existing DBSP input system queries `Block` components each tick. Verify
  and potentially mark complete.
- Task 1.2.4 ("Support slope metadata for terrain") can follow the same pattern:
  observe `SlopeProperties` and attach `BlockSlope` linked to the corresponding
  `Block` ID.

## Context and Orientation

The map integration lives in `src/map.rs` and is responsible for asset loading
and wiring in `bevy_ecs_tiled`. Task 1.1.3 registered `Collidable` and
`SlopeProperties` custom property types, which `bevy_ecs_tiled` hydrates onto
tile entities at load time.

The DBSP physics circuit consumes `Block` and `BlockSlope` components via the
input synchronization system in `src/dbsp_sync/input/sync.rs`. This task
bridges the gap: when a tile carries `Collidable`, we attach a `Block` so the
physics circuit can compute floor heights and collision behaviour.

Data flow after this task:

```text
Tiled (.tmx) -> bevy_ecs_tiled hydrates Collidable -> our system attaches Block
-> DBSP input system pushes to block_in() -> physics circuit computes floor
```

Key dependencies:

- `bevy_ecs_tiled` provides `TiledEvent<MapCreated>` and tile components
- `Block` component is defined in `src/components.rs`
- Test map with `Collidable` tiles exists at
  `assets/maps/primary-isometric-custom-properties.tmx`

## Plan of Work

Introduce a translation module `src/map/translate.rs` that listens for
`TiledEvent<MapCreated>` and attaches `Block` components to all entities with
`Collidable`. The system queries `(Entity, &TilePos)` filtered by
`With<Collidable>` and `Without<Block>` to ensure idempotency.

Add rstest unit tests verifying block attachment logic in isolation, and
rust-rspec behavioural tests loading the actual Tiled map to validate the
completion criteria: "Querying loaded maps for `Block` returns every wall tile
and no unrelated tiles."

## Concrete Steps

1. Create `src/map/translate.rs` with:
   - `attach_collision_blocks` system triggered by `TiledEvent<MapCreated>`
   - Module-level documentation explaining purpose
   - Public export of the system function

2. Update `src/map.rs`:
   - Add `mod translate;` and re-export public items
   - Register `attach_collision_blocks` in `LilleMapPlugin::build()`
   - Ensure `TiledEvent<MapCreated>` is available (from `bevy_ecs_tiled`)

3. Create `tests/map_collision_block_attachment.rs`:
   - rstest unit tests for block attachment logic
   - Test: entities with `Collidable` receive `Block` after processing
   - Test: `Block` coordinates match `TilePos` values
   - Test: idempotency (running twice doesn't duplicate)
   - Test: entities without `Collidable` don't receive `Block`

4. Create `tests/map_collision_rspec_block_attachment.rs`:
   - rust-rspec behavioural tests loading actual Tiled map
   - Verify all 4 `Collidable` tiles receive `Block` components
   - Verify no non-collidable entities receive `Block`
   - Verify `Block` coordinates are consistent

5. Update `docs/lille-map-and-presentation-roadmap.md`:
   - Mark `[x] Task 1.2.1`

6. Run validation commands:
   - `make check-fmt`
   - `make lint`
   - `make test`

## Validation and Acceptance

- Unit tests pass for block attachment logic
- Behavioural test confirms: querying `Block` returns exactly the `Collidable`
  tiles (4 in test map) and no others
- `make check-fmt`, `make lint`, `make test` all complete with zero failures
- Roadmap updated with `[x]` for Task 1.2.1

## Idempotence and Recovery

The block attachment system uses a `Without<Block>` filter, making it safe to
run multiple times. If tests fail, fix the reported error and re-run. All edits
are additive and can be re-applied safely.

## Artifacts and Notes

- Expected evidence includes rust-rspec scenario showing blocks attached to
  collidable tiles, plus captured test logs.
- The test map `primary-isometric-custom-properties.tmx` contains 4 collidable
  tiles at known positions for verification.

## Interfaces and Dependencies

The system function signature:

```rust
pub fn attach_collision_blocks(
    mut commands: Commands,
    mut map_events: EventReader<TiledEvent<MapCreated>>,
    collidable_tiles: Query<(Entity, &TilePos), (With<Collidable>, Without<Block>)>,
    mut block_id_counter: Local<i64>,
)
```

This system depends on:

- `bevy_ecs_tiled::prelude::TiledEvent`, `MapCreated`
- `bevy_ecs_tilemap::tiles::TilePos`
- `lille::map::Collidable`
- `lille::components::Block`
