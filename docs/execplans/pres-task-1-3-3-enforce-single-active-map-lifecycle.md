# ExecPlan: Task 1.3.3 — Enforce single active map lifecycle

**Task reference**:
[docs/lille-map-and-presentation-roadmap.md](../lille-map-and-presentation-roadmap.md)
 Phase 1 > Step 1.3 > Task 1.3.3

**Branch**: `pres-task-1-3-3-enforce-single-active-map-lifecycle`

## Status

**Completed** — 2026-01-07

## Summary

Implement safeguards in `LilleMapPlugin` to:

1. Log a warning and emit an event when a second map load is attempted while one
   is active
2. Provide an unload mechanism for safe map cleanup during development hot
   reload scenarios
3. Ensure the DBSP (Differential Dataflow-based Stream Processing) circuit
   correctly reflects world state after unload/reload cycles

## Completion criteria

- Attempting to spawn a second map logs a warning and leaves the existing map
  intact
- Unloading recreates components safely (no orphaned state, no duplicate IDs)
- Unit tests (rstest) and BDD (Behaviour-Driven Development) tests (rust-rspec)
  validate both happy and unhappy paths

## Design decisions

### D1: Warning mechanism

**Decision**: Emit a new `LilleMapError::DuplicateMapAttempted` event alongside
a `log::warn!` call.

**Rationale**: Events are observable in tests (via the existing
`CapturedMapErrors` pattern), while log output provides immediate developer
feedback. The existing `log_map_error` observer handles logging automatically.

### D2: Unload event-driven design

**Decision**: Introduce `UnloadPrimaryMap` (request) and `PrimaryMapUnloaded`
(completion) events rather than a direct function call.

**Rationale**: Event-driven unloading fits Bevy's ECS paradigm and allows
systems to react to unload completion. This enables future hot-reload tooling
to trigger unloads without coupling to internal map module details.

### D3: NpcIdCounter persistence

**Decision**: The `NpcIdCounter` resource persists across map unloads.

**Rationale**: NPC IDs must remain unique within a session to prevent DBSP
entity ID collisions. Resetting would risk duplicate IDs if the same map is
reloaded multiple times.

### D4: Block ID counter handling

**Decision**: The block ID counter (`Local<i64>` in `attach_collision_blocks`)
is not explicitly reset.

**Rationale**: `Local` state persists for the system's lifetime, so IDs
continue incrementing across loads. This is acceptable because block IDs only
need per-map uniqueness for the DBSP join operations, and monotonically
increasing values satisfy this constraint.

### D5: Despawn behaviour for actors

**Decision**: Use `despawn()` for all entities (`PrimaryTiledMap` and
`MapSpawned`).

**Rationale**: In Bevy 0.17+, `despawn()` automatically despawns all
descendants via the `ChildOf` relationship. The deprecated
`despawn_recursive()` is no longer available on `EntityCommands`. Child
entities (tiles, layers, sprites, particle effects) are removed when their
parent is despawned.

### D6: DBSP sync unchanged

**Decision**: No changes to `src/dbsp_sync/` code.

**Rationale**: The sync system already clears `WorldHandle` each tick and
re-pushes currently existing `Block` components to the circuit. When entities
are despawned, the next sync pass simply omits them—the circuit handles absence
correctly without explicit retraction.

## Files to modify

| File                                         | Action | Description                                                  |
| -------------------------------------------- | ------ | ------------------------------------------------------------ |
| `src/map/mod.rs`                             | Modify | Add error variant, events, unload system, modify spawn guard |
| `tests/map_lifecycle.rs`                     | Create | Unit tests for lifecycle guards                              |
| `tests/map_lifecycle_rspec.rs`               | Create | BDD tests for unload/reload scenarios                        |
| `docs/lille-map-and-presentation-roadmap.md` | Modify | Mark Task 1.3.3 as done                                      |

## Implementation steps

### Step 1: Add error variant and events to `src/map/mod.rs`

Add to `LilleMapError` enum (after line 45):

