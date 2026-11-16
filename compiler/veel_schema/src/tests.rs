//! The slice schema holds together: names are kebab-case and unique, every parameter cites the
//! file, tag and line it was read from, and each copied attribute agrees with the form Veel
//! gives it. Nothing here reads Kdenlive's files (Veel's `AGENTS.md`); 1b-04's gate compares the
//! copy with the schema Annilay generates.

use std::collections::BTreeSet;

use super::slice::{
    COMPOSITING, EASE_WORDS, EASES, EFFECTS, KEY_BLOCK_ATTRIBUTES, KINDS, QTBLEND, QTBLEND_FILE,
};
use super::*;

#[test]
fn kebab_case_is_recognised() {
    for good in [
        "rect",
        "rotate-anchor",
        "start-timecode",
        "v2",
        "bitwise-not-xor",
    ] {
        assert!(is_kebab(good), "{good}");
    }
    // The control: each of these is refused.
    for bad in [
        "",
        "rotate_anchor",
        "Rect",
        "a--b",
        "-a",
        "a-",
        "2d",
        "rotate anchor",
    ] {
        assert!(!is_kebab(bad), "{bad:?}");
    }
}

#[test]
fn the_slice_has_its_eight_kinds() {
    let names: Vec<_> = KINDS.iter().map(|k| k.name).collect();
    assert_eq!(
        names,
        [
            "source",
            "sequence",
            "timecode",
            "track",
            "clip",
            "effect",
            "crossfade",
            "key"
        ]
    );
    let top: Vec<_> = KINDS
        .iter()
        .filter(|k| k.top_level)
        .map(|k| k.name)
        .collect();
    assert_eq!(top, ["source", "sequence"]);
}

#[test]
fn kinds_and_attributes_are_kebab_case_unique_and_resolve() {
    let mut kinds = BTreeSet::new();
    for kind in KINDS {
        assert!(is_kebab(kind.name), "{}", kind.name);
        assert!(kinds.insert(kind.name), "{} twice", kind.name);
        let mut attrs = BTreeSet::new();
        for a in kind.attributes {
            assert!(is_kebab(a.name), "{}.{}", kind.name, a.name);
            assert!(attrs.insert(a.name), "{}.{} twice", kind.name, a.name);
            // A required attribute has no default, and an optional one says what leaving it out
            // means, except where it has no meaning to leave out (a hash).
            if a.required {
                assert!(a.default.is_none(), "{}.{}", kind.name, a.name);
            }
            if let Value::Handle(target) = a.value {
                assert!(slice::kind(target).is_some(), "{}.{}", kind.name, a.name);
            }
            if let Value::Word(words) = a.value {
                assert!(
                    words.iter().all(|w| is_kebab(w)),
                    "{}.{}",
                    kind.name,
                    a.name
                );
            }
        }
        for child in kind.children {
            assert!(
                slice::kind(child).is_some_and(|c| !c.top_level),
                "{} holds {child}",
                kind.name
            );
        }
    }
    // Every kind but the top-level ones is held by some other kind, or is `key`, which a
    // parameter's block holds.
    for kind in KINDS.iter().filter(|k| !k.top_level && k.name != "key") {
        assert!(
            KINDS.iter().any(|k| k.children.contains(&kind.name)),
            "nothing holds {}",
            kind.name
        );
    }
    assert_eq!(KEY_BLOCK_ATTRIBUTES[0].name, "time");
}

#[test]
fn the_effect_names_every_parameter_of_its_file() {
    assert_eq!(EFFECTS, [QTBLEND]);
    assert_eq!(QTBLEND.file, QTBLEND_FILE);
    assert_eq!(QTBLEND.tag, KDENLIVE_TAG);
    assert_eq!(KDENLIVE_TAG, "v26.08.3");
    // Every parameter the three elements of qtblend.xml declare, read by hand at the tag.
    let names: Vec<_> = QTBLEND.params.iter().map(|p| p.kdenlive_name).collect();
    assert_eq!(
        names,
        [
            "compositing",
            "distort",
            "rect",
            "rotate_anchor",
            "rotation",
            "rotate_center"
        ]
    );
    // Those of the element Kdenlive loads with the pinned MLT (version 3).
    let loaded: Vec<_> = QTBLEND.loaded_params().map(|p| p.name).collect();
    assert_eq!(
        loaded,
        [
            "compositing",
            "distort",
            "rect",
            "rotate-anchor",
            "rotation"
        ]
    );
    assert!(QTBLEND.elements.iter().any(|e| e.version == QTBLEND.loaded));
}

