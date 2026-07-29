//! Download and verify the Fira Sans font during the build.
//!
//! The build script fetches the font using an implementation of
//! [`FontFetcher`], checks its SHA-256 digest and writes the verified font to
//! disk. This ensures deterministic builds without shipping the font in the
//! repository.
//!
//! Build scripts require ambient filesystem authority to create asset
//! directories and write files to paths determined by Cargo environment
//! variables. The `cap_std` capability model cannot be applied here because
//! Cargo does not provide directory handles.
use crate::hex::to_lower_hex;
use anyhow::{anyhow, Context, Result};
use reqwest::blocking::Client;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::NamedTempFile;

/// Maximum response body accepted when downloading the font.
///
/// A Fira Sans regular TTF is ~400 KiB, so this leaves generous headroom while
/// bounding what a hostile or misbehaving origin can make the build allocate.
/// The checksum can only be verified once the whole body is buffered, so the
/// cap is the sole defence against unbounded build-time memory use — and the
/// failure would otherwise land on every developer and CI job.
const MAX_FONT_BYTES: u64 = 8 * 1024 * 1024;

/// Fetches the binary contents of the Fira Sans font.
///
/// Implementors are used by [`download_font_with`] to obtain the font data.
///
/// # Errors
/// Implementations should return an error if the font cannot be retrieved.
///
/// # Examples
/// ```rust,no_run
/// use anyhow::Result;
/// use build_support::font::{FontFetcher, download_font_with};
/// struct Dummy;
/// impl FontFetcher for Dummy {
///     fn fetch(&self) -> Result<Vec<u8>> {
///         Ok(Vec::new())
///     }
/// }
/// let _ = download_font_with(&Dummy, std::env::current_dir().unwrap());
/// ```
#[cfg_attr(test, mockall::automock)]
pub trait FontFetcher {
    fn fetch(&self) -> Result<Vec<u8>>;
}

/// Default HTTP implementation of [`FontFetcher`].
struct HttpFontFetcher;

impl FontFetcher for HttpFontFetcher {
    fn fetch(&self) -> Result<Vec<u8>> {
        fetch_font_data()
    }
}

/// Path used when the font download fails.
pub const DEFAULT_FALLBACK_FONT_PATH: &str = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf";

/// Determine a fallback font path for the current platform.
fn fallback_font_path() -> PathBuf {
    if let Ok(p) = std::env::var("FALLBACK_FONT_PATH") {
        return PathBuf::from(p);
    }
    #[cfg(target_os = "macos")]
    {
        return PathBuf::from("/System/Library/Fonts/SFNS.ttf");
    }
    #[cfg(target_os = "windows")]
    {
        return PathBuf::from("C:\\Windows\\Fonts\\arial.ttf");
    }
    PathBuf::from(DEFAULT_FALLBACK_FONT_PATH)
}

/// Ensure the Fira Sans font exists in the `assets` directory.
///
/// # Parameters
/// - `manifest_dir`: Path to the crate's manifest directory.
///
/// # Errors
/// Propagates any download or I/O errors.
///
/// # Examples
/// ```rust,no_run
/// # use std::env;
/// build_support::font::download_font(env::current_dir()?)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn download_font(manifest_dir: impl AsRef<Path>) -> Result<PathBuf> {
    download_font_with(&HttpFontFetcher, manifest_dir)
}

/// Download the font using the supplied [`FontFetcher`].
///
/// # Parameters
/// - `fetcher`: Implementation used to retrieve the font bytes.
/// - `manifest_dir`: Directory containing an `assets` folder.
///
/// # Returns
/// The path to the downloaded font, or a fallback path if fetching or writing
/// the font fails.
///
/// # Errors
/// Propagates I/O errors related to creating directories or writing files.
///
/// # Examples
/// ```rust,no_run
/// # use anyhow::Result;
/// # use build_support::font::{download_font_with, FontFetcher};
/// # struct Dummy;
/// # impl FontFetcher for Dummy {
/// #     fn fetch(&self) -> Result<Vec<u8>> {
/// #         Ok(vec![])
/// #     }
/// # }
/// let path = download_font_with(&Dummy, std::env::current_dir().unwrap()).unwrap();
/// println!("{}", path.display());
/// ```
pub fn download_font_with(
    fetcher: &dyn FontFetcher,
    manifest_dir: impl AsRef<Path>,
) -> Result<PathBuf> {
    let manifest_dir = manifest_dir.as_ref();
    let assets_dir = manifest_dir.join("assets");
    if let Err(e) = fs::create_dir_all(&assets_dir) {
        println!("cargo:warning=Failed to create assets directory: {e}");
        return Ok(fallback_font_path());
    }
    let font_path = assets_dir.join("FiraSans-Regular.ttf");

    if font_path.exists() {
        return Ok(font_path);
    }

    match fetcher.fetch() {
        Ok(data) => {
            let mut tmp = NamedTempFile::new_in(&assets_dir)?;
            if let Err(e) = tmp.write_all(&data) {
                println!("cargo:warning=Failed to write font: {e}");
                return Ok(fallback_font_path());
            }
            if let Err(e) = tmp.persist(&font_path) {
                println!("cargo:warning=Failed to rename font file: {e}");
                return Ok(fallback_font_path());
            }
            Ok(font_path)
        }
        Err(e) => {
            println!("cargo:warning=Font download failed: {e}");
            Ok(fallback_font_path())
        }
    }
}

