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
    init_with(verbose, |level| {
        let mut builder = Builder::from_env(Env::default());
        builder.filter_level(level);
        builder.format_timestamp_secs().format_module_path(true);
        builder.try_init()
    })
}

fn init_with<F, E>(verbose: bool, install: F) -> Result<()>
where
    F: FnOnce(LevelFilter) -> std::result::Result<(), E>,
    E: std::error::Error + Send + Sync + 'static,
{
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

    match install(level).with_context(|| "initialising env_logger") {
        Ok(()) => Ok(()),
        Err(err) => {
            LOGGER_INITIALISED.store(false, Ordering::SeqCst);
            Err(err)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    fn reset_logger_flag() {
        LOGGER_INITIALISED.store(false, Ordering::SeqCst);
    }

    #[test]
    fn init_with_sets_trace_logging_when_verbose() -> Result<()> {
        let _guard = TEST_LOCK.lock().expect("poisoned test lock");
        reset_logger_flag();
        let mut called = false;

        init_with(true, |level| {
            called = true;
            assert_eq!(level, LevelFilter::Trace);
            Ok::<(), TestError>(())
        })?;

        assert!(
            called,
            "installer should be invoked on first initialisation"
        );
        assert!(LOGGER_INITIALISED.load(Ordering::SeqCst));
        reset_logger_flag();
        Ok(())
    }

    #[test]
    fn init_is_idempotent_after_success() -> Result<()> {
        let _guard = TEST_LOCK.lock().expect("poisoned test lock");
        reset_logger_flag();
        LOGGER_INITIALISED.store(true, Ordering::SeqCst);
        let mut called = false;

        init_with(false, |level| {
            called = true;
            assert_eq!(level, LevelFilter::Info);
            Ok::<(), TestError>(())
        })?;

        assert!(
            !called,
            "installer must not run when logger already initialised"
        );
        reset_logger_flag();
        Ok(())
    }

    #[test]
    fn init_resets_flag_when_installation_fails() {
        let _guard = TEST_LOCK.lock().expect("poisoned test lock");
        reset_logger_flag();

        let result = init_with(false, |_| Err(TestError));

        assert!(result.is_err());
        assert!(
            !LOGGER_INITIALISED.load(Ordering::SeqCst),
            "failed initialisation should release the flag"
        );
    }
}
