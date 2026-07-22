#![cfg_attr(
    feature = "test-support",
    doc = "Compile-time coverage for the Bevy 0.18 buffered-message migration."
)]
#![cfg_attr(
    not(feature = "test-support"),
    doc = "Compile-time migration tests require `test-support`."
)]
#![cfg(feature = "test-support")]
//! Guards the Bevy 0.17 -> 0.18 buffered-message migration against regressions.
//!
//! The runtime map integration tests exercise the migrated APIs dynamically;
//! this harness adds an *isolated* compile-pass check via `trybuild`, so that a
//! future edit reintroducing the legacy `EventReader` / `World::send_event`
//! surface fails to compile the fixture rather than silently drifting.
//!
//! The fixture is compiled as a standalone crate that depends on `lille` with
//! the `test-support` feature (which activates `map` and pulls in
//! `bevy_ecs_tiled`), mirroring the feature path used by the production map
//! integration. Run it explicitly with:
//!
//! ```sh
//! cargo test --features test-support --test compile_pass
//! ```

/// Asserts the migrated buffered-message API surface still compiles.
///
/// `trybuild` builds the fixture in isolation and fails the test if it does not
/// compile, which pins `MessageReader<TiledEvent<MapCreated>>` and
/// `World::write_message` as the supported API.
#[test]
fn message_reader_migration_compiles() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/compile_pass/message_reader_migration.rs");
}
