//! The slice corpus checks clean: every folder of `tests/slice/` at the workspace root, read as one
//! project, has no diagnostic at all, and its model holds the frames the spec's section 17 works
//! out for its worked examples. Its controls plant one mistake in a copy of a folder, and the
//! same check must report it.

use std::path::{Path, PathBuf};

use veel_check::{ParamValue, check, check_folder, project_files};

fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/slice")
}

fn folders() -> Vec<PathBuf> {
    let mut out: Vec<_> = std::fs::read_dir(corpus())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir())
        .collect();
    out.sort();
    out
}

#[test]
fn every_corpus_folder_checks_with_no_diagnostic() {
    let folders = folders();
    assert_eq!(folders.len(), 5);
    for folder in folders {
        let checked = check_folder(&folder).unwrap();
        assert!(
            checked.diagnostics.is_empty(),
            "{}:\n{}",
            folder.display(),
            checked.render()
        );
        assert!(
            !checked.project.sequences.is_empty(),
            "{}",
            folder.display()
        );
    }
}

#[test]
fn the_worked_examples_hold_the_frames_the_spec_works_out() {
    // Two clips: @a takes source frames 250 to 375 at sequence frames 0 to 125; @b source
    // frames 1500 to 1600 at 125 to 225; @a's crossfade is 12 frames.
    let p = check_folder(&corpus().join("two-clips")).unwrap().project;
    let a = p.clip("a").unwrap();
    assert_eq!(
        (a.range.unwrap().start(), a.range.unwrap().end()),
        (250, 375)
    );
    assert_eq!((a.place().start(), a.place().end()), (0, 125));
    assert_eq!(a.crossfade, Some(12));
    let b = p.clip("b").unwrap();
    assert_eq!(
        (b.range.unwrap().start(), b.range.unwrap().end()),
        (1500, 1600)
    );
    assert_eq!((b.place().start(), b.place().end()), (125, 225));
    let keys = match &a.effects[0].params[0].value {
        ParamValue::Keys(k) => k
            .iter()
            .map(|k| (k.frame, k.ease.as_str()))
            .collect::<Vec<_>>(),
        other => panic!("{other:?}"),
    };
    assert_eq!(keys, [(0, "linear"), (50, "cubic-in")]);
    assert_eq!(p.source("interview").unwrap().len, Some(15000));

    // A picture in picture at 30000/1001: the drone's 00:00:12:00 is sequence frame 360 of it;
    // @inset sits at 60 to 210; @host-a is the host's frames 1950 to 2160.
    let p = check_folder(&corpus().join("picture-in-picture"))
        .unwrap()
        .project;
    let inset = p.clip("inset").unwrap();
    assert_eq!(inset.range.unwrap().start(), 360);
    assert_eq!((inset.place().start(), inset.place().end()), (60, 210));
    let host_a = p.clip("host-a").unwrap();
    assert_eq!(
        (host_a.range.unwrap().start(), host_a.range.unwrap().end()),
        (1950, 2160)
    );
    assert_eq!(p.sequence("main").unwrap().tracks[0].handle, "v2");
}

/// A copy of a corpus folder with `from` replaced by `to` in its file `file`.
fn planted(folder: &str, file: &str, from: &str, to: &str) -> String {
    let dir = corpus().join(folder);
    let files: Vec<(String, String)> = project_files(&dir)
        .unwrap()
        .into_iter()
        .map(|(name, p)| {
            let text = std::fs::read_to_string(p).unwrap();
            if name == file {
                assert!(text.contains(from), "{from}");
                (name, text.replacen(from, to, 1))
            } else {
                (name, text)
            }
        })
        .collect();
    check(&files).render()
}

/// The control: one planted mistake of each kind the done-when names, each reported.
#[test]
fn planted_mistakes_in_the_corpus_are_reported() {
    let main = "sequences/main.veel";
    let cases = [
        // An overlap on a track: @b moved one frame into @a.
        ("two-clips", main, "at 5s", "at 124", "veel-overlap"),
        // A range past its source: 600 s of interview, @b reading to 601 s.
        (
            "two-clips",
            main,
            "00:01:00:00..+4s",
            "00:09:59:00..+2s",
            "veel-range",
        ),
        // An unknown name, with its suggestion.
        (
            "two-clips",
            main,
            "rect {",
            "rekt {",
            "veel-unknown-property",
        ),
        // An unknown handle.
        (
            "picture-in-picture",
            main,
            "from=@drone",
            "from=@drones",
            "veel-handle",
        ),
        // A duplicate handle.
        ("two-clips", main, "clip @b", "clip @a", "veel-handle"),
        // A wrong type, a wrong unit, a wrong kind.
        (
            "one-clip",
            main,
            "size=[1920 1080]",
            "size=[1920]",
            "veel-type",
        ),
        ("one-clip", main, "len 10s", "len 10px", "veel-unit"),
        ("one-clip", main, "track @v1", "trak @v1", "veel-kind"),
        // A time off the frame: 9 s at 30000/1001 is 269.73 frames.
        (
            "picture-in-picture",
            main,
            "key 270",
            "key 9s",
            "veel-time-inexact",
        ),
        // The crossfade's two errors (B-44).
        ("two-clips", main, "at 5s", "at 6s", "veel-crossfade-next"),
        (
            "crossfades",
            main,
            "crossfade 1s",
            "crossfade 8s",
            "veel-crossfade-overlap",
        ),
    ];
    for (folder, file, from, to, code) in cases {
        let found = planted(folder, file, from, to);
        assert!(
            found.contains(&format!("error[{code}]")),
            "{folder}: {from} -> {to}: wanted {code}, got:\n{found}"
        );
    }
}