#[test]
fn every_parameter_cites_its_file_tag_and_line() {
    let elements = QTBLEND.elements;
    for p in QTBLEND.params {
        assert_eq!(p.read_from.file, "data/effects/qtblend.xml", "{}", p.name);
        assert_eq!(p.read_from.tag, "v26.08.3", "{}", p.name);
        assert!(!p.declared.is_empty(), "{}", p.name);
        assert!(
            p.declared.iter().any(|d| d.line == p.read_from.line),
            "{}: its attributes come from a line that declares it",
            p.name
        );
        // Read from the loaded element where it declares the parameter, else the last that does.
        let want = p
            .declared
            .iter()
            .find(|d| d.version == QTBLEND.loaded)
            .or(p.declared.last())
            .unwrap();
        assert_eq!(p.read_from.line, want.line, "{}", p.name);
        for d in p.declared {
            // Each declaring line lies inside its element: after its `<effect>` tag, before the
            // next one's.
            let i = elements
                .iter()
                .position(|e| e.version == d.version)
                .unwrap_or_else(|| panic!("{}: no element {:?}", p.name, d.version));
            assert!(d.line > elements[i].line, "{}", p.name);
            if let Some(next) = elements.get(i + 1) {
                assert!(d.line < next.line, "{}", p.name);
            }
        }
        let lines: BTreeSet<_> = p.declared.iter().map(|d| d.line).collect();
        assert_eq!(lines.len(), p.declared.len(), "{}", p.name);
    }
}

#[test]
fn every_parameter_name_is_kdenlives_in_kebab_case() {
    let mut names = BTreeSet::new();
    for p in QTBLEND.params {
        assert!(is_kebab(p.name), "{}", p.name);
        assert_eq!(p.name, p.kdenlive_name.replace('_', "-"));
        assert_eq!(p.xml("name"), Some(p.kdenlive_name));
        assert!(names.insert(p.name));
    }
    assert!(is_kebab(QTBLEND.id));
    assert_eq!(QTBLEND.id, QTBLEND.kdenlive_id);
}

#[test]
fn each_parameters_form_follows_its_kdenlive_type() {
    for p in QTBLEND.params {
        let ty = p.xml("type").unwrap();
        assert_eq!(p.keyframes, ty.starts_with("animated"), "{}", p.name);
        let want = match ty {
            "list" => Value::Choice,
            "bool" => Value::OnOff,
            "animatedrect" => Value::Rect,
            "animatedpoint" => Value::Point,
            // The one plain animated number of the file is an angle (its suffix is a degree).
            "animated" => {
                assert_eq!(p.xml("suffix"), Some("°"), "{}", p.name);
                Value::Angle
            }
            other => panic!("{}: type {other}", p.name),
        };
        assert_eq!(p.value, want, "{}", p.name);
        assert_eq!(p.choices.is_empty(), want != Value::Choice, "{}", p.name);
    }
}

#[test]
fn bounds_are_the_files_in_rfc_0001s_form() {
    for p in QTBLEND.params {
        if p.value == Value::OnOff {
            // A switch's 0 and 1 are its two values, not a bound.
            assert_eq!(p.bound, UNBOUNDED, "{}", p.name);
            continue;
        }
        let word = |xml: Option<&str>| {
            xml.map(|v| match p.value {
                Value::Angle => format!("{v}deg"),
                Value::Point => format!("[{v}]"),
                _ => v.to_string(),
            })
        };
        assert_eq!(
            p.bound.min.map(str::to_string),
            word(p.xml("min")),
            "{}",
            p.name
        );
        assert_eq!(
            p.bound.max.map(str::to_string),
            word(p.xml("max")),
            "{}",
            p.name
        );
    }
}

#[test]
fn compositing_words_are_the_labels_in_kebab_case() {
    let p = QTBLEND.param("compositing").unwrap();
    assert_eq!(p.choices, COMPOSITING);
    assert_eq!(COMPOSITING.len(), 22);
    let stored: Vec<_> = COMPOSITING.iter().map(|c| c.stored).collect();
    assert_eq!(Some(stored.join(";").as_str()), p.xml("paramlist"));
    let mut words = BTreeSet::new();
    for c in COMPOSITING {
        assert_eq!(c.word, c.display.to_lowercase().replace(' ', "-"));
        assert!(is_kebab(c.word), "{}", c.word);
        assert!(words.insert(c.word), "{} twice", c.word);
    }
    // The file's default, 0, is a choice.
    assert_eq!(
        p.choice("alpha-blend").unwrap().stored,
        p.xml("default").unwrap()
    );
}

#[test]
fn eases_are_kdenlives_fourteen_by_mlts_names() {
    assert_eq!(EASES.len(), 14);
    let words: Vec<_> = EASES.iter().map(|e| e.word).collect();
    assert_eq!(words, EASE_WORDS);
    let mut kdenlive = BTreeSet::new();
    for (i, e) in EASES.iter().enumerate() {
        assert!(is_kebab(e.word), "{}", e.word);
        assert_eq!(e.mlt, format!("mlt_keyframe_{}", e.word.replace('-', "_")));
        assert!(kdenlive.insert(e.kdenlive), "{}", e.kdenlive);
        assert_eq!(e.cited.file, "src/definitions.h");
        assert_eq!(e.cited.tag, KDENLIVE_TAG);
        // One per line, in the enum's order.
        assert_eq!(e.cited.line, 214 + i as u32, "{}", e.word);
    }
    let key = slice::kind("key").unwrap();
    assert_eq!(
        key.attribute("ease").unwrap().value,
        Value::Word(EASE_WORDS)
    );
}
