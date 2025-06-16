//! Cargo build script that delegates to the `build_support` crate.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    build_support::build()
}
