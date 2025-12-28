//! Actor spawning systems for map-driven entity creation.
//!
//! This module bridges Tiled spawn point markers with actual game entities by
//! listening for `TiledEvent<MapCreated>` and instantiating player and NPC
//! entities at their authored positions.
//!
//! The spawning system is idempotent: spawn points are marked as "consumed"
//! after use, preventing duplicate spawns on subsequent events or map reloads.

use bevy::prelude::*;
use bevy_ecs_tiled::prelude::{MapCreated, TiledEvent};

use crate::components::{DdlogId, Health, UnitType, VelocityComp};
use crate::map::{
    MapSpawned, Player, PlayerSpawn, PlayerSpawnConsumed, SpawnPoint, SpawnPointConsumed,
};

/// Resource tracking the next NPC ID to assign.
///
/// This counter persists across map loads within an application session,
/// ensuring unique `DdlogId` values for all spawned NPCs. The counter starts
/// at 0 and increments with each NPC spawned; the final ID is computed as
/// `NPC_ID_BASE + counter` to avoid collision with player entity IDs.
///
/// For cross-session persistence, serialize this resource before shutdown.
#[derive(Resource, Debug, Default)]
pub struct NpcIdCounter(pub i64);

/// Bundle of components for the player entity.
///
/// This bundle provides the minimal set of components needed for the player
/// to participate in DBSP synchronisation and physics.
#[derive(Bundle)]
pub struct PlayerBundle {
    /// Player marker for player-specific queries.
    pub player: Player,
    /// Map-spawned marker for origin tracking.
    pub map_spawned: MapSpawned,
    /// Stable DBSP identifier.
    pub ddlog_id: DdlogId,
    /// World-space transform from the spawn point.
    pub transform: Transform,
    /// Human-readable name for debugging.
    pub name: Name,
    /// Hit points.
    pub health: Health,
    /// Linear velocity (initialised to zero).
    pub velocity: VelocityComp,
}

impl PlayerBundle {
    /// Creates a new player bundle at the given transform.
    ///
    /// The `ddlog_id` is derived from the spawn point entity's bits to ensure
    /// a unique identifier that can be traced back to the originating spawn.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy::prelude::*;
    /// use lille::map::spawn::PlayerBundle;
    ///
    /// // In a real system, spawn_entity would come from a query.
    /// let spawn_entity = Entity::from_bits(42);
    /// let transform = Transform::from_xyz(100.0, 200.0, 0.0);
    /// let bundle = PlayerBundle::new(spawn_entity, transform);
    ///
    /// assert_eq!(bundle.name.as_str(), "Player");
    /// assert_eq!(bundle.health.max, 100);
    /// ```
    #[must_use]
    #[expect(
        clippy::cast_possible_wrap,
        reason = "Entity bits are unlikely to exceed i64::MAX in practice; matches DBSP ID convention."
    )]
    pub fn new(spawn_entity: Entity, transform: Transform) -> Self {
        Self {
            player: Player,
            map_spawned: MapSpawned,
            ddlog_id: DdlogId(spawn_entity.to_bits() as i64),
            transform,
            name: Name::new("Player"),
            health: Health {
                current: 100,
                max: 100,
            },
            velocity: VelocityComp::default(),
        }
    }
}

/// Bundle of components for NPC entities spawned from `SpawnPoint` markers.
///
/// The `UnitType` is determined by the spawn point's `enemy_type` field,
/// allowing Tiled-authored spawn configuration to drive entity archetypes.
#[derive(Bundle)]
pub struct NpcBundle {
    /// Map-spawned marker for origin tracking.
    pub map_spawned: MapSpawned,
    /// Stable DBSP identifier.
    pub ddlog_id: DdlogId,
    /// World-space transform from the spawn point.
    pub transform: Transform,
    /// Human-readable name for debugging.
    pub name: Name,
    /// Hit points (varies by unit type).
    pub health: Health,
    /// Linear velocity (initialised to zero).
    pub velocity: VelocityComp,
    /// Behavioural archetype.
    pub unit_type: UnitType,
}

