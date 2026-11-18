//! The negative cases: one per diagnostic code `veel check` reports, in `tests/negative/` at the
//! workspace root, each named after its code (a file, or a project folder when the code needs
//! several files), against the `.stderr` beside it, byte for byte (`tests/README.md`,
//! "`negative/`"). Each case reports only its own code; every code has its case; and every fix
//! a case offers applies and mends what it was offered for. Run with `VEEL_BLESS=1` to rewrite
//! the goldens, then read the diff.

use std::path::{Path, PathBuf};

use veel_check::{Checked, check, check_folder, codes, project_files};
use veel_syntax::apply_edits;

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/negative")
}

/// Every case: a `.veel` file or a folder, sorted.
fn cases() -> Vec<PathBuf> {
    let mut out: Vec<_> = std::fs::read_dir(dir())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir() || p.extension().is_some_and(|x| x == "veel"))
        .collect();
    out.sort();
    out
}

/// The code a case is named after.
fn code_of(case: &Path) -> String {
    case.file_stem().unwrap().to_string_lossy().into_owned()
}

/// A case's files, each named as `veel check` names it, with its text.
fn files(case: &Path) -> Vec<(String, String)> {
    if case.is_dir() {
        project_files(case)
            .unwrap()
            .into_iter()
            .map(|(name, p)| (name, std::fs::read_to_string(p).unwrap()))
            .collect()
    } else {
        let name = case.file_name().unwrap().to_string_lossy().into_owned();
        vec![(name, std::fs::read_to_string(case).unwrap())]
    }
}

fn checked(case: &Path) -> Checked {
    if case.is_dir() {
        check_folder(case).unwrap()
    } else {
        check(&files(case))
    }
}

/// Every code `veel check` reports: the syntax's, then the checker's.
fn all_codes() -> Vec<&'static str> {
    veel_syntax::codes::ALL
        .iter()
        .chain(codes::ALL.iter())
        .copied()
        .collect()
}

#[test]
fn every_case_prints_its_golden() {
    let bless = std::env::var_os("VEEL_BLESS").is_some();
    let mut wrong = Vec::new();
    for case in cases() {
        let got = checked(&case).render();
        let golden = case.with_extension("stderr");
        if bless {
            std::fs::write(&golden, &got).unwrap();
            continue;
        }
        let want = std::fs::read_to_string(&golden)
            .unwrap_or_else(|_| panic!("{} has no .stderr; run with VEEL_BLESS=1", case.display()));
        if got != want {
            wrong.push(format!(
                "{}:\n--- want\n{want}--- got\n{got}",
                case.display()
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    for e in std::fs::read_dir(dir()).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_some_and(|x| x == "stderr") {
            let stem = p.with_extension("");
            assert!(
                stem.is_dir() || p.with_extension("veel").exists(),
                "{} has no case",
                p.display()
            );
        }
    }
}

#[test]
fn every_code_has_one_case_reporting_only_it() {
    let names: Vec<String> = cases().iter().map(|c| code_of(c)).collect();
    let mut want: Vec<String> = all_codes().iter().map(|c| c.to_string()).collect();
    want.sort();
    assert_eq!(names, want, "one case per code, named after it");
    for case in cases() {
        let code = code_of(&case);
        let found = checked(&case).diagnostics;
        assert!(!found.is_empty(), "{} reports nothing", case.display());
        for d in &found {
            assert_eq!(d.code, code, "{}: {}", case.display(), d.message);
        }
    }
}

#[test]
fn every_case_says_what_it_tests_on_its_first_line() {
    for case in cases() {
        for (name, text) in files(&case) {
            assert!(text.starts_with("// "), "{}: {name}", case.display());
        }
    }
}

/// Every fix applies to its files and leaves fewer diagnostics of its code than before.
#[test]
fn every_fix_mends_what_it_was_offered_for() {
    let mut fixes = 0;
    for case in cases() {
        let files = files(&case);
        let before = checked(&case);
        for d in &before.diagnostics {
            let count = |c: &Checked| c.diagnostics.iter().filter(|x| x.code == d.code).count();
            for fix in &d.fixes {
                let mut mended = files.clone();
                for (i, (_, text)) in mended.iter_mut().enumerate() {
                    let edits: Vec<_> = fix
                        .edits
                        .iter()
                        .filter(|e| e.file.0 as usize == i)
                        .map(|e| (e.span, e.replacement.as_str()))
                        .collect();
                    *text = apply_edits(text, &edits);
                }
                let after = check(&mended);
                assert!(
                    count(&after) < count(&before),
                    "{}: `{}` did not mend `{}`:\n{}",
                    case.display(),
                    fix.message,
                    d.message,
                    after.render()
                );
                fixes += 1;
            }
        }
    }
    assert!(fixes >= 20, "only {fixes} fixes offered");
}
