# Design Document: DDlog-Driven Core Game Loop for Lille RTS

## 1. Introduction

This document outlines a re-architecture of the `leynos/lille` real-time
strategy game. The current implementation uses a traditional, imperative game
loop within the `GameWorld` struct. We propose replacing this with a
declarative, data-driven core powered by **Differential Datalog (DDlog)**.

The new architecture will model all significant game state and behaviour—unit
movement, AI decisions, and combat resolution—as a set of DDlog rules. The
game's host application, written in Rust, will interface with the DDlog runtime.
Each game tick, it will stream world-state changes and player commands into
DDlog and receive incrementally computed updates back. These updates will then
be applied to a lightweight Entity-Component-System (ECS) framework, for which
we will use **Bevy ECS** as the reference.

This approach offers several distinct advantages:

- **Determinism:** The entire game logic is captured in a pure, declarative
  dataflow. Given the same sequence of inputs, the game state will consistently
  evolve identically, which is a prerequisite for robust replay functionality
  and lock-step networking models.

- **Separation of Concerns:** The complex, stateful game logic is completely
  decoupled from the rendering and host application code. The host becomes a
  simple coordinator, translating between the ECS and DDlog's relational model.

- **Extensibility:** New game features, such as economic models, technology
  trees, or more sophisticated unit behaviours, can be added by introducing new
  DDlog relations and rules, often without modifying the existing Rust host
  code.

The flow of data each frame will be as follows:

```plaintext
┌─────────────┐  1. State Sync    ┌──────────────┐  3. Inferred Deltas
│  Bevy ECS   │ ───────────────►  │  DDlog Core  │ ────────────────► Apply Changes
└─────────────┘  (Position, etc.) │   (Workers)  │
       ▲                          └──────────────┘
       │ 2. Inject Commands
       └────── (Move, Attack)

```

This document will detail the modelling of game state in DDlog, the integration
strategy with the Bevy ECS, and the practicalities of implementation, including
debugging and performance considerations.

## 2. Modelling Game State and Rules in DDlog

The foundation of the new architecture is the translation of `lille`'s current
object-oriented concepts (`Actor`, `BadGuy`, `GameWorld`) into a relational
model within DDlog.

### 2.1. Core Types and Relations

We will define DDlog types to represent the fundamental data of the game.

```prolog
// --- Core Types ---
typedef EntityID = signed64
typedef Coord    = signed32
typedef Health   = signed32
typedef UnitType = string // e.g., "Civvy", "Baddie"

// --- Player/AI Commands ---
typedef Command = Move { dx: Coord, dy: Coord }
                | Attack { target: EntityID }
                | Stop

// --- Constants ---
const ATTACK_RANGE: float = 10.0; // Based on lille’s fear mechanics

```

With these types, we define the relations that constitute the game's state. We
distinguish between three kinds of relations:

- `input relation`: Persistent state that the host application manages.

- `input stream`: Ephemeral data, like commands, that exist only for a single
  transaction (tick).

- `output relation`: Results computed by DDlog that the host application will
  read and apply.

```prolog
// --- Input Relations (Persistent State from ECS) ---
input relation Position(entity: EntityID, x: Coord, y: Coord)
input relation Health(entity: EntityID, hp: Health)
input relation Unit(entity: EntityID, type: UnitType)
input relation Target(actor: EntityID, tx: Coord, ty: Coord) // Actor’s target destination
input relation Fraidiness(actor: EntityID, factor: float)    // Corresponds to Actor.fraidiness
input relation Meanness(baddie: EntityID, factor: float)   // Corresponds to BadGuy.meanness

// --- Input Stream (Per-tick Commands) ---
input stream Command(entity: EntityID, cmd: Command)

// --- Output Relations (Computed Results for ECS) ---
output relation NewPosition(entity: EntityID, x: Coord, y: Coord)
output relation Damage(target: EntityID, amount: Health)
output relation Despawn(entity: EntityID)

```

### 2.2. Declarative Game Logic Rules

