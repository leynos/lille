# Bevy 0.17 Migration – Phase 2 (0.13 → 0.14) – 21 November 2025

## Summary

Phase 2 bumps every Bevy crate to the 0.14 line and validates that the
Differential Dataflow Stream Processing (DBSP) sync layer still runs inside a
single `Update` tick after the schedule timing
changes introduced in 0.14. The MinimalPlugins path remains the reference
environment for CI and local testing; render work is deferred to Phase 3.

## Key changes

- `bevy`, `bevy_app`, `bevy_ecs`, `bevy_hierarchy`, `bevy_math`,
  `bevy_reflect`, `bevy_transform`, and optional `bevy_log` now target 0.14.2,
  including the Linux `x11` variant. `App::world` field access was rewritten to
  the 0.14 `world()` / `world_mut()` API across tests and DBSP helpers.
- Added `tests/dbsp_schedule_regression.rs` (`rstest`) to prove `DbspState`
  stays available via `World::get_non_send_resource` across frames and that
  damage ingestion completes within one `Update` run.
- Extended `tests/physics_bdd/dbsp_authority.rs` with a `rust-rspec` scenario
  that corrupts the `WorldHandle` mirror between ticks and asserts DBSP
  refreshes the mirror immediately, keeping the circuit authoritative for
  inferred state.
- Documented the scheduling invariants in
  `docs/bevy-0-16-plus-migration-plan.md`
  and `docs/lille-physics-engine-design.md` so contributors understand why the
  new regressions exist.

## Test evidence

Logs live under `artifacts/bevy-0-17-upgrade/phase-2/`:

- `cargo-tree-bevy.txt` – `cargo tree -i bevy` confirms the workspace resolves
  to Bevy 0.14.2 with no local forks.
- `make-fmt.log` – `make fmt` output covering Rust and Markdown formatting.
- `make-lint.log` – `make lint` (`cargo clippy --all-targets --all-features -D
  warnings`) on the 0.14 toolchain.
- `make-test.log` – `make test` integration/unit run.
- `cargo-test-all-features.log` – `cargo test --all-features` to mirror the
  migration checklist.
- `cargo-doc.log` – `cargo doc --no-deps --all-features` to verify rustdoc
  builds cleanly under the new lint defaults.

## Follow-ups

- Proceed to Phase 3 (0.14 → 0.15) by replacing deprecated bundles with
  Required Components in `spawn_world_system` and rerunning the regression
  matrix.
- Track Bevy 0.15 schedule adjustments and extend the DBSP regressions if the
  Update chain changes again.
