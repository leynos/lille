# Lille Physics Engine: A Declarative Design

This document outlines a multi-phase proposal for implementing a deterministic,
3D physics engine for the `lille` RTS, driven by Differential Datalog (DDlog).
The architecture models physical laws as a series of declarative rules,
separating the complex logic of motion from the Bevy-based host application.

## Phase 1: Gravity and Floor-Height-Based Physics

### 1. Introduction

This initial phase establishes a kinematic system concerned with position and
gravity. The core principle is to model the world as a grid of isometric blocks
and to define an entity's physical state as either **Unsupported (Falling)** or
**Supported (Standing)**. Slopes on blocks are used to define a continuous floor
height, not to induce sliding, allowing for the creation of smooth ramps and
varied terrain. The DDlog engine's primary role in this phase is to calculate
the precise floor height (`z_floor`) at any given `(x, y)` coordinate and
determine an entity's state relative to it.

### 2. DDlog Model Changes

#### 2.1 Core Type and Relation Modifications

To handle continuous height, we adjust our core types to use floating-point
numbers.

```prolog
// --- Core Type Modifications ---
typedef GCoord = float

// --- Relation Modifications ---
input relation Position(entity: EntityID, x: GCoord, y: GCoord, z: GCoord)
input relation Block(id: BlockID, x: signed32, y: signed32, z: signed32)
input relation Target(actor: EntityID, tx: GCoord, ty: GCoord)
output relation NewPosition(entity: EntityID, x: GCoord, y: GCoord, z: GCoord)

```

#### 2.2 Redesigned World Geometry Relations

The `BlockSlope` relation is redefined to describe a plane equation, enabling
precise height calculations.

```prolog
// Defines the top plane of a block with the equation:
// z = base_z + (x_in_block * grad_x) + (y_in_block * grad_y)
input relation BlockSlope(
    block: BlockID,
    grad_x: GCoord, // The gradient (steepness) in the x direction.
    grad_y: GCoord  // The gradient (steepness) in the y direction.
)

```

### 3. Declarative Physics Rules (Refined)

#### Step 1: Calculate Floor Height at Any Position

This is the new core of the physics model. We create relations that derive the
correct floor height for any given `(x, y)` coordinate.

```prolog
// --- Physics Constants --- (see `constants.toml`)

// Finds the highest block at a given (x,y) grid location.
relation HighestBlockAt(x_grid: signed32, y_grid: signed32, block: BlockID, z_grid: signed32) :-
    Block(block, x_grid, y_grid, z_grid),
    not Block(_, x_grid, y_grid, z_grid + 1).

// Calculates the floor Z coordinate at a given continuous (x,y) position.
relation FloorHeightAt(x: GCoord, y: GCoord, z_floor: GCoord) :-
    var x_grid = floor(x),
    var y_grid = floor(y),
    HighestBlockAt(x_grid, y_grid, block, z_grid),
    // Case 1: The block has a slope. Calculate height using the plane equation.
    BlockSlope(block, grad_x, grad_y),
    var x_in_block = x - x_grid,
    var y_in_block = y - y_grid,
    z_floor = (z_grid as GCoord) + 1.0 + (x_in_block * grad_x) + (y_in_block * grad_y).

relation FloorHeightAt(x: GCoord, y: GCoord, z_floor: GCoord) :-
    var x_grid = floor(x),
    var y_grid = floor(y),
    HighestBlockAt(x_grid, y_grid, block, z_grid),
    // Case 2: The block is flat. The floor height is simply the top of the block.
    not BlockSlope(block, _, _),
    z_floor = (z_grid as GCoord) + 1.0.

```

#### Step 2: Redefine Entity State (Unsupported vs. Standing)

The logic is now simpler. An entity is unsupported if it is "floating" above the
calculated floor.

```prolog
relation IsUnsupported(entity: EntityID) :-
    Position(entity, x, y, z),
    FloorHeightAt(x, y, z_floor),
    z > z_floor + GRACE_DISTANCE.

relation IsStanding(entity: EntityID) :-
    Position(entity, _, _, _),
    not IsUnsupported(entity).

```

