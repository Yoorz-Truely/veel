//! Veel's checker for the phase 1a slice: every name, kind, type, unit and handle of a project
//! against the slice's schema, with overlaps on a track, ranges past their source and the
//! crossfade's rules (spec, sections 14 and 17; task 1a-04).
//!
//! A project is a folder of `.veel` files read as one (spec, section 17, "The slice project"):
//! any file may hold any element, and a handle defined in one is found from every other. The
//! checker reads each file through `veel_syntax`, then the slice's kinds from `veel_schema`, and
//! converts every time to whole frames with `veel_time`, never rounding. Every mistake is a
//! [`Diagnostic`] with a stable code ([`codes`]), and a suggestion where one is clear: the nearest
//! name, the nearest exact choices, a free handle, the frame where an overlapping clip could
//! start. The syntax's own codes (`veel-syntax`, `veel-name`) are `veel_syntax`'s; this crate's
//! codes are its own and no code is in both lists (a test holds them apart).
//!
//! Besides its diagnostics, a check gives the project's [`Project`] model: each source, sequence,
//! track and clip with its times in whole frames, as far as its own lines could be read.
//! `veel_edit` uses it for each line's clock, and the live diff (task 1a-08) will compare two.
//!
//! # Use
//!
//! ```
//! use veel_check::check;
//!
//! let text = "source @s media=\"media/s.mov\" rate=25 len=10s\n\
//!             sequence @main rate=25 size=[1920 1080] {\n  \
//!               track @v1 video {\n    \
//!                 clip @a from=@s 0..+2s at 0\n    \
//!                 clip @b from=@s 0..+2s at 40\n  \
//!               }\n\
//!             }\n";
//! let checked = check(&[("main.veel".to_string(), text.to_string())]);
//! assert_eq!(
//!     checked.render(),
//!     "main.veel:5:5 error[veel-overlap]: clip @b overlaps @a by 10 frames on track @v1\n  \
//!        main.veel:4:5: @a covers frames 0 up to 50\n  \
//!        fix: start @b at 50, where @a ends\n"
//! );
//! assert_eq!(checked.project.clip("a").unwrap().place().len(), 50);
//! ```

mod model;
mod read;
mod run;
mod values;

use std::path::{Path, PathBuf};

pub use model::{Clip, Effect, Key, Param, ParamValue, Project, Sequence, Source, Track};
pub use veel_syntax::{Diagnostic, FileId, Parse, SourceMap};

/// The checker's diagnostic codes (spec, section 14). They are stable, so tools and AI can act on
/// them. The syntax's two codes are `veel_syntax::codes`; no code is in both lists.
pub mod codes {
    /// A node name the slice does not know, or a known kind where it cannot stand.
    pub const KIND: &str = "veel-kind";
    /// A property, child line or effect parameter its owner does not have.
    pub const UNKNOWN_PROPERTY: &str = "veel-unknown-property";
    /// A required handle, value or attribute left out, or a keyframe block with no key.
    pub const MISSING: &str = "veel-missing";
    /// A value of the wrong type or shape: a string for a word, a list of the wrong length, a
    /// value too many, a block where none may stand.
    pub const TYPE: &str = "veel-type";
    /// A word that is not one of its place's words.
    pub const ENUM: &str = "veel-enum";
    /// A unit its place cannot take, or no unit where one is needed.
    pub const UNIT: &str = "veel-unit";
    /// A time that does not land on a whole frame, or a rate written as a decimal.
    pub const TIME_INEXACT: &str = "veel-time-inexact";
    /// A value of the right type outside what its place holds: a source range past its source,
    /// a clip before frame 0 or of no frame, a negative size.
    pub const RANGE: &str = "veel-range";
    /// Two clips of one track that share a frame.
    pub const OVERLAP: &str = "veel-overlap";
    /// A handle that names nothing, names an element of the wrong kind, is not kebab-case, or
    /// is defined twice.
    pub const HANDLE: &str = "veel-handle";
    /// An attribute or line given twice, or a parameter given both a value and keys.
    pub const REPEATED: &str = "veel-repeated";
    /// A key whose time is not after the key before it.
    pub const KEY_ORDER: &str = "veel-key-order";
    /// A crossfade with no clip starting where its clip ends (board row B-44).
    pub const CROSSFADE_NEXT: &str = "veel-crossfade-next";
    /// The mixes at a clip's two ends that overlap, or a mix longer than its clip holds (B-44).
    pub const CROSSFADE_OVERLAP: &str = "veel-crossfade-overlap";

    /// Every code above, in this order.
    pub const ALL: [&str; 14] = [
        KIND,
        UNKNOWN_PROPERTY,
        MISSING,
        TYPE,
        ENUM,
        UNIT,
        TIME_INEXACT,
        RANGE,
        OVERLAP,
        HANDLE,
        REPEATED,
        KEY_ORDER,
        CROSSFADE_NEXT,
        CROSSFADE_OVERLAP,
    ];

    /// The checker's constant for a code `veel_time` gives (`TimeError::code`).
    pub fn of_time(code: &str) -> &'static str {
        ALL.iter().copied().find(|c| *c == code).unwrap_or(TYPE)
    }
}

/// A checked project: its files, their trees, every diagnostic and the model.
#[derive(Debug)]
pub struct Checked {
    /// The files, named relative to the project's folder, in the order given.
    pub sources: SourceMap,
    /// Each file's tree, by [`FileId`] order.
    pub parses: Vec<Parse>,
    /// The syntax's diagnostics and the checker's, in reading order.
    pub diagnostics: Vec<Diagnostic>,
    /// What the files say, as far as each element's own lines could be read.
    pub project: Project,
}

impl Checked {
    /// Whether no diagnostic is an error.
    pub fn is_clean(&self) -> bool {
        !self.diagnostics.iter().any(Diagnostic::is_error)
    }

    /// Every diagnostic as `veel check` prints it.
    pub fn render(&self) -> String {
        self.sources.render_all(&self.diagnostics)
    }

    /// The tree of `file`.
    pub fn parse(&self, file: FileId) -> &Parse {
        &self.parses[file.0 as usize]
    }
}

/// Checks the files of one project, each a name (relative to the project's folder) and its text.
pub fn check(files: &[(String, String)]) -> Checked {
    let mut sources = SourceMap::new();
    let mut parses = Vec::new();
    for (name, text) in files {
        sources.add(name.clone(), text.clone());
        parses.push(veel_syntax::parse(text));
    }
    let (mut diagnostics, project) = run::check(&parses);
    sources.sort(&mut diagnostics);
    Checked {
        sources,
        parses,
        diagnostics,
        project,
    }
}

/// The `.veel` files of the project folder `dir`, at any depth, each named relative to it with
/// `/` between folders, sorted. Veelindy's manifest, `veelindy.veel` at the root, is Veelindy's
/// to check and is left out (spec, section 17).
pub fn project_files(dir: &Path) -> std::io::Result<Vec<(String, PathBuf)>> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d)? {
            let p = e?.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "veel") {
                let rel = p.strip_prefix(dir).unwrap_or(&p);
                let name = rel
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                if name != "veelindy.veel" {
                    out.push((name, p));
                }
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Checks the project in the folder `dir` ([`project_files`]).
pub fn check_folder(dir: &Path) -> std::io::Result<Checked> {
    let mut files = Vec::new();
    for (name, path) in project_files(dir)? {
        files.push((name, std::fs::read_to_string(path)?));
    }
    Ok(check(&files))
}

#[cfg(test)]
mod tests;
