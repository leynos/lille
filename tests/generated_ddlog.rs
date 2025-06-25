#![cfg(feature = "ddlog")]

#[test]
fn generated_ddlog_crate_present() {
    use std::path::Path;
    let base = Path::new("generated").join("lille_ddlog");
    assert!(
        base.is_dir(),
        "Directory {:?} missing",
        base.canonicalize().unwrap_or(base.clone())
    );
    assert!(
        base.join("lib.rs").is_file(),
        "generated/lille_ddlog/lib.rs missing"
    );
    let ddlog_subcrate = base.join("differential_datalog");
    assert!(
        ddlog_subcrate.is_dir(),
        "differential_datalog subcrate missing"
    );
    assert!(
        ddlog_subcrate.join("lib.rs").is_file(),
        "differential_datalog/lib.rs missing"
    );
}
