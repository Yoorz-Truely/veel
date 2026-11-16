# agent/r02-slice: the slice language (round 0.2, area D)

Area D of Annilay's round 0.2, on branch `agent/r02-slice` in Veel only (worktree `Annilay/work/wt/r02-slice/Veel`, cut from Veel `110ea78`). Row 1a-03 of Annilay's execution plan. Annilay (`bae101b`, its `kdenlive` branch `a277c72` = Kdenlive `v26.08.3`), Scowr, Null (`b2480a4`) and Veelindy (`741d287`) were read, never changed. Nothing of Annilay was built.

**Before, on `main`** (Veel `110ea78`), repeatable:

```text
$ grep -n -E '^#+ .*[Ss]lice' docs/spec/veel-v0.md      # nothing: no slice section
$ ls compiler                                            # veel_syntax veel_time: no veel_schema
$ ls tests                                               # README.md roundtrip ui: no slice/
```

## 1a-03 The slice language

### What works

- **The section.** `docs/spec/veel-v0.md`, `## 17. The slice (phase 1a)` (line 304; `grep -n -E '^#+ .*[Ss]lice' docs/spec/veel-v0.md` prints it and `### The slice project`, line 308). It gives the slice project (a folder of `.veel` files, no manifest needed), the eight kinds (`source`, `sequence`, `timecode`, `track`, `clip`, `effect`, `crossfade`, `key`) with their attributes and what each means when left out, the time forms by place (sequence position, source range, a timecode as a length, clip-local key time), placement (first track written is the topmost; no shared frame on a track), the crossfade's exact frames, the parameter value forms (rectangle with optional opacity, point, angle, choice, switch), bounds and defaults, the 14 `ease` words, the effect's parameter table with citations, two worked examples and the corpus. The "Open" row of line 294 reads **Settled** in 1a-03: section 17.
- **The chosen effect: Transform (`qtblend`)**, not `brightness`. Why: it is what a gesture in Kdenlive's monitor writes (position, size, rotation), so 1a-09's write-back is proved on a geometry value, not one number; and the blueprint, the spec's sections 3 and 8, `tests/roundtrip/blueprint-slice.veel` and `tests/ui/syntax/good-slice.veel` were all written with it. Its costs, each settled in the section: the file holds three versions (Kdenlive loads the highest the installed MLT supports, `effectsrepository.cpp:193–198` and `abstractassetsrepository.ipp:406–413`; MLT's `qtblend` in the pinned dependencies is version 5, so element 3), two snake_case names (`rotate_anchor`, `rotate_center`: kebab-case by V18), a list parameter (words are Kdenlive's labels in kebab-case, Null's rule of names not numbers) and a rect with an opacity.
- **The crossfade** is defined on the cut, as every example already wrote it (clips abut, `crossfade N` on the first): the mix covers `[cut − ⌈N/2⌉, cut + ⌊N/2⌋)`, as Kdenlive's `TimelineModel::mixClip` centres a new mix (`timelinemodel.cpp:1153–1155`, default asset `luma`, `timelinemodel.hpp:964`). The bridge converts to Kdenlive's overlapping clips in one place.
- **`veel_schema`** (new crate, `compiler/veel_schema`, as `docs/design/compiler.md` lays it out): the slice schema as data. `slice::KINDS` (8 kinds), `slice::QTBLEND` (3 elements, 6 parameters, each with `read_from: Cited { file: "data/effects/qtblend.xml", tag: "v26.08.3", line }`, every declaring line, the `<parameter>` attributes verbatim, an RFC 0001 bound where the file gives one, and the 22 compositing choices), `slice::EASES` (14, citing `src/definitions.h:214–227`). Nothing in it reads Kdenlive's files.
- **The slice corpus**, `tests/slice/`, 5 folders and 6 files: `one-clip`, `two-clips` (worked example 1), `picture-in-picture` (worked example 2, two files), `every-parameter` (all five loaded parameters set and keyframed, all 14 eases, attributes on the line and as child lines, two effects in one stack), `crossfades` (two in a row, one odd, one in seconds). Every time in them was worked by hand to land on whole frames.
- **The spec-example test** (`compiler/veel_syntax/tests/spec_examples.rs`, moved out of `syntax.rs`): all 9 fenced Veel blocks of the spec and all 6 corpus files parse with no diagnostic and print back byte for byte; every corpus file names its path on its first line; the two worked examples are corpus files byte for byte.
- **The spec's other examples are fenced with the task that builds them**, as its line 5 asks from 1a-03 on: section 3's manifest `veel (1d-03)`, sources `veel (1d-06a)`, sequence `veel (1d-06b)`, captions `veel (1d-06f)`; section 9's pattern `veel (1d-02)`. Labels only: no meaning changed.

### Parameters, with citations (all `data/effects/qtblend.xml` at `v26.08.3`)

