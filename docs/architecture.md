# Lille Architecture Documentation

## Overview

Lille is a real-time strategy game built in Rust. The original prototype used a
custom tick-based loop with the Piston engine. The project is now transitioning
toward a Bevy and DDlog driven design. Phase 0 established the new scaffolding
with a minimal Bevy `App` and a placeholder DDlog handle.

## Core Components

### GameWorld

The central component that manages the game state and coordinates all entities.
It maintains:

- Lists of entities, actors, and bad guys
- Tick-based update system (500ms intervals)
- Position tracking for all game objects
- Threat detection and management

### Entity System

The game uses a component-based entity system with several key types:

- `Entity`: Base component with position tracking
- `Actor`: Autonomous agents that navigate towards targets while avoiding
  threats
- `BadGuy`: Threatening entities that influence actor behavior through fear
  mechanics

### Movement and Behavior System

#### Actor Behavior

Actors implement sophisticated movement behavior that balances:

- Goal-seeking behavior towards target positions
- Threat avoidance using fear vectors
- Dynamic weighting between target pursuit and threat avoidance
- Perpendicular movement for natural-looking threat avoidance

The movement calculation considers:

- Fear radius based on threat meanness and actor "fraidiness"
- Distance-based fear scaling
- Combined vector influence from both target direction and threat avoidance

### Graphics and Rendering

- The legacy prototype uses the Piston game engine for window management and
  rendering.
- Phase 0 introduced Bevy as the new runtime. The current binary starts a Bevy
  window and prints a greeting.
- The `build.rs` script downloads the Fira Sans font if needed and compiles
  `src/lille.dl` with the `ddlog` compiler. The generated crate is written to
  Cargo's `OUT_DIR` to keep the project root clean.
- A placeholder `DdlogHandle` resource is inserted during startup.
- `DefaultPlugins` are loaded with `LogPlugin` disabled, so the custom logger
  from `logging.rs` controls output.
- The grid-based visualization system from the original code remains, but will
  be ported to Bevy in later phases.
- The Piston version renders threats in a more intense shade of red.

## Technical Architecture

### Core Dependencies

- `piston_window` (0.131): Window creation and event loop (legacy)
- `piston2d-graphics` (0.45): 2D graphics rendering (legacy)
- `hashbrown` (0.14): High-performance HashMap implementation
- `glam` (0.24): Vector mathematics and linear algebra
- `clap` (4.4): Command-line argument parsing
- `bevy` (0.12): ECS and rendering framework introduced in Phase 0
- `differential-datalog` (0.53): Runtime library for the DDlog rules (generated
  as `ddlog_lille`)

### Update Cycle

1. The game runs on a fixed tick rate of 500ms
2. Each tick:
   - Collects all active threats and their positions
   - Updates actor positions based on:
     - Current position
     - Target position
     - Threat positions and fear influences
   - Updates the visual representation

### Design Decisions

#### Performance Considerations

- Use of `hashbrown` for high-performance spatial tracking
- Fixed tick rate for predictable performance
- Efficient vector calculations using `glam`

#### Extensibility

- Trait-based system for threats (`CausesFear` trait)
- Component-based entity system for easy addition of new entity types
- Modular separation of graphics from game logic

#### Debug Support

- Integrated logging system with verbose mode
- Command-line argument parsing for configuration
- Visual debugging through density-based rendering

## Future Considerations

The architecture supports several potential extensions:

- Additional entity types through the component system
- New behavior patterns through trait implementation
- Enhanced graphics and visual effects
- Multiplayer support through the tick-based system
