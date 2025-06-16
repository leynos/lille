use build_support::constants::{generate_constants, generate_code_from_constants, parse_constants, DL_FMTS, RUST_FMTS, Formats};
use std::{fs, path::PathBuf};
use tempfile::TempDir;
use toml::Value;
use std::str::FromStr;

// Keep the original snapshot tests

#[test]
fn generate_rust_constants() {
    let toml_content = r#"
[movement]
friction_coefficient = 0.7
max_speed = 5.0
gravity = 9.8

[physics]
default_mass = 70
grace_distance = 0.1
"#;

    let parsed: Value = toml_content.parse().unwrap();
    let result = generate_code_from_constants(&parsed, &RUST_FMTS);
    insta::assert_snapshot!(result);
}

#[test]
fn generate_dl_constants() {
    let toml_content = r#"
[movement]
friction_coefficient = 0.7
max_speed = 5.0
gravity = 9.8

[physics]
default_mass = 70
grace_distance = 0.1
"#;

    let parsed: Value = toml_content.parse().unwrap();
    let result = generate_code_from_constants(&parsed, &DL_FMTS);
    insta::assert_snapshot!(result);
}

// New comprehensive tests

fn create_temp_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp directory")
}

fn create_test_toml_content() -> String {
    r#"
[section1]
int_val = 42
float_val = 3.14
str_val = "hello"

[section2]
another_int = -100
another_float = 0.0
another_str = "world"
"#.to_string()
}

#[test]
fn test_formats_struct_creation() {
    let formats = Formats {
        int_fmt: "test_int: {} = {}",
        float_fmt: "test_float: {} = {}",
        str_fmt: "test_str: {} = \"{}\"",
    };
    assert_eq!(formats.int_fmt, "test_int: {} = {}");
    assert_eq!(formats.float_fmt, "test_float: {} = {}");
    assert_eq!(formats.str_fmt, "test_str: {} = \"{}\"");
}

#[test]
fn test_rust_fmts_constants() {
    assert_eq!(RUST_FMTS.int_fmt, "pub const {}: i64 = {};\n");
    assert_eq!(RUST_FMTS.float_fmt, "pub const {}: f32 = {}f32;\n");
    assert_eq!(RUST_FMTS.str_fmt, "pub const {}: &str = \"{}\";\n");
}

#[test]
fn test_dl_fmts_constants() {
    assert_eq!(DL_FMTS.int_fmt, "const {}: signed<64> = {}\n");
    assert_eq!(DL_FMTS.float_fmt, "const {}: GCoord = {}\n");
    assert_eq!(DL_FMTS.str_fmt, "const {}: string = \"{}\"\n");
}

#[test]
fn test_parse_constants_success() {
    let temp_dir = create_temp_dir();
    let manifest_dir = temp_dir.path().to_str().unwrap();
    let toml_path = temp_dir.path().join("constants.toml");
    fs::write(&toml_path, create_test_toml_content()).unwrap();

    let result = parse_constants(manifest_dir);
    assert!(result.is_ok());

    let parsed = result.unwrap();
    assert!(parsed.is_table());

    let table = parsed.as_table().unwrap();
    assert!(table.contains_key("section1"));
    assert!(table.contains_key("section2"));
}

#[test]
fn test_parse_constants_file_not_found() {
    let temp_dir = create_temp_dir();
    let manifest_dir = temp_dir.path().to_str().unwrap();

    let result = parse_constants(manifest_dir);
    assert!(result.is_err());

    let error_msg = format!("{}", result.unwrap_err());
    assert!(error_msg.contains("No such file") || error_msg.contains("cannot find"));
}

#[test]
fn test_parse_constants_invalid_toml() {
    let temp_dir = create_temp_dir();
    let manifest_dir = temp_dir.path().to_str().unwrap();
    let toml_path = temp_dir.path().join("constants.toml");
    fs::write(&toml_path, "invalid toml content [[[").unwrap();

    let result = parse_constants(manifest_dir);
    assert!(result.is_err());
}

#[test]
fn test_parse_constants_empty_toml() {
    let temp_dir = create_temp_dir();
    let manifest_dir = temp_dir.path().to_str().unwrap();
    let toml_path = temp_dir.path().join("constants.toml");
    fs::write(&toml_path, "").unwrap();

    let result = parse_constants(manifest_dir);
    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert!(parsed.is_table());
    assert!(parsed.as_table().unwrap().is_empty());
}