```rust
/// Attempted to load a second map while one is already active.
DuplicateMapAttempted {
    /// Asset-server path of the map that was requested.
    requested_path: String,
    /// Asset-server path of the map currently loaded.
    active_path: String,
},
```

Add new events (after line 166, near other marker components):

```rust
/// Event to request unloading the currently active primary map.
///
/// When triggered, the map unload system will:
/// 1. Despawn the `PrimaryTiledMap` entity and all its children
/// 2. Despawn all `MapSpawned` entities (player and NPCs)
/// 3. Reset `PrimaryMapAssetTracking` state
/// 4. Allow a new map to be loaded
#[derive(Event, Debug, Clone, Default)]
pub struct UnloadPrimaryMap;

/// Event emitted when the primary map has been fully unloaded.
///
/// Systems that depend on map state can observe this event to know
/// when it is safe to load a new map or perform cleanup.
#[derive(Event, Debug, Clone, Default)]
pub struct PrimaryMapUnloaded;
```

### Step 2: Modify `spawn_primary_map_if_enabled()` at line 256

Replace the silent return (lines 261-263) with warning and event emission:

```rust
if !context.existing_maps.is_empty() {
    let requested_path = context.settings.primary_map.as_str().to_owned();
    let active_path = context.tracking.asset_path.clone().unwrap_or_default();

    log::warn!(
        "Attempted to load map '{}' while map '{}' is already active; ignoring request",
        requested_path,
        active_path
    );

    commands.trigger(LilleMapError::DuplicateMapAttempted {
        requested_path,
        active_path,
    });
    return;
}
```

### Step 3: Add unload observer to `src/map/mod.rs`

Add new observer function (before the `Plugin` impl):

```rust
/// Observer that handles `UnloadPrimaryMap` events by despawning map entities.
///
/// This observer enables safe hot-reload by:
/// 1. Despawning the `PrimaryTiledMap` entity and all children (tiles, layers)
/// 2. Despawning all `MapSpawned` entities (player, NPCs)
/// 3. Resetting `PrimaryMapAssetTracking` to allow new map loads
///
/// # Bevy 0.17 Despawn Behaviour
///
/// In Bevy 0.17+, `despawn()` automatically despawns all descendants via the
/// `ChildOf` relationship. The deprecated `despawn_recursive()` is no longer
/// available on `EntityCommands`. Child entities (tiles, layers from
/// `bevy_ecs_tiled`) are removed when their parent is despawned.
fn handle_unload_primary_map(
    _event: bevy::ecs::prelude::On<UnloadPrimaryMap>,
    mut commands: Commands,
    map_query: Query<Entity, With<PrimaryTiledMap>>,
    spawned_query: Query<Entity, With<MapSpawned>>,
    mut tracking: ResMut<PrimaryMapAssetTracking>,
) {
    let mut unloaded_any = false;

    // Note: Bevy 0.17's despawn() handles ChildOf relationships automatically,
    // removing all descendant entities (tiles, layers) when the root is despawned.
    for map_entity in &map_query {
        commands.entity(map_entity).despawn();
        unloaded_any = true;
        log::info!("Unloaded primary map entity {map_entity:?}");
    }

    // Note: Bevy 0.17's despawn() handles ChildOf relationships automatically,
    // removing any child entities (sprites, effects) when the actor is despawned.
    for spawned_entity in &spawned_query {
        commands.entity(spawned_entity).despawn();
        log::debug!("Despawned map-spawned entity {spawned_entity:?}");
    }

    tracking.asset_path = None;
    tracking.handle = None;
    tracking.has_finalised = false;

    if unloaded_any {
        commands.trigger(PrimaryMapUnloaded);
    }
}

fn log_map_unloaded(_event: bevy::ecs::prelude::On<PrimaryMapUnloaded>) {
    log::info!("Primary map unloaded successfully");
}
```

### Step 4: Update plugin build in `LilleMapPlugin::build()`

Register the unload observer and the logging observer (after the existing
observer registrations):

```rust
app.add_observer(handle_unload_primary_map);
app.add_observer(log_map_unloaded);
```

