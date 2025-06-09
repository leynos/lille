use env_logger::{Builder, Env};
use log::LevelFilter;

/// Initializes the global logger.
///
/// When `verbose` is `true`, all debug messages are printed. Otherwise only
/// info level and above are shown.
pub fn init(verbose: bool) {
    let level = if verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    let env = Env::default().default_filter_or(level.to_string());
    let mut builder = Builder::from_env(env);

    if let Err(err) = builder.try_init() {
        // Only suppress errors caused by multiple initializations.
        if !err.to_string().contains("set a logger") {
            eprintln!("Failed to initialize logger: {err}");
        }
    }
}

#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        log::debug!($($arg)*);
    };
}