#[test]
fn test_generate_code_from_constants_rust_format() {
    let toml_content = r#"
[section1]
int_val = 42
float_val = 3.14
str_val = "hello"
"#;
    let parsed: Value = toml_content.parse().unwrap();
    let result = generate_code_from_constants(&parsed, &RUST_FMTS);
    assert!(result.contains("// @generated - do not edit"));
    assert!(result.contains("pub const INT_VAL: i64 = 42;"));
    assert!(result.contains("pub const FLOAT_VAL: f32 = 3.14f32;"));
    assert!(result.contains("pub const STR_VAL: &str = \"hello\";"));
}

#[test]
fn test_generate_code_from_constants_dl_format() {
    let toml_content = r#"
[section1]
int_val = 42
float_val = 3.14
str_val = "hello"
"#;
    let parsed: Value = toml_content.parse().unwrap();
    let result = generate_code_from_constants(&parsed, &DL_FMTS);
    assert!(result.contains("// @generated - do not edit"));
    assert!(result.contains("const INT_VAL: signed<64> = 42"));
    assert!(result.contains("const FLOAT_VAL: GCoord = 3.14"));
    assert!(result.contains("const STR_VAL: string = \"hello\""));
}

#[test]
fn test_generate_code_from_constants_empty_input() {
    let parsed = Value::from_str("{}").unwrap();
    let result = generate_code_from_constants(&parsed, &RUST_FMTS);
    assert_eq!(result, "// @generated - do not edit\n");
}

#[test]
fn test_generate_code_from_constants_mixed_types() {
    let toml_content = r#"
[section1]
bool_val = true
array_val = [1, 2, 3]
int_val = 42
datetime_val = 1979-05-27T07:32:00Z
"#;
    let parsed: Value = toml_content.parse().unwrap();
    let result = generate_code_from_constants(&parsed, &RUST_FMTS);
    assert!(result.contains("pub const INT_VAL: i64 = 42;"));
    assert!(!result.contains("bool_val"));
    assert!(!result.contains("array_val"));
    assert!(!result.contains("datetime_val"));
}

#[test]
fn test_generate_code_from_constants_negative_numbers() {
    let toml_content = r#"
[section1]
negative_int = -42
negative_float = -3.14
"#;
    let parsed: Value = toml_content.parse().unwrap();
    let result = generate_code_from_constants(&parsed, &RUST_FMTS);
    assert!(result.contains("pub const NEGATIVE_INT: i64 = -42;"));
    assert!(result.contains("pub const NEGATIVE_FLOAT: f32 = -3.14f32;"));
}

#[test]
fn test_generate_code_from_constants_special_characters_in_strings() {
    let toml_content = r#"
[section1]
special_str = "hello\nworld\t\""
unicode_str = "caf\u00e9"
"#;
    let parsed: Value = toml_content.parse().unwrap();
    let result = generate_code_from_constants(&parsed, &RUST_FMTS);
    assert!(result.contains("SPECIAL_STR"));
    assert!(result.contains("UNICODE_STR"));
    assert!(result.contains("caf\u00e9"));
}

#[test]
fn test_generate_code_from_constants_zero_values() {
    let toml_content = r#"
[section1]
zero_int = 0
zero_float = 0.0
empty_str = ""
"#;
    let parsed: Value = toml_content.parse().unwrap();
    let result = generate_code_from_constants(&parsed, &RUST_FMTS);
    assert!(result.contains("pub const ZERO_INT: i64 = 0;"));
    assert!(result.contains("pub const ZERO_FLOAT: f32 = 0f32;"));
    assert!(result.contains("pub const EMPTY_STR: &str = \"\";"));
}

#[test]
fn test_generate_code_from_constants_large_numbers() {
    let toml_content = r#"
[section1]
large_int = 9223372036854775807
small_int = -9223372036854775808
large_float = 1000000000.0
"#;
    let parsed: Value = toml_content.parse().unwrap();
    let result = generate_code_from_constants(&parsed, &RUST_FMTS);
    assert!(result.contains("LARGE_INT"));
    assert!(result.contains("SMALL_INT"));
    assert!(result.contains("LARGE_FLOAT"));
}

#[test]
fn test_case_conversion_in_generate_code() {
    let toml_content = r#"
[section1]
mixed_case_key = 42
snake_case_key = "value"
lowercase = "test"
"#;
    let parsed: Value = toml_content.parse().unwrap();
    let result = generate_code_from_constants(&parsed, &RUST_FMTS);
    assert!(result.contains("MIXED_CASE_KEY"));
    assert!(result.contains("SNAKE_CASE_KEY"));
    assert!(result.contains("LOWERCASE"));
    assert!(!result.contains("mixed_case_key"));
    assert!(!result.contains("snake_case_key"));
    assert!(!result.contains("lowercase"));
}

