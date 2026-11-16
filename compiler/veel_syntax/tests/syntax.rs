//! The slice's syntax: Scowr's shared syntax expresses every construct of the phase 1a slice
//! (ranges, timecode, keyword attributes, keyframe lists), every round-trip file prints back byte
//! for byte with no diagnostics, and kebab-case names are enforced with fixes. The spec's
//! examples and the slice corpus are `spec_examples.rs`'s.

use std::path::{Path, PathBuf};

use veel_syntax::ast::{Entry, Node, Value};
use veel_syntax::{FileId, TokenKind, apply_edits, diagnostics, dump, lex, names, parse};

pub(crate) fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn roundtrip_files() -> Vec<PathBuf> {
    let dir = root().join("tests/roundtrip");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "veel"))
        .collect();
    files.sort();
    files
}

/// What is wrong with `text` as a Veel file: its diagnostics, a lexer token missing from the
/// tree, or a print that differs from the text. Empty when it parses clean and prints back byte
/// for byte, with the tree's tokens the lexer's, covering every byte.
pub(crate) fn problems(name: &str, text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let (tokens, errors) = lex(text);
    if !errors.is_empty() {
        out.push(format!("{name}: lexer errors {errors:?}"));
    }
    let lexed: usize = tokens.iter().map(|t| t.len as usize).sum();
    if lexed != text.len() {
        out.push(format!(
            "{name}: the tokens cover {lexed} of {} bytes",
            text.len()
        ));
    }
    let file = parse(text);
    for d in diagnostics(&file, FileId(0)) {
        out.push(format!(
            "{name}: byte {}: error[{}]: {}",
            d.primary.span.start, d.code, d.message
        ));
    }
    let tree_tokens: Vec<_> = file
        .root()
        .tokens()
        .iter()
        .map(|t| (t.kind, t.text.len()))
        .collect();
    let lexer_tokens: Vec<_> = tokens.iter().map(|t| (t.kind, t.len as usize)).collect();
    if tree_tokens != lexer_tokens {
        out.push(format!("{name}: the tree's tokens are not the lexer's"));
    }
    if file.print() != text {
        out.push(format!("{name}: does not print back byte for byte"));
    }
    out
}

/// Asserts `text` has no [`problems`].
pub(crate) fn assert_lossless_and_clean(name: &str, text: &str) {
    let found = problems(name, text);
    assert!(found.is_empty(), "{}", found.join("\n"));
}

#[test]
fn every_roundtrip_file_prints_back_with_no_diagnostics() {
    let files = roundtrip_files();
    assert_eq!(files.len(), 2);
    for path in files {
        let text = std::fs::read_to_string(&path).unwrap();
        assert_lossless_and_clean(&path.display().to_string(), &text);
    }
}

/// Every node of `file`, depth first.
fn all_nodes<'a>(nodes: impl Iterator<Item = Node<'a>>) -> Vec<Node<'a>> {
    let mut out = Vec::new();
    let mut stack: Vec<Node<'a>> = nodes.collect();
    stack.reverse();
    while let Some(n) = stack.pop() {
        let mut children: Vec<_> = n.children().collect();
        children.reverse();
        stack.extend(children);
        out.push(n);
    }
    out
}

/// A node's entries as `(kind, key, value text)`.
fn entries(node: Node<'_>) -> Vec<(&'static str, String, String)> {
    node.entries()
        .map(|e| match e {
            Entry::Arg(v) => ("arg", String::new(), v.text().to_string()),
            Entry::Prop { key, value } => ("prop", key.text.to_string(), value.text().to_string()),
            Entry::Keyword { key, value } => {
                ("keyword", key.text.to_string(), value.text().to_string())
            }
        })
        .collect()
}

fn e(kind: &'static str, key: &str, value: &str) -> (&'static str, String, String) {
    (kind, key.to_string(), value.to_string())
}

