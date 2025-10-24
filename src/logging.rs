//! Logging utilities.
//! Exposes `init` which configures `env_logger` with optional verbosity.
use anyhow::{Context, Result};
use env_logger::{Builder, Env};
use log::LevelFilter;
use std::sync::atomic::{AtomicBool, Ordering};

static LOGGER_INITIALISED: AtomicBool = AtomicBool::new(false);

/// Initialises the global logger once for the entire process.
///
/// When `verbose` is `true`, all debug messages are printed. Otherwise only
/// info level and above are shown.
///
/// # Examples
/// ```
/// # use anyhow::Result;
/// # fn main() -> Result<()> {
/// lille::init_logging(true)?;
/// # Ok(())
/// # }
/// ```
///
/// # Errors
/// Returns an error if another logger implementation has already been
/// installed and prevents `env_logger` from taking ownership.
pub fn init(verbose: bool) -> Result<()> {
    if LOGGER_INITIALISED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Ok(());
    }

    let level = if verbose {
        LevelFilter::Trace
    } else {
        LevelFilter::Info
    };

    let mut builder = Builder::from_env(Env::default());
    builder.filter_level(level);
    builder.format_timestamp_secs().format_module_path(true);

    if let Err(err) = builder.try_init() {
        LOGGER_INITIALISED.store(false, Ordering::SeqCst);
        return Err(err).context("initialising env_logger");
    }

    Ok(())
}
