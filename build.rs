//! Cargo build script for the project.
//! Delegates to the `build_support` crate so the logic can be tested.
use color_eyre::eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;
    build_support::build()
}