#[test]
fn the_slice_constructs_read_as_the_slice_means_them() {
    // Blueprint 02's slice, verbatim: what each construct is in the shared tree.
    let text =
        std::fs::read_to_string(root().join("tests/roundtrip/blueprint-slice.veel")).unwrap();
    let file = parse(&text);
    let nodes = all_nodes(file.document().nodes());
    let named = |name: &str| -> Vec<Node<'_>> {
        nodes.iter().copied().filter(|n| n.name() == name).collect()
    };

    // A source with a display name and properties.
    let source = named("source")[0];
    assert_eq!(
        entries(source),
        [
            e("arg", "", "@interview"),
            e("arg", "", "\"Interview\""),
            e("prop", "media", "\"corpus/media/interview.mov\""),
            e("prop", "rate", "25"),
        ]
    );

    // A clip: its handle, `from=`, a range of timecode and seconds as one word, and `at` as a
    // keyword attribute (Scowr's section 4), with a block after it.
    let clips = named("clip");
    assert_eq!(clips.len(), 2);
    assert_eq!(
        entries(clips[0]),
        [
            e("arg", "", "@a"),
            e("prop", "from", "@interview"),
            e("arg", "", "00:00:10:00..+5s"),
            e("keyword", "at", "0"),
        ]
    );
    assert!(clips[0].block().is_some());
    assert_eq!(
        entries(clips[1]),
        [
            e("arg", "", "@b"),
            e("prop", "from", "@interview"),
            e("arg", "", "00:01:00:00..+4s"),
            e("keyword", "at", "5s"),
        ]
    );

    // The sequence's size is a list of two words.
    let size = named("size")[0];
    let Value::List(_) = size.args().next().unwrap() else {
        panic!("size is not a list");
    };
    assert_eq!(
        size.args()
            .next()
            .unwrap()
            .items()
            .iter()
            .map(|v| v.text())
            .collect::<Vec<_>>(),
        ["1920", "1080"]
    );

    // A keyframe block on one line: two `key` children of `rect`, each a time and a list.
    let rect = named("rect")[0];
    let keys: Vec<_> = rect.children().collect();
    assert_eq!(keys.len(), 2);
    for (key, (time, list)) in keys.iter().zip([
        ("0", ["0", "0", "1920", "1080"]),
        ("2s", ["-96", "-54", "2112", "1188"]),
    ]) {
        assert_eq!(key.name(), "key");
        let args: Vec<_> = key.args().collect();
        assert_eq!(args[0].as_word(), Some(time));
        assert_eq!(
            args[1].items().iter().map(|v| v.text()).collect::<Vec<_>>(),
            list
        );
    }

    // The effect names its asset id as a word; `crossfade` takes a bare frame count.
    assert_eq!(
        entries(named("effect")[0]),
        [e("arg", "", "@a-push"), e("arg", "", "qtblend")]
    );
    assert_eq!(entries(named("crossfade")[0]), [e("arg", "", "12")]);
}

#[test]
fn every_value_form_of_the_spec_is_one_word() {
    // Spec section 5: numbers, fractions, times, ranges, quantities, colours, handles and hashes
    // are words, read later by the schema; the shared syntax keeps each whole.
    let words = [
        "12",
        "-3",
        "0x1F",
        "0.62",
        "1e-3",
        "inf",
        "nan",
        "30000/1001",
        "2s",
        "250ms",
        "1:02.250",
        "01:00:10:12",
        "4800smp",
        "10:02:10:00..+12s",
        "01:00:01:00..01:00:03:12",
        "-12..+6",
        "75%",
        "-3dB",
        "0.5x",
        "48kHz",
        "12deg",
        "#ff8800",
        "#ff880080",
        "cubic-in",
        "@interview-a",
        "@pkg:lt-name",
        "@a/@grade-c",
        "sha256:9f21c4…",
        "kdenlive:clipname",
    ];
    for word in words {
        let text = format!("v {word}\n");
        let file = parse(&text);
        assert!(file.errors().is_empty(), "{word}: {:?}", file.errors());
        let node = file.document().nodes().next().unwrap();
        let args: Vec<_> = node.args().collect();
        assert_eq!(args.len(), 1, "{word}");
        assert_eq!(args[0].as_word(), Some(word), "{word}");
    }
}

