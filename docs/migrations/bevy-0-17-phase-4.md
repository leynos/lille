# Bevy 0.17 Migration – Phase 4 (0.15 → 0.16) – 25 November 2025

## Summary

Phase 4 upgrades the workspace to Bevy 0.16.1 and adopts the new hierarchy
spawn API so demo entities share a single `WorldRoot` parent via `ChildOf`. The
change keeps the Differential Dataflow Stream Processing (DBSP) circuit as the
authority for inferred behaviour while letting Bevy use faster relationship
lookups introduced in 0.16.

## Key changes

- All Bevy crates (including the Linux `x11` variant) now target 0.16.1;
  feature flags remain constrained to the minimal render stack.
- `spawn_world_system` parents the landmark, civvy, baddie, and camera under a
  `WorldRoot` entity using `ChildOf`, removing post-spawn inserts and leaning
  on the 0.16 spawn ergonomics.
- `WorldRoot` is exported for tests; `tests/spawn.rs` gained an `rstest`
  asserting every spawned entity is parented to the root, and
  `tests/spawn_dbsp_rspec.rs` now checks DBSP caches persist even if `ChildOf`
  is removed (unhappy path) alongside the existing transform-removal case.
- Observers V1 were reviewed for DBSP event routing but deferred; file a
  follow-up ticket when the event push/pull graph is ready for refactor.

## Test evidence

Logs for this phase live in `artifacts/bevy-0-17-upgrade/phase-4/`:

- `make-fmt.log` – `make fmt` to format Rust and Markdown assets.
- `make-check-fmt.log` – `make check-fmt` (`cargo fmt --check`).
- `make-lint.log` – `make lint` (`cargo doc` + `cargo clippy -D warnings`).
- `make-test.log` – `make test` (`cargo test --workspace`).
- `cargo-test-all-features.log` – `cargo test --workspace --all-features` to
  exercise the render feature alongside headless paths.
