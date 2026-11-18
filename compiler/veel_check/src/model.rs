//! The project's model: what the files say, every time in whole frames.

use veel_syntax::{FileId, Span};
use veel_time::{Range, Rate, Timecode};

/// A project's sources and sequences, in the order their files and lines give them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Project {
    /// The sources.
    pub sources: Vec<Source>,
    /// The sequences.
    pub sequences: Vec<Sequence>,
}

impl Project {
    /// The source whose handle, without `@`, is `handle`.
    pub fn source(&self, handle: &str) -> Option<&Source> {
        self.sources.iter().find(|s| s.handle == handle)
    }

    /// The sequence whose handle is `handle`.
    pub fn sequence(&self, handle: &str) -> Option<&Sequence> {
        self.sequences.iter().find(|s| s.handle == handle)
    }

    /// The clip whose handle is `handle`.
    pub fn clip(&self, handle: &str) -> Option<&Clip> {
        self.clips().map(|(_, _, c)| c).find(|c| c.handle == handle)
    }

    /// Every clip, with its sequence and track.
    pub fn clips(&self) -> impl Iterator<Item = (&Sequence, &Track, &Clip)> {
        self.sequences.iter().flat_map(|s| {
            s.tracks
                .iter()
                .flat_map(move |t| t.clips.iter().map(move |c| (s, t, c)))
        })
    }
}

/// A source: one medium.
#[derive(Clone, Debug, PartialEq)]
pub struct Source {
    /// Its handle, without `@`.
    pub handle: String,
    /// Its display name.
    pub name: Option<String>,
    /// The file it is in, and its node there.
    pub file: FileId,
    /// Its node's span.
    pub span: Span,
    /// Its media path, as written.
    pub media: Option<String>,
    /// Its content hash, as written.
    pub hash: Option<String>,
    /// Its rate.
    pub rate: Option<Rate>,
    /// Its length in frames at its own rate.
    pub len: Option<i64>,
    /// The timecode of its first frame.
    pub start: Timecode,
}

/// A sequence.
#[derive(Clone, Debug, PartialEq)]
pub struct Sequence {
    /// Its handle, without `@`.
    pub handle: String,
    /// Its display name.
    pub name: Option<String>,
    /// The file it is in.
    pub file: FileId,
    /// Its node's span.
    pub span: Span,
    /// Its rate.
    pub rate: Option<Rate>,
    /// Its frame size, width and height in pixels.
    pub size: Option<(u32, u32)>,
    /// The timecode of its frame 0.
    pub start: Timecode,
    /// Its tracks, top to bottom.
    pub tracks: Vec<Track>,
}

/// A track.
#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    /// Its handle, without `@`.
    pub handle: String,
    /// Its display name.
    pub name: Option<String>,
    /// Its node's span.
    pub span: Span,
    /// Its clips, in the author's order.
    pub clips: Vec<Clip>,
}

/// A clip.
#[derive(Clone, Debug, PartialEq)]
pub struct Clip {
    /// Its handle, without `@`.
    pub handle: String,
    /// Its node's span.
    pub span: Span,
    /// The handle of its source, without `@`.
    pub source: Option<String>,
    /// Its source range: offsets into the source, in the sequence's frames, half-open.
    pub range: Option<Range>,
    /// The sequence frame of its first frame.
    pub at: Option<i64>,
    /// Its effects, in the stack's order.
    pub effects: Vec<Effect>,
    /// Its crossfade's length in the sequence's frames.
    pub crossfade: Option<i64>,
}

impl Clip {
    /// The sequence frames it covers, `[at, at + length)`; empty when either is unknown.
    pub fn place(&self) -> Range {
        match (self.at, self.range) {
            (Some(at), Some(r)) => Range::new(at, at + r.len()).expect("a length is not negative"),
            _ => Range::new(0, 0).expect("empty"),
        }
    }
}

/// An effect in a clip's stack.
#[derive(Clone, Debug, PartialEq)]
pub struct Effect {
    /// Its handle, without `@`, unique within its clip.
    pub handle: String,
    /// Its asset's selector (`qtblend`).
    pub asset: String,
    /// The parameters it sets, in the order written.
    pub params: Vec<Param>,
}

/// A parameter an effect sets.
#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    /// Its Veel name.
    pub name: String,
    /// Its value or keys.
    pub value: ParamValue,
}

/// A parameter's value: one value, or keys.
#[derive(Clone, Debug, PartialEq)]
pub enum ParamValue {
    /// One value, as written (a switch written alone is `on`).
    Value(String),
    /// Keys, in time order.
    Keys(Vec<Key>),
}

/// One keyframe.
#[derive(Clone, Debug, PartialEq)]
pub struct Key {
    /// Its time in the clip, in frames from its first frame.
    pub frame: i64,
    /// Its value, as written.
    pub value: String,
    /// Its `ease=` word.
    pub ease: String,
}
