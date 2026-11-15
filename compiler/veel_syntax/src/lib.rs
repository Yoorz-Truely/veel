//! Veel's syntax: the lexer and lossless syntax tree Veel shares with Scowr, with Veel's codes.
//!
//! Veel writes its files in Scowr's node syntax (spec, section 4; V17), and so uses Scowr's
//! front-end crates, `scowr_syntax` and `scowr_errors`, by path from `../Scowr`. This crate is
//! the only place Veel names them: every other Veel crate reads syntax through it, so when the
//! shared crates move to their own repository only this crate's manifest changes. What is shared
//! and what is Veel's own is written in `AGENTS.md` ("The shared front end"). It mirrors Null's
//! `null_syntax`.
//!
//! What this crate adds is Veel's side of the boundary: diagnostics carry Veel's code
//! (`veel-syntax`, spec section 14) instead of Scowr's, with Veel's examples in their wording
//! ([`messages`]); names are checked to be kebab-case (`veel-name`, module [`names`]); and
//! [`dump`] prints a tree for tests. The meaning of a word (frames, timecode, ranges, handles,
//! kinds) comes from Veel's schema and `veel_time`, never from the shared syntax.
//!
//! # Use
//!
//! ```
//! use veel_syntax::{FileId, parse};
//!
//! let text = "clip @a from=@interview 00:00:10:00..+5s at 0 // the first\n";
//! let file = parse(text);
//! assert!(veel_syntax::diagnostics(&file, FileId(0)).is_empty());
//! assert_eq!(file.print(), text); // lossless
//!
//! let clip = file.document().nodes().next().unwrap();
//! assert_eq!(clip.name(), "clip");
//! assert_eq!(clip.handle().unwrap().text, "@a");
//! ```

pub use scowr_errors::{
    Diagnostic, FileId, Fix, Label, LineCol, Location, Severity, SourceFile, SourceMap, Span,
    TextEdit, apply_edits, closest, edit_distance,
};
pub use scowr_syntax::lexer::{LexError, Token, lex};
pub use scowr_syntax::{
    GreenElement, GreenNode, GreenToken, KEYWORDS, NodeKind, Parse, ParseError, Reparsed,
    SyntaxElement, SyntaxFix, SyntaxNode, SyntaxToken, TokenKind, ast, escape, raw_value, unescape,
};

pub mod messages;
pub mod names;

/// Veel's diagnostic codes for the syntax (spec, section 14). They are stable, so tools and AI
/// can act on them.
pub mod codes {
    /// Text that does not fit the node syntax.
    pub const SYNTAX: &str = "veel-syntax";
    /// A node name or property key that is not kebab-case.
    pub const NAME: &str = "veel-name";
    /// Every code this crate reports.
    pub const ALL: [&str; 2] = [SYNTAX, NAME];
}

/// Parses a whole Veel file into its lossless tree, recovering from errors.
pub fn parse(text: &str) -> Parse {
    scowr_syntax::parse(text)
}

/// The problems of `parse` as diagnostics in `file`, in text order: the syntax errors, with
/// Veel's code [`codes::SYNTAX`], the shared front end's fixes and Veel's wording
/// ([`messages`]), and the names that are not kebab-case ([`codes::NAME`], [`names::check`]).
pub fn diagnostics(parse: &Parse, file: FileId) -> Vec<Diagnostic> {
    let mut out: Vec<Diagnostic> = parse
        .errors()
        .iter()
        .map(|e| {
            let (message, fix) = messages::reword(parse, e, parse.fix(e));
            let d = Diagnostic::error(codes::SYNTAX, file, e.span, message);
            match fix {
                Some(f) => d.with_fix_replace(f.message, f.span, f.replacement),
                None => d,
            }
        })
        .collect();
    out.extend(names::check(parse, file));
    out.sort_by_key(|d| (d.primary.span.start, d.primary.span.end));
    out
}

/// The tree of `parse` as indented text, one element per line: each node's kind and span, each
/// token's kind, span and text (escaped). Every byte appears in exactly one token, so the dump
/// shows the whole file. For tests.
pub fn dump(parse: &Parse) -> String {
    let mut out = String::new();
    dump_node(parse.root(), 0, &mut out);
    out
}

fn dump_node(node: SyntaxNode<'_>, depth: usize, out: &mut String) {
    use std::fmt::Write;
    let span = node.span();
    let indent = "  ".repeat(depth);
    let _ = writeln!(
        out,
        "{indent}{:?}@{}..{}",
        node.kind(),
        span.start,
        span.end
    );
    for child in node.children() {
        match child {
            SyntaxElement::Node(n) => dump_node(n, depth + 1, out),
            SyntaxElement::Token(t) => {
                let _ = writeln!(
                    out,
                    "{indent}  {:?}@{}..{} {:?}",
                    t.kind, t.span.start, t.span.end, t.text
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Scowr's front end resolves through `../Scowr` and links: a node parses with no error and
    /// prints back byte for byte.
    #[test]
    fn scowr_syntax_links() {
        let text = "clip @a {\n  from @source\n}\n";
        let file = parse(text);
        assert!(diagnostics(&file, FileId(0)).is_empty());
        assert_eq!(file.print(), text);
    }

    /// Scowr's own code never reaches a Veel user: every syntax error is `veel-syntax`.
    #[test]
    fn syntax_errors_carry_veels_code() {
        let file = parse("clip @a {\n  at 0\n");
        let found = diagnostics(&file, FileId(0));
        assert!(!found.is_empty());
        assert!(found.iter().all(|d| d.code == codes::SYNTAX), "{found:?}");
        assert_eq!(found.len(), file.errors().len());
    }

    /// Diagnostics come in text order, syntax and names mixed.
    #[test]
    fn diagnostics_are_in_text_order() {
        let file = parse("Clip @a ]\nTrack @v1\n");
        let found = diagnostics(&file, FileId(0));
        let codes: Vec<_> = found.iter().map(|d| d.code).collect();
        assert_eq!(codes, [codes::NAME, codes::SYNTAX, codes::NAME]);
    }
}