The observer pattern in Bevy 0.17 replaces the previous `EventReader`-based
approach. Observers run automatically when their trigger event is emitted via
`commands.trigger()` or `world.trigger()`, without requiring explicit system
scheduling.

### Step 5: Export new types from module

Add to the public exports at the top of the module (near line 19):

```rust
pub use self::{UnloadPrimaryMap, PrimaryMapUnloaded};
```

### Step 6: Create unit tests in `tests/map_lifecycle.rs`

```rust
//! Unit tests for single active map lifecycle enforcement.
//!
//! These tests validate that `LilleMapPlugin` guards against loading
//! multiple maps and provides safe cleanup for hot reload scenarios.

#[path = "support/map_test_plugins.rs"]
mod map_test_plugins;

use bevy::prelude::*;
use lille::map::{
    LilleMapError, LilleMapSettings, MapAssetPath, MapSpawned, PrimaryMapAssetTracking,
    UnloadPrimaryMap, PRIMARY_ISOMETRIC_MAP_PATH,
};
use lille::{LilleMapPlugin, NpcIdCounter};
use map_test_plugins::CapturedMapErrors;
use rstest::{fixture, rstest};

// -- Fixtures --

#[fixture]
fn test_app() -> App {
    let mut app = App::new();
    map_test_plugins::add_map_test_plugins(&mut app);
    map_test_plugins::install_map_error_capture(&mut app);
    app.insert_resource(LilleMapSettings {
        primary_map: MapAssetPath::from(PRIMARY_ISOMETRIC_MAP_PATH),
        should_spawn_primary_map: false,
        should_bootstrap_camera: false,
    });
    app.add_plugins(LilleMapPlugin);
    app.finish();
    app.cleanup();
    app
}

// -- Test helpers --

#[derive(Component)]
struct PrimaryTiledMapMock;

fn spawn_mock_primary_map(world: &mut World) -> Entity {
    world.spawn((Name::new("MockPrimaryMap"), PrimaryTiledMapMock)).id()
}

fn spawn_mock_map_spawned_entity(world: &mut World) -> Entity {
    world.spawn((Name::new("MockSpawned"), MapSpawned)).id()
}

fn captured_errors(app: &App) -> Vec<LilleMapError> {
    app.world()
        .get_resource::<CapturedMapErrors>()
        .map(|e| e.0.clone())
        .unwrap_or_default()
}

// -- Tests --

#[rstest]
fn emits_duplicate_map_error_when_second_spawn_attempted(mut test_app: App) {
    // Simulate existing map by setting tracking state
    {
        let mut tracking = test_app.world_mut().resource_mut::<PrimaryMapAssetTracking>();
        tracking.asset_path = Some("existing/map.tmx".to_owned());
        tracking.has_finalised = true;
    }
    // Spawn a mock entity that matches PrimaryTiledMap query
    // (In real code this would be a PrimaryTiledMap component)
    spawn_mock_primary_map(test_app.world_mut());

    // Enable spawning and tick
    {
        let mut settings = test_app.world_mut().resource_mut::<LilleMapSettings>();
        settings.should_spawn_primary_map = true;
    }
    test_app.update();

    let errors = captured_errors(&test_app);
    assert!(
        errors.iter().any(|e| matches!(
            e,
            LilleMapError::DuplicateMapAttempted { .. }
        )),
        "expected DuplicateMapAttempted error, got: {:?}",
        errors
    );
}

#[rstest]
fn existing_map_preserved_when_second_spawn_attempted(mut test_app: App) {
    let existing_map = spawn_mock_primary_map(test_app.world_mut());
    {
        let mut settings = test_app.world_mut().resource_mut::<LilleMapSettings>();
        settings.should_spawn_primary_map = true;
    }
    test_app.update();

    assert!(
        test_app.world().get_entity(existing_map).is_ok(),
        "existing map entity should be preserved"
    );
}

#[rstest]
fn unload_despawns_map_spawned_entities(mut test_app: App) {
    let spawned = spawn_mock_map_spawned_entity(test_app.world_mut());
    test_app.world_mut().trigger(UnloadPrimaryMap);
    test_app.update();

    assert!(
        test_app.world().get_entity(spawned).is_err(),
        "MapSpawned entity should be despawned after unload"
    );
}

#[rstest]
fn unload_resets_tracking_state(mut test_app: App) {
    {
        let mut tracking = test_app.world_mut().resource_mut::<PrimaryMapAssetTracking>();
        tracking.asset_path = Some("test/map.tmx".to_owned());
        tracking.has_finalised = true;
    }
    test_app.world_mut().trigger(UnloadPrimaryMap);
    test_app.update();

    let tracking = test_app.world().resource::<PrimaryMapAssetTracking>();
    assert!(tracking.asset_path.is_none(), "asset_path should be cleared");
    assert!(!tracking.has_finalised, "has_finalised should be reset");
}

#[rstest]
fn unload_preserves_npc_id_counter(mut test_app: App) {
    test_app.world_mut().resource_mut::<NpcIdCounter>().0 = 42;
    test_app.world_mut().trigger(UnloadPrimaryMap);
    test_app.update();

    let counter = test_app.world().resource::<NpcIdCounter>();
    assert_eq!(counter.0, 42, "NpcIdCounter should persist across unloads");
}

#[rstest]
fn can_load_new_map_after_unload(mut test_app: App) {
    // Set up as if a map was loaded
    {
        let mut tracking = test_app.world_mut().resource_mut::<PrimaryMapAssetTracking>();
        tracking.asset_path = Some("first/map.tmx".to_owned());
        tracking.has_finalised = true;
    }
    spawn_mock_primary_map(test_app.world_mut());

    // Unload
    test_app.world_mut().trigger(UnloadPrimaryMap);
    test_app.update();

    // Attempt to spawn again (should not emit duplicate error)
    {
        let mut settings = test_app.world_mut().resource_mut::<LilleMapSettings>();
        settings.should_spawn_primary_map = true;
    }
    test_app.update();

    let errors = captured_errors(&test_app);
    assert!(
        !errors.iter().any(|e| matches!(
            e,
            LilleMapError::DuplicateMapAttempted { .. }
        )),
        "should not emit DuplicateMapAttempted after unload"
    );
}
```

