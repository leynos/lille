use std::env;
use std::error::Error;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn compile_ddlog(manifest_dir: &str, out_dir: &Path) -> Result<(), Box<dyn Error>> {
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
            println!("cargo:warning=ddlog --version failed with status {}", status);
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
        println!("cargo:warning=ddlog compiler exited with status: {}", status);
    }
    Ok(())
}