#### Step 3: Final Movement Calculation

Movement is now a two-stage process: calculate the 2D AI move, then "snap" the
result to the floor.

```prolog
// GRAVITY_PULL defined in `constants.toml`

// --- Falling Logic ---
relation GravityEffectVector(entity, 0.0, 0.0, GRAVITY_PULL) :-
    IsUnsupported(entity).

// -- Standing Logic --
// The AI vector is calculated for any standing entity. This determines the NEW (x,y).
relation AiMoveVector(actor: EntityID, dx: GCoord, dy: GCoord) :-
    IsStanding(actor),
    // (Your existing FleeVector/TargetVector logic goes here, adapted for GCoord)
    TargetVector(actor, dx, dy).

// Calculate the final position.
// Case 1: The entity is falling. Apply the gravity vector.
output relation NewPosition(e, x, y, z + dz) :-
    IsUnsupported(e),
    Position(e, x, y, z),
    GravityEffectVector(e, _, _, dz).

// Case 2: The entity is standing. Calculate its new (x,y) based on AI,
// then find the new floor height (z) at that new position.
output relation NewPosition(e, x + dx, y + dy, new_z_floor) :-
    IsStanding(e),
    Position(e, x, y, _),
    AiMoveVector(e, dx, dy),
    FloorHeightAt(x + dx, y + dy, new_z_floor).

```

## Phase 2: Force, Inertia, and Friction

### 1. Introduction to Dynamics

This phase builds upon the kinematic model by introducing dynamics. We add the
concepts of **force**, **velocity**, **inertia**, and **friction** to simulate
how entities react to kinetic impulses and environmental drag. An entity's final
movement becomes the sum of its self-powered **propulsive motion** (AI-driven
walking) and its **inertial motion** (velocity from external forces).

### 2. DDlog Model Expansion

#### 2.1 New and Modified Core Relations

We introduce relations to track velocity and represent transient forces.

```prolog
// --- New Physics Constants --- (see `constants.toml`)

// --- New Persistent Input Relation ---
// Tracks velocity at the start of a tick, fed back from the previous tick's output.
input relation Velocity(entity: EntityID, vx: GCoord, vy: GCoord, vz: GCoord)

// --- New Mass Relation ---
// Provides each entity's mass so forces can be converted into acceleration.
// Mass values should be positive; non-positive entries are ignored.
input relation Mass(entity: EntityID, kg: GCoord)

// --- New Per-Tick Input Relation ---
// Represents instantaneous forces applied to entities for a single tick.
// The host engine overwrites this relation every frame. These are force inputs
// (not direct accelerations); acceleration is computed as force divided by mass.
input relation Force(entity: EntityID, fx: GCoord, fy: GCoord, fz: GCoord)

// --- New Output Relation ---
// The calculated velocity at the end of a tick.
output relation NewVelocity(entity: EntityID, nvx: GCoord, nvy: GCoord, nvz: GCoord)

```

#### 2.2 New External Helper Functions

Complex vector maths is offloaded to Rust functions exposed to DDlog.

```prolog
extern function vec_mag(x: GCoord, y: GCoord, z: GCoord): GCoord
extern function vec_normalize(x: GCoord, y: GCoord, z: GCoord): (GCoord, GCoord, GCoord)

```

### 3. Declarative Dynamics Rules

#### Step 1: Sum All Applied Accelerations

We collect all acceleration vectors acting on an entity for the current tick.

```prolog
relation AppliedAcceleration(e, fx / mass, fy / mass, fz / mass) :-
    Force(e, fx, fy, fz),
    (Mass(e, mass) or mass = DEFAULT_MASS),
    mass > 0.0.
relation GravitationalAcceleration(e, 0.0, 0.0, -GRAVITY_PULL) :- IsUnsupported(e).

```

#### Step 2: Calculate Frictional Deceleration

