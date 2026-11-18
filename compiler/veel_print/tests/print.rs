//! The formatter and the canonical printer: every `.veel` file of the workspace's tests formats
//! to a fixed point keeping every token; the slice corpus is already canonical; the canonical
//! order keeps what a project means; and its controls show it moving what is out of order.

use std::path::{Path, PathBuf};

use veel_check::{ParamValue, Project, check};
use veel_print::{canonical, format};
use veel_syntax::{TokenKind, lex};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every `.veel` file under the workspace's `tests/`, sorted.
fn test_files() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root().join("tests")];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "veel") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// The tokens of `text` that are not whitespace or line ends, in order.
fn tokens(text: &str) -> Vec<String> {
    let (tokens, _) = lex(text);
    let mut at = 0;
    let mut out = Vec::new();
    for t in tokens {
        let s = &text[at..at + t.len as usize];
        at += t.len as usize;
        if !matches!(t.kind, TokenKind::Whitespace | TokenKind::Newline) {
            out.push(s.to_string());
        }
    }
    out
}

#[test]
fn the_layout_rules() {
    let cases = [
        (
            "clip   @a  [ 1  2 ]{x;y}  // note  \r\n",
            "clip @a [1 2] { x; y } // note\n",
        ),
        (
            "\n\nsource @s {\n\n\n      rate 25\n\t}\n\n",
            "source @s {\n\n  rate 25\n}\n",
        ),
        ("a key = value\nb k=v\n", "a key = value\nb k=v\n"),
        ("size [\n1920\n1080\n]\n", "size [\n  1920\n  1080\n]\n"),
        ("clip @a 0..+1s \\\n at 0\n", "clip @a 0..+1s \\\n  at 0\n"),
        ("a /* one\n   two */ b\n", "a /* one\n   two */ b\n"),
        ("", ""),
    ];
    for (from, to) in cases {
        assert_eq!(format(from), to, "{from:?}");
    }
}

#[test]
fn every_test_file_formats_to_a_fixed_point_keeping_every_token() {
    let files = test_files();
    assert!(files.len() >= 35, "{}", files.len());
    for path in files {
        let text = std::fs::read_to_string(&path).unwrap();
        let formatted = format(&text);
        assert_eq!(
            format(&formatted),
            formatted,
            "{}: format twice",
            path.display()
        );
        let printed = canonical(&text);
        assert_eq!(
            canonical(&printed),
            printed,
            "{}: canonical twice",
            path.display()
        );
        assert_eq!(
            format(&printed),
            printed,
            "{}: canonical is laid out",
            path.display()
        );
        for out in [formatted, printed] {
            // The canonical order moves tokens, and `at=12s` loses its `=` to `at 12s`.
            let sorted = |t: &str| {
                let mut v: Vec<String> = tokens(t).into_iter().filter(|t| t != "=").collect();
                v.sort();
                v
            };
            assert_eq!(
                sorted(&text),
                sorted(&out),
                "{}: a token changed",
                path.display()
            );
        }
        assert_eq!(tokens(&format(&text)), tokens(&text), "{}", path.display());
    }
}

#[test]
fn the_slice_corpus_is_canonical() {
    for path in test_files() {
        if path.starts_with(root().join("tests/slice")) {
            let text = std::fs::read_to_string(&path).unwrap();
            assert_eq!(canonical(&text), text, "{}", path.display());
        }
    }
}

