//! Error recovery (task 1a-12): truncated and garbled Veel gives a whole tree, diagnostics with
//! Veel's code and the rest of the file, never a panic. Every case also keeps every invariant of
//! `veel_syntax::invariants`, which the fuzz target (`fuzz/`) checks on arbitrary input.

use std::path::{Path, PathBuf};

use veel_syntax::ast::Node;
use veel_syntax::{FileId, Span, TokenKind, codes, diagnostics, invariants, lex, parse};

use crate::syntax::root;

/// Every clean `.veel` file of the tests: the slice corpus, the round-trip files and the good
/// UI cases.
fn clean_files() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "veel") {
                out.push(p);
            }
        }
    }
    let tests = root().join("tests");
    let mut paths = Vec::new();
    walk(&tests.join("slice"), &mut paths);
    walk(&tests.join("roundtrip"), &mut paths);
    walk(&tests.join("ui/syntax"), &mut paths);
    paths.retain(|p| {
        !p.starts_with(tests.join("ui"))
            || p.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("good-")
    });
    paths.sort();
    paths
        .into_iter()
        .map(|p| {
            let name = p.strip_prefix(&tests).unwrap().display().to_string();
            (name, std::fs::read_to_string(&p).unwrap())
        })
        .collect()
}

/// Every node of `nodes`, depth first, as its name and span.
fn named_spans<'a>(nodes: impl Iterator<Item = Node<'a>>) -> Vec<(String, Span)> {
    let mut out = Vec::new();
    let mut stack: Vec<Node<'a>> = nodes.collect();
    while let Some(n) = stack.pop() {
        out.push((n.name().into_owned(), n.0.span()));
        stack.extend(n.children());
    }
    out
}

/// Whether `text` leaves a construct open at its end: a `{` or `[` never closed, or a string or
/// block comment that runs to the end (a lexer error).
fn leaves_open(text: &str) -> bool {
    let (tokens, errors) = lex(text);
    let mut depth = 0i64;
    for t in &tokens {
        match t.kind {
            TokenKind::LBrace | TokenKind::LBracket => depth += 1,
            TokenKind::RBrace | TokenKind::RBracket => depth -= 1,
            _ => {}
        }
    }
    depth > 0 || !errors.is_empty()
}

/// The messages that say a construct is never closed.
const NEVER_CLOSED: [&str; 5] = [
    "this { is never closed",
    "this [ is never closed",
    "this string is never closed",
    "this block comment is never closed",
    "this raw string is never closed",
];

#[test]
fn every_truncation_recovers_with_diagnostics() {
    let files = clean_files();
    assert!(files.len() >= 11, "{files:?}");
    let (mut cuts, mut open) = (0, 0);
    for (name, text) in &files {
        let whole = parse(text);
        assert!(whole.errors().is_empty(), "{name} is not clean");
        let nodes = named_spans(whole.document().nodes());
        for cut in (0..=text.len()).filter(|&i| text.is_char_boundary(i)) {
            let prefix = &text[..cut];
            cuts += 1;
            invariants::check(prefix, cut / 2, cut, "");
            let file = parse(prefix);
            let found = diagnostics(&file, FileId(0));
            // A construct left open is reported as never closed, with Veel's code.
            if leaves_open(prefix) {
                open += 1;
                assert!(
                    found.iter().any(|d| d.code == codes::SYNTAX
                        && NEVER_CLOSED.iter().any(|m| d.message.starts_with(m))),
                    "{name} cut at byte {cut}: nothing says what is left open: {found:#?}"
                );
            }
            // Every node that ended on its own line before the cut is still there, in place.
            let kept = named_spans(file.document().nodes());
            for (node, span) in &nodes {
                let ended = text[..span.end].ends_with(['\n', ';']);
                if ended && span.end <= cut {
                    assert!(
                        kept.iter().any(|(n, s)| n == node && s == span),
                        "{name} cut at byte {cut}: `{node}` at {span:?} was lost"
                    );
                }
            }
        }
    }
    // Most cuts leave a block open: the test is about recovery, not clean prefixes.
    assert!(
        open * 2 > cuts,
        "{open} of {cuts} cuts left a construct open"
    );
}

