# Developer's guide

This guide records the practical facts a contributor needs about Lille's active
dependency stack. It is the source of truth for the current Bevy and
`bevy_ecs_tiled` versions; the
[Bevy migration plan](bevy-0-16-plus-migration-plan.md) is an archived
historical record and must not be used to infer current versions.

## Toolchain

- `rust-toolchain.toml` pins `nightly-2025-09-14` (rustc 1.91.0-nightly) with
  the
  `rustfmt` and `clippy` components.
- The nightly channel is required: `src/lib.rs` uses
  `#![cfg_attr(docsrs, feature(doc_cfg))]`, and `make lint` builds the docs with
  `--cfg docsrs`, which needs the unstable `doc_cfg` feature.
- **Do not bump the toolchain or Bevy to 0.19 without also satisfying the
  constraint below.** Bevy 0.19 requires Rust 1.95.0, which this nightly cannot
  provide, and `bevy_ecs_tiled` has no Bevy-0.19 release compatible with the
  rest of the stack yet.

## Bevy

The workspace targets the **Bevy 0.18.1** release line. Keep the entire Bevy
surface on one minor line: never mix major/minor families across type
signatures, imports, plugins, events, or system parameters.

| Dependency       | Version | Notes                                                                                               |
| ---------------- | ------- | --------------------------------------------------------------------------------------------------- |
| `bevy`           | 0.18.1  | Workspace dependency, `default-features = false`, `reflect_auto_register`; Linux target adds `x11`. |
| `bevy_app`       | 0.18.1  | Direct subcrate.                                                                                    |
| `bevy_ecs`       | 0.18.1  | Direct subcrate.                                                                                    |
| `bevy_math`      | 0.18.1  | Direct subcrate.                                                                                    |
| `bevy_reflect`   | 0.18.1  | Feature `auto_register_inventory`.                                                                  |
| `bevy_transform` | 0.18.1  | Direct subcrate.                                                                                    |
| `bevy_log`       | 0.18.1  | Optional; enabled through the `render` feature.                                                     |

The optional renderer is gated behind the `render` feature (which pulls in the
Bevy asset, core-pipeline, render, sprite, winit, log, and PNG features); the
`text` feature layers `bevy/bevy_text` on top.

### Buffered events use the Message API

Bevy 0.18 split buffered events from observer events:

- **Buffered events** derive `Message` and are read/written with
  `MessageReader<T>` / `MessageWriter<T>`; from a `World`, use
  `World::write_message`. This is what `TiledEvent<MapCreated>` uses.
- **Observer events** derive `Event` and are consumed via `On<T>` observers,
  emitted with `Commands::trigger` / `World::trigger` and registered with
  `App::add_observer`. Lille's `LilleMapError`, `UnloadPrimaryMap`,
  `PrimaryMapUnloaded`, `DbspSyncError`, and `DbspDamageIngress` are observer
  events.

`App` is `#[must_use]` in Bevy 0.18, so do not add a bare `#[must_use]` to
functions that return `App`; `clippy::double_must_use` will reject it.

## Map support: `bevy_ecs_tiled`

- Version **0.12.0** (optional, behind the `map` feature),
  `default-features = false`.
- Features: `png` and `user_properties` are always enabled with the crate;
  `render` is added by the `render` feature and `atlas` by `test-support`.
- 0.12 is the `bevy_ecs_tiled` line that tracks Bevy 0.18 (upstream
  compatibility
  table: 0.11–0.12 target Bevy 0.18). The Bevy-0.19 line is 0.13, which cannot
  be adopted until the toolchain constraint above is lifted.

## Commit gates

Run the deterministic gates before committing (see `AGENTS.md` and the
`Makefile`): `make check-fmt`, `make test`, `make typecheck`, and `make lint`.
`make lint` runs rustdoc (`--cfg docsrs`),
`cargo clippy --all-targets --all-features -- -D warnings`, and the Whitaker
Dylint suite.
