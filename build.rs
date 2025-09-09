//! Cargo build script for the project.
//! Delegates to the `build_support` crate so the logic can be tested.
use anyhow::Result;

fn main() -> Result<()> {
    build_support::build()
}
