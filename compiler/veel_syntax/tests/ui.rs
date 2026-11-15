//! The syntax's UI tests: each `tests/ui/syntax/*.veel` at the workspace root, good or broken,
//! against the `.stderr` file beside it, byte for byte (`tests/README.md`, "`ui/`"). A good file
//! has an empty `.stderr`. Run with `VEEL_BLESS=1` to rewrite the goldens, then read the diff.

use std::path::{Path, PathBuf};

use veel_syntax::{SourceMap, codes, diagnostics, messages, parse};

fn cases() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/ui/syntax");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "veel"))
        .collect();
    files.sort();
    files
}

/// What `veel check` would print for the syntax of `path`, named relative to its folder.
fn render(path: &Path) -> String {
    let name = path.file_name().unwrap().to_str().unwrap();
    let text = std::fs::read_to_string(path).unwrap();
    let mut map = SourceMap::new();
    let id = map.add(name, &text);
    map.render_all(&diagnostics(&parse(&text), id))
}

#[test]
fn every_case_prints_its_golden() {
    let bless = std::env::var_os("VEEL_BLESS").is_some();
    let files = cases();
    let mut wrong = Vec::new();
    for path in &files {
        let got = render(path);
        let golden = path.with_extension("stderr");
        if bless {
            std::fs::write(&golden, &got).unwrap();
            continue;
        }
        let want = std::fs::read_to_string(&golden).unwrap_or_else(|_| {
            panic!(
                "{} has no .stderr beside it; run with VEEL_BLESS=1",
                path.display()
            )
        });
        if got != want {
            wrong.push(format!(
                "{}:\n--- want\n{want}--- got\n{got}",
                path.display()
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    // Every golden has its case: a renamed case leaves no stale `.stderr`.
    let dir = files[0].parent().unwrap();
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_some_and(|x| x == "stderr") {
            assert!(
                p.with_extension("veel").exists(),
                "{} has no case",
                p.display()
            );
        }
    }
}

#[test]
fn the_cases_cover_good_and_broken_files() {
    let files = cases();
    let (good, broken): (Vec<_>, Vec<_>) = files.iter().partition(|p| render(p).is_empty());
    let named_good = |p: &&PathBuf| {
        p.file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("good-")
    };
    assert_eq!(good.len(), 3, "{good:?}");
    assert_eq!(broken.len(), 9, "{broken:?}");
    // A case is named good exactly when it has no diagnostics.
    assert!(good.iter().all(named_good), "{good:?}");
    assert!(!broken.iter().any(named_good), "{broken:?}");
    // Both of this crate's codes are reported by some case.
    let all: String = broken.iter().map(|p| render(p)).collect();
    for code in codes::ALL {
        assert!(all.contains(&format!("[{code}]")), "no case reports {code}");
    }
}

#[test]
fn every_case_says_what_it_tests_on_its_first_line() {
    for path in cases() {
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("// "), "{}", path.display());
    }
}

#[test]
fn every_message_is_read_and_free_of_scowrs_examples() {
    for path in cases() {
        let text = std::fs::read_to_string(&path).unwrap();
        let file = parse(&text);
        for e in file.errors() {
            assert!(messages::is_known(&e.message), "unread: {}", e.message);
        }
        for d in diagnostics(&file, veel_syntax::FileId(0)) {
            assert!(
                d.primary.span.end <= text.len()
                    && text.is_char_boundary(d.primary.span.start)
                    && text.is_char_boundary(d.primary.span.end)
            );
            for example in messages::SCOWR_EXAMPLES {
                assert!(!d.message.contains(example), "{}", d.message);
            }
        }
    }
}
