//! Small wrapper binary to invoke build_support::build for testing.
//!
//! Allows running the build pipeline without compiling the entire game.
use anyhow::Result;

fn main() -> Result<()> {
    build_support::build()
}