/// Read at most `max_bytes` from `reader`, rejecting anything larger.
///
/// The reader is capped at `max_bytes + 1` rather than `max_bytes` because a
/// plain cap cannot distinguish a body sitting exactly on the limit from one
/// silently truncated by it. Reading the extra byte makes an oversized body
/// observable, so it is reported instead of quietly hashed as a short font.
fn read_capped(reader: impl Read, max_bytes: u64) -> Result<Vec<u8>> {
    let mut limited = reader.take(max_bytes + 1);
    let mut bytes = Vec::new();
    limited.read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max_bytes {
        return Err(anyhow!("body exceeds the maximum of {max_bytes} bytes"));
    }
    Ok(bytes)
}

/// Download the font bytes over HTTP and verify the checksum.
fn fetch_font_data() -> Result<Vec<u8>> {
    const FONT_URL: &str = "https://raw.githubusercontent.com/mozilla/Fira/fd8c8c0a3d353cd99e8ca1662942d165e6961407/ttf/FiraSans-Regular.ttf";
    const FONT_SHA256: &str = "a389cef71891df1232370fcebd7cfde5f74e741967070399adc91fd069b2094b";
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("lille-build/1.0")
        .build()
        .context("create HTTP client")?;
    let resp = client
        .get(FONT_URL)
        .send()
        .with_context(|| format!("requesting font from {FONT_URL}"))?
        .error_for_status()
        .with_context(|| format!("unexpected HTTP status for {FONT_URL}"))?;
    let bytes = read_capped(resp, MAX_FONT_BYTES)
        .with_context(|| format!("reading response body from {FONT_URL}"))?;
    let actual = to_lower_hex(&Sha256::digest(&bytes));
    if actual != FONT_SHA256 {
        return Err(anyhow!(
            "font checksum mismatch (expected {FONT_SHA256}, got {actual})"
        ));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};
    use std::fs;
    use std::path::Path;
    use std::sync::Arc;
    use std::thread;
    use tempfile::TempDir;

    #[fixture]
    fn temp_dir() -> TempDir {
        TempDir::new().expect("Failed to create temp dir")
    }

    #[rstest]
    fn creates_assets_directory(temp_dir: TempDir) {
        let manifest_path = temp_dir.path().to_path_buf();
        let mut fetcher = MockFontFetcher::new();
        fetcher.expect_fetch().returning(|| Ok(vec![1, 2, 3]));
        let _ = download_font_with(&fetcher, &manifest_path);
        let assets_path = temp_dir.path().join("assets");
        assert!(assets_path.exists());
        assert!(assets_path.is_dir());
    }

    #[rstest]
    fn returns_existing_font_path(temp_dir: TempDir) {
        let manifest_path = temp_dir.path().to_path_buf();
        let assets_dir = temp_dir.path().join("assets");
        let font_path = assets_dir.join("FiraSans-Regular.ttf");
        fs::create_dir_all(&assets_dir).expect("create assets dir");
        fs::write(&font_path, b"fake font data").expect("write fake font");
        let mut fetcher = MockFontFetcher::new();
        fetcher.expect_fetch().times(0);
        let result = download_font_with(&fetcher, &manifest_path).expect("existing font path");
        assert_eq!(result, font_path);
        assert!(result.exists());
    }

    #[rstest]
    fn fallback_on_write_error(temp_dir: TempDir) {
        let manifest_path = temp_dir.path().to_path_buf();
        let mut fetcher = MockFontFetcher::new();
        fetcher
            .expect_fetch()
            .returning(|| Err(anyhow!("network error")));
        let result =
            download_font_with(&fetcher, &manifest_path).expect("fallback path on write error");
        assert!(result == fallback_font_path() || result.exists());
    }

    #[rstest]
    fn invalid_manifest_dir() {
        let mut fetcher = MockFontFetcher::new();
        fetcher
            .expect_fetch()
            .returning(|| Err(anyhow!("network error")));
        let result = download_font_with(&fetcher, Path::new("/non/existent/path"));
        assert!(result.is_ok());
        let p = result.expect("fallback path when manifest dir is invalid");
        assert!(p == fallback_font_path() || p.exists());
    }

    #[rstest]
    fn concurrent_calls(temp_dir: TempDir) {
        let manifest_path = temp_dir.path().to_path_buf();
        let mut fetcher = MockFontFetcher::new();
        fetcher
            .expect_fetch()
            .returning(|| Ok(vec![1, 2, 3]))
            .times(1..=3);
        let fetcher = Arc::new(fetcher);
        let handles: Vec<_> = (0..3)
            .map(|_| {
                let f = Arc::clone(&fetcher);
                let path = manifest_path.clone();
                thread::spawn(move || download_font_with(&*f, &path).is_ok())
            })
            .collect();
        for h in handles {
            assert!(h.join().expect("thread panicked"));
        }
    }

    /// A body at or below the cap is returned intact; the boundary case matters
    /// because the reader is deliberately given one byte of slack.
    #[rstest]
    #[case::empty(0)]
    #[case::below_cap(3)]
    #[case::exactly_at_cap(4)]
    fn read_capped_accepts_bodies_within_the_limit(#[case] len: usize) {
        let body = vec![0xab; len];
        let bytes = read_capped(body.as_slice(), 4).expect("body within the cap is accepted");
        assert_eq!(bytes, body);
    }

    #[rstest]
    fn read_capped_rejects_oversized_body() {
        let body = vec![0xab; 5];
        let error = read_capped(body.as_slice(), 4).expect_err("oversized body is rejected");
        assert!(
            error.to_string().contains("exceeds the maximum of 4 bytes"),
            "unexpected error: {error}"
        );
    }

    #[rstest]
    fn fallback_font_path_constant() {
        assert_eq!(
            DEFAULT_FALLBACK_FONT_PATH,
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"
        );
    }
}
