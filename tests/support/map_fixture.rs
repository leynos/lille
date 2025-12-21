#![cfg(feature = "test-support")]
//! Shared fixture infrastructure for map plugin behavioural tests.
//!
//! The rspec-based map tests tick a Bevy `App` and need to:
//! - safely share that `App` across rspec closures,
//! - ensure `app.finish()` / `app.cleanup()` is run exactly once, and
//! - provide a single place for the per-tick sleep used to reduce busy-waiting.

use std::sync::MutexGuard;

use bevy::prelude::*;

use crate::thread_safe_app::{lock_app, SharedApp, ThreadSafeApp};

const TICK_SLEEP_MS: u64 = 1;

#[derive(Resource, Debug, Default)]
struct PluginsFinalized;

/// A shared base fixture that owns a `ThreadSafeApp` and provides consistent
/// ticking behaviour.
#[derive(Debug, Clone)]
pub struct MapPluginFixtureBase {
    app: SharedApp,
}

impl MapPluginFixtureBase {
    /// Wraps an already-configured Bevy `App` into a thread-safe fixture.
    #[must_use]
    pub fn new(app: App) -> Self {
        Self {
            app: std::sync::Arc::new(std::sync::Mutex::new(ThreadSafeApp(app))),
        }
    }

    /// Locks the underlying `App` for direct inspection or mutation.
    pub fn app_guard(&self) -> MutexGuard<'_, ThreadSafeApp> {
        lock_app(&self.app)
    }

    /// Advances the application by a single tick.
    ///
    /// The first tick finalizes plugins and performs cleanup so schedules can
    /// run deterministically.
    pub fn tick(&self) {
        let mut app = self.app_guard();
        if app.world().get_resource::<PluginsFinalized>().is_none() {
            app.finish();
            app.cleanup();
            app.insert_resource(PluginsFinalized);
        }

        app.update();
        std::thread::sleep(std::time::Duration::from_millis(TICK_SLEEP_MS));
    }
}
