use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use toml::Value;

const FALLBACK_FONT_PATH: &str = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf";

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets");
    println!("cargo:rerun-if-changed=src/lille.dl");
    println!("cargo:rerun-if-changed=constants.toml");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);

    generate_constants(&manifest_dir, &out_dir)?;
    let font_path = download_font(&manifest_dir)?;
    compile_ddlog(&manifest_dir, &out_dir)?;

    println!("cargo:rustc-env=FONT_PATH={}", font_path.display());

    Ok(())
}

fn generate_constants(manifest_dir: &str, out_dir: &Path) -> Result<(), Box<dyn Error>> {
    let parsed = parse_constants(manifest_dir)?;
    fs::write(out_dir.join("constants.rs"), generate_rs_constants(&parsed))?;
    fs::write(
        Path::new(manifest_dir).join("src/constants.dl"),
        generate_dl_constants(&parsed),
    )?;
    Ok(())
}

fn parse_constants(manifest_dir: &str) -> Result<Value, Box<dyn Error>> {
    let const_path = Path::new(manifest_dir).join("constants.toml");
    let toml_str = fs::read_to_string(const_path)?;
    Ok(toml_str.parse()?)
}

fn for_each_constant<F>(parsed: &Value, mut f: F)
where
    F: FnMut(&str, &Value),
{
    if let Some(map) = parsed.as_table() {
        for table in map.values() {
            if let Some(tab) = table.as_table() {
                for (k, v) in tab {
                    f(k, v);
                }
            }
        }
    }
}

fn generate_rs_constants(parsed: &Value) -> String {
    generate_code_from_constants(parsed, |k, v| {
        let name = k.to_uppercase();
        match v {
            Value::Integer(i) => format!("pub const {}: i64 = {};\n", name, i),
            Value::Float(f) => format!("pub const {}: f32 = {}f32;\n", name, f),
            Value::String(s) => format!("pub const {}: &str = \"{}\";\n", name, s),
            _ => String::new(),
        }
    })
}

fn generate_dl_constants(parsed: &Value) -> String {
    generate_code_from_constants(parsed, |k, v| {
        let name = k.to_uppercase();
        match v {
            Value::Integer(i) => format!("const {}: signed<64> = {}\n", name, i),
            Value::Float(f) => format!("const {}: GCoord = {}\n", name, f),
            Value::String(s) => format!("const {}: string = \"{}\"\n", name, s),
            _ => String::new(),
        }
    })
}

fn generate_code_from_constants<F>(parsed: &Value, mut emit: F) -> String
where
    F: FnMut(&str, &Value) -> String,
{
    let mut code = String::from("// @generated - do not edit\n");
    for_each_constant(parsed, |k, v| {
        code += &emit(k, v);
    });
    code
}

fn download_font(manifest_dir: &str) -> Result<PathBuf, Box<dyn Error>> {
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

fn compile_ddlog(manifest_dir: &str, out_dir: &Path) -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();
    if !ddlog_available() {
        return Ok(());
    }

    let ddlog_file = Path::new(manifest_dir).join("src/lille.dl");
    if !ddlog_file.exists() {
        println!("cargo:warning=src/lille.dl missing; skipping ddlog compilation");
        return Ok(());
    }

    run_ddlog(&ddlog_file, out_dir)
}

fn ddlog_available() -> bool {
    match Command::new("ddlog")
        .arg("--version")
        .stdout(Stdio::null())
        .status()
    {
        Ok(status) if status.success() => true,
        Ok(status) => {
            println!(
                "cargo:warning=ddlog --version failed with status {}",
                status
            );
            false
        }
        Err(e) => {
            println!("cargo:warning=failed to run ddlog --version: {}", e);
            false
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
        .arg(ddlog_file)
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
