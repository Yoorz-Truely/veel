//! The phase 1a slice's schema, written by hand (task 1a-03): its kinds, their attributes and
//! value forms (the spec's section 17), and its one effect, Kdenlive's Transform (`qtblend`).
//!
//! Every parameter of the effect was read by hand from `data/effects/qtblend.xml` at Kdenlive's
//! tag [`KDENLIVE_TAG`] and cites the line it was read from. No code here or in the tests reads
//! Kdenlive's files: 1b-04's gate compares this copy with the schema Annilay generates.

use crate::{
    Arg, Attribute, Bound, Choice, Cited, Declared, Ease, Effect, Element, KDENLIVE_TAG, Kind,
    Param, UNBOUNDED, Value,
};

/// The asset file the slice's effect is read from.
pub const QTBLEND_FILE: &str = "data/effects/qtblend.xml";

const fn qtblend_line(line: u32) -> Cited {
    Cited {
        file: QTBLEND_FILE,
        tag: KDENLIVE_TAG,
        line,
    }
}

const fn definitions_line(line: u32) -> Cited {
    Cited {
        file: "src/definitions.h",
        tag: KDENLIVE_TAG,
        line,
    }
}

/// The word of a track's kind in the slice. Audio tracks come later (1d-06b).
pub const TRACK_KINDS: &[&str] = &["video"];

/// The timecode counting the slice takes. Drop-frame counting (V42) comes with `veel_time`'s
/// task 1c-01.
pub const TIMECODE_COUNTING: &[&str] = &["non-drop"];

/// The keyframe times the slice takes. `sequence` and `source` (V22) come later (1d-06c).
pub const KEY_TIMES: &[&str] = &["clip"];

