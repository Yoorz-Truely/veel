//! Veel's single-field patches: one value of a checked project rewritten in place, as the
//! smallest edit to the lossless text (spec, section 13; task 1a-04, for write-back's 1a-09).
//!
//! A patch names one value: a clip's `at`, the in or out point of its source range, its
//! crossfade's length, or any attribute or effect parameter as a word. It gives one
//! [`FileEdit`]: the span of that value's word and the word that replaces it. Every other byte of
//! every file stays as it is, comments, spacing, spelling and the other values included. A time
//! is written in the unit its word already uses (frames, `s`, `ms` or timecode) whenever that
//! unit names the new frame exactly, and in frames otherwise, as the printing rule says (section
//! 5): `at 5s` moved to frame 130 at 25 frames per second becomes `at 5.2s`, and a range written
//! `in..+length` keeps that form.
//!
//! ```
//! use veel_check::check;
//! use veel_edit::{ClipTime, set_clip_time};
//!
//! let text = "source @s media=\"m.mov\" rate=25 len=60s\n\
//!             sequence @main rate=25 size=[1920 1080] {\n  \
//!               track @v1 video {\n    \
//!                 clip @a from=@s 00:00:10:00..+5s at 5s // the first\n  \
//!               }\n\
//!             }\n";
//! let checked = check(&[("main.veel".into(), text.into())]);
//! let edit = set_clip_time(&checked, "a", ClipTime::At, 130).unwrap();
//! let after = edit.apply(text);
//! assert!(after.contains("clip @a from=@s 00:00:10:00..+5s at 5.2s // the first\n"));
//! let edit = set_clip_time(&checked, "a", ClipTime::Out, 300).unwrap();
//! assert!(edit.apply(text).contains("00:00:10:00..+2s at 5s"));
//! ```

use std::fmt;

use veel_check::{Checked, Clip, Sequence};
use veel_syntax::ast::{Entry, Node, Value};
use veel_syntax::{FileId, Span};
use veel_time::{Clock, Time, Unit};

/// One edit to one file: replace `span` with `replacement`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileEdit {
    /// The file.
    pub file: FileId,
    /// The bytes replaced: exactly one value's word.
    pub span: Span,
    /// What replaces them.
    pub replacement: String,
}

impl FileEdit {
    /// `text` (the file's) with the edit made.
    pub fn apply(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len() + self.replacement.len());
        out.push_str(&text[..self.span.start]);
        out.push_str(&self.replacement);
        out.push_str(&text[self.span.end..]);
        out
    }
}

/// Why a patch could not be made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditError {
    /// No element of the project has that handle.
    NoSuchElement(String),
    /// The element does not write that value, so there is no word to change.
    NotWritten(String),
    /// The value's clock is unknown: its sequence's or source's rate did not check.
    NoClock(String),
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EditError::NoSuchElement(h) => write!(f, "no element is {h}"),
            EditError::NotWritten(w) => {
                write!(f, "{w} is not written, so there is no word to change")
            }
            EditError::NoClock(w) => write!(
                f,
                "{w}: its rate did not check, so its time cannot be written"
            ),
        }
    }
}

impl std::error::Error for EditError {}

/// A time of a clip that a patch can set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipTime {
    /// The sequence frame of its first frame (a move).
    At,
    /// The first frame of its source range, its end kept (a trim of its head).
    In,
    /// The first frame after its source range, its start kept (a trim of its tail).
    Out,
    /// Its crossfade's length, in frames.
    Crossfade,
}

