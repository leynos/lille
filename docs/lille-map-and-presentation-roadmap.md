# Lille development roadmap: map and presentation layers

This document outlines the development plan for implementing the isometric map
integration and presentation layer described in the design proposals. The
roadmap is structured into phases, steps, and tasks so each increment is
measurable and achievable without binding to calendar commitments.

## Phase 1: Data-driven world foundation

Why this matters: This phase replaces hardcoded world geometry with a
data-driven pipeline sourced from the Tiled editor. Completing it enables level
designers to describe environments that flow directly into Lille's ECS and DBSP
physics circuit.

### Step 1.1: Integrate LilleMapPlugin and asset pipeline

What we will build: A Bevy plugin that encapsulates map asset loading and
exposes a single entry point for spawning the active isometric map.

- [ ] Task 1.1.1 — Create LilleMapPlugin skeleton
  - Outcome: `src/map.rs` exposes `LilleMapPlugin` registering
    `bevy_ecs_tiled::TiledPlugin` and the module is wired into `main.rs`.
  - Completion criteria: The application compiles with the plugin enabled and
    the legacy `spawn_world_system` path is removed.
  - Dependencies: None.
- [ ] Task 1.1.2 — Load primary isometric map asset
  - Outcome: `LilleMapPlugin` spawns an entity with a `TiledMap` component
    pointing at the selected `.tmx` file in `assets/maps` and ensures the Bevy
    hierarchy loads.
  - Completion criteria: Launching the game loads the baseline isometric map
    without runtime errors and renders base tile layers.
  - Dependencies: Task 1.1.1.
- [ ] Task 1.1.3 — Register custom property types for map data
  - Outcome: `LilleMapPlugin` enables the `user_properties` feature of
    `bevy_ecs_tiled` and registers Lille components via `App::register_type`.
  - Completion criteria: Maps containing typed custom properties yield ECS
    entities with matching components populated from Tiled data.
  - Dependencies: Task 1.1.2.

### Step 1.2: Translate map data into engine state

What we will build: Systems that honour Tiled annotations by attaching Lille
components and feeding static geometry into DBSP precisely once per map load.

- [ ] Task 1.2.1 — Map collision annotations to Wall component
  - Outcome: A `Wall` component is defined and automatically attached to tile
    entities whose Tiled custom property marks them as collision geometry.
  - Completion criteria: Querying loaded maps for `Wall` returns every wall
    tile and no unrelated tiles.
  - Dependencies: Task 1.1.3.
- [ ] Task 1.2.2 — Attach physics blocks via Tiled events
  - Outcome: Systems listening to `TiledEvent<ObjectCreated>`
    or `TiledEvent<LayerCreated>` compute `Block` identifiers and insert
    `Block` components on relevant entities after spawn.
  - Completion criteria: Each wall entity gains a stable `Block` with
    coordinates and identifier derived from its tile position.
  - Dependencies: Task 1.2.1.
- [ ] Task 1.2.3 — Feed static geometry to DBSP
  - Outcome: A one-shot system reacting to `TiledEvent<MapCreated>` gathers all
    `Block` (and later `BlockSlope`) records and pushes them into the DBSP
    `block_in` and `block_slope_in` streams.
  - Completion criteria: DBSP physics receives block data matching the loaded
    map, confirmed via logging or debug visualisation.
  - Dependencies: Task 1.2.2.
- [ ] Task 1.2.4 — Support slope metadata for terrain
  - Outcome: Tiles flagged with slope information in Tiled produce
    `BlockSlope` components linked to their parent `Block` identifiers.
  - Completion criteria: Sloped tiles expose gradient data that DBSP consumes
    without panics, even if the initial map contains no slopes.
  - Dependencies: Task 1.2.2.

### Step 1.3: Spawn entities from map metadata

What we will build: Event-driven systems that interpret Tiled object layers as
spawn locations and gameplay markers, ensuring Lille entities are created at
authored positions.

- [ ] Task 1.3.1 — Define PlayerSpawn and spawn point components
  - Outcome: Typed components such as `PlayerSpawn` and `SpawnPoint` (with
    enemy metadata) are added, registered, and serialised from Tiled classes.
  - Completion criteria: Loading a map with these objects results in ECS
    entities carrying the expected component values.
  - Dependencies: Task 1.1.3.
- [ ] Task 1.3.2 — Spawn player and actors on map readiness
  - Outcome: The map readiness system locates `PlayerSpawn` entities and
    instantiates the player, NPCs, and other scripted actors at those
    transforms.
  - Completion criteria: Running the game spawns player and sample NPCs at
    their Tiled-authored coordinates exactly once per map load.
  - Dependencies: Task 1.3.1.