/// The slice's kinds, in the order the spec's section 17 lists them.
pub const KINDS: &[Kind] = &[
    Kind {
        name: "source",
        doc: "One medium: a video file the clips cut from",
        kdenlive: "A bin clip (`ProjectClip`) of one audio-video file",
        handle: true,
        args: &[Arg {
            name: "display name",
            value: Value::Text,
            required: false,
        }],
        attributes: &[
            Attribute {
                name: "media",
                value: Value::Path,
                required: true,
                default: None,
                doc: "The file, from the project's folder",
            },
            Attribute {
                name: "hash",
                value: Value::Hash,
                required: false,
                default: None,
                doc: "The file's content hash (V10)",
            },
            Attribute {
                name: "rate",
                value: Value::Rate,
                required: true,
                default: None,
                doc: "The medium's frame rate, as probed",
            },
            Attribute {
                name: "len",
                value: Value::SourceLength,
                required: true,
                default: None,
                doc: "The medium's length, as probed: no clip's range may run past it",
            },
            Attribute {
                name: "start-timecode",
                value: Value::Timecode,
                required: false,
                default: Some("00:00:00:00"),
                doc: "The timecode of the medium's first frame, at its rate, non-drop",
            },
        ],
        children: &[],
        top_level: true,
    },
    Kind {
        name: "sequence",
        doc: "A timeline: its rate, frame size, timecode and tracks",
        kdenlive: "A timeline (a tractor with a `kdenlive:uuid`)",
        handle: true,
        args: &[Arg {
            name: "display name",
            value: Value::Text,
            required: false,
        }],
        attributes: &[
            Attribute {
                name: "rate",
                value: Value::Rate,
                required: true,
                default: None,
                doc: "The sequence's frame rate: a bare integer anywhere in it is frames at this rate",
            },
            Attribute {
                name: "size",
                value: Value::Size,
                required: true,
                default: None,
                doc: "The frame size in pixels, square pixels, progressive",
            },
        ],
        children: &["timecode", "track"],
        top_level: true,
    },
    Kind {
        name: "timecode",
        doc: "How the sequence counts timecode, and the timecode of its frame 0",
        kdenlive: "The project's timecode settings and the timeline's start offset",
        handle: false,
        args: &[Arg {
            name: "counting",
            value: Value::Word(TIMECODE_COUNTING),
            required: true,
        }],
        attributes: &[Attribute {
            name: "start",
            value: Value::Timecode,
            required: false,
            default: Some("00:00:00:00"),
            doc: "The timecode of frame 0",
        }],
        children: &[],
        top_level: false,
    },
    Kind {
        name: "track",
        doc: "A video track, its clips placed by position; the first written is the topmost",
        kdenlive: "A timeline track",
        handle: true,
        args: &[
            Arg {
                name: "track kind",
                value: Value::Word(TRACK_KINDS),
                required: true,
            },
            Arg {
                name: "display name",
                value: Value::Text,
                required: false,
            },
        ],
        attributes: &[],
        children: &["clip"],
        top_level: false,
    },
    Kind {
        name: "clip",
        doc: "A span of a source placed on a track",
        kdenlive: "A timeline clip, an instance of a bin clip (V13)",
        handle: true,
        args: &[Arg {
            name: "source range",
            value: Value::SourceRange,
            required: true,
        }],
        attributes: &[
            Attribute {
                name: "from",
                value: Value::Handle("source"),
                required: true,
                default: None,
                doc: "The source it cuts from",
            },
            Attribute {
                name: "at",
                value: Value::Position,
                required: true,
                default: None,
                doc: "The sequence position of its first frame",
            },
        ],
        children: &["effect", "crossfade"],
        top_level: false,
    },
    Kind {
        name: "effect",
        doc: "An effect in the clip's stack, in the block's order; its block holds parameters",
        kdenlive: "An effect in a clip's effect stack (`EffectStackModel`)",
        handle: true,
        args: &[Arg {
            name: "asset",
            value: Value::Word(&["qtblend"]),
            required: true,
        }],
        attributes: &[],
        children: &[],
        top_level: false,
    },
    Kind {
        name: "crossfade",
        doc: "A dissolve into the next clip on the track, centred on the cut",
        kdenlive: "A same-track mix with Kdenlive's default `luma` transition",
        handle: false,
        args: &[Arg {
            name: "length",
            value: Value::Length,
            required: true,
        }],
        attributes: &[],
        children: &[],
        top_level: false,
    },
    Kind {
        name: "key",
        doc: "One keyframe in a parameter's block: a time in the clip and a value",
        kdenlive: "A keyframe of a `KeyframeModel`",
        handle: false,
        args: &[
            Arg {
                name: "time",
                value: Value::ClipTime,
                required: true,
            },
            Arg {
                name: "value",
                value: Value::Parameter,
                required: true,
            },
        ],
        attributes: &[Attribute {
            name: "ease",
            value: Value::Word(EASE_WORDS),
            required: false,
            default: Some("linear"),
            doc: "How the value moves from this key to the next",
        }],
        children: &[],
        top_level: false,
    },
];

/// The attribute a parameter's keyframe block may carry: `time=clip` (V22), the default.
pub const KEY_BLOCK_ATTRIBUTES: &[Attribute] = &[Attribute {
    name: "time",
    value: Value::Word(KEY_TIMES),
    required: false,
    default: Some("clip"),
    doc: "Which time the keys count in",
}];

/// The kind named `name`.
pub fn kind(name: &str) -> Option<&'static Kind> {
    KINDS.iter().find(|k| k.name == name)
}

/// The keyframe types, Kdenlive's 14 (`KeyframeType::KeyframeEnum`, `src/definitions.h:213–228`
/// at the tag), each named by MLT's `mlt_keyframe_type` name in kebab-case. Provisional: 1b-05
/// confirms the names over the whole schema and closes the spec's row for them.
pub const EASES: &[Ease] = &[
    ease("linear", "Linear", "mlt_keyframe_linear", 214),
    ease("discrete", "Discrete", "mlt_keyframe_discrete", 215),
    ease("smooth", "Curve", "mlt_keyframe_smooth", 216),
    ease(
        "smooth-natural",
        "CurveSmooth",
        "mlt_keyframe_smooth_natural",
        217,
    ),
    ease("bounce-in", "BounceIn", "mlt_keyframe_bounce_in", 218),
    ease("bounce-out", "BounceOut", "mlt_keyframe_bounce_out", 219),
    ease("cubic-in", "CubicIn", "mlt_keyframe_cubic_in", 220),
    ease("cubic-out", "CubicOut", "mlt_keyframe_cubic_out", 221),
    ease(
        "exponential-in",
        "ExponentialIn",
        "mlt_keyframe_exponential_in",
        222,
    ),
    ease(
        "exponential-out",
        "ExponentialOut",
        "mlt_keyframe_exponential_out",
        223,
    ),
    ease("circular-in", "CircularIn", "mlt_keyframe_circular_in", 224),
    ease(
        "circular-out",
        "CircularOut",
        "mlt_keyframe_circular_out",
        225,
    ),
    ease("elastic-in", "ElasticIn", "mlt_keyframe_elastic_in", 226),
    ease("elastic-out", "ElasticOut", "mlt_keyframe_elastic_out", 227),
];

