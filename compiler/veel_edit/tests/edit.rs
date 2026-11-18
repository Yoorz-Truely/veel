//! Single-field patches on the slice corpus. The property: a random one-field edit of a random
//! clip of a random corpus folder replaces exactly one word, keeps every other byte, every
//! comment and the word's unit wherever that unit names the new value, and the checker reads the
//! new value back.

use std::path::{Path, PathBuf};

use proptest::prelude::*;
use veel_check::{Checked, check, project_files};
use veel_edit::{ClipTime, EditError, FileEdit, Owner, set_clip_time, set_word};
use veel_syntax::{TokenKind, lex};
use veel_time::{Time, Unit};

fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/slice")
}

/// Each corpus folder's files, by name and text.
fn folders() -> Vec<Vec<(String, String)>> {
    let mut dirs: Vec<_> = std::fs::read_dir(corpus())
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    dirs.sort();
    dirs.iter()
        .map(|d| {
            project_files(d)
                .unwrap()
                .into_iter()
                .map(|(n, p)| (n, std::fs::read_to_string(p).unwrap()))
                .collect()
        })
        .collect()
}

/// `files` with `edit` made.
fn applied(files: &[(String, String)], edit: &FileEdit) -> Vec<(String, String)> {
    let mut out = files.to_vec();
    let text = &mut out[edit.file.0 as usize].1;
    *text = edit.apply(text);
    out
}

/// The comments of `text`, in order.
fn comments(text: &str) -> Vec<String> {
    let (tokens, _) = lex(text);
    let mut at = 0;
    let mut out = Vec::new();
    for t in tokens {
        let s = &text[at..at + t.len as usize];
        at += t.len as usize;
        if matches!(t.kind, TokenKind::LineComment | TokenKind::BlockComment) {
            out.push(s.to_string());
        }
    }
    out
}

/// The units of a time word's parts: one for a time, two for a range.
fn units(word: &str) -> Vec<Unit> {
    let unit = |w: &str| Time::parse(w).map_or(Unit::Frames, |t| t.unit());
    match word.split_once("..") {
        Some((a, b)) => vec![unit(a), unit(b.trim_start_matches('+'))],
        None => vec![unit(word)],
    }
}

/// The value the checker reads for `field` of `clip`.
fn read_back(checked: &Checked, clip: &str, field: ClipTime) -> Option<i64> {
    let c = checked.project.clip(clip)?;
    match field {
        ClipTime::At => c.at,
        ClipTime::In => c.range.map(|r| r.start()),
        ClipTime::Out => c.range.map(|r| r.end()),
        ClipTime::Crossfade => c.crossfade,
    }
}

const FIELDS: [ClipTime; 4] = [
    ClipTime::At,
    ClipTime::In,
    ClipTime::Out,
    ClipTime::Crossfade,
];

/// One random edit's checks; `pick` chooses the folder, clip, field and value.
fn one_edit(folder: usize, clip: usize, field: usize, pick: u32) -> Result<(), TestCaseError> {
    let all = folders();
    let files = &all[folder % all.len()];
    let before = check(files);
    prop_assert!(before.is_clean(), "{}", before.render());
    let clips: Vec<_> = before.project.clips().map(|(_, _, c)| c.clone()).collect();
    let c = &clips[clip % clips.len()];
    let field = FIELDS[field % FIELDS.len()];
    let range = c.range.unwrap();
    let source = before.project.source(c.source.as_deref().unwrap()).unwrap();
    let (seq, _, _) = before
        .project
        .clips()
        .find(|(_, _, x)| x.handle == c.handle)
        .unwrap();
    let src_len = veel_time_frames(source, seq.rate.unwrap());
    let pick = i64::from(pick);
    let frames = match field {
        ClipTime::At => pick % 5000,
        ClipTime::In => (range.end() - 1 - pick % 500).max(0),
        ClipTime::Out => (range.start() + 1 + pick % 500).min(src_len),
        ClipTime::Crossfade => 1 + pick % 30,
    };
    let edit = match set_clip_time(&before, &c.handle, field, frames) {
        Ok(e) => e,
        Err(EditError::NotWritten(_)) if field == ClipTime::Crossfade => return Ok(()),
        Err(e) => return Err(TestCaseError::fail(format!("{e}"))),
    };
    let text = &files[edit.file.0 as usize].1;
    let old = &text[edit.span.start..edit.span.end];
    // Exactly one word is replaced, by one word.
    let (tokens, _) = lex(old);
    prop_assert_eq!(tokens.len(), 1, "{:?} is not one word", old);
    prop_assert_eq!(tokens[0].kind, TokenKind::Word);
    prop_assert!(!edit.replacement.is_empty() && !edit.replacement.contains(char::is_whitespace));
    // Every other byte, and every comment, stays.
    let after = applied(files, &edit);
    let new = &after[edit.file.0 as usize].1;
    prop_assert_eq!(&new[..edit.span.start], &text[..edit.span.start]);
    prop_assert_eq!(
        &new[edit.span.start + edit.replacement.len()..],
        &text[edit.span.end..]
    );
    for ((_, a), (_, b)) in files.iter().zip(&after) {
        prop_assert_eq!(comments(a), comments(b));
    }
    // The unit is kept wherever it names the new value; frames otherwise.
    let (was, now) = (units(old), units(&edit.replacement));
    for (w, n) in was.iter().zip(&now) {
        prop_assert!(
            w == n || *n == Unit::Frames,
            "{} -> {}",
            old,
            edit.replacement
        );
    }
    // The checker reads the new value back.
    let checked = check(&after);
    prop_assert!(checked.parses.iter().all(|p| p.errors().is_empty()));
    prop_assert_eq!(
        read_back(&checked, &c.handle, field),
        Some(frames),
        "{} -> {}",
        old,
        edit.replacement
    );
    Ok(())
}

