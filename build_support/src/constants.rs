//! Generate Rust and DDlog constant definitions from `constants.toml`.
//!
//! This module is invoked from the build script to read `constants.toml` and
//! output Rust and Differential Datalog source files. It keeps the two
//! languages in sync so both parts of the project share the same numerical and
//! string constants.
use color_eyre::eyre::{eyre, Context, Result};
use jsonschema::{validator_for, Validator};
use once_cell::sync::OnceCell;
use serde_json::Value as JsonValue;
use std::fs;
use std::path::Path;

use toml::Value;

/// Cached validator for constants schema
static SCHEMA_VALIDATOR: OnceCell<Validator> = OnceCell::new();

/// Format strings used when generating code.
///
/// Each field contains a template with two `{}` placeholders that will be
/// substituted with the constant name and its value.
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum FormatFlavor {
    Rust,
    Ddlog,
}

pub struct Formats {
    /// Indicates the target language these format strings generate.
    pub flavor: FormatFlavor,
    /// Format used for integer constants.
    pub int_fmt: &'static str,
    /// Format used for floating point constants.
    pub float_fmt: &'static str,
    /// Format used for string constants.
    pub str_fmt: &'static str,
}

/// Default format templates for generating Rust code.
pub const RUST_FMTS: Formats = Formats {
    flavor: FormatFlavor::Rust,
    int_fmt: "pub const {}: i64 = {};\n",
    float_fmt: "pub const {}: f64 = {};\n",
    str_fmt: "pub const {}: &str = {};\n",
};

/// Default format templates for generating DDlog code.
pub const DL_FMTS: Formats = Formats {
    flavor: FormatFlavor::Ddlog,
    int_fmt: "function {}(): signed<64> { {} }\n",
    float_fmt: "function {}(): GCoord { {} }\n",
    str_fmt: "function {}(): string { {} }\n",
};

/// Generate Rust and DDlog constant files from `constants.toml`.
///
/// # Parameters
/// - `manifest_dir`: Directory containing `constants.toml`.
/// - `out_dir`: Directory where the generated Rust file will be written.
///
/// # Errors
/// Propagates I/O or TOML parsing errors encountered while reading or writing
/// files.
///
/// # Examples
/// ```rust,no_run
/// use build_support::constants::generate_constants;
/// # use std::path::Path;
/// let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
/// let out = Path::new(env!("OUT_DIR"));
/// generate_constants(manifest, out).unwrap();
/// ```
pub fn generate_constants(manifest_dir: impl AsRef<Path>, out_dir: impl AsRef<Path>) -> Result<()> {
    let manifest_dir = manifest_dir.as_ref();
    let out_dir = out_dir.as_ref();
    let parsed = parse_constants(manifest_dir)?;
    fs::write(
        out_dir.join("constants.rs"),
        generate_code_from_constants(&parsed, &RUST_FMTS),
    )?;
    // Write the DDlog constants next to the other `.dl` modules so the
    // compiler's import resolution can locate them without additional flags.
    let ddlog_dir = manifest_dir.join("src/ddlog");
    fs::create_dir_all(&ddlog_dir)?;
    fs::write(
        ddlog_dir.join("constants.dl"),
        generate_code_from_constants(&parsed, &DL_FMTS),
    )?;
    Ok(())
}

/// Parse the `constants.toml` file into a [`toml::Value`].
///
/// # Parameters
/// - `manifest_dir`: Path where `constants.toml` resides.
///
/// # Errors
/// Returns any error produced when reading or parsing the TOML file.
///
/// # Examples
/// ```rust,no_run
/// use build_support::constants::parse_constants;
/// # use std::path::Path;
/// let value = parse_constants(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap();
/// assert!(value.is_table());
/// ```
pub fn parse_constants(manifest_dir: impl AsRef<Path>) -> Result<Value> {
    let manifest_dir = manifest_dir.as_ref();
    let const_path = manifest_dir.join("constants.toml");
    let toml_str = fs::read_to_string(&const_path)
        .with_context(|| format!("Failed to read constants file at {}", const_path.display()))?;
    let parsed: Value = toml_str
        .parse()
        .with_context(|| format!("Failed to parse TOML from {}", const_path.display()))?;
    validate_constants(manifest_dir, &parsed).with_context(|| {
        format!(
            "Schema validation failed for constants at {}",
            const_path.display()
        )
    })?;
    Ok(parsed)
}

/// Validate `constants.toml` against `constants.schema.json`.
fn validate_constants(dir: &Path, data: &Value) -> Result<()> {
    let schema = load_schema(dir)?;
    let instance = serde_json::to_value(data)?;
    schema
        .validate(&instance)
        .map(|_| ())
        .map_err(|error| eyre!(error.to_string()))
        .wrap_err("constants.toml schema validation failed")
}

