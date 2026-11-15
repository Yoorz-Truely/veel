# Veel: Annilay's project language

Draft v0, November 8, 2026. Nothing is built yet. This is the spec in outline, written before the slice so that phase 1a builds towards one design. Decisions cited as V-numbers are in Annilay's `docs/DECISIONS.md`.

The examples are illustrative until 1a-03. From then on every example is checked by a `spec_examples` test, as Null's are, and an example of a form not built yet is fenced `veel (<task>)` with the task that builds it. The grammar is Scowr's (V17), and every kind and attribute spelling is settled as the slice is built (1a-03) and as the schema is generated (1b). The spec is frozen after phase 1 (1d-01); from then on any new form, word, unit, default or diagnostic, and any change to what a file means, its canonical form or its write-back, needs an RFC first, as Null v0's do since November 8, 2026.

**Aligned with frozen Null v0 on November 15, 2026** (V50; [direction review](../../../Annilay/docs/plans/2026-11-15-direction-review.md), A10). Null's spec now settles much that this draft first left open. Where Veel has no reason to differ, it takes Null's rule, cited below by Null's section and line (`~/Projects/Null/docs/spec/null-v0.md` at `1a22fa1`). Where Veel differs, the section says why.

## 1. Principles

- **The project is the source** (V9). A Veel project describes a complete edit: which media exist and how they are read, which sequences cut them together, where every clip sits, how it is trimmed, retimed, treated and graded, which captions and titles appear, and how the result is delivered. Annilay shows it, plays it, renders it and edits it.
- **Declarative** (V16). A file says what the edit is, never how to build it. That is what lets a drag in the timeline be written back as one changed word. Files describe; commands act (render, seek, play, export).
- **Kdenlive's vocabulary** (V18). Editorial kinds use the common NLE names. Effects, transitions and their parameters use Kdenlive's own asset ids and parameter names, in kebab-case, from a schema generated from Kdenlive's asset XML and MLT's metadata.
- **Exact** (V20–V23). Time is integer frames at rational rates. Every other unit converts by one exact rule, rounded once. Ranges are half-open. Floats print as the shortest text that reads back to the same bits.
- **Abstraction first** (V24). The shortest exact form is the written form, and the full granular form is always available beneath it, down to MLT property names in `raw`.
- **Strict** (V19). Every name, kind, type and unit is checked. An unknown name or a wrong type is an error with a suggestion.
- **Total** (V25). Every file finishes loading. Veel is loaded, never run.

## 2. Name and files

- `.veel` files, UTF-8, LF line endings, kebab-case names.
- A project is a folder with `veelindy.veel` at its root, laid out by Veelindy (`Veelindy/docs/spec/veelindy-v0.md`, V35).
- `.annilay/ids.veel` maps handles to native identity and is source (V12). Everything else in `.annilay/` is machine-local.

## 3. A first look

```veel
// veelindy.veel
package "harbor-doc" {
  kind project
  version 0.1.0
  annilay 0.1
  veel-format 1
}
```

```veel
// sources/interviews.veel
source @interview-a "Interview A" {
  media "media/9f21c4….mov"
  hash sha256:9f21c4…
  rate 25
  start-timecode 10:00:00:00
  color-space rec709
}

source @drone "Drone 04" {
  media "media/41d07a….mp4"
  hash sha256:41d07a…
  rate 50
}
```

```veel
// sequences/main.veel
sequence @main "Main" {
  rate 25
  size [1920 1080]
  audio 48kHz stereo
  timecode non-drop start=01:00:00:00

  track @v2 video "Graphics" {
    clip @lower-third from=@lt-name at 01:00:04:00 len 5s
  }

  track @v1 video "Picture" {
    clip @a from=@interview-a 10:02:10:00..+12s at 0
    clip @b from=@drone 0..+6s at 12s {
      crossfade 12                             // a mix into the next clip, 12 frames
      effect @push qtblend {                   // Kdenlive's Transform effect
        rect {
          key 0  [0 0 1920 1080]
          key 6s [-96 -54 2112 1188] ease=cubic-in
        }
      }
    }
    clip @c from=@interview-a 10:05:00:00..+8s at 18s {
      effect @grade-c lift-gamma-gain {
        gain-r 1.02
        gain-b 0.98
      }
    }
  }

  track @a1 audio "Dialogue" {
    clip @a-audio from=@interview-a 10:02:10:00..+12s at 0 linked=@a {
      effect @lvl volume { level -3dB }
    }
  }

  marker "Act 2" at 01:00:18:00 category=purple
}
```

