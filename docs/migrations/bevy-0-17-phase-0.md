# Bevy 0.17 Migration – Phase 0 baseline (18 November 2025)

## Context

Phase 0 validates the current 0.12 toolchain so that later bumps to 0.13+ can
be compared against a known-good state. The goals were to confirm that Lille
only depends on upstream Bevy crates, capture reproducible build artefacts, and
prove that the render feature still boots a windowed app with DBSP acting as
the source of truth for world-state inference.

## Captured artefacts

All logs live under `artifacts/bevy-0-17-upgrade/phase-0/` to make diffs
against future phases trivial. Each command was executed on 18 November 2025
using the workspace’s pinned nightly toolchain.

- `cargo tree -i bevy` → `cargo-tree-bevy.txt`. Confirms that the workspace only
  references `bevy v0.12.1` from crates.io with no local forks in play.
- `cargo check --all-features` → `cargo-check-all-features.log`. Establishes a
  zero-warning baseline for the full feature matrix so that new clippy or
  borrow errors introduced by 0.13 can be attributed to the upgrade.
- `cargo test` → `cargo-test.log`. Includes the rstest-powered unit suites plus
  the behavioural `rust-rspec` specs that assert DBSP remains the canonical
  world model. These results are the canonical “before” snapshot for regression
  hunting.
- Render smoke test → `render-smoke.log`. Executed via
  `RUST_LOG=info timeout 5s cargo run -p lille --features render -- --verbose`.
  The five-second timeout cleanly terminates the Bevy loop after confirming
  that window creation, plugin wiring, and DBSP synchronisation still
  initialise correctly. A non-zero exit status is expected because `timeout`
  sends SIGTERM.

## Observations

- No downstream crates override `bevy` or related `bevy_*` crates, so crates.io
  releases remain the single source of truth.
- `cargo test` verified that the DBSP circuit drives entity inference and emits
  the expected events, giving us a baseline for “DBSP is authoritative”.
- The render smoke run exercised `DefaultPlugins` with logging enabled, proving
  that MinimalPlugins plus `DbspPlugin` still coexist with the render feature.

## Next actions

- Begin Phase 1 by bumping every `bevy*` crate to `0.13.*`, regenerating
  `Cargo.lock`, and rerunning the artefact suite for comparison.
- Update the design docs with any deviations observed during the Phase 1 work,
  especially if DBSP scheduling assumptions no longer hold on Bevy 0.13.
