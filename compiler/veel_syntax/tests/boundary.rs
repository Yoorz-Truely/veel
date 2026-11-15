//! The shared front end's boundary (`AGENTS.md`, "The shared front end"; V17): only this crate
//! names Scowr's crates, so when they move to their own repository only its manifest changes.

use std::path::{Path, PathBuf};

fn rust_and_toml_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n != "target") {
                rust_and_toml_files(&p, out);
            }
        } else if p.extension().is_some_and(|x| x == "rs" || x == "toml") {
            out.push(p);
        }
    }
}

#[test]
fn only_veel_syntax_names_scowr() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut crates = 0;
    let mut offenders = Vec::new();
    for e in std::fs::read_dir(root.join("compiler")).unwrap() {
        let dir = e.unwrap().path();
        if !dir.join("Cargo.toml").exists() {
            continue;
        }
        crates += 1;
        if dir.file_name().is_some_and(|n| n == "veel_syntax") {
            continue;
        }
        let mut files = Vec::new();
        rust_and_toml_files(&dir, &mut files);
        for f in files {
            // A crate is named by its identifier in code or a manifest; prose may mention Scowr.
            let text = std::fs::read_to_string(&f).unwrap();
            for (i, line) in text.lines().enumerate() {
                let code = line.trim_start();
                if code.contains("scowr_") && !code.starts_with("//") && !code.starts_with('#') {
                    offenders.push(format!("{}:{}", f.display(), i + 1));
                }
            }
        }
    }
    assert!(crates >= 1);
    assert!(offenders.is_empty(), "these name Scowr: {offenders:?}");

    // In the root manifest, Scowr appears only as the shared crates' paths and their comment.
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let naming: Vec<&str> = manifest
        .lines()
        .filter(|l| l.contains("scowr_") && !l.trim_start().starts_with('#'))
        .collect();
    assert_eq!(naming.len(), 2, "{naming:?}");
    assert!(
        naming
            .iter()
            .all(|l| l.contains("path = \"../Scowr/compiler/scowr_"))
    );
}
