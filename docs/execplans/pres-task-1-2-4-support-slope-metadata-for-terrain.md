# Support Slope Metadata for Terrain (Task 1.2.4)

This Execution Plan (ExecPlan) is a living document. The sections `Progress`,
`Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must
be kept up to date as work proceeds.

## Purpose / Big Picture

Extend the map translation system to attach `BlockSlope` components to tiles
that carry `SlopeProperties` metadata from Tiled. This enables the Differential
Dataflow-Based Stream Processing (DBSP) physics circuit to calculate
gradient-adjusted floor heights for sloped terrain, providing smooth movement
on inclines. When complete, sloped tiles expose gradient data that DBSP
consumes, enabling terrain-aware physics without breaking maps that contain no
slopes.

## Progress

- [x] Review roadmap and design context for Task 1.2.4.
- [x] Extend `attach_collision_blocks` to also attach `BlockSlope`.
- [x] Add inline unit test for gradient conversion.
- [x] Add rstest unit tests for slope attachment logic.
- [x] Add rust-rspec behavioural tests for slope property hydration.
- [x] Update roadmap to mark Task 1.2.4 complete.
- [x] Run validation commands and capture evidence.

## Surprises & Discoveries

- The test fixture map (`primary-isometric-custom-properties.tmx`) already
  contains `SlopeProperties` on all tiles with `grad_x=0.25` and `grad_y=0.5`,
  so behavioural tests work immediately without modifying the map.
- The DBSP input sync system (`src/dbsp_sync/input/sync.rs`) already queries
  `Option<&BlockSlope>` and pushes to the circuit when present, so no changes
  were needed to the DBSP integration layer.
- The floor height stream (`src/dbsp_circuit/streams/floor/mod.rs`) already
  uses `outer_join` to handle blocks with or without slopes, applying gradient
  adjustments correctly.

## Decision Log

- Decision: Extend existing `attach_collision_blocks` system rather than create
  a separate system for slope attachment. Rationale: Single event handler,
  guaranteed ID linkage (BlockSlope.block_id = Block.id in same pass), simpler
  code. Date/Author: 2025-12-24 / Codex.

- Decision: Use `f64::from(slope_props.grad_x)` for lossless widening from f32
  to f64. Rationale: Explicit conversion is clearer than `as` cast, and f32 to
  f64 is lossless. Wrapping in `OrderedFloat` is required for DBSP
  compatibility. Date/Author: 2025-12-24 / Codex.

- Decision: Only tiles with both `Collidable` and `SlopeProperties` receive
  `BlockSlope`. Rationale: Slopes are a property of terrain blocks; a slope
  without collision makes no physical sense. Simplifies query and aligns with
  design doc stating "slopes modify block floor height". Date/Author:
  2025-12-24 / Codex.

## Outcomes & Retrospective

**Outcome:** Task 1.2.4 is complete. Tiles marked with `SlopeProperties` in
Tiled now receive `BlockSlope` components when the map loads, linked to their
corresponding `Block` via matching IDs. The DBSP physics circuit can consume
these slopes for gradient-adjusted floor height calculations.

**Evidence:**

- 6 new rstest unit tests covering BlockSlope attachment, gradient conversion,
  ID linkage, and parameterized gradient values.
- 5 rust-rspec behavioural assertions confirming slope attachment, gradient
  values matching fixture (0.25, 0.5), and ID linkage (consolidated from 6).
- Validation complete: `make check-fmt`, `make lint`, `make test` all pass.

**Notes for future work:**

- Step 1.2 is now complete. All tasks (1.2.1, 1.2.2, 1.2.3, 1.2.4) are done.
- Step 1.3 can now proceed with spawn point handling.
- Future vertical stacking (z > 0) will require updates to both Block and
  BlockSlope attachment logic.

## Context and Orientation

The map translation module (`src/map/translate.rs`) bridges Tiled annotations
with DBSP physics components. Task 1.2.1 established the pattern: listen for
`TiledEvent<MapCreated>`, query tiles with markers, attach engine components.

`SlopeProperties` was registered as a Tiled custom property type in Task 1.1.3.
It contains `grad_x` and `grad_y` fields (f32) authored in the Tiled editor.
`BlockSlope` is the DBSP-compatible component that stores the same gradients as
`OrderedFloat<f64>` plus a `block_id` linking to the parent `Block`.

Data flow after this task:

```text
Tiled (.tmx) -> bevy_ecs_tiled hydrates SlopeProperties
             -> map translation system attaches BlockSlope (and Block)
             -> DBSP input system pushes to block_slope_in()
             -> floor_height_stream applies gradient adjustment
             -> physics circuit computes sloped floor heights
