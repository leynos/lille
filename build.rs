use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets");
    println!("cargo:rerun-if-changed=src/lille.dl");

    // Get manifest directory
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;

    // Create assets directory in the project root
    let assets_dir = Path::new(&manifest_dir).join("assets");
    match fs::create_dir_all(&assets_dir) {
        Ok(_) => (),
        Err(e) => println!("cargo:warning=Failed to create assets directory: {}", e),
    }

    // Font file path
    let font_path = assets_dir.join("FiraSans-Regular.ttf");

    // Only download if the font doesn't exist
    if !font_path.exists() {
        // Using Mozilla's Fira Sans font
        let font_url = "https://github.com/mozilla/Fira/raw/master/ttf/FiraSans-Regular.ttf";

        match reqwest::blocking::get(font_url) {
            Ok(response) => {
                match response.bytes() {
                    Ok(font_data) => {
                        if let Err(e) = fs::write(&font_path, font_data) {
                            println!("cargo:warning=Failed to write font file: {}", e);
                            println!("cargo:rustc-env=FONT_PATH=/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf");
                            return Ok(());
                        }
                    }
                    Err(e) => {
                        println!("cargo:warning=Failed to get font data: {}", e);
                        println!("cargo:rustc-env=FONT_PATH=/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf");
                        return Ok(());
                    }
                }
            }
            Err(e) => {
                println!("cargo:warning=Failed to download font: {}", e);
                println!(
                    "cargo:rustc-env=FONT_PATH=/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"
                );
                return Ok(());
            }
        }
    }

    // Make the font path available to the main program
    println!("cargo:rustc-env=FONT_PATH={}", font_path.display());

    // Compile DDlog program if `ddlog` executable is available
    if Command::new("ddlog")
        .arg("--version")
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        let ddlog_file = Path::new(&manifest_dir).join("src/lille.dl");
        if ddlog_file.exists() {
            let status = Command::new("ddlog")
                .arg(ddlog_file.to_str().unwrap())
                .arg("-o")
                .arg("ddlog_lille")
                .status();
            match status {
                Err(e) => {
                    println!("cargo:warning=Failed to run ddlog compiler: {}", e);
                }
                Ok(exit_status) => {
                    if !exit_status.success() {
                        println!(
                            "cargo:warning=ddlog compiler exited with status: {}",
                            exit_status
                        );
                    }
                }
            }
        }
    } else {
        println!("cargo:warning=ddlog compiler not found; skipping ddlog generation");
    }

    Ok(())
}
