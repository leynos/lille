# Developer's guide

This guide records the practical facts a contributor needs about Lille's active
dependency stack. It is the source of truth for the current Bevy and
`bevy_ecs_tiled` versions; the
[Bevy migration plan](bevy-0-16-plus-migration-plan.md) is an archived
historical record and must not be used to infer current versions.

## Toolchain

- `rust-toolchain.toml` pins `nightly-2025-09-14` (rustc 1.91.0-nightly) with
  the `rustfmt` and `clippy` components.
- The nightly channel is required: `src/lib.rs` uses
  `#![cfg_attr(docsrs, feature(doc_cfg))]`, and `make lint` builds the docs with
  `--cfg docsrs`, which needs the unstable `doc_cfg` feature.
- **Do not bump the toolchain or Bevy to 0.19 without also satisfying the
  constraint below.** Bevy 0.19 requires Rust 1.95.0, which this nightly cannot
  provide. `bevy_ecs_tiled` 0.13 already tracks Bevy 0.19, so the plugin is not
  the blocker; the toolchain is.

## Bevy

The workspace targets the **Bevy 0.18.1** release line. Keep the entire Bevy
surface on one minor line: never mix major/minor families across type
signatures, imports, plugins, events, or system parameters.

*Active Bevy dependency versions (workspace and direct subcrates):*

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

The migrated buffered-message surface is guarded two ways. The runtime map
integration tests exercise it dynamically, and a `trybuild` compile-pass
harness pins it statically: `tests/compile_pass.rs` compiles the fixture
`tests/compile_pass/message_reader_migration.rs`, which uses
`MessageReader<TiledEvent<MapCreated>>` and `World::write_message`.
Reintroducing the legacy `EventReader` / `World::send_event` names breaks the
fixture. Run it with:

```sh
cargo test --features test-support --test compile_pass
```

The harness is gated on `test-support` (like the other map tests), so it also
runs as part of `make test` (which passes `--features test-support`) and the CI
coverage step. The fixture is a standalone crate, so `bevy_ecs_tiled` is
carried as a non-optional dev-dependency purely to make it nameable there.

## Map support: `bevy_ecs_tiled`

- Version **0.12.0** (optional, behind the `map` feature),
  `default-features = false`.
- Features: `png` and `user_properties` are always enabled with the crate;
  `render` is added by the `render` feature and `atlas` by `test-support`.
- 0.12 is the `bevy_ecs_tiled` line that tracks Bevy 0.18 (upstream
  compatibility table: 0.11–0.12 target Bevy 0.18). The Bevy-0.19 line is 0.13,
  which already supports Bevy 0.19; adopting it is blocked solely by the Rust
  1.95.0 toolchain constraint above, not by plugin availability.

## `ordered-float` v5 and the vendored `feldera-size-of` fork

Lille's DBSP records store floating-point values through
`ordered_float::OrderedFloat<f64>` so that they have a total order, which DBSP
requires for keys, joins, and aggregations. Those records also derive
`feldera_size_of::SizeOf` for memory accounting. Reconciling the two across a
major `ordered-float` upgrade is the reason for the arrangement described here.