/// A source's length in a sequence's frames, rounded down.
fn veel_time_frames(source: &veel_check::Source, seq: veel_time::Rate) -> i64 {
    let secs = source.rate.unwrap().seconds(source.len.unwrap());
    seq.frames_exact(secs).unwrap().floor() as i64
}

proptest! {
    #[test]
    fn a_one_field_edit_changes_only_its_word(folder in 0usize..5, clip in 0usize..16, field in 0usize..4, pick in 0u32..100_000) {
        one_edit(folder, clip, field, pick)?;
    }
}

/// Every clip of the corpus, every field, a few values: the same checks, exhaustively.
#[test]
fn every_clip_and_field_of_the_corpus() {
    let n = folders().len();
    for folder in 0..n {
        let clips = check(&folders()[folder]).project.clips().count();
        for clip in 0..clips {
            for field in 0..FIELDS.len() {
                for pick in [0, 1, 7, 24, 499] {
                    one_edit(folder, clip, field, pick).unwrap();
                }
            }
        }
    }
}

#[test]
fn a_time_keeps_its_unit_where_it_can_and_frames_otherwise() {
    let files = &folders()[4]; // two-clips
    let checked = check(files);
    // `at 5s`, moved to frame 130, is `5.2s`; to frame 131, 5.24 s; 25 fps make each exact.
    let e = set_clip_time(&checked, "b", ClipTime::At, 130).unwrap();
    assert_eq!(e.replacement, "5.2s");
    // The range `00:01:00:00..+4s` trimmed at its head to source frame 1510 keeps its end.
    let e = set_clip_time(&checked, "b", ClipTime::In, 1510).unwrap();
    assert_eq!(e.replacement, "00:01:00:10..+3.6s");
    // A position in frames stays in frames; one in timecode stays in timecode.
    let files = &folders()[3]; // picture-in-picture
    let checked = check(files);
    let e = set_clip_time(&checked, "host-b", ClipTime::At, 270).unwrap();
    assert_eq!(e.replacement, "270");
    let e = set_clip_time(&checked, "inset", ClipTime::At, 90).unwrap();
    assert_eq!(e.replacement, "01:00:03:00");
}

#[test]
fn a_word_is_set_where_it_is_written() {
    let files = &folders()[3];
    let checked = check(files);
    let e = set_word(
        &checked,
        Owner::Effect("inset", "pip"),
        "compositing",
        "multiply",
    )
    .unwrap();
    let text = &files[e.file.0 as usize].1;
    assert_eq!(&text[e.span.start..e.span.end], "screen");
    let e = set_word(&checked, Owner::Source("host"), "rate", "25").unwrap();
    assert_eq!(
        &files[e.file.0 as usize].1[e.span.start..e.span.end],
        "30000/1001"
    );
    // A switch written alone gets its word.
    let files = &folders()[1]; // every-parameter
    let checked = check(files);
    let e = set_word(&checked, Owner::Effect("plain", "all"), "distort", "off").unwrap();
    let after = e.apply(&files[0].1);
    assert!(
        after.contains("        distort off\n        rect"),
        "{after}"
    );
    assert!(check(&applied(files, &e)).is_clean());
    // What is not written cannot be changed in place.
    assert_eq!(
        set_word(&checked, Owner::Clip("plain"), "crossfade", "2"),
        Err(EditError::NotWritten("`crossfade`".into()))
    );
    assert!(matches!(
        set_clip_time(&checked, "nobody", ClipTime::At, 0),
        Err(EditError::NoSuchElement(_))
    ));
}