- [ ] Task 1.3.3 — Enforce single active map lifecycle
  - Outcome: `LilleMapPlugin` guards against loading multiple maps at once and
    provides a cleanup path for reloading during development hot reload.
  - Completion criteria: Attempting to spawn a second map logs a warning and
    leaves the existing map intact; unloading recreates components safely.
  - Dependencies: Task 1.1.2.

## Phase 2: Visualisation and interaction layer

Why this matters: This phase gives life to the data-driven world by rendering
entities with sprites, managing camera control, and preserving the separation
between simulation and presentation.

### Step 2.1: Establish presentation plugin

What we will build: A dedicated `PresentationPlugin` that owns camera setup and
input handling while remaining a passive observer of simulation state.

- [ ] Task 2.1.1 — Create PresentationPlugin and camera marker
  - Outcome: `src/presentation.rs` defines `PresentationPlugin`, spawns a
    `Camera2dBundle`, and tags it with a `CameraController` component.
  - Completion criteria: The application renders through the presentation
    camera and legacy camera setup code is removed from map systems.
  - Dependencies: Task 1.1.1.
- [ ] Task 2.1.2 — Implement camera panning controls
  - Outcome: An update system reads keyboard input to move the camera based on
    configurable speed scaled by `Time.delta_seconds()`.
  - Completion criteria: Holding WASD or arrow keys pans smoothly across the
    map without frame-rate coupling.
  - Dependencies: Task 2.1.1.
- [ ] Task 2.1.3 — Implement camera zoom controls
  - Outcome: Mouse wheel input adjusts the camera's
    `OrthographicProjection::scale` within clamped bounds.
  - Completion criteria: Zooming in and out behaves predictably and never
    exceeds configured minimum or maximum zoom levels.
  - Dependencies: Task 2.1.2.
- [ ] Task 2.1.4 — Order presentation systems after DBSP outputs
  - Outcome: System scheduling ensures camera and rendering systems run after
    DBSP output application so they always read up-to-date transforms.
  - Completion criteria: Bevy schedule diagnostics confirm ordering and no
    stale positions reach the presentation layer.
  - Dependencies: Task 2.1.1.

### Step 2.2: Render dynamic entities

What we will build: Sprite-based representations for the player and NPCs,
including atlas loading, Y-sorting, and integration with map-driven spawns.

- [ ] Task 2.2.1 — Load actor sprite sheet resource
  - Outcome: A startup system loads the actor texture atlas and exposes it as
    an `ActorSpriteSheet` resource for reuse.
  - Completion criteria: Other systems can access the resource and the asset
    handles stay valid across hot reload.
  - Dependencies: Task 2.1.1.
- [ ] Task 2.2.2 — Spawn player sprite at PlayerSpawn
  - Outcome: The map readiness workflow spawns the player with a
    `SpriteSheetBundle` referencing the atlas and aligns it with the
    `PlayerSpawn` transform.
  - Completion criteria: The player sprite appears at the authored start
    location and no primitive placeholder meshes remain.
  - Dependencies: Task 2.2.1, Task 1.3.2.
- [ ] Task 2.2.3 — Implement Y-sorted depth management
  - Outcome: A `YSorted` marker and update system set
    `Transform.translation.z` based on the entity's Y coordinate.
  - Completion criteria: Entities nearer the bottom of the screen render in
    front of those higher up, verified with overlapping sprites.
  - Dependencies: Task 2.2.2.
- [ ] Task 2.2.4 — Render NPCs spawned from map metadata
  - Outcome: NPC entities generated from Tiled `SpawnPoint` data gain sprites
    and join the Y-sorted system without duplicating spawn logic.
  - Completion criteria: Sample NPC sprites appear at their map-defined
    locations and respect depth ordering.
  - Dependencies: Task 2.2.2, Task 1.3.2.

### Step 2.3: Maintain separation between logic and rendering

What we will build: Guardrails that keep presentation code observational so the
simulation remains authoritative.

- [ ] Task 2.3.1 — Audit presentation systems for read-only access
  - Outcome: System parameters are updated to request read-only access to
    gameplay components, and mutable access is limited to presentation data.
  - Completion criteria: Clippy and compiler checks show no unintended mutable
    borrows of logic components within the presentation module.
  - Dependencies: Task 2.1.1.
- [ ] Task 2.3.2 — Document presentation extension points
  - Outcome: Module-level docs and inline comments describe how to add new
    visual elements without coupling to logic systems.
  - Completion criteria: The documentation in `src/presentation.rs` aligns with
    the style guide and explains extension steps for future work.
  - Dependencies: Task 2.2.3.

Completing these phases establishes the foundation for richer tooling-driven
level design and a maintainable visual presentation layer.
