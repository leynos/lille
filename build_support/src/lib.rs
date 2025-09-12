//! Build support utilities used by the project's build script.
//! Handles asset downloads for font resources.

pub mod font;

use anyhow::Result;
use std::path::PathBuf;

/// Execute all build steps required by `build.rs`.
/// This function downloads the Fira Sans font.
/// Environment variables such as `CARGO_MANIFEST_DIR` must be set
/// by Cargo before this function is called.
///
/// # Examples
/// ```rust,no_run
/// use anyhow::Result;
/// fn main() -> Result<()> {
///     build_support::build()
/// }
/// ```
///
/// # Returns
/// `Ok(())` when build steps complete. Network and write failures in font
/// setup fall back to a platform font and still return `Ok(())`.
///
/// # Errors
/// Returns an error if required environment variables are missing or if creating
/// temporary files fails.
pub fn build() -> Result<()> {
    dotenvy::dotenv_override().ok();
    set_rerun_triggers();

    let manifest_dir = manifest_dir()?;

    let font_path = font::download_font(&manifest_dir)?;

    println!("cargo:rustc-env=FONT_PATH={}", font_path.display());

    Ok(())
}

/// Execute all build steps. Retained for backward compatibility with older
/// scripts that expected a configurable entry point.
pub fn build_with_options() -> Result<()> {
    build()
}

fn manifest_dir() -> Result<PathBuf> {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    Ok(manifest)
}

fn set_rerun_triggers() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets");
}
