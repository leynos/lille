//! Small wrapper binary to invoke build_support::build for testing.
//!
//! Allows running the build pipeline without compiling the entire game.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    build_support::build()
}
