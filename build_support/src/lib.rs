//! Build support utilities used by the project's build script.
//! Coordinates constants generation and font downloads.
pub mod constants;
pub mod font;

use color_eyre::eyre::Result;
use std::path::PathBuf;

#[expect(
    unexpected_cfgs,
    non_snake_case,
    unused_imports,
    reason = "OrthoConfig macro generates names that trigger these lints"
)]
#[allow(unfulfilled_lint_expectations)]
mod build_options {
    use ortho_config::OrthoConfig;
    use serde::Deserialize;

    /// Configuration options for the build pipeline.
    #[derive(Clone, Default, OrthoConfig, Debug, Deserialize)]
    #[ortho_config(prefix = "BUILD_SUPPORT")]
    #[expect(non_snake_case, reason = "OrthoConfig derives non-snake case fields")]
    #[allow(unfulfilled_lint_expectations)]
    pub struct BuildOptions {
        // currently no configurable options
    }
}

pub use build_options::BuildOptions;

/// Execute all build steps required by `build.rs`.
///
/// This function generates constants and downloads the Fira Sans font.
/// Environment variables such as `CARGO_MANIFEST_DIR` and `OUT_DIR` must be set
/// by Cargo before this function is called.
///
/// # Examples
/// ```rust,no_run
/// use color_eyre::eyre::Result;
/// fn main() -> Result<()> {
///     build_support::build()
/// }
/// ```
///
/// # Returns
/// `Ok(())` if all build steps succeed, otherwise an error is returned from the
/// failing step.
///
/// # Errors
/// Returns an error if required environment variables are missing or if any file
/// operation fails.
pub fn build() -> Result<()> {
    build_with_options(&BuildOptions::default())
}

/// Execute all build steps with configurable behaviour.
///
pub fn build_with_options(_options: &BuildOptions) -> Result<()> {
    dotenvy::dotenv_override().ok();
    set_rerun_triggers();

    let (manifest_dir, out_dir) = manifest_and_out_dir()?;

    constants::generate_constants(&manifest_dir, &out_dir)?;
    let font_path = font::download_font(&manifest_dir)?;

    println!("cargo:rustc-env=FONT_PATH={}", font_path.display());

    Ok(())
}

fn manifest_and_out_dir() -> Result<(PathBuf, PathBuf)> {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let out = PathBuf::from(std::env::var("OUT_DIR")?);
    Ok((manifest, out))
}

fn set_rerun_triggers() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets");
    println!("cargo:rerun-if-changed=constants.toml");
    println!("cargo:rerun-if-env-changed=DDLOG_HOME");
}

#[cfg(test)]
mod tests {}