fn load_schema(dir: &Path) -> Result<&'static Validator> {
    SCHEMA_VALIDATOR.get_or_try_init(|| {
        let schema_path = dir.join("constants.schema.json");
        let schema_str = fs::read_to_string(&schema_path)
            .with_context(|| format!("Failed to read schema file at {}", schema_path.display()))?;
        let schema_json: JsonValue = serde_json::from_str(&schema_str).with_context(|| {
            format!("Failed to parse JSON schema from {}", schema_path.display())
        })?;
        validator_for(&schema_json)
            .with_context(|| format!("Failed to compile JSON schema at {}", schema_path.display()))
    })
}

/// Traverse all scalar constants in the parsed TOML value.
///
/// The provided closure is called with each key/value pair in sorted order.
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

/// Replace two `{}` markers in `fmt` with `a` and `b`.
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

/// Determine if a numeric string is a plain integer literal.
///
/// A plain integer literal contains neither a decimal point nor an exponent.
///
/// # Parameters
/// - `s`: The numeric literal to inspect.
///
/// # Returns
/// `true` if `s` lacks a `.` and does not include `e` or `E`.
///
/// # Examples
/// ```rust,no_run
/// assert!(is_plain_integer_literal("42"));
/// assert!(!is_plain_integer_literal("3.14"));
/// assert!(!is_plain_integer_literal("1e5"));
/// ```
fn is_plain_integer_literal(s: &str) -> bool {
    !s.contains('.') && !s.contains('e') && !s.contains('E')
}

/// Convert parsed constants into source code using the given formats.
///
/// # Parameters
/// - `parsed`: The TOML data returned by [`parse_constants`].
/// - `fmts`: Formatting strings describing how to emit each value type.
///
/// # Returns
/// A string containing the generated source code.
///
/// # Examples
/// ```rust,no_run
/// # use toml::Value;
/// # use build_support::constants::{generate_code_from_constants, RUST_FMTS};
/// let data: Value = "answer = 42".parse().unwrap();
/// let src = generate_code_from_constants(&data, &RUST_FMTS);
/// assert!(src.contains("ANSWER"));
/// ```
pub fn generate_code_from_constants(parsed: &Value, fmts: &Formats) -> String {
    let mut code = String::from("// @generated - do not edit\n");
    if matches!(fmts.flavor, FormatFlavor::Ddlog) {
        code.push_str("import types\n\n");
    }
    let mut append = |k: &str, v: &Value| {
        // Always emit UPPER_CASE constant names regardless of target language.
        // DDlog typically uses lower_snake case for variables, but our
        // generated constants are exported functions so casing is flexible.
        let name = k.to_uppercase();
        match v {
            Value::Integer(i) => code.push_str(&fill2(fmts.int_fmt, name, i)),
            Value::Float(f) => {
                let mut val = f.to_string();
                if f.is_finite() && is_plain_integer_literal(&val) {
                    val.push_str(".0");
                }
                code.push_str(&fill2(fmts.float_fmt, name, val));
            }
            Value::String(s) => {
                let lit = format!("{s:?}");
                code.push_str(&fill2(fmts.str_fmt, name, lit));
            }
            other => {
                println!("cargo:warning=Unsupported constant `{name}` of type {other:?}");
            }
        }
    };
    for_each_constant(parsed, &mut append);
    code
}

#[cfg(test)]
mod tests {
    use super::is_plain_integer_literal;
    use super::parse_constants;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn identifies_plain_integers() {
        assert!(is_plain_integer_literal("0"));
        assert!(is_plain_integer_literal("42"));
    }

    #[test]
    fn rejects_non_plain_integers() {
        assert!(!is_plain_integer_literal("1.0"));
        assert!(!is_plain_integer_literal("2e10"));
        assert!(!is_plain_integer_literal("3E5"));
        assert!(!is_plain_integer_literal("inf"));
        assert!(!is_plain_integer_literal("NaN"));
    }

    #[test]
    fn parse_constants_validates_schema() {
        let dir = TempDir::new().expect("failed to create temp dir");
        fs::write(dir.path().join("constants.toml"), "[physics]\nvalue = 1\n")
            .expect("unable to write constants.toml");
        fs::write(
            dir.path().join("constants.schema.json"),
            r#"{
                "type": "object",
                "properties": {
                    "physics": {
                        "type": "object",
                        "required": ["other"]
                    }
                }
            }"#,
        )
        .expect("unable to write constants.schema.json");
        assert!(parse_constants(dir.path()).is_err());
    }

    #[test]
    fn parse_constants_succeeds_with_valid_schema() {
        let dir = TempDir::new().expect("failed to create temp dir");
        fs::write(dir.path().join("constants.toml"), "[physics]\nvalue = 1\n")
            .expect("unable to write constants.toml");
        fs::write(
            dir.path().join("constants.schema.json"),
            r#"{
                "type": "object",
                "required": ["physics"],
                "properties": {
                    "physics": {
                        "type": "object",
                        "required": ["value"],
                        "properties": {
                            "value": {"type": "integer"}
                        }
                    }
                }
            }"#,
        )
        .expect("unable to write constants.schema.json");
        assert!(parse_constants(dir.path()).is_ok());
    }
}
