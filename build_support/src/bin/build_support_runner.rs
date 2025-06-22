//! Small wrapper binary to invoke build_support::build for testing.
//!
//! Allows running the build pipeline without compiling the entire game.
use build_support::BuildOptions;
use color_eyre::eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;
    build_support::build_with_options(&BuildOptions {
        fail_on_ddlog_error: false,
    })
}
