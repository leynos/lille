use build_support::constants::{generate_code_from_constants, RUST_FMTS};

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