Friction is an opposing acceleration dependent on the entity's state and
velocity.

```prolog
relation FrictionalDeceleration(e, fdx, fdy, 0.0) :-
    IsStanding(e),
    Velocity(e, vx, vy, _),
    var h_mag = vec_mag(vx, vy, 0.0), h_mag > 0.0,
    var nvec = vec_normalize(vx, vy, 0.0),
    var nx = nvec.0,
    var ny = nvec.1,
    var decel_mag = min(h_mag, GROUND_FRICTION),
    fdx = -nx * decel_mag, fdy = -ny * decel_mag.

relation FrictionalDeceleration(e, fdx, fdy, 0.0) :-
    IsUnsupported(e),
    Velocity(e, vx, vy, _),
    var h_mag = vec_mag(vx, vy, 0.0), h_mag > 0.0,
    var nvec2 = vec_normalize(vx, vy, 0.0),
    var nx = nvec2.0,
    var ny = nvec2.1,
    var decel_mag = min(h_mag, AIR_FRICTION),
    fdx = -nx * decel_mag, fdy = -ny * decel_mag.

```

#### Step 3: Calculate Net Acceleration and New Velocity

We sum all accelerations to get a net acceleration, then use it to update the
entity's velocity.

```prolog
relation NetAccelRow(e, ax, ay, az) :- AppliedAcceleration(e, ax, ay, az).
relation NetAccelRow(e, ax, ay, az) :- GravitationalAcceleration(e, ax, ay, az).
relation NetAccelRow(e, ax, ay, az) :- FrictionalDeceleration(e, ax, ay, az).

relation SumAx(e, ax) :- ax = sum ax_i : { NetAccelRow(e, ax_i, _, _) }.
relation SumAy(e, ay) :- ay = sum ay_i : { NetAccelRow(e, _, ay_i, _) }.
relation SumAz(e, az) :- az = sum az_i : { NetAccelRow(e, _, _, az_i) }.

relation NetAcceleration(e, ax, ay, az) :-
    SumAx(e, ax),
    SumAy(e, ay),
    SumAz(e, az).

relation UnclampedNewVelocity(e, vx + ax, vy + ay, vz + az) :-
    Velocity(e, vx, vy, vz),
    NetAcceleration(e, ax, ay, az).

// Apply state-based constraints to produce the final NewVelocity.
output relation NewVelocity(e, nvx, nvy, final_nvz) :-
    IsUnsupported(e),
    UnclampedNewVelocity(e, nvx, nvy, raw_nvz),
    var final_nvz = max(raw_nvz, -TERMINAL_VELOCITY).

output relation NewVelocity(e, nvx, nvy, 0.0) :-
    IsStanding(e),
    UnclampedNewVelocity(e, nvx, nvy, _).

```

#### Step 4: Final Position Calculation

An entity's final displacement is the sum of its new inertial velocity and its
separate, AI-driven walking vector.

```prolog
// The AI-driven walking vector, which only applies to standing entities.
relation AiWalkVector(actor, dx, dy, 0.0) :-
    IsStanding(actor),
    TargetVector(actor, dx, dy).

// Case 1: An unsupported entity's position is updated only by its inertial velocity.
output relation NewPosition(e, px + nvx, py + nvy, pz + nvz) :-
    IsUnsupported(e),
    Position(e, px, py, pz),
    NewVelocity(e, nvx, nvy, nvz).

// Case 2: A standing entity's position is updated by both its inertial velocity
// AND its self-powered walking vector. Its final height is then snapped to the floor.
output relation NewPosition(e, new_x, new_y, new_z_floor) :-
    IsStanding(e),
    Position(e, px, py, _),
    NewVelocity(e, nvx, nvy, _),
    (AiWalkVector(e, walk_dx, walk_dy, _) or (walk_dx=0.0, walk_dy=0.0)),
    var new_x = px + nvx + walk_dx,
    var new_y = py + nvy + walk_dy,
    FloorHeightAt(new_x, new_y, new_z_floor).

```
