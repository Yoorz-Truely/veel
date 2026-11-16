//! The slice corpus and the slice schema spell alike: every node name, attribute, effect,
//! parameter, `ease` and choice word of `tests/slice/` is one the schema holds, where the schema
//! puts it. This is a spelling test, not the check: types, times and handles are `veel_check`'s
//! (1a-04). Its control plants a misspelling of each kind and must report each.

use std::path::{Path, PathBuf};

use veel_schema::slice::{self, EASE_WORDS, KEY_BLOCK_ATTRIBUTES, KEY_TIMES};
use veel_schema::{Effect, Kind, Value};
use veel_syntax::ast::{Entry, Node};
use veel_syntax::parse;

fn veel_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
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

/// The words of `node`'s positional arguments after its handle.
fn args(node: Node<'_>) -> Vec<String> {
    let skip = usize::from(node.handle().is_some());
    node.args()
        .skip(skip)
        .map(|v| v.text().to_string())
        .collect()
}

/// Checks a word against a `Word` value.
fn word_in(value: Value, word: &str) -> bool {
    match value {
        Value::Word(words) => words.contains(&word),
        _ => true,
    }
}

/// What in `text` the slice schema does not spell.
fn misspelt(text: &str) -> Vec<String> {
    let file = parse(text);
    let mut out = Vec::new();
    for node in file.document().nodes() {
        match slice::kind(&node.name()) {
            Some(kind) if kind.top_level => walk_kind(kind, node, &mut out),
            _ => out.push(format!("top-level node {}", node.name())),
        }
    }
    out
}

fn walk_kind(kind: &Kind, node: Node<'_>, out: &mut Vec<String>) {
    let words = args(node);
    // The words of a kind's arguments that are words (a track's kind, a timecode's counting,
    // an effect's asset).
    for (arg, word) in kind.args.iter().zip(&words) {
        if !word_in(arg.value, word) {
            out.push(format!("{} {}: {word}", kind.name, arg.name));
        }
    }
    let effect: Option<&Effect> = if kind.name == "effect" {
        let e = words.first().and_then(|w| slice::effect(w));
        if e.is_none() {
            out.push(format!("effect {:?}", words.first()));
        }
        e
    } else {
        None
    };
    for entry in node.entries() {
        let (key, value) = match entry {
            Entry::Arg(_) => continue,
            Entry::Prop { key, value } | Entry::Keyword { key, value } => {
                (key.text.to_string(), value.text().to_string())
            }
        };
        attribute(kind, effect, &key, &value, out);
    }
    for child in node.children() {
        let name = child.name().to_string();
        if kind.children.contains(&name.as_str()) {
            walk_kind(slice::kind(&name).unwrap(), child, out);
        } else if let Some(e) = effect {
            parameter(e, child, out);
        } else if kind.attribute(&name).is_some() {
            let value = child.args().next().map(|v| v.text().to_string());
            attribute(kind, None, &name, value.as_deref().unwrap_or(""), out);
        } else {
            out.push(format!("{} holds {name}", kind.name));
        }
    }
}

fn attribute(kind: &Kind, effect: Option<&Effect>, key: &str, value: &str, out: &mut Vec<String>) {
    if let Some(e) = effect {
        match e.loaded_params().find(|p| p.name == key) {
            Some(p) if p.value == Value::Choice && p.choice(value).is_none() => {
                out.push(format!("{key} {value}"));
            }
            Some(_) => {}
            None => out.push(format!("effect parameter {key}")),
        }
        return;
    }
    match kind.attribute(key) {
        Some(a) if !word_in(a.value, value) => out.push(format!("{}.{key} {value}", kind.name)),
        Some(_) => {}
        None => out.push(format!("{}.{key}", kind.name)),
    }
}

fn parameter(effect: &Effect, node: Node<'_>, out: &mut Vec<String>) {
    let name = node.name().to_string();
    let Some(param) = effect.loaded_params().find(|p| p.name == name) else {
        out.push(format!("effect parameter {name}"));
        return;
    };
    if param.value == Value::Choice {
        for word in args(node) {
            if param.choice(&word).is_none() {
                out.push(format!("{name} {word}"));
            }
        }
    }
    for entry in node.entries() {
        if let Entry::Prop { key, value } = entry {
            let ok = KEY_BLOCK_ATTRIBUTES.iter().any(|a| a.name == key.text)
                && KEY_TIMES.contains(&value.text());
            if !ok || !param.keyframes {
                out.push(format!("{name} {}={}", key.text, value.text()));
            }
        }
    }
    let key_kind = slice::kind("key").unwrap();
    for key in node.children() {
        if key.name() != "key" || !param.keyframes {
            out.push(format!("{name} holds {}", key.name()));
            continue;
        }
        for entry in key.entries() {
            if let Entry::Prop { key: k, value } = entry {
                let known = key_kind.attribute(k.text).is_some();
                if !known || (k.text == "ease" && !EASE_WORDS.contains(&value.text())) {
                    out.push(format!("key {}={}", k.text, value.text()));
                }
            }
        }
    }
}

#[test]
fn the_corpus_spells_only_what_the_slice_schema_holds() {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/slice");
    let files = veel_files(&corpus);
    assert_eq!(files.len(), 6);
    for path in files {
        let found = misspelt(&std::fs::read_to_string(&path).unwrap());
        assert!(found.is_empty(), "{}: {found:?}", path.display());
    }
}

/// The control: each kind of misspelling, planted in a good file, is reported.
#[test]
fn a_planted_misspelling_is_reported() {
    let good = "source @s media=\"m.mov\" rate=25 len=10s\n\
                sequence @q rate=25 size=[1920 1080] {\n  timecode non-drop start=01:00:00:00\n  \
                track @v1 video {\n    clip @a from=@s 0..+2s at 0 {\n      \
                effect @e qtblend compositing=screen {\n        \
                rotation { key 0 0; key 1s 90deg ease=cubic-in }\n        distort\n      }\n      \
                crossfade 12\n    }\n  }\n}\n";
    assert_eq!(misspelt(good), Vec::<String>::new());
    let plants = [
        ("len=10s", "length=10s"),
        ("sequence @q", "timeline @q"),
        ("non-drop", "drop-frame"),
        ("video {", "audio {"),
        ("effect @e", "filter @e"),
        ("qtblend", "brightness"),
        ("compositing=screen", "compositing=14"),
        ("ease=cubic-in", "ease=cubic"),
        ("distort", "rotate-center"),
        ("crossfade 12", "mix 12"),
        ("rotation {", "rotation time=sequence {"),
        ("key 1s", "keyframe 1s"),
    ];
    for (from, to) in plants {
        assert_eq!(good.matches(from).count(), 1, "{from}");
        let bad = good.replace(from, to);
        assert!(!misspelt(&bad).is_empty(), "{to} was not reported");
    }
}
