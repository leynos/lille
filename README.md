# Lille

A simple real-time strategy prototype demonstrating a dataflow-driven game loop
with optional Bevy subsystems. Rendering is enabled with the `render` feature,
while text rendering builds on top via the `text` feature. The project
currently implements "Phase 1" of the migration roadmap, synchronising the
legacy `GameWorld` state into Bevy and rendering static entities.

## Features & usage

- Rendering: enable with `render`.
- Text rendering: enable with `text` (implies `render`).

Run the game:

```bash
cargo run -p lille --features render
```

Enable text:

```bash
cargo run -p lille --features text
```

Run tests that depend on rendering:

```bash
cargo test -p lille --features render -- tests/spawn.rs
```

Sprites that load PNG assets require Bevy's `png` feature; the `render` feature
in this crate enables it (see Cargo.toml).

## Game Setting

*A fractured city of steel, brick, and neon — and your next battlefield.*

A city with too many masters and too little future. Once the jewel of a
vanished federation, its streets are now ruled by whoever can seize them —
corporate militias, rogue syndicates, data cults, and the last vestiges of an
irrelevant state.

The skyline tells the story: rusted iron arcades tower over flooded canals and
cracked boulevards. Century-old stone buildings are patched with ferroglass and
recycled scaffolds. Above them loom crumbling superstructures from a failed age
of expansion, their data relays and skyways flickering with black market
signals.

Beneath this decaying grandeur, commerce thrives in the margins. Street-level
markets sprawl through abandoned plazas; encrypted auction houses operate out
of gutted tram stations. Every corner offers a new opportunity — or an ambush.

Here, real power belongs to those who can move fastest, strike hardest, and
control the flow of information. Squads operate in the open, armed, and
augmented. Every mission risks crossing the unseen lines between factions — and
starting a war you cannot finish.

## Prepare to Deploy

Control is an illusion. Ownership is temporary. The streets belong to those who
can take them — and keep them.

## Build script

The `build.rs` entry point delegates to the `build_support` crate. This helper
crate downloads the project font during compilation.

The font download uses the operating system's certificate store for TLS
verification. Ensure your environment has a valid set of root certificates so
the HTTPS request succeeds.

## Isolated build support

Run the build support logic without compiling the whole game using the helper
script:

```bash
./scripts/build_support_runner.sh
```

The script compiles the helper binary and executes it directly so
`CARGO_MANIFEST_DIR` points to the repository root while running.

This performs the same steps as `build.rs`, downloading the font asset.
