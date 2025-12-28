# Execution Plan: Task 1.3.2 — Spawn Player and Actors on Map Readiness

## Summary

This task implements the spawning system that instantiates player and
non-player character (NPC) entities at their Tiled-authored coordinates when
`TiledEvent<MapCreated>` fires. The system ensures spawning happens exactly
once per map load through idempotent marker components, maintaining the
Differential Dataflow-Based Stream Processing (DBSP) circuit as the source of
truth for inferred behaviour.

**Roadmap reference:** Phase 1, Step 1.3, Task 1.3.2

## Prior State

Before this task, the following components were already in place:

- `PlayerSpawn` and `SpawnPoint` components defined in `src/map/mod.rs`
- DBSP record types `PlayerSpawnLocation` and `SpawnPointRecord` in
  `src/dbsp_circuit/types.rs`
- Tiled custom property hydration for spawn markers
- `TiledEvent<MapCreated>` pattern established by `attach_collision_blocks`

## Implementation

### Marker Components

Added four new marker components in `src/map/mod.rs`:

```rust
/// Marker indicating that this entity represents the player character.
pub struct Player;

/// Marker indicating that this PlayerSpawn point has been consumed.
pub struct PlayerSpawnConsumed;

/// Marker indicating that this SpawnPoint has spawned its actor.
pub struct SpawnPointConsumed;

/// Marker for entities spawned by the map spawn system.
pub struct MapSpawned;
```

All are registered with `app.register_type::<T>()` for reflection.

### Spawn Module

Created `src/map/spawn.rs` containing:

**Entity Bundles:**

- `PlayerBundle` — Components for spawned player entity: `Player`,
  `MapSpawned`, `DdlogId`, `Transform`, `Name`, `Health`, `VelocityComp`
- `NpcBundle` — Components for spawned NPC entities: `MapSpawned`, `DdlogId`,
  `Transform`, `Name`, `Health`, `VelocityComp`, `UnitType`

**Spawning System:**

- `spawn_actors_at_spawn_points` — Main system triggered by
  `TiledEvent<MapCreated>`:
  - Queries `PlayerSpawn` entities `Without<PlayerSpawnConsumed>`
  - Queries `SpawnPoint` entities `Without<SpawnPointConsumed>`
  - Spawns player at first available `PlayerSpawn`, marks it consumed
  - Spawns NPCs at each `SpawnPoint`, marks non-respawning ones consumed

**Helper Functions:**

- `spawn_player()` — Creates player entity from spawn point
- `spawn_npcs()` — Creates NPC entities from spawn points
- `archetype_from_enemy_type()` — Maps `enemy_type` to `UnitType` and stats

### ID Assignment Strategy

| Entity | ID Source                          |
| ------ | ---------------------------------- |
| Player | `spawn_entity.to_bits() as i64`    |
| NPC    | `NPC_ID_BASE (i64::MIN) + counter` |

The offset ensures no collision between player entity bits and NPC IDs.

### System Integration

The spawn system is added to the `Update` schedule alongside other map systems:

```rust
app.add_systems(Update, (
    monitor_primary_map_load_state,
    translate::attach_collision_blocks,
    spawn::spawn_actors_at_spawn_points,
));
```

## Design Decisions

| Decision                         | Rationale                                           |
| -------------------------------- | --------------------------------------------------- |
| New `spawn.rs` module            | Single responsibility; parallel to `translate.rs`   |
| `*Consumed` marker components    | Idempotency via `Without<T>` filter (matches Block) |
| `Player` marker component        | Simple, query-friendly player identification        |
| ID = entity bits + counter       | Unique, traceable, handles respawns                 |
| `TiledEvent<MapCreated>` trigger | Consistent with collision block system              |
| Floor height query deferred      | Z from Tiled sufficient for Phase 1                 |

## Files Modified

- `src/map/mod.rs` — Added marker components, module export, type registration
- `src/map/spawn.rs` — **NEW** — Core spawning system and bundles

## Files Created for Testing

- `tests/map_spawn_actors.rs` — 14 rstest unit tests covering:
  - Player spawning at correct location
  - Player has required components
  - PlayerSpawn marked consumed
  - Idempotent spawning (no duplicates)
  - NPC spawning at correct locations
  - NPC UnitType mapping from enemy_type
  - Non-respawning SpawnPoints marked consumed
  - Respawning SpawnPoints NOT marked consumed
  - Unique DdlogId assignment

- `tests/map_spawn_actors_rspec.rs` — 2 behaviour-driven development (BDD)
  scenarios with 15 assertions:
  - Player spawning scenario (position, components, consumption)
  - NPC spawning scenario (position, UnitType, consumption logic)

## Test Updates

- `tests/map_plugin_rspec_custom_properties.rs` — Updated assertions:
  - "DBSP world handle includes spawned entities" (expects 2: player + NPC)
  - "spawned entities have DdlogId for DBSP sync" (expects 2 IDs)

## Validation

All quality gates pass:

- `make check-fmt` — Formatting valid
- `make lint` — No Clippy warnings
- `make test` — All tests pass (14 unit + 15 BDD assertions for spawn module)

## Completion Criteria

> Running the game spawns player and sample NPCs at their Tiled-authored
> coordinates exactly once per map load.

This criterion is satisfied by:

1. The `spawn_actors_at_spawn_points` system triggers on `MapCreated` events
2. Player and NPC entities spawn at `Transform` positions from Tiled data
3. `*Consumed` markers prevent duplicate spawning on subsequent events
4. Unit and BDD tests verify all behaviours across multiple scenarios

## Future Work

- Task 1.3.3 will enforce single active map lifecycle
- Phase 2 will add sprite-based rendering for spawned entities
- Optional: Query DBSP for floor heights at spawn locations