/// Garbled slice text, a node that must still be found after the break, and what the first
/// syntax diagnostic says.
const GARBLED: [(&str, &str, &str, &str); 16] = [
    (
        "unclosed-block",
        "sequence @main {\n  rate 25\n  track @v1 video {\n    clip @a from=@walk 0..+4s at 0\n}\n",
        "clip",
        "this { is never closed",
    ),
    (
        "stray-brace",
        "track @v1 video {\n  clip @a from=@walk 0..+4s at 0\n}\n}\ntrack @v2 video\n",
        "@v2",
        "this } closes nothing",
    ),
    (
        "unclosed-list",
        "sequence @main {\n  size [1920 1080\n  rate 25\n}\nsource @walk \"Walk\"\n",
        "@main",
        "this [ is never closed",
    ),
    (
        "stray-bracket",
        "sequence @main { size 1920 1080] }\nsource @walk\n",
        "@walk",
        "this ] closes no list",
    ),
    (
        "reserved-paren",
        "sequence @main { size (1920 1080) }\nsource @walk\n",
        "@walk",
        "`(` is reserved",
    ),
    (
        "unclosed-string",
        "source @walk \"Walk {\n  rate 25\n}\nsource @talk\n",
        "@talk",
        "this string is not closed on its line",
    ),
    (
        "missing-value",
        "clip @a from= {\n  crossfade 12\n}\nclip @b\n",
        "@b",
        "expected a value after from=",
    ),
    (
        "property-as-node",
        "sequence @main {\n  rate=25\n  size [1920 1080]\n}\n",
        "size",
        "a child node is its name, a space and its value",
    ),
    (
        "missing-key",
        "source @walk =25 { rate 25 }\n",
        "rate",
        "expected a key before =, such as rate=25",
    ),
    (
        "block-without-name",
        "{ rate 25 }\nsequence @main { rate 25 }\n",
        "@main",
        "expected a node name before this block",
    ),
    (
        "text-after-block",
        "clip @a { crossfade 12 } at=0\nclip @b\n",
        "@b",
        "a node ends after its block",
    ),
    (
        "bad-escape",
        "source @walk \"W\\qalk\\u{zz}\"\nsource @talk\n",
        "@talk",
        "unknown escape",
    ),
    (
        "unclosed-comment",
        "source @walk \"Walk\"\n/* the cut\nsequence @main { rate 25 }\n",
        "@walk",
        "this block comment is never closed",
    ),
    (
        "misplaced-slashdash",
        "clip @a from=@walk /- { crossfade 12 }\nclip @b\n",
        "@b",
        "",
    ),
    (
        "lone-equals",
        "sequence @main {\n  =\n  rate 25\n}\n",
        "rate",
        "expected a node name, found `=`",
    ),
    (
        "control-bytes",
        "source @walk \u{0}\u{7}\u{feff}\r\nsequence @main { rate 25 }\n",
        "@main",
        "",
    ),
];

#[test]
fn garbled_slice_text_recovers_with_diagnostics() {
    for (name, text, survivor, says) in GARBLED {
        invariants::check(text, 0, 0, "");
        let file = parse(text);
        let found = diagnostics(&file, FileId(0));
        let syntax: Vec<_> = found.iter().filter(|d| d.code == codes::SYNTAX).collect();
        if says.is_empty() {
            // Not every garble is a syntax error (a control byte is a word); then only the
            // invariants and the survivor are checked.
            assert!(found.len() <= text.len(), "{name}");
        } else {
            assert!(
                syntax.iter().any(|d| d.message.starts_with(says)),
                "{name}: no `{says}` in {syntax:#?}"
            );
        }
        // The rest of the file still parses: the node after the break is there.
        let mut names = Vec::new();
        for (node, _) in named_spans(file.document().nodes()) {
            names.push(node);
        }
        let mut stack: Vec<_> = file.document().nodes().collect();
        while let Some(n) = stack.pop() {
            if let Some(h) = n.handle() {
                names.push(h.text.to_string());
            }
            stack.extend(n.children());
        }
        assert!(
            names.iter().any(|n| n == survivor),
            "{name}: `{survivor}` was lost: {names:?}"
        );
    }
}

