use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const FALLBACK_FONT_PATH: &str = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf";

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets");
    println!("cargo:rerun-if-changed=src/lille.dl");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);

    let font_path = download_font(&manifest_dir)?;
    compile_ddlog(&manifest_dir, &out_dir)?;

    println!("cargo:rustc-env=FONT_PATH={}", font_path.display());

    Ok(())
}

fn download_font(manifest_dir: &str) -> Result<PathBuf, Box<dyn Error>> {
    let assets_dir = Path::new(manifest_dir).join("assets");
    fs::create_dir_all(&assets_dir)?;
    let font_path = assets_dir.join("FiraSans-Regular.ttf");

    if font_path.exists() {
        return Ok(font_path);
    }

    let font_url = "https://github.com/mozilla/Fira/raw/master/ttf/FiraSans-Regular.ttf";

    let data = match reqwest::blocking::get(font_url).and_then(|resp| resp.error_for_status()) {
        Ok(resp) => match resp.bytes() {
            Ok(bytes) => bytes,
            Err(e) => {
                println!("cargo:warning=Failed to read downloaded font: {}", e);
                return Ok(PathBuf::from(FALLBACK_FONT_PATH));
            }
        },
        Err(e) => {
            println!("cargo:warning=Failed to download font: {}", e);
            return Ok(PathBuf::from(FALLBACK_FONT_PATH));
        }
    };

    if let Err(e) = fs::write(&font_path, data) {
        println!("cargo:warning=Failed to write font: {}", e);
        return Ok(PathBuf::from(FALLBACK_FONT_PATH));
    }

    Ok(font_path)
}

fn compile_ddlog(manifest_dir: &str, out_dir: &Path) -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();

    if !ddlog_available()? {
        return Ok(());
    }

    let ddlog_file = Path::new(manifest_dir).join("src/lille.dl");
    if !ddlog_file.exists() {
        println!("cargo:warning=src/lille.dl missing; skipping ddlog compilation");
        return Ok(());
    }

    run_ddlog(&ddlog_file, out_dir)
}

fn ddlog_available() -> Result<bool, Box<dyn Error>> {
    let version = Command::new("ddlog")
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match version {
        Ok(output) if output.status.success() => {
            println!(
                "Found ddlog: {}",
                String::from_utf8_lossy(&output.stdout).trim()
            );
            Ok(true)
        }
        // Use stdout so Cargo captures these warnings correctly
        Ok(output) => {
            println!(
                "cargo:warning=ddlog --version failed with status {}",
                output.status
            );
            println!(
                "cargo:warning=stderr: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
            Ok(false)
        }
        Err(e) => {
            // Also print to stdout for Cargo's warning detection
            println!("cargo:warning=failed to run ddlog --version: {}", e);
            Ok(false)
        }
    }
}

fn run_ddlog(ddlog_file: &Path, out_dir: &Path) -> Result<(), Box<dyn Error>> {
    let target_dir = out_dir.join("ddlog_lille");
    let mut cmd = Command::new("ddlog");

    if let Ok(home) = env::var("DDLOG_HOME") {
        cmd.arg("-L").arg(format!("{}/lib", home));
    }

    let status = cmd
        .arg("-i")
        .arg(ddlog_file.to_string_lossy().to_string())
        .arg("-o")
        .arg(&target_dir)
        .status()?;

    if !status.success() {
        println!(
            "cargo:warning=ddlog compiler exited with status: {}",
            status
        );
    }

    Ok(())
}
