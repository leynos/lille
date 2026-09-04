//! Guards the dependency-resolution constraints this workspace relies on.
//!
//! `tinyvec` reaches the build transitively through Bevy's text and font
//! stack, and no `Cargo.lock` is committed, so resolution is decided afresh on
//! every machine. When 1.13.0 is selected the build fails deep inside the
//! crate with `cannot find macro `vec` in this scope`, which says nothing
//! about why. This test fails first, and says exactly which version was
//! selected and what to do about it.
//!
//! Remove it with the constraint itself, per
//! <https://github.com/leynos/lille/issues/340>.

use std::process::Command;

use rstest::rstest;

/// Crate whose resolved version this workspace constrains.
const CONSTRAINED_CRATE: &str = "tinyvec";

/// First minor release of that crate which does not build here.
const FIRST_BROKEN_MINOR: u64 = 13;

/// Returns the minor version numbers `cargo metadata` resolved for a crate.
///
/// Returns an error rather than panicking so the test reports a resolution
/// failure and a parse failure differently.
fn resolved_minor_versions(name: &str) -> Result<Vec<u64>, String> {
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .map_err(|err| format!("cannot run `cargo metadata`: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("`cargo metadata` failed: {stderr}"));
    }
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|err| format!("cannot parse `cargo metadata` output: {err}"))?;
    let packages = metadata
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "`cargo metadata` reported no packages".to_owned())?;
    packages
        .iter()
        .filter(|package| package.get("name").and_then(serde_json::Value::as_str) == Some(name))
        .map(|package| {
            let version = package
                .get("version")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| format!("`{name}` has no version string"))?;
            version
                .split('.')
                .nth(1)
                .and_then(|minor| minor.parse::<u64>().ok())
                .ok_or_else(|| format!("cannot read a minor version from `{version}`"))
        })
        .collect()
}

#[rstest]
fn the_constrained_crate_resolves_below_its_broken_release() {
    let minors = resolved_minor_versions(CONSTRAINED_CRATE)
        .unwrap_or_else(|err| panic!("dependency resolution must be readable: {err}"));
    assert!(
        !minors.is_empty(),
        "`{CONSTRAINED_CRATE}` must still be in the dependency graph; if it has gone, drop its \
         constraint from Cargo.toml and delete this test"
    );
    let broken: Vec<u64> = minors
        .iter()
        .copied()
        .filter(|minor| *minor >= FIRST_BROKEN_MINOR)
        .collect();
    assert!(
        broken.is_empty(),
        "`{CONSTRAINED_CRATE}` resolved to 1.{broken:?}, which does not build on the pinned \
         nightly; keep the `~1.12` requirement in Cargo.toml until leynos/lille#340 is closed"
    );
}
