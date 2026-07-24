//! Guards the Bevy 0.17 -> 0.18 buffered-message migration against regressions.
//!
//! The runtime map integration tests exercise the migrated APIs dynamically;
//! this harness adds an *isolated* compile-pass check via `trybuild`, so that a
//! future edit reintroducing the legacy `EventReader` / `World::send_event`
//! surface fails to compile the fixture rather than silently drifting.
//!
//! `trybuild` builds the fixture as its own standalone crate, which names
//! `bevy_ecs_tiled` directly. The repository therefore carries a *non-optional*
//! `bevy_ecs_tiled` dev-dependency purely so that separate crate can name it; an
//! optional dependency is only linked into `lille`'s own targets, not into the
//! trybuild fixture crate. Independently, the `lille` dev-dependency enables the
//! `test-support` feature (which activates `map` and pulls in `bevy_ecs_tiled`),
//! mirroring the feature path the production map integration uses. Run it
//! explicitly with:
//!
//! ```sh
//! cargo test --features test-support --test compile_pass
//! ```
#![cfg_attr(
    feature = "test-support",
    doc = "Compile-time coverage for the Bevy 0.18 buffered-message migration."
)]
#![cfg_attr(
    not(feature = "test-support"),
    doc = "Compile-time migration tests require `test-support`."
)]
#![cfg(feature = "test-support")]

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
