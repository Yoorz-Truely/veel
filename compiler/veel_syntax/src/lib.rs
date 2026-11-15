//! Veel's syntax: the lexer and lossless syntax tree Veel shares with Scowr, with Veel's codes.
//!
//! The scaffold of task 0-01. The adapter itself (re-exports, Veel's `veel-syntax` code and the
//! `veel-name` check) is task 1a-01, modelled on Null's `null_syntax` (V17). Only this crate
//! names Scowr's crates.

#[cfg(test)]
mod tests {
    /// Scowr's front end resolves through `../Scowr` and links: a node parses with no error and
    /// prints back byte for byte.
    #[test]
    fn scowr_syntax_links() {
        let text = "clip @a {\n  from @source\n}\n";
        let file = scowr_syntax::parse(text);
        assert!(file.errors().is_empty());
        assert_eq!(file.print(), text);
        let _ = scowr_errors::FileId(0);
    }
}
