use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

const FALLBACK_FONT_PATH: &str = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf";

pub fn download_font(manifest_dir: &str) -> Result<PathBuf, Box<dyn Error>> {
    let assets_dir = Path::new(manifest_dir).join("assets");
    fs::create_dir_all(&assets_dir)?;
    let font_path = assets_dir.join("FiraSans-Regular.ttf");

    if font_path.exists() {
        return Ok(font_path);
    }

    match fetch_font_data() {
        Ok(data) => {
            if let Err(e) = fs::write(&font_path, data) {
                println!("cargo:warning=Failed to write font: {}", e);
                return Ok(PathBuf::from(FALLBACK_FONT_PATH));
            }
            Ok(font_path)
        }
        Err(e) => {
            println!("cargo:warning=Font download failed: {}", e);
            Ok(PathBuf::from(FALLBACK_FONT_PATH))
        }
    }
}

fn fetch_font_data() -> Result<Vec<u8>, Box<dyn Error>> {
    const FONT_URL: &str = "https://github.com/mozilla/Fira/raw/master/ttf/FiraSans-Regular.ttf";
    let resp = reqwest::blocking::get(FONT_URL)?.error_for_status()?;
    Ok(resp.bytes()?.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn setup_test_dir() -> TempDir {
        TempDir::new().expect("Failed to create temp dir")
    }

    #[test]
    fn test_download_font_creates_assets_directory() {
        let temp_dir = setup_test_dir();
        let manifest_path = temp_dir.path().to_str().unwrap();
        let _ = download_font(manifest_path);
        let assets_path = temp_dir.path().join("assets");
        assert!(assets_path.exists(), "Assets directory should be created");
        assert!(assets_path.is_dir(), "Assets path should be a directory");
    }

    #[test]
    fn test_download_font_returns_existing_font_path() {
        let temp_dir = setup_test_dir();
        let manifest_path = temp_dir.path().to_str().unwrap();
        let assets_dir = temp_dir.path().join("assets");
        let font_path = assets_dir.join("FiraSans-Regular.ttf");
        fs::create_dir_all(&assets_dir).unwrap();
        fs::write(&font_path, b"fake font data").unwrap();
        let result = download_font(manifest_path).unwrap();
        assert_eq!(result, font_path);
        assert!(result.exists());
    }

    #[test]
    fn test_download_font_fallback_on_write_error() {
        let temp_dir = setup_test_dir();
        let manifest_path = temp_dir.path().to_str().unwrap();
        let assets_dir = temp_dir.path().join("assets");
        fs::create_dir_all(&assets_dir).unwrap();
        let mut perms = fs::metadata(&assets_dir).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&assets_dir, perms).unwrap();
        let result = download_font(manifest_path).unwrap();
        assert!(result == PathBuf::from(FALLBACK_FONT_PATH) || result.exists());
    }

    #[test]
    fn test_download_font_invalid_manifest_dir() {
        let result = download_font("/non/existent/path");
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path == PathBuf::from(FALLBACK_FONT_PATH) || path.ends_with("FiraSans-Regular.ttf"));
    }

    #[test]
    fn test_download_font_empty_manifest_dir() {
        let result = download_font("");
        assert!(result.is_ok());
    }

    #[test]
    fn test_download_font_relative_path() {
        let temp_dir = setup_test_dir();
        let manifest_path = temp_dir.path().to_str().unwrap();
        let result = download_font(manifest_path);
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.exists() || path == PathBuf::from(FALLBACK_FONT_PATH));
    }

    #[test]
    fn test_fallback_font_path_constant() {
        assert_eq!(FALLBACK_FONT_PATH, "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf");
        assert!(!FALLBACK_FONT_PATH.is_empty());
    }

    #[test]
    fn test_assets_directory_structure() {
        let temp_dir = setup_test_dir();
        let manifest_path = temp_dir.path().to_str().unwrap();
        let _ = download_font(manifest_path);
        let assets_path = temp_dir.path().join("assets");
        let expected_font_path = assets_path.join("FiraSans-Regular.ttf");
        assert!(assets_path.exists());
        assert_eq!(expected_font_path.parent().unwrap(), assets_path);
    }

    #[test]
    fn test_download_font_creates_nested_directories() {
        let temp_dir = setup_test_dir();
        let nested_manifest = temp_dir.path().join("deeply").join("nested").join("manifest");
        fs::create_dir_all(&nested_manifest).unwrap();
        let manifest_path = nested_manifest.to_str().unwrap();
        let result = download_font(manifest_path);
        assert!(result.is_ok());
        let assets_path = nested_manifest.join("assets");
        assert!(assets_path.exists());
    }

    #[test]
    fn test_download_font_concurrent_calls() {
        use std::thread;
        use std::sync::Arc;
        let temp_dir = Arc::new(setup_test_dir());
        let manifest_path = temp_dir.path().to_str().unwrap().to_string();
        let handles: Vec<_> = (0..3)
            .map(|_| {
                let path = manifest_path.clone();
                thread::spawn(move || download_font(&path).is_ok())
            })
            .collect();
        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        for result in results {
            assert!(result);
        }
    }
}
