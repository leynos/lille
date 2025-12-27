# Execution Plan: Task 1.3.1 — Define PlayerSpawn and Spawn Point Components

## Summary

This task extends the existing `PlayerSpawn` and `SpawnPoint` Bevy components
with Differential Dataflow for Batch and Stream Processing (DBSP) circuit
integration, enabling the circuit to be the source of truth for spawn-related
behaviour such as floor height queries at spawn locations.

**Roadmap reference:** Phase 1, Step 1.3, Task 1.3.1

## Prior State

Before this task, the following components were already in place:

- `PlayerSpawn` and `SpawnPoint` components defined in `src/map/mod.rs`
- Type registration via `app.register_type::<PlayerSpawn>()` and
  `app.register_type::<SpawnPoint>()`
- Tiled custom property mapping in
  `assets/maps/primary-isometric-custom-properties.tmx`
- Comprehensive tests for hydration in:
  - `tests/map_plugin_property_type_registry.rs` (type registration)
  - `tests/map_plugin_rspec_custom_properties.rs` (Tiled hydration)

## Implementation

### DBSP Record Types

Added two new DBSP-compatible record types in `src/dbsp_circuit/types.rs`:

```rust
pub struct PlayerSpawnLocation {
    pub id: i64,                    // Entity bits
    pub x: OrderedFloat<f64>,       // World-space X
    pub y: OrderedFloat<f64>,       // World-space Y
    pub z: OrderedFloat<f64>,       // World-space Z
}

pub struct SpawnPointRecord {
    pub id: i64,                    // Entity bits
    pub x: OrderedFloat<f64>,       // World-space X
    pub y: OrderedFloat<f64>,       // World-space Y
    pub z: OrderedFloat<f64>,       // World-space Z
    pub enemy_type: u32,            // Enemy archetype identifier
    pub respawn: bool,              // Respawn after use
}
```

### Circuit Input Handles

Added input handles to `DbspCircuit` in `src/dbsp_circuit/circuit.rs`:

- `player_spawn_in: ZSetHandle<PlayerSpawnLocation>`
- `spawn_point_in: ZSetHandle<SpawnPointRecord>`

With corresponding accessor methods:

- `pub const fn player_spawn_in(&self) -> &ZSetHandle<PlayerSpawnLocation>`
- `pub const fn spawn_point_in(&self) -> &ZSetHandle<SpawnPointRecord>`

### Sync Functions

Added sync functions in `src/dbsp_sync/input/sync.rs`:

- `player_spawns()` — Queries `(Entity, &Transform), With<PlayerSpawn>` and
  pushes `PlayerSpawnLocation` records
- `spawn_points()` — Queries `(Entity, &Transform, &SpawnPoint)` and pushes
  `SpawnPointRecord` records

Both functions are gated behind `#[cfg(feature = "map")]` to match the
conditional compilation of the `map` module.

### System Integration

The `cache_state_for_dbsp_system` in `src/dbsp_sync/input/mod.rs` was
refactored to:

1. Extract shared logic into `cache_state_for_dbsp_impl()` helper
2. Provide two variants via `#[cfg(feature = "map")]` and
   `#[cfg(not(feature = "map"))]`
3. The `map` variant adds spawn queries and calls the sync functions before
   the shared implementation

## Design Decisions

| Decision                              | Rationale                                                 |
| ------------------------------------- | --------------------------------------------------------- |
| Feed spawns each tick                 | Consistent with blocks pattern; DBSP clears inputs anyway |
| Use `entity.to_bits() as i64` for IDs | Guaranteed unique per entity; no counter state needed     |
| No output streams initially           | Spawn floor heights can be added later if needed          |
| Conditional compilation               | Respects existing `map` feature gate                      |

## Files Modified

- `src/dbsp_circuit/types.rs` — Added `PlayerSpawnLocation`, `SpawnPointRecord`
- `src/dbsp_circuit/circuit.rs` — Added input handles and accessors
- `src/dbsp_circuit/mod.rs` — Exported new types
- `src/dbsp_sync/input/sync.rs` — Added spawn sync functions
- `src/dbsp_sync/input/mod.rs` — Integrated spawn sync into system

## Validation

All quality gates pass:

- `make check-fmt` — Formatting valid
- `make lint` — No Clippy warnings
- `make test` — All tests pass, including:
  - Existing Tiled hydration tests
  - Doc tests for new DBSP types

The existing test suite in `tests/map_plugin_rspec_custom_properties.rs`
validates the end-to-end flow: Tiled map loading → component hydration → DBSP
sync (via `app.update()` in the test harness).

## Completion Criteria

> Loading a map with these objects results in Entity Component System (ECS)
> entities carrying the expected component values.

This criterion is satisfied by:

1. The existing Tiled hydration tests verifying component values match Tiled
   definitions
2. The DBSP sync system pushing those components into the circuit on each tick
3. All tests passing after the changes

## Future Work

- Task 1.3.2 will use spawn locations to instantiate player/NPC entities
- Optional: Add `PlayerSpawnFloor` and `SpawnPointFloor` output streams to
  provide floor heights at spawn locations via DBSP join operations
