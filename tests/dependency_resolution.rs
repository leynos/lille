//! Guards the dependency-resolution constraints this workspace relies on.
//!
//! `tinyvec` reaches the build transitively through Bevy's text and font
//! stack, and no `Cargo.lock` is committed, so resolution is decided afresh on
//! every machine. This test reads what Cargo resolved and names the version it
//! chose.
//!
//! It is a post-resolution check, not a pre-build gate. Cargo compiles the
//! dependency graph before an integration test runs, so when a broken release
//! is selected the build fails first, deep inside the crate, with
//! ``cannot find macro `vec` in this scope``. This test then reports the
//! selected version on the next run, once the constraint is back. What it
//! catches directly is the case that would otherwise pass silently: a widened
//! requirement whose resolved version still compiles but is outside the range
//! this workspace has verified.
//!
//! Remove it with the constraint itself, per
//! <https://github.com/leynos/lille/issues/340>.

use cap_std::{ambient_authority, fs_utf8::Dir};
use rstest::rstest;

/// Crate whose resolved version this workspace constrains.
const CONSTRAINED_CRATE: &str = "tinyvec";

/// First release of that crate which does not build here, as major and minor.
const FIRST_BROKEN_RELEASE: (u64, u64) = (1, 13);

/// A resolved semantic version, kept whole so failures can name it.
#[derive(Debug, Clone)]
struct ResolvedVersion {
    /// Version exactly as Cargo reported it.
    text: String,
    /// Major and minor components, for ordering against a known-bad release.
    series: (u64, u64),
}

impl ResolvedVersion {
    /// Parses a Cargo version string, keeping the original text.
    fn parse(text: &str) -> Result<Self, String> {
        let mut parts = text.split(['.', '-', '+']);
        let mut component = |name: &str| -> Result<u64, String> {
            parts
                .next()
                .and_then(|part| part.parse::<u64>().ok())
                .ok_or_else(|| format!("cannot read the {name} version from `{text}`"))
        };
        let major = component("major")?;
        let minor = component("minor")?;
        Ok(Self {
            text: text.to_owned(),
            series: (major, minor),
        })
    }

    /// Reports whether this release is the broken one or anything later.
    const fn is_broken_or_later(&self) -> bool {
        self.series.0 > FIRST_BROKEN_RELEASE.0
            || (self.series.0 == FIRST_BROKEN_RELEASE.0 && self.series.1 >= FIRST_BROKEN_RELEASE.1)
    }
}

/// Returns the quoted value of a `key = "value"` line of `Cargo.lock`, if the
/// line is that key.
fn lock_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.strip_prefix(key)?
        .strip_prefix(" = \"")?
        .strip_suffix('"')
}

/// Returns every version of a crate that the lockfile records.
///
/// The build that compiled this test has already resolved and written
/// `Cargo.lock`, which is exactly what Cargo resolved, for every target and
/// without any network or child process. Reading it directly replaces a
/// `cargo metadata` spawn, which would need every target's packages cached to
/// stay offline.
///
/// Returns an error rather than panicking so the test reports a missing lock
/// and a parse failure differently.
fn resolved_versions(name: &str) -> Result<Vec<ResolvedVersion>, String> {
    // The one ambient step: the lock is read through a capability on the
    // workspace root.
    let root = Dir::open_ambient_dir(env!("CARGO_MANIFEST_DIR"), ambient_authority())
        .map_err(|err| format!("cannot open the workspace root: {err}"))?;
    let lock = root
        .read_to_string("Cargo.lock")
        .map_err(|err| format!("cannot read `Cargo.lock`: {err}"))?;
    let mut current_name: Option<&str> = None;
    let mut versions = Vec::new();
    for line in lock.lines() {
        if line == "[[package]]" {
            current_name = None;
        } else if let Some(package) = lock_value(line, "name") {
            current_name = Some(package);
        } else if let Some(version) = lock_value(line, "version") {
            if current_name == Some(name) {
                versions.push(ResolvedVersion::parse(version)?);
            }
        }
    }
    Ok(versions)
}

#[rstest]
fn the_constrained_crate_resolves_below_its_broken_release() {
    let resolved = resolved_versions(CONSTRAINED_CRATE)
        .unwrap_or_else(|err| panic!("dependency resolution must be readable: {err}"));
    assert!(
        !resolved.is_empty(),
        "`{CONSTRAINED_CRATE}` must still be in the dependency graph; if it has gone, drop its \
         constraint from Cargo.toml and delete this test"
    );
    let broken: Vec<&str> = resolved
        .iter()
        .filter(|version| version.is_broken_or_later())
        .map(|version| version.text.as_str())
        .collect();
    let (major, minor) = FIRST_BROKEN_RELEASE;
    assert!(
        broken.is_empty(),
        "`{CONSTRAINED_CRATE}` resolved to {broken:?}, at or beyond {major}.{minor}, which does \
         not build on the pinned nightly; keep the `~1.12` requirement in Cargo.toml until \
         leynos/lille#340 is closed"
    );
}