/// The `ease=` words, in [`EASES`]'s order.
pub const EASE_WORDS: &[&str] = &[
    "linear",
    "discrete",
    "smooth",
    "smooth-natural",
    "bounce-in",
    "bounce-out",
    "cubic-in",
    "cubic-out",
    "exponential-in",
    "exponential-out",
    "circular-in",
    "circular-out",
    "elastic-in",
    "elastic-out",
];

const fn ease(word: &'static str, kdenlive: &'static str, mlt: &'static str, line: u32) -> Ease {
    Ease {
        word,
        kdenlive,
        mlt,
        cited: definitions_line(line),
    }
}

/// The slice's effects: Kdenlive's Transform alone.
pub const EFFECTS: &[Effect] = &[QTBLEND];

/// The effect whose selector is `id`.
pub fn effect(id: &str) -> Option<&'static Effect> {
    EFFECTS.iter().find(|e| e.id == id)
}

/// The compositing modes, from `paramlist` and `paramlistdisplay` of `qtblend.xml:56–58` at
/// the tag, in the file's order. The stored values are Qt's `QPainter::CompositionMode`.
pub const COMPOSITING: &[Choice] = &[
    choice("alpha-blend", "0", "Alpha blend"),
    choice("xor", "11", "Xor"),
    choice("plus", "12", "Plus"),
    choice("multiply", "13", "Multiply"),
    choice("screen", "14", "Screen"),
    choice("overlay", "15", "Overlay"),
    choice("darken", "16", "Darken"),
    choice("lighten", "17", "Lighten"),
    choice("color-dodge", "18", "Color dodge"),
    choice("color-burn", "19", "Color burn"),
    choice("hard-light", "20", "Hard light"),
    choice("soft-light", "21", "Soft light"),
    choice("difference", "22", "Difference"),
    choice("exclusion", "23", "Exclusion"),
    choice("bitwise-or", "24", "Bitwise or"),
    choice("bitwise-and", "25", "Bitwise and"),
    choice("bitwise-xor", "26", "Bitwise xor"),
    choice("bitwise-nor", "27", "Bitwise nor"),
    choice("bitwise-nand", "28", "Bitwise nand"),
    choice("bitwise-not-xor", "29", "Bitwise not xor"),
    choice("destination-in", "6", "Destination in"),
    choice("destination-out", "8", "Destination out"),
];

const fn choice(word: &'static str, stored: &'static str, display: &'static str) -> Choice {
    Choice {
        word,
        stored,
        display,
    }
}

const PARAMLIST: &str = "0;11;12;13;14;15;16;17;18;19;20;21;22;23;24;25;26;27;28;29;6;8";

/// The version attribute of the first element, which has none.
const V1: Option<&str> = None;
const V2: Option<&str> = Some("2");
const V3: Option<&str> = Some("3");

const fn at(version: Option<&'static str>, line: u32) -> Declared {
    Declared { version, line }
}

