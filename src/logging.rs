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

    // `try_init` only fails if a logger was already set. Ignore that case so
    // tests can call `init` multiple times without panicking.
    let _ = builder.try_init();
}
