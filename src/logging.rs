use env_logger::Builder;
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

    // Ignore reinitialization errors when tests call `init` multiple times.
    let _ = Builder::new().filter_level(level).try_init();
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        log::debug!($($arg)*);
    };
}
