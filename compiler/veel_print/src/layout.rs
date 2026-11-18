//! The layout pass: canonical whitespace and indentation, every other token kept as written.

use veel_syntax::{TokenKind, lex};

/// `text` with canonical layout (the crate's documentation lists the rules). Formatting twice
/// gives the same text.
pub fn format(text: &str) -> String {
    let (tokens, _) = lex(text);
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    let mut depth: usize = 0;
    let mut line_start = true;
    let mut blank = 0;
    // The whitespace seen since the last token on this line, as written.
    let mut space: Option<&str> = None;
    let mut prev: Option<TokenKind> = None;
    let mut continued = false;
    for token in tokens {
        let kind = token.kind;
        let t = &text[at..at + token.len as usize];
        at += token.len as usize;
        match kind {
            TokenKind::Whitespace => {
                if !line_start {
                    space = Some(t);
                }
            }
            TokenKind::Newline => {
                trim_line_end(&mut out);
                if line_start {
                    blank += 1;
                    if blank == 1 && !out.is_empty() {
                        out.push('\n');
                    }
                } else {
                    out.push('\n');
                    blank = 0;
                }
                continued = prev == Some(TokenKind::Continuation);
                line_start = true;
                space = None;
                prev = Some(kind);
            }
            _ => {
                if line_start {
                    let closing = matches!(kind, TokenKind::RBrace | TokenKind::RBracket);
                    let level = if closing {
                        depth.saturating_sub(1)
                    } else {
                        depth
                    };
                    out.push_str(&"  ".repeat(level + usize::from(continued)));
                    line_start = false;
                } else if let Some(p) = prev {
                    push_space(&mut out, p, kind, space);
                }
                out.push_str(t);
                match kind {
                    TokenKind::LBrace | TokenKind::LBracket => depth += 1,
                    TokenKind::RBrace | TokenKind::RBracket => depth = depth.saturating_sub(1),
                    _ => {}
                }
                space = None;
                prev = Some(kind);
            }
        }
    }
    trim_line_end(&mut out);
    while out.ends_with('\n') {
        out.pop();
    }
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

/// The space between two tokens on one line: what was written around `=`, none just inside
/// brackets or before `;`, one after `;` and around braces, and one where any was written.
fn push_space(out: &mut String, prev: TokenKind, kind: TokenKind, space: Option<&str>) {
    use TokenKind as K;
    if prev == K::Equals || kind == K::Equals {
        out.push_str(space.unwrap_or(""));
    } else if prev == K::LBracket || kind == K::RBracket || kind == K::Semicolon {
    } else if prev == K::Semicolon
        || matches!(kind, K::LBrace | K::RBrace)
        || prev == K::LBrace
        || space.is_some()
    {
        out.push(' ');
    }
}

/// Removes the spaces, tabs and carriage returns at the end of `out`'s last line.
fn trim_line_end(out: &mut String) {
    let kept = out.trim_end_matches([' ', '\t', '\r']).len();
    out.truncate(kept);
}
