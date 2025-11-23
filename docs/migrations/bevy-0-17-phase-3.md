# Bevy 0.17 Migration – Phase 3 (0.14 → 0.15) – 23 November 2025

## Summary

Phase 3 upgrades the workspace to Bevy 0.15.1 and replaces deprecated bundles
in `spawn_world_system` with the Required Components model introduced in 0.15.
Sprites now spawn via `Sprite` + `Transform` + `Visibility`, and the camera uses
`Camera2d` with an orthographic `Projection` positioned above the world to keep
render parity with the old bundle defaults. A `rust-rspec` scenario ensures the
DBSP circuit stays authoritative when components go missing between ticks.

## Key changes

- Pinned all Bevy crates to 0.15.1; newer 0.15.x releases drag `uuid` = 1.12,
  which conflicts with DBSP’s `uuid` ^1.17 requirement, so 0.15.1 is the
  highest compatible version today.
- `spawn_world_system` now builds entities from Required Components rather than
  bundles, using a `basic_sprite` helper for clarity. The camera combines
  `Camera2d`, `Projection::Orthographic`, and a high `z` value to mirror the
  previous `Camera2dBundle` placement.
- Added `tests/spawn_dbsp_rspec.rs` (`rust-rspec`) to prove DBSP caches spawned
  IDs and drops entries when transforms disappear, keeping the circuit as the
  source of truth for inferred state. Existing `rstest` coverage for spawning
  was updated to assert visibility and projection components.
- `WorldHandle` now exposes `entity_ids()` and `entity_count()` to aid DBSP
  state assertions, and the physics BDD helpers wrap `App` in a Send + Sync
  guard so `rust-rspec` can execute safely.

## Test evidence

Logs for this phase live in `artifacts/bevy-0-17-upgrade/phase-3/`:

- `make-check-fmt.log` – `cargo fmt --check` via `make check-fmt`.
- `make-lint.log` – `cargo clippy --workspace --all-targets --all-features -D
  warnings`.
- `make-test.log` – `cargo test --workspace`.
- `cargo-test-all-features.log` – `cargo test --all-features` including render
  coverage.
- `cargo-run-render-help.log` – `cargo run -p lille --features render --
  --help` to compile and exercise the render feature entrypoint headlessly.

## Follow-ups

- Proceed to Phase 4 (0.15 → 0.16) once DBSP’s `uuid` dependency permits a
  higher Bevy patch or a new release aligns versions.
- Run a visual render smoke test under X11/Wayland or xvfb when a display is
  available to double-check material and visibility behaviour after the switch
  to Required Components.