```veel
// captions/en.veel
captions @en "English" sequence=@main lang=en {
  cue 01:00:01:00..01:00:03:12 "We came here in the winter."
  cue 01:00:03:12..+2s "Nobody else did."
}
```

Every time in this example lands on a whole frame at 25 frames per second, as it must: at `24000/1001`, `at 12s` would be 287.71 frames, an error (section 6).

## 4. Syntax

Scowr's lossless syntax, as Null uses it (`Scowr/docs/spec/scowr-v0.md`, section 4; V17). A file is a list of nodes. A node is a name, then entries (values and `key=value` properties), then an optional `{ }` block of child nodes. Comments are `//` and nestable `/* */`, and `/-` comments out a whole node.

What Veel takes from Scowr's lexical rules, and what follows from them:

- **A word** is any run of characters other than whitespace and `{ } [ ] ( ) ; = "`. Numbers, timecodes, ranges, quantities, handles, colours and enum names are words, and the schema decides what a word means.
- **`;` ends a node.** So drop-frame timecode cannot be written with a `;` as other editors print it; it is written with colons, and the sequence's `timecode` setting says how frames are counted (V42).
- **`(` and `)` are reserved** and are an error wherever they appear. Pattern arithmetic (section 9) is written without them.
- **Keyword attributes.** `at`, `to` and `len` may be written as the keyword, a space and the value (`clip at 12s len 6s` is `clip at=12s len=6s`), as in Scowr. The formatter prints them that way.
- **Attributes or children.** Any attribute can be written on its owner's line or as a child line: `clip @b from=@drone 0..+6s at 12s` and a block holding `at 12s` mean the same.
- **Names inside `raw`** are written bare, exactly as Kdenlive's XML spells them (`kdenlive:clipname` is a word: `:` is not a reserved character), and the `veel-name` check does not apply there, as Null exempts raw names (`null-v0.md:1257`). Outside `raw`, a name that is not a word (it holds a space or a reserved character) is a string.

The parser keeps every byte, comments and whitespace included, in a lossless tree. The formatter, the language server and write-back all work on that tree.

## 5. Values

| Type | Written as | Stored as |
| --- | --- | --- |
| Integer | `12`, `-3`, `0x1F` | The parameter's integer type |
| Float | `0.62`, `1e-3`, `inf`, `nan` | The parameter's float type, by the printing rule (V23). `nan` is written without a sign or payload; another NaN's bits only inside `raw`, as Null (`null-v0.md:192`) |
| Fraction | `30000/1001`, `1/3` | An exact rational, for rates and ratios |
| Frames | `24`, `-12` | Integer frames at the enclosing sequence's rate (V20) |
| Time | `2s`, `250ms`, `1:02.250`, `01:00:10:12`, `4800smp` | Integer frames, by one exact conversion (section 6) |
| Range | `in..out`, `in..+duration` | A half-open `[in, out)` range (V21) |
| Percent | `75%` | The parameter's stored scale (often 0–1 or 0–100, from the schema) |
| Gain | `-3dB`, `0.5x` | A linear factor. Decibels convert correctly rounded, never through a platform `pow`, so every machine reads the same bits (as Null's rule for irrational factors, S62) |
| Frequency | `48kHz` | Hertz, exact |
| Angle | `12deg` | The parameter's stored unit |
| Colour | `#rrggbb`, `#rrggbbaa` | 8-bit sRGB as MLT stores it (`0xRRGGBBAA`), in CSS order; Kdenlive's `#AARRGGBB` is reordered in one place (V23, corrected November 15, 2026: Null's `#rrggbb` converts to linear floats and has no alpha, which MLT does not need). A grading value stays in the space its effect declares |
| Size, rect, point | `[1920 1080]`, `[x y w h]` | Kdenlive's geometry types, in pixels of the sequence; `%` of the frame where the parameter allows it |
| Enum | `cubic-in`, `rec709` | Kdenlive's or MLT's enum values, kebab-case |
| Handle | `@interview-a`, `@pkg:lt-name` | Section 7 |
| Path | `"media/9f21c4….mov"` | A string, relative to the project folder |
| Hash | `sha256:9f21c4…` | The content hash of a medium (V10) |

