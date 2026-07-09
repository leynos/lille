//! Helpers for running `rspec` suites with predictable threading.

use rspec::block::Suite;
use rspec::{Configuration, Logger, Runner};
use std::sync::Arc;

/// Runs an rspec suite serially to keep `NonSend` Bevy resources on a single
/// thread.
pub fn run_serial<T>(suite: &Suite<T>)
where
    T: Clone + Send + Sync + std::fmt::Debug,
{
    let logger = Arc::new(Logger::new(std::io::stdout()));
    // Construct the configuration directly rather than via the fallible
    // builder so no panicking fallback is required. `exit_on_failure`
    // ensures rspec failures fail the Rust test binary.
    let config = Configuration {
        parallel: false,
        exit_on_failure: true,
    };
    Runner::new(config, vec![logger]).run(suite);
}
