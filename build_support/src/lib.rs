pub mod constants;
pub mod font;
pub mod ddlog;

use std::{error::Error, path::PathBuf};

/// Runs the build script logic used by `build.rs`.
pub fn build() -> Result<(), Box<dyn Error>> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);

    constants::generate_constants(&manifest_dir, &out_dir)?;
    let font_path = font::download_font(&manifest_dir)?;
    ddlog::compile_ddlog(&manifest_dir, &out_dir)?;

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets");
    println!("cargo:rerun-if-changed=src/lille.dl");
    println!("cargo:rerun-if-changed=constants.toml");
    println!("cargo:rustc-env=FONT_PATH={}", font_path.display());

    Ok(())
}