For the full decision record, see
[ADR 002](adr-002-ordered-float-v5-vendored-feldera-size-of-fork.md). The fork
itself is documented in
[`third_party/README.md`](../third_party/README.md). Fork lifecycle and removal
are tracked in
[issue #294](https://github.com/leynos/lille/issues/294).

### Why `ordered-float` is pinned at v5

The workspace standardizes on `ordered-float` 5.x
(`ordered-float = { version = "5", features = ["serde", "rkyv_64"] }` in the
root `Cargo.toml`). This is the current major version, and Lille's own records
are built against it. The `rkyv_64` feature still targets rkyv 0.7, matching
Lille's `rkyv = "0.7"`, so the upgrade needs no rkyv changes.

Note that `dbsp` 0.98 independently requires `ordered-float ^4.2.0`, so
ordered-float 4.x and 5.x coexist in the dependency graph: dbsp resolves to v4
for its own internals, while Lille's records use v5. This is expected and
supported.

### Why `feldera-size-of` is patched through `[patch.crates-io]`

Every published `feldera-size-of` release, up to and including 0.1.7 (and
upstream `main`), pins its optional `ordered-float` dependency at `^3.0.0`. Its
`SizeOf` impl for `OrderedFloat`/`NotNan` therefore applies only to
ordered-float 3.x. Against 5.x the derive fails to compile:

```plaintext
error[E0277]: the trait bound `ordered_float::OrderedFloat<f64>:
feldera_size_of::SizeOf` is not satisfied
```

Lille cannot implement `SizeOf` for `OrderedFloat` itself, because both the
trait and the type are foreign (the orphan rule forbids it). No upstream
release accepts ordered-float 5.x, so there is nothing to upgrade to.

The workaround redirects the crate to a minimal vendored fork:

```toml
[patch.crates-io]
feldera-size-of = { path = "third_party/feldera-size-of" }
```

The fork widens its `ordered-float` constraint to 5 and switches the `SizeOf`
impl bound from `Float` to `FloatCore` (ordered-float 5.x bounds its `Deref`
impls on `FloatCore`, and the impl relies on the `&OrderedFloat<T>` → `&T`
deref coercion). That bound switch is the only source change the upgrade
requires.

### Why `third_party/feldera-size-of` is excluded from the workspace

```toml
[workspace]
members = ["build_support", "test_utils"]
exclude = ["third_party/feldera-size-of"]
```

The fork is a `[patch.crates-io]` target, not a first-class workspace member.
Excluding it keeps Cargo from folding it into this workspace and keeps its
upstream source out of the workspace-wide gates (`cargo fmt`, `cargo clippy`,
`cargo test`, `cargo doc`). It is still built and linked, but only as a patched
dependency of `lille`.

### Ownership and scope: carried, not maintained

`third_party/feldera-size-of` is **carried upstream code, not adopted or
maintained Lille code**. Except for the deliberately minimal changes listed in
`third_party/README.md`, its source is byte-identical to `feldera-size-of`
0.1.7, and Lille does not hold it to this repository's code-health, testing,
documentation, or lint standards. This is why the crate carries
`#![allow(warnings)]` and `#![cfg_attr(coverage_nightly, coverage(off))]`, and
why `.codescene/code-health-rules.json` disables the Code Duplication rule under
`third_party/**`.

Feedback about the upstream code that is outside the scope of the vendored fix
— for example requests to test, refactor, or re-architect functionality Lille
does not use, or to change the fork's fallible `SizeOf` traversal semantics or
lint allowances — is out of scope for this repository. Raise such concerns
upstream against [`feldera/size-of`](https://github.com/feldera/size-of).

The Lille-owned guard for this arrangement is
`tests/ordered_float_size_of.rs`, a compile-time integration regression test
asserting that `OrderedFloat<f64>`, `NotNan<f64>`, `Position`, and `BlockSlope`
implement `SizeOf`. It is Lille code and is maintained normally.

### Removing the fork

Once `feldera-size-of` publishes a release whose optional `ordered-float`
dependency accepts 5.x (ideally including the `Float` → `FloatCore` bound fix):

1. Bump Lille's `feldera-size-of` dependency to that upstream release.
2. Delete `third_party/feldera-size-of/`.
3. Drop the `[patch.crates-io]` entry and the `[workspace] exclude` line from
   the root `Cargo.toml`.
4. Remove the CodeScene `third_party/**` rule set from
   `.codescene/code-health-rules.json` if nothing else needs it.
5. Keep `tests/ordered_float_size_of.rs` as the regression guard.
6. Regenerate `Cargo.lock` and run the standard gates.

Progress against these steps is tracked in
[issue #294](https://github.com/leynos/lille/issues/294).

## DBSP synchronization

Each frame, `DbspPlugin` chains two systems so the first runs to completion
before the second starts: `cache_state_for_dbsp_system` reads ECS component
state into the DBSP circuit's input handles, then
`apply_dbsp_outputs_system` steps the circuit and writes its outputs back
onto ECS components.

`DbspState` exposes frame-rollback methods that keep Rust-side bookkeeping
in step with the circuit, called in this order:

- `begin_frame_rollback` — start of the cache pass; clears the previous
  frame's rollback log.
- `record_unsequenced_undo` — during damage ingestion, captures an
  `applied_unsequenced` entry's pre-frame value before it is mutated.
- `stash_frame_rollback` — saves the pre-frame health-snapshot and
  pending-damage values the cache pass extracted.
- `commit_frame_tracking` — on a successful step, discards the rollback
  log.
- `rollback_frame_tracking` — on a failed step, restores the pre-frame
  tracking.

When `state.step_circuit()` returns `Err`, the output system clears the
circuit inputs, restores the Rust-side tracking
(`rollback_frame_tracking`), emits a `DbspSyncError` event, and applies no
ECS output writes that frame.

`apply_positions`, `apply_velocities`, and `apply_health_deltas` apply only
consolidated records with a positive Z-set weight; non-positive
(retraction) weights are skipped.

For the detailed walkthrough, see
[DBSP synchronization developer's guide](dbsp-synchronization-guide.md).

## Commit gates

Run the deterministic gates before committing (see `AGENTS.md` and the
`Makefile`): `make check-fmt`, `make test`, `make typecheck`, and `make lint`.
`make test` passes `--features test-support`, so it also runs the
buffered-message compile-pass harness
(`cargo test --features test-support --test compile_pass`; see
[Buffered events use the Message API](#buffered-events-use-the-message-api)).
`make lint` runs rustdoc (`--cfg docsrs`),
`cargo clippy --all-targets --all-features -- -D warnings`, and the Whitaker
Dylint suite.
