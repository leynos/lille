use env_logger::{Builder, Env};
use log::{LevelFilter, SetLoggerError};

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

    let env = Env::default().default_filter_or(level.to_string().to_lowercase());
    let mut builder = Builder::from_env(env);
    builder.format_timestamp_secs().format_module_path(true);

    if let Err(e) = builder.try_init() {
        // Only suppress the AlreadyInit error so tests can call `init` multiple
        // times. Log any other error so it's not silently ignored.
        if !matches!(e, SetLoggerError { .. }) {
            eprintln!("Failed to initialize logger: {e}");
        }
    }
}
