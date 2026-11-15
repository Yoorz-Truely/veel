//! This crate's integration tests, in one test binary (as Null's crates, Sau's S69): each file
//! beside this one is a module here, not a binary of its own, so `cargo test` links one
//! executable. A new test file must be listed here: `every_test_file_is_a_module` fails otherwise.

/// The test files of this folder, each a module above.
const MODULES: &[&str] = &[];

#[test]
fn every_test_file_is_a_module() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut found: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".rs") && n != "main.rs")
        .collect();
    found.sort();
    assert_eq!(
        found, MODULES,
        "a test file in tests/ is not a module of tests/main.rs, so it would never run; add it there"
    );
}
