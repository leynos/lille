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