**Gradients and curves** take Null's forms from its RFC 0010 (`null-v0.md:491–539`): a gradient is `stop <position> <colour>` lines, all of them or none, sorted by the formatter; a curve is `point [x y]` lines by place, with the editor's view state left out. Which of Kdenlive's parameters these fit is settled in 1b-05.

**Large blocks.** A keyframe block or a `remap` curve of more than 2,000 keys is written in a Veel file of its own beside its owner, named after it, with `count=` and `frames=[first last]` checked against the file, as Null's RFC 0004 does (S86).

**The printing rule** (V23, Scowr section 5). The formatter and write-back print the friendliest form that reads back to exactly the same bits. A value is never rounded on its way through text. A parameter changed in the timeline is written in the unit its line already uses when that unit can hold it exactly, and otherwise in the stored unit.

## 6. Time

### Rates

A rate is an exact rational: `24`, `25`, `30`, `24000/1001`, `30000/1001`, `60000/1001`. A sequence has one rate. A source has its own rate, read from its media and written in its node. A decimal such as `29.97` is an error with a fix: `29.97 is not exact, write 30000/1001`.

### Frames are the atomic unit

A bare integer is frames at the enclosing sequence's rate (V20), everywhere, source ranges included, because Kdenlive's requests and MLT's playlists take integer frames at the project profile's rate. Every other form is an abstraction over frames, converted by one exact rule. **A value that does not land on a whole frame is a `veel-time-inexact` error naming the nearest exact choices; it is never rounded** (V20, corrected November 15, 2026, as Null makes an inexact integer value an error, `null-v0.md:245`). The one rounding, to nearest even, applies only where the stored value is a float. Scowr's `scowr_time` rounds halves up (`ratio.rs:153`); `veel_time` does not copy it. The forms:

- `s`, `ms`, `min`, `h`: exact decimals times the rate;
- `smp`: audio samples at the sequence's sample rate;
- timecode `hh:mm:ss:ff`: four fields at the rate the context says (below).

### Timecode and drop-frame (V42)

Timecode is always written with colons. Drop-frame counting is a property of the sequence (`timecode drop-frame`), available at `30000/1001` and `60000/1001` only, and it changes how a timecode word maps to a frame number, never which media frames play. A label that drop-frame counting skips (`00:01:00:00` and `00:01:00:01` at `30000/1001`) is an error with the next valid label as its fix. `veel_time` has no `;` branch: Scowr's lexer never makes such a word, though `scowr_time` still parses one. Veel keeps its own `start=` on the sequence's `timecode` line, which Scowr's `frame-rate` line lacks, because an edit needs a start timecode.

A timecode word is absolute: in sequence time it is the sequence's timecode, so with `start=01:00:00:00` the word `01:00:04:00` is frame 100 at 25 frames per second, and `00:00:04:00` is before frame 0, an error. Frames, `s`, `ms` and `smp` count from frame 0. In a source range, a timecode word is the source's timecode, from its `start-timecode`.

### Positions and ranges