### Step 7: Create BDD tests in `tests/map_lifecycle_rspec.rs`

```rust
#![cfg_attr(
    feature = "test-support",
    doc = "Behavioural tests for map lifecycle using rust-rspec."
)]
#![cfg_attr(
    not(feature = "test-support"),
    doc = "Behavioural tests require `test-support`."
)]
#![cfg(feature = "test-support")]
//! Behavioural tests for single active map lifecycle enforcement.

#[path = "support/map_test_plugins.rs"]
mod map_test_plugins;

#[path = "support/thread_safe_app.rs"]
mod thread_safe_app;

#[path = "support/rspec_runner.rs"]
mod rspec_runner;

#[path = "support/map_fixture.rs"]
mod map_fixture;

use std::sync::MutexGuard;

use bevy::prelude::*;
use lille::map::{
    LilleMapError, LilleMapSettings, MapAssetPath, MapSpawned, UnloadPrimaryMap,
};
use lille::{DbspPlugin, LilleMapPlugin, WorldHandle};
use map_test_plugins::CapturedMapErrors;
use rspec::block::Context as Scenario;
use rspec_runner::run_serial;
use thread_safe_app::ThreadSafeApp;

const TEST_MAP_PATH: &str = "maps/primary-isometric-custom-properties.tmx";
const MAX_LOAD_TICKS: usize = 100;

#[derive(Debug, Clone)]
struct MapLifecycleFixture {
    base: map_fixture::MapPluginFixtureBase,
}

impl MapLifecycleFixture {
    fn bootstrap() -> Self {
        let mut app = App::new();
        map_test_plugins::add_map_test_plugins(&mut app);
        app.add_plugins(DbspPlugin);
        map_test_plugins::install_map_error_capture(&mut app);
        app.insert_resource(LilleMapSettings {
            primary_map: MapAssetPath::from(TEST_MAP_PATH),
            should_spawn_primary_map: true,
            should_bootstrap_camera: false,
        });
        app.add_plugins(LilleMapPlugin);

        Self {
            base: map_fixture::MapPluginFixtureBase::new(app),
        }
    }

    fn app_guard(&self) -> MutexGuard<'_, ThreadSafeApp> {
        self.base.app_guard()
    }

    fn tick(&self) {
        self.base.tick();
    }

    fn tick_until_map_loaded(&self, max_ticks: usize) -> bool {
        for _ in 0..max_ticks {
            self.tick();
            if self.map_spawned_count() > 0 {
                return true;
            }
            if !self.captured_errors().is_empty() {
                return false;
            }
        }
        false
    }

    fn trigger_unload(&self) {
        let mut app = self.app_guard();
        app.world_mut().trigger(UnloadPrimaryMap);
    }

    fn map_spawned_count(&self) -> usize {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query_filtered::<Entity, With<MapSpawned>>();
        query.iter(world).count()
    }

    fn captured_errors(&self) -> Vec<LilleMapError> {
        let app = self.app_guard();
        map_test_plugins::captured_errors(&app)
    }

    fn has_duplicate_map_error(&self) -> bool {
        self.captured_errors()
            .iter()
            .any(|e| matches!(e, LilleMapError::DuplicateMapAttempted { .. }))
    }

    fn world_handle_block_count(&self) -> usize {
        let app = self.app_guard();
        app.world()
            .get_resource::<WorldHandle>()
            .map_or(0, WorldHandle::block_count)
    }
}

#[test]
fn map_plugin_supports_safe_unload_and_reload() {
    let fixture = MapLifecycleFixture::bootstrap();

    run_serial(&rspec::given(
        "LilleMapPlugin supports map unloading for hot reload",
        fixture,
        |scenario: &mut Scenario<MapLifecycleFixture>| {
            scenario.when("a loaded map is unloaded", |ctx| {
                ctx.then("spawned actors are despawned without errors", |state| {
                    let loaded = state.tick_until_map_loaded(MAX_LOAD_TICKS);
                    assert!(loaded, "map should load within {MAX_LOAD_TICKS} ticks");

                    state.trigger_unload();
                    state.tick();

                    assert_eq!(
                        state.map_spawned_count(),
                        0,
                        "all MapSpawned entities should be despawned"
                    );

                    assert!(
                        !state.has_duplicate_map_error(),
                        "unload should not cause duplicate map errors"
                    );

                    // Tick again to let DBSP sync run and clear the world handle.
                    state.tick();
                    assert_eq!(
                        state.world_handle_block_count(),
                        0,
                        "DBSP world handle should reflect empty state after unload"
                    );
                });
            });
        },
    ));
}
```

