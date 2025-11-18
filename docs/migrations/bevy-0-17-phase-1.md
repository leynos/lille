# Bevy 0.17 Migration – Phase 1 (0.12 → 0.13) – 18 November 2025

## Summary

Phase 1 bumps every `bevy*` dependency to the 0.13 line while keeping the
existing feature set unchanged. The upgrade validates that Lille's Minimal
Plugins path (`DbspPlugin` + `MinimalPlugins`) continues to drive the DBSP
circuit as the single source of truth ahead of the rendering stages targeted in
later phases.

## Key changes

- `Cargo.toml` now pins `bevy`, `bevy_app`, `bevy_ecs`, `bevy_hierarchy`,
  `bevy_math`, `bevy_reflect`, `bevy_transform`, and the optional `bevy_log`
  crate to `0.13`. The linux-only dependency mirrors the same version bump.
- Added a `TestWorld::spawn_orphan_entity` helper plus a tracked-entity
  despawner so behavioural tests can exercise both sides of the synchronisation
  contract.
- Introduced `tests/physics_bdd/dbsp_authority.rs`, an `rstest` + `rust-rspec`
  scenario that asserts only entities carrying `DdlogId` receive DBSP-driven
  gravity updates. This captures the happy (registered) and unhappy (orphan)
  paths demanded by the migration plan and doubles as a regression test for
  MinimalPlugins smoke runs.
- Documented the ownership rule in `docs/lille-physics-engine-design.md`, tying
  the new test to the architecture section that describes `DbspState`.

## Test evidence

Raw logs live under `artifacts/bevy-0-17-upgrade/phase-1/`:

- `cargo-tree-bevy.txt` – confirms every Bevy crate resolves to 0.13.2 pulled
  from crates.io with no local forks.
- `cargo-check-all-features.log` – `cargo check --all-features` output for the
  new dependency graph.
- `cargo-clippy.log` – `cargo clippy --all-targets --all-features -D warnings`
  output; zero lints reported.
- `cargo-test.log` – `cargo test` run covering the DBSP `rstest` suites and the
  updated BDD specs (`tests/physics_bdd/dbsp_authority.rs` et al.).
- `render-smoke.log` –
  `RUST_LOG=info timeout 5s cargo run -p lille --features render -- --verbose`
  execution. The timeout's `SIGTERM` exit is expected; it records plugin
  registration and window initialisation on Bevy 0.13.

## Follow-ups

- Proceed to Phase 2 (0.13 → 0.14) by repeating the dependency bump and rerun
  the artefact suite plus the MinimalPlugins BDD scenarios.
- Expand CI to upload the artefact bundles so reviewers can diff phase outputs
  without rerunning locally.