impl NpcBundle {
    /// Creates a new NPC bundle at the given transform with the specified type.
    ///
    /// The `ddlog_id` uses a global counter to ensure uniqueness across all NPCs.
    /// The counter is offset by `NPC_ID_BASE` to avoid collisions with player IDs
    /// (which are derived from entity bits).
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy::prelude::*;
    /// use lille::map::{SpawnPoint, spawn::NpcBundle};
    ///
    /// let transform = Transform::from_xyz(50.0, 75.0, 0.0);
    /// let spawn_point = SpawnPoint { enemy_type: 1, respawn: false };
    /// let bundle = NpcBundle::new(transform, &spawn_point, 1000);
    ///
    /// assert_eq!(bundle.health.max, 75);
    /// ```
    #[must_use]
    pub fn new(transform: Transform, spawn_point: &SpawnPoint, npc_id: i64) -> Self {
        let (unit_type, health, name) = archetype_from_enemy_type(spawn_point.enemy_type);

        Self {
            map_spawned: MapSpawned,
            ddlog_id: DdlogId(npc_id),
            transform,
            name: Name::new(name),
            health,
            velocity: VelocityComp::default(),
            unit_type,
        }
    }
}

/// Base offset for NPC IDs to avoid collision with player entity bits.
///
/// Player IDs are derived from entity bits (small positive numbers), so NPCs
/// start from a large negative value to ensure no overlap.
const NPC_ID_BASE: i64 = i64::MIN;

/// Maps `enemy_type` to concrete `UnitType` and stats.
///
/// This is a placeholder mapping that handles `enemy_type` values 0–10 with
/// explicit archetypes; all other values fall through to a default. Production
/// code would likely load archetypes from a data file or registry. To add new
/// enemy types, extend the `match` arms below.
#[expect(
    clippy::missing_const_for_fn,
    reason = "Deliberately non-const; future versions may load from data files."
)]
fn archetype_from_enemy_type(enemy_type: u32) -> (UnitType, Health, &'static str) {
    match enemy_type {
        0 => (
            UnitType::Civvy { fraidiness: 0.8 },
            Health {
                current: 50,
                max: 50,
            },
            "Civilian",
        ),
        1..=5 => (
            UnitType::Baddie { meanness: 0.5 },
            Health {
                current: 75,
                max: 75,
            },
            "Grunt",
        ),
        6..=10 => (
            UnitType::Baddie { meanness: 0.8 },
            Health {
                current: 100,
                max: 100,
            },
            "Elite",
        ),
        _ => (
            UnitType::Civvy { fraidiness: 0.5 },
            Health {
                current: 50,
                max: 50,
            },
            "Unknown",
        ),
    }
}

/// Spawns player and NPC entities at their Tiled-authored spawn points.
///
/// This system listens for `TiledEvent<MapCreated>` and processes:
/// - `PlayerSpawn` entities without `PlayerSpawnConsumed` -> spawns player
/// - `SpawnPoint` entities without `SpawnPointConsumed` -> spawns NPCs
///
/// The system is idempotent: entities that have already spawned their actors
/// are marked with `*Consumed` components and skipped on subsequent runs.
///
/// # Entity ID generation
///
/// Spawned entities receive `DdlogId` values derived from their spawn point
/// entity bits. For NPCs, the `NpcIdCounter` resource provides additional
/// uniqueness to handle respawning spawn points in future phases.
///
/// # Player spawn selection
///
/// When multiple `PlayerSpawn` points exist, the spawn with the lowest entity
/// ID is selected to ensure deterministic behaviour across runs.
///
/// # Coordinate source
///
/// Spawn coordinates come from the `Transform` component on spawn point
/// entities, which is hydrated by `bevy_ecs_tiled` from the Tiled object
/// positions.
#[expect(deprecated, reason = "bevy_ecs_tiled 0.10 uses the legacy Event API.")]
#[expect(
    clippy::type_complexity,
    reason = "Bevy ECS query with filter combinators is inherently verbose."
)]
#[expect(
    clippy::too_many_arguments,
    reason = "Bevy systems require query parameters; grouping would obscure intent."
)]
pub fn spawn_actors_at_spawn_points(
    mut commands: Commands,
    mut map_events: EventReader<TiledEvent<MapCreated>>,
    player_spawns: Query<(Entity, &Transform), (With<PlayerSpawn>, Without<PlayerSpawnConsumed>)>,
    npc_spawns: Query<(Entity, &Transform, &SpawnPoint), Without<SpawnPointConsumed>>,
    mut npc_id_counter: ResMut<NpcIdCounter>,
) {
    // Only process when a map has just finished loading.
    if map_events.is_empty() {
        return;
    }

    // Drain all events (we only care that at least one occurred).
    for _ in map_events.read() {}

    spawn_player(&mut commands, &player_spawns);
    spawn_npcs(&mut commands, &npc_spawns, &mut npc_id_counter.0);
}

