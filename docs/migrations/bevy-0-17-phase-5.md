# Bevy 0.17 Migration – Phase 5 (0.16 → 0.17.3) – 27 November 2025

## Summary

Phase 5 completes the upgrade to Bevy 0.17.3. Reflect auto-registration is
enabled, the Events V2 observer pipeline is adopted for Differential
Bidirectional Stream Processing (DBSP) error reporting, and the DBSP circuit is
validated as the single authority over inferred behaviour even when a circuit
step fails.

## Key changes

- All Bevy crates now target 0.17.3; `reflect_auto_register` is enabled so new
  `Reflect` types register automatically without manual `App::register_type`
  calls.
- `DbspPlugin` emits `DbspSyncError` events through Events V2 observers and
  logs both initialization and step failures while bailing out before any
  Entity Component System (ECS) writes, keeping DBSP authoritative during error
  paths.
- Added `rstest` coverage for the step failure path and a `rust-rspec`
  scenario that proves observers capture the error while the world state stays
  unchanged; a test hook allows the circuit stepper to be swapped during tests.
- DBSP test helpers now live in `test_utils`, and the circuit step override is
  compiled only for tests and debug builds to avoid production usage.

## Test evidence

Logs for this phase live in `artifacts/bevy-0-17-upgrade/phase-5/`:

- `make-fmt.log` – `make fmt` to format Rust and Markdown assets.
- `make-check-fmt.log` – `make check-fmt` (`cargo fmt --check`).
- `make-lint.log` – `make lint` (`cargo doc` + `cargo clippy -D warnings`).
- `make-test.log` – `make test` (`cargo test --workspace`).
- `cargo-test-render.log` – `cargo test --workspace --features render`.
- `cargo-test-text.log` – `cargo test --workspace --features text`.
- `cargo-test-all-features.log` – `cargo test --workspace --all-features`.
- `cargo-test-build-support.log` – `cargo test -p build_support`.
