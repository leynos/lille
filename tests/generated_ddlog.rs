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
    let lib_rs = base.join("lib.rs");
    assert!(
        lib_rs.is_file(),
        "File {:?} missing",
        lib_rs.canonicalize().unwrap_or_else(|_| lib_rs.clone())
    );
    let ddlog_subcrate = base.join("differential_datalog");
    assert!(
        ddlog_subcrate.is_dir(),
        "Directory {:?} missing",
        ddlog_subcrate
            .canonicalize()
            .unwrap_or_else(|_| ddlog_subcrate.clone())
    );
    let ddlog_lib = ddlog_subcrate.join("lib.rs");
    assert!(
        ddlog_lib.is_file(),
        "File {:?} missing",
        ddlog_lib
            .canonicalize()
            .unwrap_or_else(|_| ddlog_lib.clone())
    );
}