```

Key dependencies:

- `SlopeProperties` component defined in `src/map/mod.rs`
- `BlockSlope` component defined in `src/components.rs`
- `ordered_float::OrderedFloat` for DBSP-compatible floating-point
- Test map with slope data:
  `assets/maps/primary-isometric-custom-properties.tmx`

## Plan of Work

Extend the existing `attach_collision_blocks` system to also query for
`SlopeProperties` and attach `BlockSlope` when present. The query becomes
`(Entity, &TilePos, Option<&SlopeProperties>)` with filters
`(With<Collidable>, Without<Block>)`.

Add rstest unit tests verifying slope attachment logic, and rust-rspec
behavioural tests confirming the completion criteria: "Sloped tiles expose
gradient data that DBSP consumes without panics, even if the initial map
contains no slopes."

## Concrete Steps

1. Modify `src/map/translate.rs`:
   - Add imports for `BlockSlope`, `SlopeProperties`, `OrderedFloat`
   - Update query to include `Option<&SlopeProperties>`
   - After inserting `Block`, check for `SlopeProperties` and insert
     `BlockSlope`
   - Update module and function documentation

2. Add to `tests/map_collision_block_attachment.rs`:
   - Helper function `spawn_sloped_collidable_tile`
   - Test: `attaches_block_slope_to_sloped_entity`
   - Test: `does_not_attach_block_slope_when_no_slope_properties`
   - Test: `block_slope_id_matches_block_id`
   - Test: `block_slope_gradients_converted_correctly` (parameterized)
   - Test: `multiple_sloped_tiles_have_unique_block_ids`

3. Add to `tests/map_collision_rspec_block_attachment.rs`:
   - Helper methods: `block_slope_count`, `blocks_with_slopes_count`, etc.
   - Assertions for slope attachment, ID linkage, gradient values.

4. Update `docs/lille-map-and-presentation-roadmap.md`:
   - Mark `[x] Task 1.2.4`

5. Run validation commands:
   - `make check-fmt`
   - `make lint`
   - `make test`

## Validation and Acceptance

- Unit tests pass for BlockSlope attachment logic
- Behavioural tests confirm: all 4 tiles receive `BlockSlope` with correct
  gradients (0.25, 0.5) and IDs matching their `Block`
- `make check-fmt`, `make lint`, `make test` all complete with zero failures
- Roadmap updated with `[x]` for Task 1.2.4

## Idempotence and Recovery

The block attachment system uses `Without<Block>` filter, making it safe to run
multiple times. BlockSlope is inserted in the same command as Block, so partial
application is not possible. If tests fail, fix the reported error and re-run.

## Artifacts and Notes

- The test map `primary-isometric-custom-properties.tmx` contains 4 tiles with
  `grad_x=0.25` and `grad_y=0.5`.
- The DBSP floor height stream already handles the "no slopes" case via
  `outer_join`, so maps without slopes work correctly.

## Interfaces and Dependencies

The extended system function signature:

```rust
pub fn attach_collision_blocks(
    mut commands: Commands,
    mut map_events: EventReader<TiledEvent<MapCreated>>,
    collidable_tiles: Query<
        (Entity, &TilePos, Option<&SlopeProperties>),
        (With<Collidable>, Without<Block>),
    >,
    mut block_id_counter: Local<i64>,
)
```

This system depends on:

- `bevy_ecs_tiled::prelude::TiledEvent`, `MapCreated`
- `bevy_ecs_tilemap::tiles::TilePos`
- `lille::map::{Collidable, SlopeProperties}`
- `lille::components::{Block, BlockSlope}`
- `ordered_float::OrderedFloat`
