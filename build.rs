use std::env;
use std::fs;
use std::path::Path;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets");
    
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
                println!("cargo:rustc-env=FONT_PATH=/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf");
                return Ok(());
            }
        }
    }

    // Make the font path available to the main program
    println!("cargo:rustc-env=FONT_PATH={}", font_path.display());

    Ok(())
}