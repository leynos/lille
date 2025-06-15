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
    float_fmt: "pub const {}: f32 = {}f32;\n",
    str_fmt: "pub const {}: &str = \"{}\";\n",
};

pub const DL_FMTS: Formats = Formats {
    int_fmt: "const {}: signed<64> = {}\n",
    float_fmt: "const {}: GCoord = {}\n",
    str_fmt: "const {}: string = \"{}\"\n",
};

pub fn generate_constants(manifest_dir: &str, out_dir: &Path) -> Result<(), Box<dyn Error>> {
    let parsed = parse_constants(manifest_dir)?;
    fs::write(
        out_dir.join("constants.rs"),
        generate_code_from_constants(&parsed, &RUST_FMTS),
    )?;
    fs::write(
        Path::new(manifest_dir).join("src/constants.dl"),
        generate_code_from_constants(&parsed, &DL_FMTS),
    )?;
    Ok(())
}

pub fn parse_constants(manifest_dir: &str) -> Result<Value, Box<dyn Error>> {
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

fn fill2(fmt: &str, a: impl std::fmt::Display, b: impl std::fmt::Display) -> String {
    let mut parts = fmt.splitn(3, "{}");
    let mut s = String::new();
    s.push_str(parts.next().unwrap_or(""));
    s.push_str(&a.to_string());
    s.push_str(parts.next().unwrap_or(""));
    s.push_str(&b.to_string());
    s.push_str(parts.next().unwrap_or(""));
    s
}

pub fn generate_code_from_constants(parsed: &Value, fmts: &Formats) -> String {
    let mut code = String::from("// @generated - do not edit\n");
    for_each_constant(parsed, |k, v| {
        let name = k.to_uppercase();
        match v {
            Value::Integer(i) => code.push_str(&fill2(fmts.int_fmt, name, i)),
            Value::Float(f) => code.push_str(&fill2(fmts.float_fmt, name, f)),
            Value::String(s) => code.push_str(&fill2(fmts.str_fmt, name, s)),
            _ => (),
        }
    });
    code
}
