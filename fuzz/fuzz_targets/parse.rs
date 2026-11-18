//! Fuzz target: any text, and an edit of it, keeps every invariant of Veel's syntax (task 1a-12).
//!
//! The first three bytes choose an edit: its start and end as fractions of the text, and what
//! to write there. The rest, read as UTF-8 with bad bytes replaced, is the text.
//! `veel_syntax::invariants::check` panics on any broken invariant (a lost byte, an unreported
//! or unread error, a place outside the text, a reparse that differs from a fresh parse), which
//! libFuzzer reports as a crash. Run: `tools/fuzz.sh [seconds]`, or
//! `cargo +nightly fuzz run parse` from the workspace root.
#![no_main]

use libfuzzer_sys::fuzz_target;

/// What an edit writes: nothing, the pieces that open and close Veel's constructs, and a line.
const PIECES: [&str; 16] = [
    "",
    "{",
    "}",
    "[",
    "]",
    "\"",
    "#\"",
    "\\\n",
    "/*",
    "/-",
    "\n",
    ";",
    "=",
    "..",
    "@a",
    "clip @a at 0",
];

fuzz_target!(|data: &[u8]| {
    let Some((head, body)) = data.split_first_chunk::<3>() else {
        return;
    };
    let text = String::from_utf8_lossy(body);
    let len = text.len();
    let start = head[0] as usize * len / 255;
    let end = head[1] as usize * len / 255;
    let replacement = PIECES[head[2] as usize % PIECES.len()];
    veel_syntax::invariants::check(&text, start, end, replacement);
});
