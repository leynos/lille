//! Logging utilities.
//! Exposes `init` which configures `env_logger` with optional verbosity.
use anyhow::{Context, Result};
use env_logger::{Builder, Env};
use log::LevelFilter;
use once_cell::sync::OnceCell;

static LOGGER_INITIALISED: OnceCell<()> = OnceCell::new();

/// Initialises the global logger once for the entire process.
///
/// When `verbose` is `true`, all debug messages are printed. Otherwise only
/// info level and above are shown.
///
/// # Examples
/// ```
/// # use anyhow::Result;
/// # fn main() -> Result<()> {
/// lille::init(true)?;
fn init_with_cell<F, E>(cell: &OnceCell<()>, verbose: bool, install: F) -> Result<()>
where
    F: FnOnce(LevelFilter) -> std::result::Result<(), E>,
    E: std::error::Error + Send + Sync + 'static,
{
    cell.get_or_try_init(move || {
        let level = if verbose {
            LevelFilter::Trace
        } else {
            LevelFilter::Info
        };

        install(level).with_context(|| "initialising env_logger")?;
        Ok(())
    })
    .map(|_| ())
}

/// Initialises the global logger once for the entire process.
///
/// When `verbose` is `true`, all debug messages are printed. Otherwise only
/// info level and above are shown.
///
/// # Examples
/// ```
/// # use anyhow::Result;
/// # fn main() -> Result<()> {
/// lille::init(true)?;
/// # Ok(())
/// # }
/// ```
///
/// # Errors
/// Returns an error if another logger implementation has already been
/// installed and prevents `env_logger` from taking ownership.
pub fn init(verbose: bool) -> Result<()> {
    init_with_cell(&LOGGER_INITIALISED, verbose, |level| {
        let mut builder = Builder::from_env(Env::default());
        builder.filter_level(level);
        builder.format_timestamp_secs().format_module_path(true);
        builder.try_init()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::ensure;
    use std::fmt;
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[derive(Debug, Clone, Copy)]
    struct TestError;

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "test logger install failure")
        }
    }

    impl std::error::Error for TestError {}

    #[test]
    fn init_with_sets_trace_logging_when_verbose() -> Result<()> {
        let _guard = TEST_LOCK.lock().expect("poisoned test lock");
        let cell = OnceCell::new();
        let mut called = false;
        let mut observed_level = None;

        init_with_cell(&cell, true, |level| {
            called = true;
            observed_level = Some(level);
            Ok::<(), TestError>(())
        })?;

        ensure!(
            called,
            "installer should be invoked on first initialisation"
        );
        ensure!(
            cell.get().is_some(),
            "logger flag should remain set after success"
        );
        ensure!(
            observed_level == Some(LevelFilter::Trace),
            "expected trace level when verbose initialises"
        );
        Ok(())
    }

    #[test]
    fn init_is_idempotent_after_success() -> Result<()> {
        let _guard = TEST_LOCK.lock().expect("poisoned test lock");
        let cell = OnceCell::new();
        cell.set(()).expect("failed to seed logger flag");
        let mut called = false;
        let mut observed_level = None;

        init_with_cell(&cell, false, |level| {
            called = true;
            observed_level = Some(level);
            Ok::<(), TestError>(())
        })?;

        ensure!(
            !called,
            "installer must not run when logger already initialised"
        );
        ensure!(
            observed_level.is_none(),
            "installer closure must not run when logger already initialised"
        );
        Ok(())
    }

    #[test]
    fn init_resets_flag_when_installation_fails() {
        let _guard = TEST_LOCK.lock().expect("poisoned test lock");
        let cell = OnceCell::new();

        let result = init_with_cell(&cell, false, |_| Err(TestError));

        assert!(result.is_err());
        assert!(
            cell.get().is_none(),
            "failed initialisation should release the flag"
        );
    }
}