- **Sequence positions** (`at`) are in the sequence's time.
- **Source ranges** (a clip's `in..out`) are offsets into the source: a source timecode reads at the source's rate and its `start-timecode`, `s` and `ms` are time from the source's start, and bare integers are frames at the sequence's rate (V20), which is what MLT stores. Every form is converted once to those frames; a range that does not land on a whole sequence frame is a `veel-time-inexact` error naming the nearest exact choices.
- **Ranges are half-open** (V21): `a..b` runs from `a` up to but not including `b`, and `a..+d` is `a..a+d`. A clip's length is `out - in`, with no ±1. The bridge converts to MLT's inclusive out-points in one place, tested at every boundary.

### Keyframe time (V22)

Keyframes belong to the parameter they animate. Their time is clip-local by default; `time=sequence` or `time=source` on a parameter's keyframe block says otherwise. Trimming a clip keeps clip-local keyframes at their clip-local time; slipping moves source-local keyframes with the source; retiming scales clip-local keyframes only when the block says `retime=scale`.

### Variable frame rate

A variable-frame-rate source carries a timestamp map in `.annilay/` keyed by its hash, and is never assumed constant (V20). Source ranges in such a source are written in seconds or as source frame indexes, and the map converts them.

## 7. Identity and references

- **Handles** are the only identity in the text (V12): `clip @interview-a "Interview A"`. A handle is separate from the display name. New handles are made from names (`Clip 2` → `clip-2`).
- **One project-wide namespace per kind**, with no imports (V15). A handle defined twice is an error naming both places. Packages are addressed as `@package:handle`.
- **`.annilay/ids.veel`** pairs each handle with its native identity: the bin clip id and sequence UUID Kdenlive saves, and for timeline items the key Annilay assigns and saves as `annilay:id` (V44). Kdenlive's runtime ids are never written (V12). Elements read from a `.kdenlive` file that predates Annilay get IDs derived from the sequence UUID, the track and the position, so converting the same file twice gives the same IDs.
- **Owner-scoped handles.** Effects, keyframe blocks and other parts of a clip or track are unique within their owner and addressed as a path, `@a/@grade-c`, as Null's sub-objects are (`null-v0.md:279–305`). Inside an owner, a handle names the owner's own part first (S81), and a handle Annilay makes for a part never equals a top-level handle (S89).
- **Sub-elements** without their own handle (a keyframe, a caption cue) are identified by their parent's handle and their place in its list, as Null's items are (S77, S106). Identity by time was the first draft's rule; it breaks when write-back moves a key, so 1b-02 tests place against time and settles it. Markers get handles, made from their name or, for a nameless one, from their frame (`@f120`), as Null's timeline markers do (S88).
- **`.annilay/ids.veel`** has Null's form (`null-v0.md:309–326`): `ids { <kind> @<handle> "<id>" }`, sorted by kind then handle, rewritten whole and atomically, comments not kept, an owner's parts in a block on its line, and a bad entry reported and left out.
- **A repeated display name** within a kind is kept as written and warned (`veel-repeated-name`), as Null does (S112): Kdenlive allows two clips with one name.
- **Renaming** a handle is `annilay rename @old @new` (V51): it changes the handle everywhere in the project and in `ids.veel` (the map first), and refuses what it cannot change, as `sau rename` does (`null-v0.md:258–274`).

## 8. The elements

| Kind | In Kdenlive | Holds |
| --- | --- | --- |
| `source` | A bin clip (`ProjectClip`): AV, video, audio, image, image sequence, colour, title, generator, playlist, timeline | Media path, hash, rate, start timecode, stream map, colour space, field order, proxy association, interpretation overrides |
| `folder` | A bin folder | Its sources, for the bin's view only; membership is a property of the source |
| `sequence` | A timeline (a tractor with a UUID) | Rate, size, sample rate and channels, timecode, tracks, markers, guides, sequence-level effects |
| `track` | A timeline track | `video` or `audio`, name, `mute`, `hide`, `lock`, track effects, clips |
| `clip` | A timeline clip (an instance of a bin clip, V13) | `from=@source`, its source range, `at`, speed and time remap, `linked=`, `group=`, effects, a `crossfade` into the next clip |
| `gap` | Space on a `packed` track (V43) | A length |
| `effect` | An effect in an effect stack | The asset id as its selector, then its parameters, each a value or a keyframe block |
| `composition` | A transition between two tracks (Kdenlive's compositions) | The asset id, the two tracks, `at` and `len`, parameters |
| `crossfade`, `mix` | A same-track mix | `crossfade N` is the short form of a `mix` with the default dissolve; `mix` names the asset and its alignment |
| `marker`, `guide` | Clip markers and timeline guides | Time, name, category |
| `captions`, `cue` | A subtitle track and its entries (`SubtitleModel`) | Language, style, cues with half-open ranges and text |
| `title` | A Kdenlive title clip, or a Glaxnimate animation, or a reference to an Ixelaytor graphic | Its own file, by reference |
| `delivery` | A render preset and its settings | Container, codecs, rate control, range, and the source revision stamp an export records (V37) |
| `raw` | Anything the schema cannot yet name | Kdenlive's own XML property names and values, strictly typed, kept and passed through (V24) |

### Placement (V43)

A track places its clips by absolute position by default: each `clip` has an `at`, so moving one clip changes one word and leaves every other clip bit-identical. A track marked `packed` places them one after another with no `at`, as OpenTimelineIO tracks do, with `gap` nodes for space between them, so a ripple edit changes one word. Write-back keeps the form each track uses, and `annilay fmt` never changes it.

### Effects and keyframes

```veel
effect @push qtblend {
  rect time=clip {
    key 0  [0 0 1920 1080]
    key 2s [-96 -54 2112 1188] ease=cubic-in
  }
  rotation 0deg
}
```

- The selector (`qtblend`, which Kdenlive shows as Transform) is Kdenlive's asset id in kebab-case; its parameters (`rect`, `rotation`, …) come from the generated schema, with their types, units, ranges, defaults and enums.
- A parameter takes either a value or a block of `key` lines. Each `key` has a time, a value and an optional `ease=`, from Kdenlive's 14 keyframe types (`KeyframeType`: linear, discrete, smooth, smooth natural, and in and out forms of bounce, cubic, exponential, circular and elastic).
- Effect order in the block is the stack's order.

### Speed and time remap

`speed 50%` and `reverse` on a clip are the short forms. `remap { key … }` writes Kdenlive's time-remap curve, mapping clip time to source time, in frames.

## 9. Components and patterns

**Nested sequences are components** (V14). A sequence is placed in another as a clip whose source is the sequence, `clip @act1-in from=@act1 …`, and each placement may override the fields the nested sequence exposes. An edit to a placement is an override by default.

**Components, loops, conditions and arithmetic take Null's settled spelling** (V50; `null-v0.md:541–634`): `component` with typed `param`s, `use @c @h count=N` or `each=[…] as=x`, copies addressed `@h.N`, `if "…"` and `else`, `override`, `+ * /` inside words with subtraction, grouping and comparisons quoted, recursion only through a `{ countdown }` parameter, and guards of 1,000,000 elements and 256 levels of nesting. Expansion comes before checking, and an error in an expansion points back at the source line.

**Patterns** (V25) are Veel's own: they apply one treatment to many existing clips in one line, each clip keeping its own overridable handle. Null has no counterpart, so their spelling is decided by an RFC in 1d-01:

```veel
// illustrative only: the spelling is settled in 1d-01
pattern @talking-head-grade {
  effect lift-gamma-gain { gain-r 1.02; gain-b 0.98 }
  effect volume { level -3dB }
}
apply @talking-head-grade to=[@a @c @a-audio]
```

Since `( )` are reserved in Scowr's syntax, arithmetic is written as words (`i*256px`), as in Null. Any change to Scowr's lexer goes through Scowr with RL's agreement (V17).

## 10. Generators

Generators are declared in the manifest and run by Veelindy (roadmap phase 5): any deterministic command that writes Veel, such as captions from a transcript, a montage from a shot list, or an edit from a Sau shot list. Their output is live-reloaded like any edit and can be frozen into ordinary text.

## 11. Checks

Files in `checks/` state what must hold for the project: no gaps on `@v1`, no captions outside the safe area, loudness within −14 ±1 LUFS, no offline media, a golden still at a set timecode. Annilay runs them after every change (roadmap phase 8) and `annilay test` runs them from the command line.

## 12. Canonical form

`annilay fmt` and every write by Annilay print the canonical form, following Null's section 15 (`null-v0.md:1108–1137`): two-space indent, one node per line, a short block kept on one line as `{ a; b }` (Scowr's formatter would undo it), CRLF made LF, attributes in schema order, defaults omitted, values by the printing rule in the unit the line last used, comments kept with the node below them. From Scowr's section 12, an attribute whose value is a handle (`from=@drone`, `linked=@a`, `sequence=@main`) stays on its owner's line, so it never reads as a definition, and a node with a bare `at`, `to` or `len` word is kept as written. **A default** is the value Kdenlive gives a newly created element of that kind, measured in the census (0-08), not the asset XML's `default` (Null's rule, S111). **Short forms** (`crossfade N`, `speed 50%`, a `gain [r g b]` list) are listed with their full forms, a `full_form` pass writes each out, write-back keeps the short form, and writing one value both ways is an error (S70, S77). `annilay fmt --check` reports a file that is not in canonical form (V51). Clips are kept in the author's order on a positioned track and in time order on a packed one. Formatting twice changes nothing, and formatting never changes the model.