/// What a project means, without where it is written: its sources, sequences, tracks, clips and
/// effects, parameters sorted by name.
fn meaning(p: &Project) -> String {
    let mut out = String::new();
    for s in &p.sources {
        out += &format!(
            "{} {:?} {:?} {:?} {:?} {:?} {:?}\n",
            s.handle, s.name, s.media, s.hash, s.rate, s.len, s.start
        );
    }
    for s in &p.sequences {
        out += &format!(
            "{} {:?} {:?} {:?} {:?}\n",
            s.handle, s.name, s.rate, s.size, s.start
        );
        for t in &s.tracks {
            out += &format!(" {} {:?}\n", t.handle, t.name);
            for c in &t.clips {
                out += &format!(
                    "  {} {:?} {:?} {:?} {:?}\n",
                    c.handle, c.source, c.range, c.at, c.crossfade
                );
                for e in &c.effects {
                    let mut params: Vec<String> = e
                        .params
                        .iter()
                        .map(|p| match &p.value {
                            ParamValue::Value(v) => format!("{}={v}", p.name),
                            ParamValue::Keys(k) => format!("{}={k:?}", p.name),
                        })
                        .collect();
                    params.sort();
                    out += &format!("   {} {} {params:?}\n", e.handle, e.asset);
                }
            }
        }
    }
    out
}

const SCRAMBLED: &str = "// main.veel: every line out of the schema's order
source @s len=10s rate=25 media=\"m.mov\" \"Shot\"

sequence @main size=[1920 1080] rate=25 {
  track @v1 video {
    clip @a at=0 from=@s 0..+2s {
      crossfade 12
      // the stack, in its order
      effect @e qtblend rotation=5deg compositing=screen {
        rect { key 0 [0 0 1920 1080]; key 10 [0 0 960 540] ease=smooth }
        distort
      }
      effect @f qtblend { rotation 1; compositing multiply }
    }
    clip @b from=@s 2s..+2s at 50
  }
  timecode non-drop start=01:00:00:00
}
";

const SORTED: &str = "// main.veel: every line out of the schema's order
source @s media=\"m.mov\" rate=25 len 10s \"Shot\"

sequence @main rate=25 size=[1920 1080] {
  timecode non-drop start=01:00:00:00
  track @v1 video {
    clip @a from=@s at 0 0..+2s {
      // the stack, in its order
      effect @e qtblend compositing=screen rotation=5deg {
        distort
        rect { key 0 [0 0 1920 1080]; key 10 [0 0 960 540] ease=smooth }
      }
      effect @f qtblend { compositing multiply; rotation 1 }
      crossfade 12
    }
    clip @b from=@s 2s..+2s at 50
  }
}
";

/// The control: a file with every line out of order comes out in the schema's order, means the
/// same, and checks as clean as before.
#[test]
fn the_canonical_order_moves_what_is_out_of_order_and_keeps_its_meaning() {
    let out = canonical(SCRAMBLED);
    assert_eq!(out, SORTED);
    let before = check(&[("main.veel".into(), SCRAMBLED.into())]);
    let after = check(&[("main.veel".into(), out.clone())]);
    assert!(before.is_clean(), "{}", before.render());
    assert!(after.is_clean(), "{}", after.render());
    assert_eq!(meaning(&before.project), meaning(&after.project));
    assert_eq!(canonical(&out), out);
}

#[test]
fn the_canonical_order_keeps_every_corpus_folders_meaning() {
    for folder in std::fs::read_dir(root().join("tests/slice")).unwrap() {
        let folder = folder.unwrap().path();
        let files = veel_check::project_files(&folder).unwrap();
        let texts: Vec<(String, String)> = files
            .iter()
            .map(|(n, p)| (n.clone(), std::fs::read_to_string(p).unwrap()))
            .collect();
        // Its layout spoilt: deeper indentation and whitespace at every line's end.
        let spoilt: Vec<(String, String)> = texts
            .iter()
            .map(|(n, t)| {
                (
                    n.clone(),
                    t.replace("\n  ", "\n      ").replace('\n', " \t\n"),
                )
            })
            .collect();
        let mended: Vec<(String, String)> = spoilt
            .iter()
            .map(|(n, t)| (n.clone(), canonical(t)))
            .collect();
        assert_eq!(mended, texts, "{}", folder.display());
        assert_eq!(
            meaning(&check(&spoilt).project),
            meaning(&check(&texts).project),
            "{}",
            folder.display()
        );
    }
}