/// Spawns the player entity at the lowest-ID `PlayerSpawn` point.
///
/// When multiple spawn points exist, the one with the lowest entity ID is
/// selected to ensure deterministic behaviour across runs.
#[expect(
    clippy::type_complexity,
    reason = "Bevy ECS query with filter combinators is inherently verbose."
)]
fn spawn_player(
    commands: &mut Commands,
    player_spawns: &Query<(Entity, &Transform), (With<PlayerSpawn>, Without<PlayerSpawnConsumed>)>,
) {
    // Collect and sort by entity ID for deterministic selection.
    let mut spawns: Vec<_> = player_spawns.iter().collect();
    spawns.sort_by_key(|(entity, _)| *entity);

    if let Some((spawn_entity, transform)) = spawns.first() {
        let player_entity = commands
            .spawn(PlayerBundle::new(*spawn_entity, **transform))
            .id();

        commands.entity(*spawn_entity).insert(PlayerSpawnConsumed);

        log::info!(
            "Spawned player at ({}, {}, {}) from spawn point {:?} -> entity {:?}",
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
            spawn_entity,
            player_entity
        );
    }
}

/// Spawns NPC entities at all unconsumed `SpawnPoint` locations.
fn spawn_npcs(
    commands: &mut Commands,
    npc_spawns: &Query<(Entity, &Transform, &SpawnPoint), Without<SpawnPointConsumed>>,
    npc_id_counter: &mut i64,
) {
    for (spawn_entity, transform, spawn_point) in npc_spawns.iter() {
        let npc_id = NPC_ID_BASE + *npc_id_counter;
        let npc_entity = commands
            .spawn(NpcBundle::new(*transform, spawn_point, npc_id))
            .id();

        // Mark non-respawning spawn points as consumed.
        if !spawn_point.respawn {
            commands.entity(spawn_entity).insert(SpawnPointConsumed);
        }

        log::debug!(
            "Spawned NPC (type={}, respawn={}) at ({}, {}, {}) from {:?} -> {:?}",
            spawn_point.enemy_type,
            spawn_point.respawn,
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
            spawn_entity,
            npc_entity
        );

        *npc_id_counter += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[expect(
        clippy::cast_possible_wrap,
        reason = "Test entity bits are small and fit in i64."
    )]
    fn player_bundle_has_correct_defaults() {
        let entity = Entity::from_bits(123);
        let transform = Transform::from_xyz(10.0, 20.0, 5.0);
        let bundle = PlayerBundle::new(entity, transform);

        assert_eq!(bundle.name.as_str(), "Player");
        assert_eq!(bundle.health.current, 100);
        assert_eq!(bundle.health.max, 100);
        assert_eq!(bundle.ddlog_id.0, entity.to_bits() as i64);
    }

    #[test]
    fn npc_bundle_maps_enemy_type_to_unit_type() {
        let transform = Transform::from_xyz(0.0, 0.0, 0.0);

        // Type 0 = Civvy.
        let civvy_spawn = SpawnPoint {
            enemy_type: 0,
            respawn: false,
        };
        let civvy = NpcBundle::new(transform, &civvy_spawn, 1000);
        assert!(matches!(civvy.unit_type, UnitType::Civvy { .. }));

        // Type 3 = Grunt (Baddie).
        let grunt_spawn = SpawnPoint {
            enemy_type: 3,
            respawn: false,
        };
        let grunt = NpcBundle::new(transform, &grunt_spawn, 1001);
        assert!(matches!(grunt.unit_type, UnitType::Baddie { meanness } if meanness < 0.6));

        // Type 8 = Elite (Baddie with higher meanness).
        let elite_spawn = SpawnPoint {
            enemy_type: 8,
            respawn: true,
        };
        let elite = NpcBundle::new(transform, &elite_spawn, 1002);
        assert!(matches!(elite.unit_type, UnitType::Baddie { meanness } if meanness > 0.7));
    }

    #[test]
    fn archetype_maps_unknown_types_to_civvy() {
        let (unit_type, health, name) = archetype_from_enemy_type(999);
        assert!(matches!(unit_type, UnitType::Civvy { .. }));
        assert_eq!(health.max, 50);
        assert_eq!(name, "Unknown");
    }
}