The imperative logic currently in `actor.rs` and `world.rs` will be rewritten as
declarative DDlog rules.

#### Fear and Threat Modelling

The complex fear calculation in `Actor::calculate_fear_vector` can be expressed
as a series of relational joins and aggregations.

```prolog
// extern function provided by the Rust host for vector math if needed
extern function sqrt(f: float): float
extern function sign(c: Coord): Coord

// Helper to calculate squared distance between two entities
relation Dist2(e1: EntityID, e2: EntityID, d2: float) :-
    Position(e1, x1, y1),
    Position(e2, x2, y2),
    var dx = float(x1 - x2),
    var dy = float(y1 - y2),
    d2 = dx*dx + dy*dy.

// The "fear" contribution of a single baddie on an actor
relation FearContribution(actor: EntityID, baddie: EntityID, fear: float) :-
    Unit(actor, "Civvy"), Unit(baddie, "Baddie"),
    Dist2(actor, baddie, d2),
    Fraidiness(actor, fraidiness), Meanness(baddie, meanness),
    var fear_radius = fraidiness * meanness * 2.0,
    d2 < fear_radius*fear_radius,
    fear = (1.0 / (d2 + 0.001)). // Add epsilon to avoid division by zero

// Aggregate fear from all nearby baddies for each actor
relation TotalFear(actor: EntityID, total_fear: float) :-
    total_fear = sum f : { FearContribution(actor, _, f) }.

// Determine the direction to flee from the nearest baddie
relation FleeVector(actor: EntityID, dx: Coord, dy: Coord) :-
    TotalFear(actor, _), // Only for actors that feel fear
    // Find the nearest baddie
    Dist2(actor, baddie, d2),
    min(d2) = min_d2,
    Dist2(actor, baddie, min_d2),
    // Flee directly away from them
    Position(actor, ax, ay), Position(baddie, bx, by),
    dx = sign(ax - bx),
    dy = sign(ay - by).

```

#### Movement Logic

The decision-making process for movement combines seeking a target with avoiding
threats. This maps cleanly to conditional rules.

```prolog
// Vector towards the actor’s primary target
relation TargetVector(actor: EntityID, dx: Coord, dy: Coord) :-
    Unit(actor, "Civvy"),
    Target(actor, tx, ty),
    Position(actor, ax, ay),
    dx = sign(tx - ax),
    dy = sign(ty - ay).

// FINAL MOVEMENT DECISION
// Rule 1: If scared, the flee vector dominates.
relation MoveVector(actor: EntityID, dx, dy) :-
    TotalFear(actor, f), f > 0.2, // Fear threshold
    FleeVector(actor, dx, dy).

// Rule 2: If not scared, move towards the target.
relation MoveVector(actor: EntityID, dx, dy) :-
    not TotalFear(actor, _),
    TargetVector(actor, dx, dy).

// Calculate the final new position based on the chosen vector
// Note: Speed is applied in the Rust host for simplicity, or could be a relation.
output relation NewPosition(e, nx, ny) :-
    MoveVector(e, dx, dy),
    Position(e, x, y),
    nx = x + dx, // Speed multiplier applied in host
    ny = y + dy.

```

#### Combat and Health

Combat is simplified to a rule that generates `Damage` facts when an `Attack`
command is issued and the target is in range.

```prolog
relation Hit(attacker: EntityID, target: EntityID, damage: Health) :-
    Command(attacker, Attack{target}),
    Dist2(attacker, target, d2),
    d2 < ATTACK_RANGE*ATTACK_RANGE,
    damage = 10. // Base damage

// Accumulate all damage for a given target in a single tick
output relation Damage(target, total_dmg) :-
    total_dmg = sum d : { Hit(_, target, d) }.

// A unit is despawned if its health goes to or below zero
output relation Despawn(e) :-
    Health(e, hp),
    Damage(e, dmg),
    hp - dmg <= 0.

```

## 3. ECS Integration Strategy

