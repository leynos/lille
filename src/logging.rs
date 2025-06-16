//! Logging utilities.
use env_logger::{Builder, Env};
use log::LevelFilter;

/// Initializes the global logger.
///
/// When `verbose` is `true`, all debug messages are printed. Otherwise only
/// info level and above are shown.
pub fn init(verbose: bool) {
    let level = if verbose {
        LevelFilter::Trace
    } else {
        LevelFilter::Info
    };

    let mut builder = Builder::from_env(Env::default());
    builder.filter_level(level);
    builder.format_timestamp_secs().format_module_path(true);

    // Ignore the error if the logger has already been initialized.
    let _ = builder.try_init();
}
