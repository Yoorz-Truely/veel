//! The `veel` command, run as the binary runs it: `veel check` and `veel fmt --check` pass on the
//! slice corpus; `veel check` prints a negative case's golden and exits 1; `veel fmt` writes the
//! canonical form, after which its check passes; `veel print` writes nothing.

use std::path::{Path, PathBuf};

use veel_cli::{HELP, run};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn veel(args: &[&str]) -> veel_cli::Outcome {
    run(&args.iter().map(|a| a.to_string()).collect::<Vec<_>>())
}

fn corpus_folders() -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(root().join("tests/slice"))
        .unwrap()
        .map(|e| e.unwrap().path().display().to_string())
        .collect();
    out.sort();
    out
}

#[test]
fn check_and_fmt_pass_on_the_slice_corpus() {
    let folders = corpus_folders();
    assert_eq!(folders.len(), 5);
    for folder in &folders {
        let out = veel(&["check", folder]);
        assert_eq!(
            (out.code, out.stderr.as_str(), out.stdout.as_str()),
            (0, "", ""),
            "{folder}"
        );
        let out = veel(&["fmt", "--check", folder]);
        assert_eq!(
            (out.code, out.stderr.as_str(), out.stdout.as_str()),
            (0, "", ""),
            "{folder}"
        );
    }
    let all: Vec<&str> = folders.iter().map(String::as_str).collect();
    assert_eq!(veel(&[&["check"], all.as_slice()].concat()).code, 0);
}

#[test]
fn check_prints_a_negative_cases_golden_and_exits_1() {
    let case = root().join("tests/negative/veel-overlap.veel");
    let out = veel(&["check", &case.display().to_string()]);
    let golden = std::fs::read_to_string(case.with_extension("stderr")).unwrap();
    assert_eq!(out.code, 1);
    assert_eq!(out.stderr, golden);
    let folder = root().join("tests/negative/veel-handle");
    let out = veel(&["check", &folder.display().to_string()]);
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr,
        std::fs::read_to_string(root().join("tests/negative/veel-handle.stderr")).unwrap()
    );
}

#[test]
fn fmt_writes_the_canonical_form_and_print_writes_nothing() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("veel-fmt");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("sequences")).unwrap();
    let file = dir.join("sequences/main.veel");
    let messy = "// sequences/main.veel\nsource @s len=10s rate=25 media=\"m.mov\"\nsequence @main rate=25 size=[1920 1080] {\n    track @v1 video {\n clip @a from=@s 0..+2s at=0   \n}\n}\n";
    std::fs::write(&file, messy).unwrap();
    let d = dir.display().to_string();
    let f = file.display().to_string();
    let out = veel(&["fmt", "--check", &d]);
    assert_eq!(out.code, 1);
    assert_eq!(out.stdout, format!("{f}: not in canonical form\n"));
    let out = veel(&["print", &f]);
    assert_eq!(out.code, 0);
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        messy,
        "print wrote"
    );
    let canonical = "// sequences/main.veel\nsource @s media=\"m.mov\" rate=25 len 10s\nsequence @main rate=25 size=[1920 1080] {\n  track @v1 video {\n    clip @a from=@s 0..+2s at 0\n  }\n}\n";
    assert_eq!(out.stdout, canonical);
    let out = veel(&["fmt", &d]);
    assert_eq!((out.code, out.stdout), (0, format!("{f}: formatted\n")));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), canonical);
    assert_eq!(veel(&["fmt", "--check", &d]).code, 0);
    assert_eq!(veel(&["check", &d]).code, 0);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn help_version_and_commands_it_cannot_run() {
    let out = veel(&["--help"]);
    assert_eq!((out.code, out.stdout.as_str()), (0, HELP));
    for command in ["veel check", "veel fmt", "veel print", "section 17"] {
        assert!(HELP.contains(command), "{command}");
    }
    assert!(veel(&["--version"]).stdout.starts_with("veel "));
    for bad in [
        &["check"][..],
        &["fmt"],
        &["print"],
        &["lint", "x"],
        &["fmt", "x", "--check"],
    ] {
        let out = veel(bad);
        assert_eq!(out.code, 2, "{bad:?}");
        assert!(out.stderr.contains("veel --help"));
    }
    let out = veel(&["check", "/no/such/folder.veel"]);
    assert_eq!(out.code, 2);
    assert!(out.stderr.starts_with("veel: cannot read"));
}
