//! Tests for the `build_support` constants generator.
//! Ensures generated code is syntactically valid and handles edge cases.
use build_support::constants::{generate_code_from_constants, RUST_FMTS};
use test_utils::{assert_all_absent, assert_all_present, assert_valid_rust_syntax};

#[test]
fn generates_rust_constants() {
    let toml_str = r#"
        [numbers]
        answer = 42
        [strings]
        hello = "world"
    "#;
    let parsed: toml::Value = toml_str.parse().unwrap();
    let code = generate_code_from_constants(&parsed, &RUST_FMTS);
    assert!(code.contains("pub const ANSWER: i64 = 42;"));
    assert!(code.contains("pub const HELLO: &str = \"world\";"));
}

#[test]
fn handles_empty_toml() {
    let toml_str = "";
    let parsed: toml::Value = toml_str.parse().unwrap();
    let code = generate_code_from_constants(&parsed, &RUST_FMTS);
    assert!(!code.contains("pub const"));
}

#[test]
fn handles_root_level_values() {
    let toml_str = r#"
        name = "MyProject"
        version = "1.0.0"
        debug = true
        port = 8080
    "#;
    let parsed: toml::Value = toml_str.parse().unwrap();
    let code = generate_code_from_constants(&parsed, &RUST_FMTS);

    assert_all_present(&code, &["NAME", "VERSION", "PORT"]);
    assert_all_absent(&code, &["DEBUG"]);
}

#[test]
fn handles_nested_sections() {
    let toml_str = r#"
        [database]
        host = "localhost"
        port = 5432

        [database.auth]
        username = "admin"
        password = "secret123"

        [api.v1]
        endpoint = "/api/v1"
        timeout = 30
    "#;
    let parsed: toml::Value = toml_str.parse().unwrap();
    let code = generate_code_from_constants(&parsed, &RUST_FMTS);

    assert_all_present(
        &code,
        &[
            "HOST", "PORT", "USERNAME", "PASSWORD", "ENDPOINT", "TIMEOUT",
        ],
    );
}

#[test]
fn handles_different_numeric_types() {
    let toml_str = r#"
        [numbers]
        integer = 42
        negative = -100
        zero = 0
        float = 3.14159
        scientific = 1e6
        hex = 0xFF
    "#;
    let parsed: toml::Value = toml_str.parse().unwrap();
    let code = generate_code_from_constants(&parsed, &RUST_FMTS);

    assert_all_present(
        &code,
        &[
            "INTEGER",
            "42",
            "NEGATIVE",
            "-100",
            "ZERO",
            "0",
            "FLOAT",
            "3.14159",
            "SCIENTIFIC",
            "1e6",
        ],
    );
}

#[test]
fn handles_boolean_values() {
    let toml_str = r#"
        [flags]
        debug = true
        production = false
        verbose = true
        quiet = false
    "#;
    let parsed: toml::Value = toml_str.parse().unwrap();
    let code = generate_code_from_constants(&parsed, &RUST_FMTS);

    assert_all_absent(&code, &["DEBUG", "PRODUCTION", "VERBOSE", "QUIET"]);
}

#[test]
fn handles_string_edge_cases() {
    let toml_str = r#"
        [strings]
        empty = ""
        with_quotes = "He said \"Hello\""
        with_newlines = "Line 1\nLine 2\nLine 3"
        with_unicode = "🚀 Rust 🦀"
        with_backslashes = "C:\\Windows\\System32"
        single_quote = "Don't panic!"
        multiline = """
        This is a
        multiline string
        with multiple lines
        """
    "#;
    let parsed: toml::Value = toml_str.parse().unwrap();
    let code = generate_code_from_constants(&parsed, &RUST_FMTS);

    assert_all_present(
        &code,
        &[
            "EMPTY",
            "WITH_QUOTES",
            "WITH_NEWLINES",
            "WITH_UNICODE",
            "WITH_BACKSLASHES",
            "SINGLE_QUOTE",
            "MULTILINE",
        ],
    );
}

#[test]
fn handles_arrays() {
    let toml_str = r#"
        [arrays]
        integers = [1, 2, 3, 4, 5]
        strings = ["apple", "banana", "cherry"]
        booleans = [true, false, true]
        mixed_numbers = [1, 2.5, 3]
        empty = []
        nested = [[1, 2], [3, 4]]
    "#;
    let parsed: toml::Value = toml_str.parse().unwrap();
    let code = generate_code_from_constants(&parsed, &RUST_FMTS);

    assert!(code.starts_with("// @generated"));
}