## 13. Live reload and write-back

- **Text to timeline** (V27). A change that leaves the project valid is applied at once, paced by its measured cost; an invalid change leaves the last good state on screen and shows its diagnostics. The project is diffed against the live model by handle, and the difference is applied as one transaction and one undo step.
- **Timeline to text** (V24, V29). Every native undo push is written back as a minimal edit: only the changed tokens, in the author's units and abstract forms. A gesture writes its file when it ends. Undo in the app rewrites the file.
- **Two writers** (V28) merge by value. When both sides changed the same value, the outside save wins and Annilay's edit is offered back, as Scowr's section 14 settles it.
- **Null's apply and write-back rules hold** (`null-v0.md:1146–1171`): applying the same text twice pushes no undo step; an element made in Annilay that no file defines is left alone; each change is the smallest edit to one value; a file is written atomically and only while it holds the text Annilay last read, otherwise the outside change wins; undo restores files byte for byte; Annilay knows its own writes by their text.

## 14. Diagnostics

Every diagnostic has a stable code of its own (`veel-syntax`, `veel-name`, `veel-kind`, `veel-unknown-property`, `veel-type`, `veel-enum`, `veel-unit`, `veel-time-inexact`, `veel-overlap`, `veel-range`, `veel-handle`, `veel-readonly`, `veel-repeated-name`, …), a span, a message, and a suggestion where one exists. **Every new diagnostic gets a new code**, never an existing code whose meaning differs (S97); the codes are listed once, in `codes::ALL`, with one negative test each. Wording follows Null's rules (`null-v0.md:1219–1230`): the nearest choices first, one mistake gives one error, and a fix for `key = value`. Rendering is Scowr's `file:line:col error[code]: message`:

