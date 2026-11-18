//! What must hold for any text, broken or not: the checks the fuzz target and the tests share.
//!
//! [`check`] parses a text, edits it, and panics, naming the invariant, if the tree loses or
//! reorders a byte, its tokens are not the lexer's, a lexer or parser error goes unreported, a
//! message is one Veel has not read ([`messages::KNOWN`]) or still carries Scowr's example, a
//! diagnostic's place or fix leaves the text, or an incremental reparse differs from a fresh
//! parse. The fuzz target (`fuzz/fuzz_targets/parse.rs` at the workspace root) calls it on
//! arbitrary input, where a panic is a crash; the recovery tests (`tests/recovery.rs`) call it
//! on every truncation and on garbled copies of the slice's files.

use scowr_errors::{FileId, SourceMap, Span, apply_edits};
use scowr_syntax::lexer::lex;

use crate::{codes, diagnostics, dump, messages, parse};

/// Checks every invariant on `text`, then on `text` with `start..end` (each moved back to a
/// character boundary and clamped to the text, then put in order) replaced by `replacement`.
///
/// # Panics
///
/// Panics, naming the invariant, if one does not hold.
pub fn check(text: &str, start: usize, end: usize, replacement: &str) {
    let mut file = parse(text);
    assert_eq!(file.print(), text, "the tree prints back the text");

    // The tree's tokens are the lexer's, contiguous, and cover every byte.
    let (lexed, lex_errors) = lex(text);
    let tokens = file.root().tokens();
    let mut at = 0;
    for t in &tokens {
        assert_eq!(t.span.start, at, "tokens are contiguous");
        at = t.span.end;
    }
    assert_eq!(at, text.len(), "tokens cover the text");
    let tree: Vec<_> = tokens.iter().map(|t| (t.kind, t.text.len())).collect();
    let lexer: Vec<_> = lexed.iter().map(|t| (t.kind, t.len as usize)).collect();
    assert_eq!(tree, lexer, "the tree's tokens are the lexer's");
    // Every lexer error reaches the parse's errors, at the same place.
    for e in &lex_errors {
        assert!(
            file.errors().iter().any(|p| p.span == e.span),
            "lexer error unreported: {}",
            e.message
        );
    }

    for e in file.errors() {
        assert!(
            messages::is_known(&e.message),
            "unread message: {}",
            e.message
        );
        assert_span(text, e.span);
    }
    let found = diagnostics(&file, FileId(0));
    for d in &found {
        assert!(codes::ALL.contains(&d.code), "unknown code {}", d.code);
        assert_span(text, d.primary.span);
        for fix in &d.fixes {
            let edits: Vec<_> = fix
                .edits
                .iter()
                .map(|e| {
                    assert_span(text, e.span);
                    (e.span, e.replacement.as_str())
                })
                .collect();
            let _ = apply_edits(text, &edits);
        }
        for example in messages::SCOWR_EXAMPLES {
            assert!(
                !d.message.contains(example),
                "Scowr's example in {}",
                d.message
            );
        }
    }
    let syntax = found.iter().filter(|d| d.code == codes::SYNTAX).count();
    assert_eq!(
        syntax,
        file.errors().len(),
        "every syntax error is reported"
    );
    assert!(
        found
            .windows(2)
            .all(|w| w[0].primary.span.start <= w[1].primary.span.start),
        "diagnostics are in text order"
    );
    let mut map = SourceMap::new();
    map.add("fuzz.veel", text);
    let _ = map.render_all(&found);
    let _ = dump(&file);

    // An edit reparses to the same tree and errors as a fresh parse of the new text.
    let floor = |mut i: usize| {
        i = i.min(text.len());
        while !text.is_char_boundary(i) {
            i -= 1;
        }
        i
    };
    let (a, b) = (floor(start), floor(end));
    let span = Span::new(a.min(b), a.max(b));
    let mut edited = text.to_string();
    edited.replace_range(span.range(), replacement);
    file.edit(span, replacement);
    let fresh = parse(&edited);
    assert_eq!(
        file.print(),
        edited,
        "the edited tree prints the edited text"
    );
    assert_eq!(
        dump(&file),
        dump(&fresh),
        "an edit reparses as a fresh parse"
    );
    assert_eq!(
        file.errors(),
        fresh.errors(),
        "an edit finds a fresh parse's errors"
    );
}

fn assert_span(text: &str, span: Span) {
    assert!(
        span.start <= span.end && span.end <= text.len(),
        "span in the text"
    );
    assert!(
        text.is_char_boundary(span.start) && text.is_char_boundary(span.end),
        "span on character boundaries"
    );
}