#[test]
fn keyword_attributes_and_their_child_lines() {
    // `at`, `to` and `len` and a value are keyword attributes on an owner's line (spec section
    // 4); on a line of their own they are child nodes, which the checker reads the same.
    let file = parse(
        "clip @a at 01:00:04:00 len 5s {\n  at 12s\n}\nmarker \"Act 2\" at 0 category=purple\n",
    );
    let nodes: Vec<_> = file.document().nodes().collect();
    assert_eq!(
        entries(nodes[0]),
        [
            e("arg", "", "@a"),
            e("keyword", "at", "01:00:04:00"),
            e("keyword", "len", "5s")
        ]
    );
    let child = nodes[0].children().next().unwrap();
    assert_eq!(child.name(), "at");
    assert_eq!(entries(child), [e("arg", "", "12s")]);
    assert_eq!(
        entries(nodes[1]),
        [
            e("arg", "", "\"Act 2\""),
            e("keyword", "at", "0"),
            e("prop", "category", "purple")
        ]
    );
    // `to=` is a property, not a keyword.
    let file = parse("apply @grade to=[@a @c]\n");
    let node = file.document().nodes().next().unwrap();
    assert_eq!(entries(node)[1].0, "prop");
}

#[test]
fn kebab_case_is_the_rule_for_names() {
    for good in [
        "clip",
        "start-timecode",
        "lift-gamma-gain",
        "gain-r",
        "x",
        "h264",
    ] {
        assert!(names::is_kebab_case(good), "{good}");
    }
    for bad in [
        "Clip",
        "start_timecode",
        "startTimecode",
        "-x",
        "x-",
        "a--b",
        "2d",
        "",
        "field order",
        "kdenlive:clipname",
    ] {
        assert!(!names::is_kebab_case(bad), "{bad}");
    }
    let cases = [
        ("start_timecode", Some("start-timecode")),
        ("Clip", Some("clip")),
        ("startTimecode", Some("start-timecode")),
        ("START_TIMECODE", Some("start-timecode")),
        ("Field Order", Some("field-order")),
        ("use2D", Some("use2d")),
        ("2d", None),
        ("kdenlive:clipname", None),
    ];
    for (name, kebab) in cases {
        assert_eq!(names::to_kebab_case(name).as_deref(), kebab, "{name}");
    }
}

#[test]
fn the_fixes_make_the_file_kebab_case() {
    let text = "source @a startTimecode=10:00:00:00 { color_space rec709; \"field order\" top }\n";
    let file = parse(text);
    let found = diagnostics(&file, FileId(0));
    assert_eq!(found.len(), 3);
    let mut edits: Vec<_> = found
        .iter()
        .flat_map(|d| &d.fixes[0].edits)
        .map(|e| (e.span, e.replacement.as_str()))
        .collect();
    edits.sort_by_key(|e| e.0.start);
    let fixed = apply_edits(text, &edits);
    assert_eq!(
        fixed,
        "source @a start-timecode=10:00:00:00 { color-space rec709; field-order top }\n"
    );
    assert!(diagnostics(&parse(&fixed), FileId(0)).is_empty());
}

#[test]
fn a_raw_blocks_lines_keep_kdenlives_names_and_nothing_else_does() {
    // Spec section 4: inside `raw`, at any depth, names are Kdenlive's own; the `raw` line itself
    // and everything outside it are held to kebab-case.
    let text = "clip @c {\n  raw {\n    kdenlive:clipname \"C\"\n    deep_block {\n      mlt_service avformat\n    }\n  }\n  not_kebab 1\n}\nRaw { x_y 1 }\n";
    let file = parse(text);
    let found = names::check(&file, FileId(0));
    let named: Vec<&str> = found
        .iter()
        .map(|d| &text[d.primary.span.start..d.primary.span.end])
        .collect();
    assert_eq!(named, ["not_kebab", "Raw", "x_y"], "{found:#?}");
}

#[test]
fn the_dump_shows_every_byte() {
    let text = "sequence @main { rate 25 } // x\n";
    let file = parse(text);
    let dumped = dump(&file);
    let words = dumped.lines().filter(|l| l.contains("Word@")).count();
    assert_eq!(words, 4);
    let tokens = file.root().tokens();
    assert!(tokens.iter().any(|t| t.kind == TokenKind::LineComment));
    assert_eq!(
        dumped.lines().filter(|l| l.contains('"')).count(),
        tokens.len()
    );
}