```text
sequences/main.veel:40:5 error[veel-overlap]: clip @b overlaps @a by 3 frames on track @v1
sources/interviews.veel:12:8 error[veel-time-inexact]: 29.97 is not exact; write 30000/1001
```

Severities are error, warning and note, as in Null (V19, corrected November 15, 2026). Warnings so far: a valid, typed property Annilay cannot yet apply live, and a repeated display name.

## 15. Converting `.kdenlive`

`annilay convert` turns a `.kdenlive` file into a project folder and back with no property changed (V38, roadmap 1d). What the schema cannot yet name goes into `raw` blocks, and the coverage audit shrinks them over time. A `.kdenlive` file is otherwise a cache and an export (V9). Lossless is the target; the conversion gate measures it as a ratchet (V49), so every remaining difference is classed and none may grow.

`annilay migrate` lifts a project one `veel-format` at a time, staged, and refuses a newer format, a missing step or a file that does not parse, as `sau migrate` does (V26, V51).

## 16. Open, and where each is settled

| Question | Settled in |
| --- | --- |
| The slice's exact kind and attribute spellings | 1a-03 |
| How Kdenlive parameter names that are not kebab-case words map (strings, or a mapping table) | 1b-05 |
| Keyframe `ease` names, from Kdenlive's 14 keyframe types | 1b-05 |
| Named abstractions over asset ids (a `transform` short form over `qtblend`, a `gain [r g b]` list over `gain-r`, `gain-g`, `gain-b`) | 1d-01 |
| Pattern spelling (`pattern … apply`), by RFC; loops and arithmetic are Null's (V50) | 1d-01 |
| Whether a keyframe or cue is identified by its place or its time | 1b-02 |
| Bounds on parameters: errors or notes, and how they print. Scowr's RFC 0001 (Crescend's G6, proposed November 15, 2026) adds typed inclusive bounds as errors; Null makes a value beyond an engine's range a note. If Veel mirrors RFC 0001, it must not print an inclusive bound with `..`, which means half-open here, and Kdenlive's XML limits are often only a slider's | 1b-04, after RL's answer at G6 |
| Whether captions live in their sequence's file or their own by default | 1d-01 |
| The stream map and channel layout forms for multi-stream sources | 1d-03 |
