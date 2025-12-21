//! Helpers for running `rspec` suites with predictable threading.

use rspec::{block::Suite, ConfigurationBuilder, Logger, Runner};
use std::sync::Arc;

/// Runs an rspec suite serially to keep `NonSend` Bevy resources on a single
/// thread.
pub fn run_serial<T>(suite: &Suite<T>)
where
    T: Clone + Send + Sync + std::fmt::Debug,
{
    let logger = Arc::new(Logger::new(std::io::stdout()));
    let config = ConfigurationBuilder::default()
        .parallel(false)
        // Ensure rspec failures fail the Rust test binary (by exiting with the
        // standard `cargo test` failure code via rspec's runner).
        .exit_on_failure(true)
        .build()
        .unwrap_or_else(|e| panic!("rspec configuration failed: {e}"));
    Runner::new(config, vec![logger]).run(suite);
}
