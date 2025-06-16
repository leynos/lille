//! Cargo build script for the project.
//! Delegates to the `build_support` crate so the logic can be tested.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    build_support::build()
}