| Veel | Kdenlive | Type | Read from | Declared at | Bound (RFC 0001 form) | In the slice |
| --- | --- | --- | --- | --- | --- | --- |
| `compositing` | `compositing` | `list`, 22 choices | line 56 (labels line 58) | 14, 29, 56 | none | yes |
| `distort` | `distort` | `bool` | line 60 | 18, 33, 60 | none (0/1 is the switch) | yes |
| `rect` | `rect` | `animatedrect` | line 63 | 8, 39, 63 | none | yes |
| `rotate-anchor` | `rotate_anchor` | `animatedpoint` | line 66 | 42, 66 | `[-1 -1]` to `[1 1]` | yes |
| `rotation` | `rotation` | `animated` | line 69 | 11, 45, 69 | `-360deg` to `360deg` | yes |
| `rotate-center` | `rotate_center` | `bool` | line 36 | 36 | none | no (element 2 only) |

Elements: line 4 (no version), 22 (`version="2"`), 49 (`version="3"`, loaded).

### Measured

- Spec examples: 9 blocks (6 before the slice, 3 in section 17), 0 diagnostics. Corpus: 5 folders, 6 files, 0 diagnostics.
- **Controls.** `a_planted_bad_example_fails_the_spec_example_test` plants four broken examples in a copy of the spec (an unclosed block, a non-kebab name in a `veel (1a-04)` fence, a reserved `(`, a fence never closed): each adds one block and each is reported; and in a copy of the corpus a broken file in a new folder is reported (and only it), then an empty folder is. `a_planted_misspelling_is_reported` (veel_schema) plants 12 misspellings in a clean slice file (attribute, kind, counting word, track kind, child kind, asset, choice, ease, a parameter not loaded, `mix`, `time=sequence`, `keyframe`): each is reported, the clean file is not. `kebab_case_is_recognised` refuses 8 bad names.
- veel_schema: 10 unit tests (names kebab-case and unique; every parameter's file, tag and line, and each line inside its element; each form follows Kdenlive's type; bounds equal the file's `min`/`max` in RFC form; 22 choices whose stored values join to the file's `paramlist`; 14 eases on consecutive lines) and the corpus spelling test.
- The check: see "The check" below.

### Found

- MLT's own metadata bounds `qtblend`'s `rotation` at 0 to 360, the XML at −360 to 360: Kdenlive's bounds are sometimes a slider's (recorded in section 17; the decision is 1b-04's, row already open).
- `Annilay-build/deps` holds MLT 7.43.0 (`mlt-framework-7.pc`), where Annilay's `docs/BUILDING.md:290` names 0-04's pin as MLT 7.38.0. The choice of `qtblend` element 3 holds for both if 7.38.0's filter is version 3 or later, which was not read here (proposed row 2).
- Section 8's effect example (spec line 217) is a fragment, an effect with no clip, so it cannot be checked as a project once 1a-04 runs the spec's examples through `veel check` (proposed row 1).
- Section 5's value table has no Boolean row; the slice states Null's `on`/`off` form in section 17 (proposed row 4).

### Unverified

- That Kdenlive loads element 3 on the integrator's build, and the values a new Transform gets (the defaults section 17 takes from the XML until the census measures them): for the census (0-08).
- The crossfade rule against Kdenlive's mix as saved: read from `mixClip`'s source, not from a built app. 1a-05 and 1a-13 test it.
- Everything the slice means is checked only by 1a-04's `veel check`; here the corpus is proved to parse and to spell only what the schema holds.

## Proposed board rows

| Issue | Seen | Severity | Repo | Done when |
| --- | --- | --- | --- | --- |
| The spec's section 8 effect example (line 217) is a fragment with no clip, so it cannot pass `veel check` as a project | Reading the spec's blocks for 1a-03 | friction | Veel | Before 1a-04 checks the spec's examples, that block is fenced `veel (<task>)` or set in a clip, and 1a-04's spec-example check runs every unfenced block clean |
| Which `qtblend` element Kdenlive loads on the pinned build (MLT 7.38.0 per BUILDING.md, 7.43.0 in `Annilay-build/deps`), and a new Transform's parameter values | `veel_schema::slice::QTBLEND.loaded` assumes element 3 | degrades fidelity | Annilay | `docs/CENSUS.md` records the loaded `qtblend` version and a new Transform's `rect`, `rotation`, `rotate_anchor`, `compositing` and `distort`, and they equal `veel_schema`'s loaded parameters and section 17's defaults, or section 17 is corrected |
| What Kdenlive does to a keyframe past a clip's end on a trim | Section 17 leaves such a key unrefused | friction | Annilay | The census records it on the build, and section 17 says whether the slice refuses such a key |
| Spec section 5's value table has no Boolean row | Writing section 17's switch form | friction | Veel | Section 5 lists Boolean, `on`/`off`, the name alone for `on`, citing `null-v0.md:184` |
| The crossfade's two new errors (no next clip; mixes at a clip's two ends overlap) have no code in section 14 | Section 17's crossfade rules | friction | Veel | 1a-04 gives each its own code in `codes::ALL`, with a negative case each |

## The check

To be filled with the final `just check` run.
