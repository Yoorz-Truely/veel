//! Veel's lossless formatter and canonical printer, both on the lossless syntax tree (spec,
//! section 12; task 1a-04).
//!
//! - [`format`] is the layout: two spaces of indentation per open block and list, one space
//!   between entries, none just inside `[ ]`, one on each side inside a `{ }` that stays on one
//!   line, none before `;`, whitespace around `=` as written, no trailing whitespace, LF line
//!   ends, at most one blank line in a row, none at the start and one line feed at the end, a
//!   line a `\` continues one level deeper. It keeps every other token as written, comments
//!   included, so the file means the same. It is Null's layout (`null-v0.md`, section 15), which
//!   Veel's section 12 follows.
//! - [`canonical`] is the canonical form the `veel fmt` command writes: `at`, `to` and `len`
//!   written as keywords (`at 12s`, section 4), each line's `key=value`s and each block's lines
//!   in the schema's order (a kind's attributes, then the kinds it holds; an effect's parameters
//!   in the order of the element Kdenlive loads), then the layout. Lines of one name keep their
//!   order among themselves (tracks, clips, effects and keys mean something by theirs); lines
//!   the schema does not know, and lines commented out with `/-`, come last as written; a
//!   comment just above a line moves with it. A file that does not parse only gets the layout.
//!
//! Defaults are not left out yet: section 12's default is the value Kdenlive gives a new
//! element, which the census has not measured for the slice (Annilay's board row B-41), and a
//! line dropped for the asset file's stand-in would change meaning once it is measured.
//!
//! ```
//! use veel_print::{canonical, format};
//!
//! assert_eq!(format("clip   @a  [ 1  2 ]{x;y}  // note  \r\n"), "clip @a [1 2] { x; y } // note\n");
//! assert_eq!(
//!     canonical("source @s len=10s rate=25 media=\"m.mov\"\n"),
//!     "source @s media=\"m.mov\" rate=25 len 10s\n"
//! );
//! ```

mod canonical;
mod layout;

pub use canonical::canonical;
pub use layout::format;