We will replace the `piston_window` backend with `bevy`. DDlog will act as a
"logic system" within Bevy's schedule. The `GameWorld` struct will be
dismantled, its responsibilities distributed among ECS components and DDlog
relations.

### 3.1. Bevy ECS Components

The ECS components will be minimal, primarily holding data that needs to be
synchronised with DDlog or is required for rendering.

```rust
// In src/main.rs or a new components.rs
use bevy::prelude::*;

#[derive(Component)]
struct DdlogId(i64); // The entity’s unique ID in DDlog

#[derive(Component)]
struct Health(i32);

#[derive(Component)]
struct Target(Vec2); // Equivalent to Actor.target

#[derive(Component)]
enum UnitType {
    Civvy { fraidiness: f32 },
    Baddie { meanness: f32 },
}

```

### 3.2. The DDlog-Bevy Sync Systems

Two Bevy systems will manage the interaction with DDlog.

1. `push_state_to_ddlog`: This system runs first. It queries the ECS for all
   relevant components and constructs a `DeltaMap` of changes to send to DDlog.

2. `apply_ddlog_deltas`: This system runs after `push_state_to_ddlog`. It
   commits the transaction, retrieves the resulting changes from DDlog's output
   relations, and applies them back to the ECS components (e.g., updating
   `Transform`, `Health`).

```rust
// In a new ddlog_integration.rs module

use bevy::prelude::*;
use std::sync::{Arc, Mutex};
use lille_ddlog::{api::HDDlog, run, DDlogRecord};
// Assuming `lille_ddlog` is the generated crate name

// A Bevy resource to hold the DDlog instance
#[derive(Resource)]
struct DdlogHandle(Arc<Mutex<HDDlog>>);

// Startup system to initialize DDlog
fn init_ddlog_system(mut commands: Commands) {
    let (hddlog, _err_handler) = run(2, false)
        .expect("Failed to start DDlog");
    // 2 worker threads
    commands.insert_resource(DdlogHandle(Arc::new(Mutex::new(hddlog))));
}

// System to push ECS state into DDlog input relations
fn push_state_to_ddlog_system(
    ddlog_handle: Res<DdlogHandle>,
    // Query for all entities that should be in the logic simulation
    query: Query<(Entity, &Transform, &Health, &UnitType, Option<&Target>)>,
    // EventReader for player commands
    mut player_commands: EventReader<PlayerCommand>,
) {
    let mut ddlog = ddlog_handle.0.lock().unwrap();
    ddlog.transaction_start().unwrap();

    let mut changes = Vec::new();

    // 1. Sync ECS state to DDlog relations
    for (entity, transform, health, unit_type, target) in query.iter() {
        let id = entity.to_bits() as i64;
        changes.push(DDlogRecord::insert(
            "Position",
            (id, transform.translation.x as i32, transform.translation.y as i32),
        ));
        changes.push(DDlogRecord::insert("Health", (id, health.0)));

        match unit_type {
            UnitType::Civvy { fraidiness } => {
                changes.push(DDlogRecord::insert("Unit", (id, "Civvy".to_string())));
                changes.push(
                    DDlogRecord::insert("Fraidiness", (id, *fraidiness as f64)),
                );
                if let Some(t) = target {
                    changes.push(
                        DDlogRecord::insert(
                            "Target",
                            (id, t.0.x as i32, t.0.y as i32),
                        ),
                    );
                }
            }
            UnitType::Baddie { meanness } => {
                changes.push(DDlogRecord::insert("Unit", (id, "Baddie".to_string())));
                changes.push(DDlogRecord::insert("Meanness", (id, *meanness as f64)));
            }
        }
    }
    
    // 2. Inject player commands into the Command stream
    for command in player_commands.iter() {
        // ... logic to convert PlayerCommand events to DDlogRecord ...
        // e.g., changes.push(DDlogRecord::insert("Command", ...));
    }

    ddlog.apply_updates_dynamic(&mut changes.into_iter()).unwrap();
}

// System to apply DDlog's computed changes back to the ECS
fn apply_ddlog_deltas_system(
    mut commands: Commands,
    ddlog_handle: Res<DdlogHandle>,
    mut query: Query<(&mut Transform, &mut Health)>,
) {
    let mut ddlog = ddlog_handle.0.lock().unwrap();
    let changes = ddlog.transaction_commit_dump_changes_dynamic().unwrap();

    // Apply NewPosition changes
    for rec in changes.get_records("NewPosition") {
        if let DDValue::Tuple3(id, x, y) = rec.val {
            let entity = Entity::from_bits(id as u64);
            if let Ok((mut transform, _)) = query.get_mut(entity) {
                // Apply speed multiplier here
                let speed = 5.0; // Or get from a component
                let direction = Vec3::new(*x as f32, *y as f32, 0.0) - transform.translation;
                transform.translation += direction.normalize_or_zero() * speed;
            }
        }
    }
    
    // Apply Damage changes
    for rec in changes.get_records("Damage") {
        if let DDValue::Tuple2(id, dmg) = rec.val {
            let entity = Entity::from_bits(id as u64);
            if let Ok((_, mut health)) = query.get_mut(entity) {
                health.0 -= *dmg as i32;
            }
        }
    }
    
    // Apply Despawn changes
    for rec in changes.get_records("Despawn") {
        if let DDValue::Tuple1(id) = rec.val {
            let entity = Entity::from_bits(id as u64);
            commands.entity(entity).despawn();
        }
    }
}

```