/// An element that holds an attribute or parameter, by handles without `@`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owner<'a> {
    /// A source.
    Source(&'a str),
    /// A sequence.
    Sequence(&'a str),
    /// A clip.
    Clip(&'a str),
    /// An effect of a clip: the clip's handle, then the effect's.
    Effect(&'a str, &'a str),
}

/// Sets `field` of the clip `clip` (its handle, without `@`) to `frames`, in the sequence's
/// frames: positions from frame 0, range ends as offsets into the source.
pub fn set_clip_time(
    checked: &Checked,
    clip: &str,
    field: ClipTime,
    frames: i64,
) -> Result<FileEdit, EditError> {
    let (seq, model) = find_clip(checked, clip)?;
    let node = clip_node(checked, seq, clip)?;
    let rate = seq
        .rate
        .ok_or_else(|| EditError::NoClock(format!("@{clip}")))?;
    let seq_clock = Clock::with_timecode(rate, rate, seq.start)
        .map_err(|_| EditError::NoClock(format!("@{clip}")))?;
    let not_written = |what: &str| EditError::NotWritten(format!("@{clip}'s {what}"));
    match field {
        ClipTime::At => {
            let v = attribute(node, "at").ok_or_else(|| not_written("`at`"))?;
            Ok(edit(seq.file, v, rewrite(&seq_clock, v, frames, false)))
        }
        ClipTime::Crossfade => {
            let line = node
                .children()
                .find(|c| c.name() == "crossfade")
                .ok_or_else(|| not_written("crossfade"))?;
            let v = line
                .args()
                .next()
                .ok_or_else(|| not_written("crossfade's length"))?;
            Ok(edit(seq.file, v, rewrite(&seq_clock, v, frames, true)))
        }
        ClipTime::In | ClipTime::Out => {
            let v = node
                .args()
                .find(|v| v.as_word().is_some_and(|w| w.contains("..")))
                .ok_or_else(|| not_written("source range"))?;
            let range = model
                .range
                .ok_or_else(|| not_written("checked source range"))?;
            let src = model
                .source
                .as_deref()
                .and_then(|s| checked.project.source(s))
                .ok_or_else(|| EditError::NoClock(format!("@{clip}'s source")))?;
            let src_rate = src
                .rate
                .ok_or_else(|| EditError::NoClock(format!("@{}", src.handle)))?;
            let clock = Clock::with_timecode(rate, src_rate, src.start)
                .map_err(|_| EditError::NoClock(format!("@{}", src.handle)))?;
            let word = v.as_word().expect("a range is a word");
            let (a, b) = word.split_once("..").expect("a range");
            let (start, end) = match field {
                ClipTime::In => (frames, range.end()),
                _ => (range.start(), frames),
            };
            let new_a = if field == ClipTime::In {
                write(&clock, a, start, false)
            } else {
                a.to_string()
            };
            let new_b = match b.strip_prefix('+') {
                Some(len) => format!("+{}", write(&clock, len, end - start, true)),
                None if field == ClipTime::Out => write(&clock, b, end, false),
                None => b.to_string(),
            };
            Ok(FileEdit {
                file: seq.file,
                span: v.span(),
                replacement: format!("{new_a}..{new_b}"),
            })
        }
    }
}

/// Sets the attribute or parameter `name` of `owner` to the word `text`, written where the file
/// already writes it: on the owner's line (`name=value`) or as a child line (`name value`). A
/// switch written alone gets its word after its name.
pub fn set_word(
    checked: &Checked,
    owner: Owner<'_>,
    name: &str,
    text: &str,
) -> Result<FileEdit, EditError> {
    let (file, node) = owner_node(checked, owner)?;
    if let Some(v) = attribute(node, name) {
        return Ok(edit(file, v, text.to_string()));
    }
    let line = node
        .children()
        .find(|c| c.name() == name)
        .ok_or_else(|| EditError::NotWritten(format!("`{name}`")))?;
    let at = line.name_token().expect("a line has a name").span.end;
    Ok(FileEdit {
        file,
        span: Span::empty(at),
        replacement: format!(" {text}"),
    })
}

fn edit(file: FileId, v: Value<'_>, replacement: String) -> FileEdit {
    FileEdit {
        file,
        span: v.span(),
        replacement,
    }
}

/// The word naming `frames` in the unit `v` is written in, or in frames.
fn rewrite(clock: &Clock, v: Value<'_>, frames: i64, duration: bool) -> String {
    write(clock, v.as_word().unwrap_or(""), frames, duration)
}

/// The word naming `frames` in the unit `old` is written in, when that unit names it exactly;
/// otherwise frames.
fn write(clock: &Clock, old: &str, frames: i64, duration: bool) -> String {
    let unit = Time::parse(old).map_or(Unit::Frames, |t| t.unit());
    clock
        .write(frames, unit, duration)
        .unwrap_or_else(|| frames.to_string())
}

/// The value of the attribute `name` written on `node`'s line or as one of its child lines.
fn attribute<'a>(node: Node<'a>, name: &str) -> Option<Value<'a>> {
    for entry in node.entries() {
        if let Entry::Prop { key, value } | Entry::Keyword { key, value } = entry
            && key.text == name
        {
            return Some(value);
        }
    }
    node.children()
        .find(|c| c.name() == name)
        .and_then(|c| c.args().next())
}

fn find_clip<'c>(checked: &'c Checked, clip: &str) -> Result<(&'c Sequence, &'c Clip), EditError> {
    checked
        .project
        .clips()
        .find(|(_, _, c)| c.handle == clip)
        .map(|(s, _, c)| (s, c))
        .ok_or_else(|| EditError::NoSuchElement(format!("@{clip}")))
}

fn has_handle(node: Node<'_>, handle: &str) -> bool {
    node.handle().is_some_and(|t| &t.text[1..] == handle)
}

fn clip_node<'c>(checked: &'c Checked, seq: &Sequence, clip: &str) -> Result<Node<'c>, EditError> {
    checked
        .parse(seq.file)
        .document()
        .nodes()
        .filter(|n| n.name() == "sequence" && has_handle(*n, &seq.handle))
        .flat_map(|s| s.children().filter(|t| t.name() == "track"))
        .flat_map(|t| t.children().filter(|c| c.name() == "clip"))
        .find(|c| has_handle(*c, clip))
        .ok_or_else(|| EditError::NoSuchElement(format!("@{clip}")))
}

fn owner_node<'c>(checked: &'c Checked, owner: Owner<'_>) -> Result<(FileId, Node<'c>), EditError> {
    let top = |kind: &str, handle: &str| -> Result<(FileId, Node<'c>), EditError> {
        for (i, parse) in checked.parses.iter().enumerate() {
            if let Some(n) = parse
                .document()
                .nodes()
                .find(|n| n.name() == kind && has_handle(*n, handle))
            {
                return Ok((FileId(i as u32), n));
            }
        }
        Err(EditError::NoSuchElement(format!("{kind} @{handle}")))
    };
    match owner {
        Owner::Source(h) => top("source", h),
        Owner::Sequence(h) => top("sequence", h),
        Owner::Clip(h) => {
            let (seq, _) = find_clip(checked, h)?;
            Ok((seq.file, clip_node(checked, seq, h)?))
        }
        Owner::Effect(c, e) => {
            let (seq, _) = find_clip(checked, c)?;
            let clip = clip_node(checked, seq, c)?;
            let effect = clip
                .children()
                .find(|n| n.name() == "effect" && has_handle(*n, e))
                .ok_or_else(|| EditError::NoSuchElement(format!("@{c}/@{e}")))?;
            Ok((seq.file, effect))
        }
    }
}