/// Kdenlive's Transform, `data/effects/qtblend.xml` at the tag: three `<effect>` elements of one
/// MLT service, lines 4, 22 and 49. Kdenlive loads the highest version the installed MLT filter
/// supports (`EffectsRepository::parseCustomAssetFile`, `src/effects/effectsrepository.cpp:193–
/// 198`, after `AbstractAssetsRepository::parseInfoFromXml` refuses a version above MLT's,
/// `src/assets/abstractassetsrepository.ipp:406–413`). MLT's `qtblend` filter in Annilay's
/// pinned dependencies declares version 5, so Kdenlive loads element 3, and those are the
/// parameters a Veel file sets. Element 2's `rotate_center` is listed, read from line 36, but
/// a Veel file of the slice does not set it.
pub const QTBLEND: Effect = Effect {
    id: "qtblend",
    kdenlive_id: "qtblend",
    mlt_service: "qtblend",
    label: "Transform",
    file: QTBLEND_FILE,
    tag: KDENLIVE_TAG,
    elements: &[
        Element {
            version: V1,
            line: 4,
        },
        Element {
            version: V2,
            line: 22,
        },
        Element {
            version: V3,
            line: 49,
        },
    ],
    loaded: V3,
    params: &[
        Param {
            name: "compositing",
            kdenlive_name: "compositing",
            label: "Compositing",
            value: Value::Choice,
            keyframes: false,
            bound: UNBOUNDED,
            choices: COMPOSITING,
            xml: &[
                ("type", "list"),
                ("name", "compositing"),
                ("default", "0"),
                ("paramlist", PARAMLIST),
            ],
            read_from: qtblend_line(56),
            declared: &[at(V1, 14), at(V2, 29), at(V3, 56)],
        },
        Param {
            name: "distort",
            kdenlive_name: "distort",
            label: "Distort",
            value: Value::OnOff,
            keyframes: false,
            bound: UNBOUNDED,
            choices: &[],
            xml: &[
                ("type", "bool"),
                ("name", "distort"),
                ("default", "0"),
                ("min", "0"),
                ("max", "1"),
            ],
            read_from: qtblend_line(60),
            declared: &[at(V1, 18), at(V2, 33), at(V3, 60)],
        },
        Param {
            name: "rect",
            kdenlive_name: "rect",
            label: "Rectangle",
            value: Value::Rect,
            keyframes: true,
            bound: UNBOUNDED,
            choices: &[],
            xml: &[
                ("type", "animatedrect"),
                ("name", "rect"),
                ("default", "0 0 %width %height 1"),
            ],
            read_from: qtblend_line(63),
            declared: &[at(V1, 8), at(V2, 39), at(V3, 63)],
        },
        Param {
            name: "rotate-anchor",
            kdenlive_name: "rotate_anchor",
            label: "Rotation Point",
            value: Value::Point,
            keyframes: true,
            bound: Bound {
                min: Some("[-1 -1]"),
                max: Some("[1 1]"),
            },
            choices: &[],
            xml: &[
                ("type", "animatedpoint"),
                ("name", "rotate_anchor"),
                ("default", "0.5 0.5 0 0"),
                ("factor", "100 100"),
                ("min", "-1 -1"),
                ("max", "1 1"),
                ("decimals", "2"),
                ("suffix", "%"),
            ],
            read_from: qtblend_line(66),
            declared: &[at(V2, 42), at(V3, 66)],
        },
        Param {
            name: "rotation",
            kdenlive_name: "rotation",
            label: "Rotation",
            value: Value::Angle,
            keyframes: true,
            bound: Bound {
                min: Some("-360deg"),
                max: Some("360deg"),
            },
            choices: &[],
            xml: &[
                ("type", "animated"),
                ("name", "rotation"),
                ("max", "360"),
                ("min", "-360"),
                ("default", "0"),
                ("decimals", "2"),
                ("notintimeline", "1"),
                ("suffix", "°"),
                ("compact", "1"),
            ],
            read_from: qtblend_line(69),
            declared: &[at(V1, 11), at(V2, 45), at(V3, 69)],
        },
        Param {
            name: "rotate-center",
            kdenlive_name: "rotate_center",
            label: "Rotate from center",
            value: Value::OnOff,
            keyframes: false,
            bound: UNBOUNDED,
            choices: &[],
            xml: &[
                ("type", "bool"),
                ("name", "rotate_center"),
                ("default", "1"),
                ("min", "0"),
                ("max", "1"),
            ],
            read_from: qtblend_line(36),
            declared: &[at(V2, 36)],
        },
    ],
};
