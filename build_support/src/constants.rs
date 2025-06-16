use std::error::Error;
use std::fs;
use std::path::Path;

use toml::Value;

pub struct Formats {
    pub int_fmt: &'static str,
    pub float_fmt: &'static str,
    pub str_fmt: &'static str,
}

pub const RUST_FMTS: Formats = Formats {
    int_fmt: "pub const {}: i64 = {};\n",
    float_fmt: "pub const {}: f64 = {};\n",
    str_fmt: "pub const {}: &str = \"{}\";\n",
};

pub const DL_FMTS: Formats = Formats {
    int_fmt: "const {}: signed<64> = {}\n",
    float_fmt: "const {}: GCoord = {}\n",
    str_fmt: "const {}: string = \"{}\"\n",
};

pub fn generate_constants(
    manifest_dir: impl AsRef<Path>,
    out_dir: impl AsRef<Path>,
) -> Result<(), Box<dyn Error>> {
    let manifest_dir = manifest_dir.as_ref();
    let out_dir = out_dir.as_ref();
    let parsed = parse_constants(manifest_dir)?;
    fs::write(
        out_dir.join("constants.rs"),
        generate_code_from_constants(&parsed, &RUST_FMTS),
    )?;
    fs::write(
        manifest_dir.join("src/constants.dl"),
        generate_code_from_constants(&parsed, &DL_FMTS),
    )?;
    Ok(())
}

pub fn parse_constants(manifest_dir: impl AsRef<Path>) -> Result<Value, Box<dyn Error>> {
    let const_path = manifest_dir.as_ref().join("constants.toml");
    let toml_str = fs::read_to_string(const_path)?;
    Ok(toml_str.parse()?)
}

fn for_each_constant<F>(parsed: &Value, mut f: F)
where
    F: FnMut(&str, &Value),
{
    if let Some(map) = parsed.as_table() {
        for (k, v) in map {
            if let Some(tab) = v.as_table() {
                for (subk, subv) in tab {
                    f(subk, subv);
                }
            } else {
                f(k, v);
            }
        }
    }
}

fn fill2(fmt: &str, a: impl std::fmt::Display, b: impl std::fmt::Display) -> String {
    fmt.replacen("{}", &a.to_string(), 1)
        .replacen("{}", &b.to_string(), 1)
}

pub fn generate_code_from_constants(parsed: &Value, fmts: &Formats) -> String {
    let mut code = String::from("// @generated - do not edit\n");
    for_each_constant(parsed, |k, v| {
        let name = k.to_uppercase();
        match v {
            Value::Integer(i) => code.push_str(&fill2(fmts.int_fmt, name, i)),
            Value::Float(f) => code.push_str(&fill2(fmts.float_fmt, name, format!("{f:?}"))),
            Value::String(s) => code.push_str(&fill2(fmts.str_fmt, name, s)),
            other => {
                println!(
                    "cargo:warning=Unsupported constant `{}` of type {:?}",
                    name, other
                );
            }
        }
    });
    code
}