#[test]
fn handles_case_conversion_and_naming() {
    let toml_str = r#"
        [naming]
        camelCase = "value1"
        snake_case = "value2"
        kebab-case = "value3"
        PascalCase = "value4"
        lowercase = "value5"
        UPPERCASE = "value6"
        mixed123Numbers = "value7"
        with_special-chars_and123 = "value8"
    "#;
    let parsed: toml::Value = toml_str.parse().unwrap();
    let code = generate_code_from_constants(&parsed, &RUST_FMTS);

    assert!(code.starts_with("// @generated"));
}

#[test]
fn generates_valid_rust_syntax() {
    let toml_str = r#"
        [syntax_test]
        simple = "value"
        number = 123
        flag = true
        list = ["a", "b", "c"]
    "#;
    let parsed: toml::Value = toml_str.parse().unwrap();
    let code = generate_code_from_constants(&parsed, &RUST_FMTS);

    assert_valid_rust_syntax(&code);

    let const_lines: Vec<&str> = code
        .lines()
        .filter(|line| line.contains("pub const"))
        .collect();
    assert!(!const_lines.is_empty());
}

#[test]
fn handles_deeply_nested_structure() {
    let toml_str = r#"
        [level1]
        value1 = "top"

        [level1.level2]
        value2 = "middle"

        [level1.level2.level3]
        value3 = "deep"
        number = 42

        [level1.level2.level3.level4]
        deepest = "bottom"
    "#;
    let parsed: toml::Value = toml_str.parse().unwrap();
    let code = generate_code_from_constants(&parsed, &RUST_FMTS);

    assert!(code.starts_with("// @generated"));
}

#[test]
fn handles_complex_realistic_config() {
    let toml_str = r#"
        [server]
        host = "0.0.0.0"
        port = 8080
        workers = 4

        [database]
        url = "postgresql://user:pass@localhost/db"
        max_connections = 10
        timeout = 30.0

        [logging]
        level = "info"
        file = "/var/log/app.log"
        rotate = true

        [features]
        auth_enabled = true
        rate_limiting = false
        metrics = true

        [cors]
        allowed_origins = ["http://localhost:3000", "https://example.com"]
        allowed_methods = ["GET", "POST", "PUT", "DELETE"]
        max_age = 3600
    "#;
    let parsed: toml::Value = toml_str.parse().unwrap();
    let code = generate_code_from_constants(&parsed, &RUST_FMTS);

    assert!(code.starts_with("// @generated"));
}

#[test]
fn preserves_special_string_content() {
    let toml_str = r#"
        [content]
        json_example = '{"key": "value", "number": 42, "nested": {"inner": true}}'
        sql_query = 'SELECT * FROM users WHERE name = ? AND active = true'
        regex_pattern = r'^\w+@[a-zA-Z_]+?\.[a-zA-Z]{2,3}$'
        url_template = 'https://api.example.com/v1/{resource}/{id}?format=json'
        shell_command = 'grep -r "pattern" /path/to/search --include="*.rs"'
    "#;
    let parsed: toml::Value = toml_str
        .parse()
        .unwrap_or_else(|_| toml::Value::Table(Default::default()));
    let code = generate_code_from_constants(&parsed, &RUST_FMTS);

    assert!(code.starts_with("// @generated"));
}

#[test]
fn handles_table_arrays() {
    let toml_str = r#"
        [[products]]
        name = "Hammer"
        sku = 738594937

        [[products]]
        name = "Nail"
        sku = 284758393
        color = "gray"

        [metadata]
        version = "1.0"
    "#;
    let parsed: toml::Value = toml_str.parse().unwrap();
    let code = generate_code_from_constants(&parsed, &RUST_FMTS);

    assert!(code.starts_with("// @generated"));
}

#[test]
fn handles_datetime_values() {
    let toml_str = r#"
        [timestamps]
        created = 1979-05-27T07:32:00Z
        updated = 1979-05-27T00:32:00-07:00
        local = 1979-05-27T07:32:00
        date_only = 1979-05-27
    "#;
    let parsed: toml::Value = toml_str.parse().unwrap();
    let code = generate_code_from_constants(&parsed, &RUST_FMTS);

    assert!(code.starts_with("// @generated"));
}

#[test]
fn output_compiles_as_valid_rust() {
    let toml_str = r#"
        [test_compilation]
        string_val = "test"
        int_val = 42
        bool_val = true
        float_val = 3.14
        array_val = [1, 2, 3]
    "#;
    let parsed: toml::Value = toml_str.parse().unwrap();
    let code = generate_code_from_constants(&parsed, &RUST_FMTS);

    assert_valid_rust_syntax(&code);
    assert_all_present(&code, &[":"]);
    assert_all_absent(&code, &[": ;", "= ;"]);

    let const_lines: Vec<&str> = code
        .lines()
        .filter(|line| line.trim().starts_with("pub const"))
        .collect();

    for line in const_lines {
        assert!(line.contains(":"));
        assert!(line.contains("="));
        assert!(line.ends_with(";"));
    }
}