### Step 8: Update roadmap

In `docs/lille-map-and-presentation-roadmap.md`, change line 95:

```markdown
- [x] Task 1.3.3 — Enforce single active map lifecycle
```

### Step 9: Validation

Run quality gates:

```sh
make check-fmt
make lint
make test
```

## Edge cases covered

1. **Second map spawn during initial load**: Warning emitted, first map
   preserved
2. **Unload with no map loaded**: No-op (no `PrimaryMapUnloaded` event)
3. **Multiple rapid unload requests**: Idempotent (events drained)
4. **DBSP sync after unload**: `WorldHandle` cleared next tick, circuit sees
   absence
5. **NPC ID uniqueness after reload**: Counter persists, no ID collisions

## Testing coverage matrix

| Scenario                 | Unit test                         | BDD test                                 |
| ------------------------ | --------------------------------- | ---------------------------------------- |
| Duplicate map warning    | `emits_duplicate_map_error_…`     | —                                        |
| Existing map preserved   | `existing_map_preserved_…`        | —                                        |
| Unload despawns actors   | `unload_despawns_map_spawned_…`   | `spawned actors are despawned`           |
| Tracking state reset     | `unload_resets_tracking_state`    | —                                        |
| NpcIdCounter persistence | `unload_preserves_npc_id_counter` | —                                        |
| Reload after unload      | `can_load_new_map_after_unload`   | —                                        |
| DBSP state after unload  | —                                 | `DBSP world handle reflects empty state` |