#[test]
fn test_generate_constants_success() {
    let temp_dir = create_temp_dir();
    let manifest_dir = temp_dir.path().to_str().unwrap();
    let out_dir = temp_dir.path().join("output");
    fs::create_dir(&out_dir).unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();
    let toml_path = temp_dir.path().join("constants.toml");
    fs::write(&toml_path, create_test_toml_content()).unwrap();
    let result = generate_constants(manifest_dir, &out_dir);
    assert!(result.is_ok());
    let rust_constants = out_dir.join("constants.rs");
    let dl_constants = src_dir.join("constants.dl");
    assert!(rust_constants.exists());
    assert!(dl_constants.exists());
    let rust_content = fs::read_to_string(&rust_constants).unwrap();
    assert!(rust_content.contains("// @generated - do not edit"));
    assert!(rust_content.contains("pub const INT_VAL: i64 = 42;"));
    let dl_content = fs::read_to_string(&dl_constants).unwrap();
    assert!(dl_content.contains("// @generated - do not edit"));
    assert!(dl_content.contains("const INT_VAL: signed<64> = 42"));
}

#[test]
fn test_generate_constants_parse_error() {
    let temp_dir = create_temp_dir();
    let manifest_dir = temp_dir.path().to_str().unwrap();
    let out_dir = temp_dir.path().join("output");
    fs::create_dir(&out_dir).unwrap();
    let toml_path = temp_dir.path().join("constants.toml");
    fs::write(&toml_path, "invalid toml [[[").unwrap();
    let result = generate_constants(manifest_dir, &out_dir);
    assert!(result.is_err());
}

#[test]
fn test_generate_constants_missing_constants_file() {
    let temp_dir = create_temp_dir();
    let manifest_dir = temp_dir.path().to_str().unwrap();
    let out_dir = temp_dir.path().join("output");
    fs::create_dir(&out_dir).unwrap();
    let result = generate_constants(manifest_dir, &out_dir);
    assert!(result.is_err());
}

#[test]
fn test_generate_constants_write_permission_error() {
    let temp_dir = create_temp_dir();
    let manifest_dir = temp_dir.path().to_str().unwrap();
    let out_dir = PathBuf::from("/nonexistent/path");
    let toml_path = temp_dir.path().join("constants.toml");
    fs::write(&toml_path, create_test_toml_content()).unwrap();
    let result = generate_constants(manifest_dir, &out_dir);
    assert!(result.is_err());
}

#[test]
fn test_nested_sections_processing() {
    let toml_content = r#"
[section1]
value1 = 1

[section1.subsection]
nested_value = 2

[section2]
value2 = 3
"#;
    let parsed: Value = toml_content.parse().unwrap();
    let result = generate_code_from_constants(&parsed, &RUST_FMTS);
    assert!(result.contains("VALUE1"));
    assert!(result.contains("VALUE2"));
    assert!(!result.contains("NESTED_VALUE"));
}

#[test]
fn test_multiple_sections_with_same_key_names() {
    let toml_content = r#"
[section1]
common_key = 42

[section2]
common_key = "different_value"
"#;
    let parsed: Value = toml_content.parse().unwrap();
    let result = generate_code_from_constants(&parsed, &RUST_FMTS);
    let common_key_count = result.matches("COMMON_KEY").count();
    assert_eq!(common_key_count, 2);
}

#[test]
fn test_scientific_notation_floats() {
    let toml_content = r#"
[section1]
scientific_float = 1.23e-4
large_scientific = 1.5e10
"#;
    let parsed: Value = toml_content.parse().unwrap();
    let result = generate_code_from_constants(&parsed, &RUST_FMTS);
    assert!(result.contains("SCIENTIFIC_FLOAT"));
    assert!(result.contains("LARGE_SCIENTIFIC"));
}

#[test]
fn test_custom_formats_struct() {
    let custom_formats = Formats {
        int_fmt: "INTEGER_{} = {};",
        float_fmt: "FLOAT_{} = {};",
        str_fmt: "STRING_{} = '{}';",
    };
    let toml_content = r#"
[section1]
test_int = 42
test_float = 3.14
test_str = "hello"
"#;
    let parsed: Value = toml_content.parse().unwrap();
    let result = generate_code_from_constants(&parsed, &custom_formats);
    assert!(result.contains("INTEGER_TEST_INT = 42;"));
    assert!(result.contains("FLOAT_TEST_FLOAT = 3.14;"));
    assert!(result.contains("STRING_TEST_STR = 'hello';"));
}

#[test]
fn test_empty_section_names() {
    let toml_content = r#"
[section1]

[section2]
some_value = 42
"#;
    let parsed: Value = toml_content.parse().unwrap();
    let result = generate_code_from_constants(&parsed, &RUST_FMTS);
    assert!(result.contains("SOME_VALUE"));
    assert_eq!(result.matches("pub const").count(), 1);
}

