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
    str_fmt: "pub const {}: &str = {};\n",
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
    let src_dir = manifest_dir.join("src");
    fs::create_dir_all(&src_dir)?;
    fs::write(
        src_dir.join("constants.dl"),
        generate_code_from_constants(&parsed, &DL_FMTS),
    )?;
    Ok(())
}

pub fn parse_constants(manifest_dir: impl AsRef<Path>) -> Result<Value, Box<dyn Error>> {
    let const_path = manifest_dir.as_ref().join("constants.toml");
    let toml_str = fs::read_to_string(const_path)?;
    Ok(toml_str.parse()?)
}

fn for_each_constant<F>(parsed: &Value, f: &mut F)
where
    F: FnMut(&str, &Value),
{
    if let Some(map) = parsed.as_table() {
        let mut entries: Vec<_> = map.iter().collect();
        entries.sort_by_key(|(k, _)| *k);
        for (k, v) in entries {
            if v.is_table() {
                for_each_constant(v, f);
            } else {
                f(k, v);
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
    let mut append = |k: &str, v: &Value| {
        let name = k.to_uppercase();
        match v {
            Value::Integer(i) => code.push_str(&fill2(fmts.int_fmt, name, i)),
            Value::Float(f) => {
                let mut val = f.to_string();
                if !val.contains('.') && !val.contains('e') && !val.contains('E') {
                    val.push_str(".0");
                }
                code.push_str(&fill2(fmts.float_fmt, name, val));
            }
            Value::String(s) => {
                let lit = format!("{:?}", s);
                code.push_str(&fill2(fmts.str_fmt, name, lit));
            }
            other => {
                println!(
                    "cargo:warning=Unsupported constant `{}` of type {:?}",
                    name, other
                );
            }
        }
    };
    for_each_constant(parsed, &mut append);
    code
}
