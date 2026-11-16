//! The spec-example test: every fenced Veel example of `docs/spec/veel-v0.md` (```` ```veel ````,
//! and ```` ```veel (<task>) ```` for a form a later task builds) and every `.veel` file of the
//! slice corpus, `tests/slice/`, parses with no diagnostic and prints back byte for byte (task
//! 1a-03). Its controls plant a broken example in a copy of the spec and a broken file in a copy
//! of the corpus, and the same functions must report each.

use std::path::{Path, PathBuf};

use crate::syntax::{problems, root};

const SPEC: &str = "docs/spec/veel-v0.md";

/// The fenced Veel blocks of `markdown`, each with the line its first line is on, and whether
/// its fence is closed.
fn fenced(markdown: &str) -> Vec<(usize, String, bool)> {
    let mut out = Vec::new();
    let mut current: Option<(usize, String)> = None;
    for (i, line) in markdown.lines().enumerate() {
        match &mut current {
            None if line == "```veel" || line.starts_with("```veel (") => {
                current = Some((i + 2, String::new()));
            }
            Some(_) if line == "```" => {
                let (line, text) = current.take().unwrap();
                out.push((line, text, true));
            }
            Some((_, text)) => {
                text.push_str(line);
                text.push('\n');
            }
            None => {}
        }
    }
    if let Some((line, text)) = current {
        out.push((line, text, false));
    }
    out
}

/// The fenced Veel blocks of `markdown`, each with the line its first line is on.
fn veel_blocks(markdown: &str) -> Vec<(usize, String)> {
    fenced(markdown)
        .into_iter()
        .map(|(line, text, _)| (line, text))
        .collect()
}

/// The problems of every Veel block of `markdown`, named `name:line`. A fence never closed is
/// one too.
fn spec_problems(name: &str, markdown: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (line, text, closed) in fenced(markdown) {
        let at = format!("{name}:{line}");
        if !closed {
            out.push(format!("{at}: the fence is never closed"));
        }
        out.extend(problems(&at, &text));
    }
    out
}

/// The example folders of a corpus, sorted.
fn folders(corpus: &Path) -> Vec<PathBuf> {
    let mut out: Vec<_> = std::fs::read_dir(corpus)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir())
        .collect();
    out.sort();
    out
}

/// Every `.veel` file under `dir`, at any depth, sorted.
fn veel_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "veel") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// The problems of every file of the corpus at `corpus`, and of every folder with no file.
fn corpus_problems(corpus: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for folder in folders(corpus) {
        let files = veel_files(&folder);
        if files.is_empty() {
            out.push(format!("{}: no .veel file", folder.display()));
        }
        for path in files {
            let text = std::fs::read_to_string(&path).unwrap();
            out.extend(problems(&path.display().to_string(), &text));
        }
    }
    out
}

fn spec() -> String {
    std::fs::read_to_string(root().join(SPEC)).unwrap()
}

fn corpus() -> PathBuf {
    root().join("tests/slice")
}

#[test]
fn every_spec_example_parses_with_no_diagnostic() {
    let spec = spec();
    // Six before the slice, three in its section (17): one for the first worked example, two
    // files for the second.
    assert_eq!(veel_blocks(&spec).len(), 9, "the spec's veel blocks");
    let found = spec_problems(SPEC, &spec);
    assert!(found.is_empty(), "{}", found.join("\n"));
}

#[test]
fn every_slice_corpus_file_parses_with_no_diagnostic() {
    let corpus = corpus();
    let names: Vec<_> = folders(&corpus)
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        names,
        [
            "crossfades",
            "every-parameter",
            "one-clip",
            "picture-in-picture",
            "two-clips"
        ]
    );
    assert_eq!(veel_files(&corpus).len(), 6, "the corpus's files");
    let found = corpus_problems(&corpus);
    assert!(found.is_empty(), "{}", found.join("\n"));
}

#[test]
fn every_corpus_file_names_its_path_on_its_first_line() {
    let corpus = corpus();
    for folder in folders(&corpus) {
        for path in veel_files(&folder) {
            let rel = path.strip_prefix(&folder).unwrap().to_string_lossy();
            let text = std::fs::read_to_string(&path).unwrap();
            let first = text.lines().next().unwrap();
            assert!(
                first == format!("// {rel}") || first.starts_with(&format!("// {rel}: ")),
                "{}: first line {first:?}",
                path.display()
            );
        }
    }
}

#[test]
fn the_worked_examples_are_corpus_folders_byte_for_byte() {
    let blocks: Vec<String> = veel_blocks(&spec()).into_iter().map(|(_, t)| t).collect();
    for folder in ["two-clips", "picture-in-picture"] {
        let files = veel_files(&corpus().join(folder));
        assert!(!files.is_empty(), "{folder}");
        for path in files {
            let text = std::fs::read_to_string(&path).unwrap();
            assert!(
                blocks.contains(&text),
                "{} is not an example of the spec, byte for byte",
                path.display()
            );
        }
    }
}

/// The control: a broken example planted in a copy of the spec, in either kind of fence, and a
/// broken file planted in a copy of the corpus, each fail the functions the tests above use.
#[test]
fn a_planted_bad_example_fails_the_spec_example_test() {
    let spec = spec();
    assert!(spec_problems(SPEC, &spec).is_empty());
    let cases = [
        // An unclosed block.
        "```veel\nclip @a from=@interview 0..+5s at 0 {\n```\n",
        // A name that is not kebab-case, in a fence naming its task.
        "```veel (1a-04)\nsource @s media=\"m.mov\" Rate=25\n```\n",
        // A reserved character.
        "```veel\nclip @a (0..+5s)\n```\n",
        // A fence never closed.
        "```veel\nclip @a from=@interview 0..+5s at 0\n",
    ];
    for planted in cases {
        let bad = format!("{spec}\n{planted}");
        assert_eq!(
            veel_blocks(&bad).len(),
            veel_blocks(&spec).len() + 1,
            "{planted}"
        );
        assert!(!spec_problems(SPEC, &bad).is_empty(), "{planted}");
    }

    // A copy of the corpus with one broken file planted in a new folder.
    let copy = Path::new(env!("CARGO_TARGET_TMPDIR")).join("planted-slice-corpus");
    let _ = std::fs::remove_dir_all(&copy);
    for folder in folders(&corpus()) {
        for path in veel_files(&folder) {
            let to = copy.join(path.strip_prefix(corpus()).unwrap());
            std::fs::create_dir_all(to.parent().unwrap()).unwrap();
            std::fs::copy(&path, &to).unwrap();
        }
    }
    assert!(corpus_problems(&copy).is_empty());
    let planted = copy.join("planted/sequences/main.veel");
    std::fs::create_dir_all(planted.parent().unwrap()).unwrap();
    std::fs::write(
        &planted,
        "// sequences/main.veel\nsequence @main {\n  rate 25\n",
    )
    .unwrap();
    let found = corpus_problems(&copy);
    assert!(
        !found.is_empty() && found.iter().all(|p| p.contains("planted")),
        "{found:?}"
    );
    // An example folder with no file fails too.
    std::fs::remove_file(&planted).unwrap();
    assert!(
        corpus_problems(&copy)
            .iter()
            .any(|p| p.contains("no .veel file"))
    );
    std::fs::remove_dir_all(&copy).unwrap();
}