### 3.3. Bevy App Configuration

The main application setup will chain these systems together in the correct
order.

```rust
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_event::<PlayerCommand>()
        .add_startup_system(init_ddlog_system)
        .add_systems(
            (
                push_state_to_ddlog_system,
                apply_ddlog_deltas_system,
            )
            .chain() // Ensures they run in order
        )
        // ... other systems for rendering, input handling, etc. ...
        .run();
}

```

## 4. Debugging and Introspection

A key advantage of DDlog is its debuggability.

- **Command Logging:** We can enable command recording to produce a
  human-readable log of every transaction.

  ```rust
  // In init_ddlog_system
  ddlog.record_commands(Some(Path::new("game_replay.dat"))).unwrap();


  ```

  This `game_replay.dat` file can be replayed through the DDlog command-line
  tool to inspect state at any tick, without the Bevy host.

- **Dumping Relations:** By temporarily marking intermediate relations like
  `TotalFear` or `FleeVector` as `output`, we can `dump` their contents during a
  replay to see exactly what the logic engine is thinking. This is invaluable
  for debugging complex AI behaviours.

## 5. Performance and Limitations

- **Scalability:** DDlog's incremental nature means that performance depends on
  the size of the *delta* (changes per tick), not the total state size. For an
  RTS with hundreds of units, per-tick updates should be well within a
  millisecond budget on modern hardware, especially with multiple worker
  threads.

- **Complex Algorithms:** Algorithms that are inherently global or require
  complex graph traversal, like A\* pathfinding, are not a natural fit for
  DDlog's relational model. The best practice is to offload these to the Rust
  host. The host can run A\*, for example, and feed the resulting path waypoints
  into the `Target` relation for the DDlog movement logic to follow.

- **Memory Usage:** DDlog holds all persistent relations in memory. While
  efficient, for games with extremely large maps or unit counts, memory usage
  should be monitored. Using `stream` relations for ephemeral data is critical
  to prevent unbounded memory growth.

## 6. Conclusion

Migrating `lille` to a DDlog-driven architecture represents a significant shift
from an imperative to a declarative model. This change promises to deliver a
more robust, deterministic, and extensible foundation for future development.
The core logic of the game becomes a verifiable set of rules, clearly separated
from the concerns of rendering and real-time host execution. By leveraging Bevy
for the ECS and rendering backend, we can focus development effort on the game's
unique logic within DDlog, confident that the underlying engine is sound.
