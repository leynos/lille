//! Compile-pass fixture for the Bevy 0.18 buffered-message migration.
//!
//! This fixture compiles only when the migrated buffered-message APIs are used
//! the way the production map systems use them:
//!
//! - buffered `TiledEvent<MapCreated>` is read with `MessageReader`, the Bevy
//!   0.18 rename of `EventReader`;
//! - a `TiledEvent<MapCreated>` is enqueued with `World::write_message`, the
//!   rename of `World::send_event`.
//!
//! `TiledEvent` derives both `Message` (buffered) and `EntityEvent` (observer)
//! upstream; this fixture deliberately exercises only the buffered-message path,
//! keeping it distinct from the observer `Event` surface (`On<T>` / `trigger`).
//!
//! `trybuild` compiles this file as its own standalone crate, so it names
//! `bevy_ecs_tiled` through that crate's *direct, non-optional* `bevy_ecs_tiled`
//! dev-dependency. That is distinct from the repository's `lille` dependency,
//! whose `test-support` feature activates `map` and pulls in `bevy_ecs_tiled`
//! for the in-crate map integration tests.

use bevy::prelude::*;
use bevy_ecs_tiled::prelude::{MapCreated, TiledEvent};

/// System reading buffered `TiledEvent<MapCreated>` messages.
///
/// Uses `MessageReader`; the legacy `EventReader<TiledEvent<MapCreated>>` no
/// longer exists in Bevy 0.18 and would fail to compile here.
fn reads_map_created(mut messages: MessageReader<TiledEvent<MapCreated>>) {
    for _message in messages.read() {}
}

/// Enqueues a `TiledEvent<MapCreated>` into the world.
///
/// Uses `World::write_message`; the legacy `World::send_event` was removed in
/// Bevy 0.18 and would fail to compile here.
fn writes_map_created(world: &mut World) {
    world.write_message(TiledEvent::new(Entity::PLACEHOLDER, MapCreated));
}

fn main() {
    let mut app = App::new();
    app.add_message::<TiledEvent<MapCreated>>();
    app.add_systems(Update, reads_map_created);
    writes_map_created(app.world_mut());
}
