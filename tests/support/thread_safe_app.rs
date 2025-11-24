//! Thread-safe wrapper for Bevy `App` used across integration tests.

use bevy::prelude::App;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// Wrapper that forwards `Send` and `Sync` because access is mutex-guarded.
#[derive(Debug)]
pub struct ThreadSafeApp(pub App);

impl Deref for ThreadSafeApp {
    type Target = App;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ThreadSafeApp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// SAFETY: rstest fixtures must implement `Clone + Send + Sync`, and rspec still
// executes the suite serially. All access to the wrapped `App` is guarded by
// the mutex, so the combination of single-threaded execution and the mutex
// lock makes forwarding `Send`/`Sync` sound for this test-only wrapper.
unsafe impl Send for ThreadSafeApp {}
unsafe impl Sync for ThreadSafeApp {}

/// Shared pointer type for the wrapped app.
pub type SharedApp = Arc<Mutex<ThreadSafeApp>>;

/// Locks the shared app, recovering from a poisoned mutex.
pub fn lock_app(app: &SharedApp) -> MutexGuard<'_, ThreadSafeApp> {
    app.lock().unwrap_or_else(PoisonError::into_inner)
}