/// A small deterministic generator, so the mutations are the same on every run.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
    /// A character boundary of `text` at or before a random place.
    fn place(&mut self, text: &str) -> usize {
        let mut at = self.below(text.len() + 1);
        while !text.is_char_boundary(at) {
            at -= 1;
        }
        at
    }
}

#[test]
fn garbled_slice_files_keep_every_invariant() {
    // Pieces that break the syntax, inserted, deleted and replaced at random.
    const PIECES: [&str; 26] = [
        "{", "}", "[", "]", "(", ")", "\"", "#\"", "\"#", "\\", "\\\n", "//", "/*", "*/", "/-",
        ";", "=", "\n", "\r\n", "\t", "é", "\u{2028}", "\u{0}", "..", "@", ":",
    ];
    let texts: Vec<String> = clean_files().into_iter().map(|(_, t)| t).collect();
    let mut rng = Lcg(0x0007_ee1f_0a2e);
    let mut broken = 0;
    let rounds = 3000;
    for round in 0..rounds {
        let mut text = texts[round % texts.len()].clone();
        for _ in 0..1 + rng.below(4) {
            let at = rng.place(&text);
            let piece = PIECES[rng.below(PIECES.len())];
            match rng.below(3) {
                0 => text.insert_str(at, piece),
                1 => {
                    let end = rng.place(&text).max(at).min(at + 16);
                    let end = (end..=text.len())
                        .find(|&i| text.is_char_boundary(i))
                        .unwrap();
                    text.replace_range(at..end, "");
                }
                _ => {
                    let end = (at + 1..=text.len())
                        .find(|&i| text.is_char_boundary(i))
                        .unwrap_or(at);
                    text.replace_range(at..end, piece);
                }
            }
        }
        let (start, end) = (rng.place(&text), rng.place(&text));
        let piece = PIECES[rng.below(PIECES.len())];
        invariants::check(&text, start, end, piece);
        if !parse(&text).errors().is_empty() {
            broken += 1;
        }
    }
    // The pieces break most files: the test is about broken text.
    assert!(broken * 2 > rounds, "only {broken} of {rounds} were broken");
}

#[test]
fn garbage_and_deep_nesting_recover() {
    // Bytes no Veel file holds, read as text with bad bytes replaced.
    let mut rng = Lcg(0x0000_0bad_b17e);
    for _ in 0..500 {
        let bytes: Vec<u8> = (0..rng.below(300)).map(|_| rng.next() as u8).collect();
        let text = String::from_utf8_lossy(&bytes);
        let (start, end) = (rng.place(&text), rng.place(&text));
        invariants::check(&text, start, end, "}");
    }
    // Nesting far deeper than any file (the shared parser stops at 256): reported, with no
    // overflow of the stack.
    for (open, says) in [
        ("{", "blocks are nested too deeply"),
        ("[", "lists are nested too deeply"),
        ("a {", "blocks are nested too deeply"),
        ("a [", "lists are nested too deeply"),
    ] {
        let text = open.repeat(2_000);
        invariants::check(&text, 0, 1, "");
        let found = diagnostics(&parse(&text), FileId(0));
        assert!(
            found.iter().any(|d| d.message == says),
            "{open}: {:?}",
            found.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
    // A line of 64 KB, eight times the fuzz target's longest input, and a file of nothing but
    // line feeds.
    invariants::check(&"x".repeat(1 << 16), 10, 20, "\"");
    invariants::check(&"\n".repeat(10_000), 0, 0, "{");
}
