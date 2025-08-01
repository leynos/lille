//! Download and verify the Fira Sans font during the build.
//!
//! The build script fetches the font using an implementation of
//! [`FontFetcher`], checks its SHA-256 digest and writes the verified font to
//! disk. This ensures deterministic builds without shipping the font in the
//! repository.
use color_eyre::eyre::{eyre, Result};
use reqwest::blocking::Client;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::NamedTempFile;

/// Fetches the binary contents of the Fira Sans font.
///
/// Implementors are used by [`download_font_with`] to obtain the font data.
///
/// # Errors
/// Implementations should return an error if the font cannot be retrieved.
///
/// # Examples
/// ```rust,no_run
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
/// # Ok::<(), color_eyre::Report>(())
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

/// Download the font bytes over HTTP and verify the checksum.
fn fetch_font_data() -> Result<Vec<u8>> {
    const FONT_URL: &str = "https://raw.githubusercontent.com/mozilla/Fira/fd8c8c0a3d353cd99e8ca1662942d165e6961407/ttf/FiraSans-Regular.ttf";
    const FONT_SHA256: &str = "a389cef71891df1232370fcebd7cfde5f74e741967070399adc91fd069b2094b";
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("lille-build/1.0")
        .build()?;
    let resp = client.get(FONT_URL).send()?.error_for_status()?;
    let bytes = resp.bytes()?;
    let digest = Sha256::digest(&bytes);
    let actual = format!("{digest:x}");
    if actual != FONT_SHA256 {
        return Err(eyre!("font checksum mismatch"));
    }
    Ok(bytes.to_vec())
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
        fs::create_dir_all(&assets_dir).unwrap();
        fs::write(&font_path, b"fake font data").unwrap();
        let mut fetcher = MockFontFetcher::new();
        fetcher.expect_fetch().times(0);
        let result = download_font_with(&fetcher, &manifest_path).unwrap();
        assert_eq!(result, font_path);
        assert!(result.exists());
    }

    #[rstest]
    fn fallback_on_write_error(temp_dir: TempDir) {
        let manifest_path = temp_dir.path().to_path_buf();
        let mut fetcher = MockFontFetcher::new();
        fetcher
            .expect_fetch()
            .returning(|| Err(eyre!("network error")));
        let result = download_font_with(&fetcher, &manifest_path).unwrap();
        assert!(result == fallback_font_path() || result.exists());
    }

    #[rstest]
    fn invalid_manifest_dir() {
        let mut fetcher = MockFontFetcher::new();
        fetcher
            .expect_fetch()
            .returning(|| Err(eyre!("network error")));
        let result = download_font_with(&fetcher, Path::new("/non/existent/path"));
        assert!(result.is_ok());
        let p = result.unwrap();
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
            assert!(h.join().unwrap());
        }
    }

    #[rstest]
    fn fallback_font_path_constant() {
        assert_eq!(
            DEFAULT_FALLBACK_FONT_PATH,
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"
        );
    }
}
