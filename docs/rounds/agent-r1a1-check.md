# agent/r1a1-check: check, print, edit (round 1a.1, area A)

Area agent A of round 1a.1, on branch `agent/r1a1-check` in Veel only (worktree `Annilay/work/wt/r1a1-check/`). Rows: plan task 1a-04 and board rows B-40, B-43, B-44, B-56, B-58. `veel_syntax` was used and not edited (area B's this round); Scowr, Annilay, Veelindy and Null were read, never changed. Nothing here needs the app.

**Before, on `main`** (Veel `4c3f3a3`): `ls compiler` lists `veel_schema`, `veel_syntax`, `veel_time` only; `tests/negative/` does not exist; the spec's section 8 example is a bare `effect` fragment (line 217); section 5 has no boolean row; section 17's crossfade errors have no code.

## 1a-04 `veel_check`, `veel_print`, `veel_edit`, `veel_cli`

### What works

- **`veel_check`** (`compiler/veel_check`): checks a slice project (a folder of `.veel` files read as one; `veelindy.veel` at the root left to Veelindy) against `veel_schema`'s slice and `veel_time`, and gives its diagnostics and a model (`Project`: sources, sequences, tracks, clips with ranges and positions in whole frames, effects, keys, crossfades). It reports:
  - unknown names: an unknown kind (`veel-kind`) or property, child line or effect parameter (`veel-unknown-property`), each with the nearest name as its fix;
  - wrong kind: a known kind where it cannot stand (`veel-kind`), a handle naming an element of another kind (`veel-handle`: "`@main` is a sequence, not a source");
  - wrong type (`veel-type`), word (`veel-enum`, with the nearest word; a stored number such as `compositing 14` gets the word for it), unit (`veel-unit`; a bare-frames source `len` gets its timecode as the fix), and `veel_time`'s off-frame times (`veel-time-inexact`, both nearest exact choices as fixes, never rounded);
  - unknown or duplicate handles (`veel-handle`: nearest handle as the fix; a duplicate names the first place, across files, with a free handle `@a-2` as the fix);
  - overlaps on a track (`veel-overlap`, fixed by starting the later clip where the earlier ends, in its own unit);
  - ranges past their source and other values outside their place (`veel-range`, the past-the-end fix ending the range at the source's last frame in the range's own form);
  - a required handle, value or attribute left out (`veel-missing`), a value given twice (`veel-repeated`), keys out of order (`veel-key-order`), and the crossfade's two errors (`veel-crossfade-next`, `veel-crossfade-overlap`, B-44), with the crossfade's source-frame needs as `veel-range`.
  - One mistake gives one error: a project with a syntax error reports its syntax only; non-kebab names are left to `veel-name`; an effect whose asset is unknown has its parameters left unread.
- **The two code lists stay distinct.** `veel_syntax::codes::ALL` is `veel-syntax`, `veel-name` (untouched). `veel_check::codes::ALL` is the checker's 14, none of them the syntax's; the unit test `no_code_is_also_the_syntaxs` fails if a code is in both, or listed twice. `veel_time`'s `TimeError::code()` words map onto the checker's constants (`every_code_veel_time_gives_is_the_checkers`).
- **`tests/negative/`**: 16 cases, one per code `veel check` reports (2 syntax + 14 checker; `veel-handle` is a two-file project folder), each with its `.stderr` golden: 76 diagnostics, 45 fixes. The test (`veel_check`'s `negative`) checks the goldens byte for byte (`VEEL_BLESS=1` rewrites), one case per code named after it, that each case reports only its code, and that every fix applied leaves fewer diagnostics of its code.
- **`veel_print`**: `format` is the layout (Null's section 15: two-space indent, one space between entries, none inside `[ ]`, `{ a; b }`, `=` as written, no trailing whitespace, LF, at most one blank line, one final line feed); `canonical` adds section 4's keyword form (`at=12s` → `at 12s`), each line's properties and each block's lines in the schema's order (attributes, then held kinds; an effect's parameters in the loaded element's order; tracks, clips, effects and keys keep theirs; unknown and `/-` lines last; a comment above a line moves with it). Defaults are not left out: the census has not measured the slice's (B-41), and dropping a line for the asset file's stand-in would change meaning once measured.
- **`veel_edit`**: `set_clip_time` (a clip's `at`, the in or out point of its range keeping the other end and the `..+length` or `..out` form, its crossfade) and `set_word` (any attribute or parameter where it is written; a switch written alone gets its word). Each gives one `FileEdit` replacing exactly one word, in the unit the word used where that unit names the new frame exactly, frames otherwise.
- **`veel_cli`**: the `veel` binary. `veel check <project>...` (exit 1 on errors), `veel fmt [--check] <path>...` (writes, or lists the files not canonical, exit 1), `veel print <file>` (canonical form to stdout, writes nothing), `--help` (written for U1a.1: what a slice project holds, how times read, the exit codes), `--version`; exit 2 for a command it cannot run.
- **The slice corpus** checks with no diagnostic and is canonical. To make it so, its layout and order changed, nothing else: alignment spaces (`key 0  [` → `key 0 [`, aligned comments), `len=10s` → `len 10s` (section 4: "The formatter prints them that way"), the picture in picture's `compositing` first, every-parameter's `hash=` and `distort` to their schema places. The spec's two worked examples changed the same way, so they stay byte-for-byte copies (veel_syntax's test). Main's corpus also checks clean (below), so nothing it means changed.
- `docs/design/compiler.md` and `tests/README.md` say where the formatter, the codes and the negative cases now live. `_typos.toml` skips `tests/negative/` (its names are misspelt on purpose) and knows `trak`.

### Measured

- `just affected` passed on the committed trees (`86d39e359d80` before the first commit's spelling fix, `a4c25058b503` for `db35dd9`, and again for `1f42017`): Clippy with warnings as errors, tests of `veel_check`, `veel_cli`, `veel_edit`, `veel_print`, `veel_schema`, `veel_syntax`, `veel_time`, cargo deny.
- Tests: `veel_check` 4 unit + 10 integration + 1 doc; `veel_print` 6 + 1 doc; `veel_edit` 5 + 1 doc (the property at proptest's 256 cases, plus every clip and field of the corpus at 5 values each); `veel_cli` 5. All pass.
- **Done when, part by part:**
  - `veel check` and `veel fmt --check` exit 0 with no output on each of the 5 corpus folders (built binary, `target/debug/veel`, and `veel_cli`'s test `check_and_fmt_pass_on_the_slice_corpus`).
  - `tests/negative` has one case per code, snapshotted with its suggestions (above).
  - The property test `a_one_field_edit_changes_only_its_word`: a random edit of `at`, in, out or crossfade of a random corpus clip replaces one word token by one word, every byte outside it identical, every comment of every file identical, each part's unit exactly the one `veel_time` can write the new value in (its old unit wherever exact), and the checker reads the new value back.
- **Controls** (each fault planted in the source, run, then restored with `git checkout -- <file>`):
  - Overlaps never reported (`n == 0` → `n >= 0` in `overlaps`): 3 of 10 `veel_check` integration tests failed (`every_case_prints_its_golden`, `every_code_has_one_case_reporting_only_it`, `planted_mistakes_in_the_corpus_are_reported`).
  - `veel-crossfade-next` never pushed: the same 3 failed.
  - `veel-key-order`'s constant set to `veel-name`: `no_code_is_also_the_syntaxs` failed.
  - Edits always written in frames: first run, only the example test failed and the property passed, because it let any part fall back to frames. The property was strengthened (commit `1f42017`, the oracle is `veel_time`'s writer); then 3 of 5 `veel_edit` tests failed, the property among them.
  - The edit replacing the whole clip line instead of its word: 3 of 5 failed (the property, `every_clip_and_field_of_the_corpus`, the example test).
  - `veel fmt --check` on a copy of `main`'s corpus (`4c3f3a3`): all 5 folders listed "not in canonical form", exit 1; `veel check` on that copy exits 0 for all 5.
  - In-suite controls: `planted_mistakes_in_the_corpus_are_reported` (one planted mistake per kind the done-when names, each reported) and `the_canonical_order_moves_what_is_out_of_order_and_keeps_its_meaning` (a scrambled file comes out sorted, checks clean before and after, same model).
- Read-only extra: U0.2b's hand-written folder (`../Annilay-u02-slice`) checks clean (its zero-size rect now valid by B-56); `veel fmt --check` lists its `sequences/main.veel` (alignment spaces).
- `just check` passed on the final code tree (`a2f3fc732726`, after `git merge main`: already up to date): 10 s against the 11 s budget, warm, load 10 on 18 cores. It warned that `../Scowr` is at `399dbf90f247`, not the recorded `6d674e7303a7`; `tools/scowr-commit` is a gate file and was left for the integrator.
- Load: the build guard held compiles about 35 minutes at heavy thermal pressure (load 8 to 12 on 18 cores) before the first build; later runs waited briefly.

### Found

- A file with a syntax error stops the semantic check of the whole project (one mistake, one error; a handle defined in the broken file would otherwise be "unknown" everywhere). A session fixing a large project sees its syntax errors first.
- Section 17's "keys are written in strictly increasing time from 0" is read as times from 0 up, not as "the first key is at 0"; the checker accepts a first key after 0 (proposed row below).
- The crossfade rule is applied to every clip: the parts of the mixes at its two ends, each 0 when there is none, fit in its length. Section 17 now says so in one clause; Kdenlive's own refusal of a mix longer than its clip is not measured (proposed row).
- `veel-repeated-name` (section 7, a warning) is not in the slice's check (proposed row).
- Annilay's `docs/REPOSITORIES.md` "Veel" table, which `compiler.md` says changes first, still lists `veel_fmt` / `veel_print`; this area may not edit Annilay (proposed row).
- `tests/ui/` has only the syntax's cases; the checker's diagnostics are covered by `tests/negative/` (the README row now says so).

### Unverified

- That a zero-size `rect` draws nothing (B-56's wording) is read from Kdenlive's editor allowing 0 and from geometry, not seen on the build.

## Board rows

### B-40: section 8's example

- **What works:** the example is now a whole project (a source, a sequence, a track, a clip holding the effect), so it checks; `veel_check`'s `spec_examples` test checks every unfenced block, blocks under one heading read as one project with each file named by its first line: 3 projects (section 8; section 17's two worked examples, the second in two files), all clean.
- **Measured:** `every_unfenced_spec_example_checks_clean` passes. Control: `the_section_8_fragment_fails_the_check` plants the old fragment unfenced and gets `veel-kind` (an `effect` at a file's top).
- **Found / Unverified:** none.

### B-43: booleans in section 5

- **What works:** section 5 has a "Switch" row: `on`, `off`, the name alone on a line of its own meaning `on`, citing `null-v0.md:184`. The checker reads it so (`distort`, `distort off`; `distort yes` is `veel-enum`).

### B-44: the crossfade's two errors

- **What works:** `veel-crossfade-next` and `veel-crossfade-overlap` are in `veel_check::codes::ALL`, each with its negative case (`tests/negative/veel-crossfade-next.veel`: a gap after the clip, and a last clip; `veel-crossfade-overlap.veel`: both ends of one clip, a mix longer than its clip at its end, and an incoming mix longer than its clip, each with a fix shortening the crossfade). Section 17 names both codes; section 14 lists all of the checker's codes.
- **Measured:** the control above (crossfade-next never pushed) fails 3 tests.

### B-56: a `rect`'s bounds

- **What works:** section 17's Transform table's `rect` row gives its bounds ("None in the file; its own: `w` and `h` at least 0 (0 shows nothing), `x` and `y` any"), and the "A rectangle" bullet says why: Kdenlive's keyframed rectangle editor takes 0 (`allowNullRect`, `src/assets/view/widgets/keyframecontainer.cpp:757`, `src/widgets/geometrywidget.cpp:85,103` at `v26.08.3`, read in Annilay's `kdenlive` branch), a zero width or height scales the picture to no area, and a negative one or an opacity outside 0 to 1 is `veel-range` (negative case `veel-range.veel`).
- **Unverified:** what a size animation grows from (the issue's second half, outside its done-when) is not stated; see the proposed row.

### B-58: whether `timecode` is required

- **What works:** the Kinds table now says a sequence holds "at most one `timecode` line (left out, `timecode non-drop start=00:00:00:00`)", as the attributes table's default says; the checker accepts none and reports a second (`veel-repeated`, negative case `veel-repeated.veel`).

## Proposed board rows

| Issue | Severity | Repo | Done when |
| --- | --- | --- | --- |
| The slice's check does not warn a repeated display name (`veel-repeated-name`, spec section 7) | friction | Veel | `veel_check` warns two elements of one kind with one display name, with a negative case, and `codes::ALL` lists the code |
| `veel fmt` leaves no defaults out, since the slice's defaults are not measured (B-41) | friction | Veel | After B-41, `veel_print::canonical` leaves out a line of its own holding its measured default (Null's rule), and every-parameter keeps its defaults by a comment saying why |
| Section 17 does not say whether a keyframe block's first key must be at frame 0 ("strictly increasing time from 0") | friction | Veel | Section 17 says it, citing Kdenlive's keyframe model, and `veel_check` follows it with a negative case |
| What a `rect` size animation grows from (B-56's second half) is not stated or measured | friction | Annilay, Veel | The census records `qtblend`'s interpolation of `rect` from a zero-size key on the build, and section 17 says it |
| The crossfade rule's one-ended case (a clip's own crossfade's part before its end fits in its length with no incoming mix) is not measured against Kdenlive | friction | Annilay | A census or gateway case shows Kdenlive refusing such a mix (`TimelineModel::mixClip`), or section 17 drops the clause |
| Annilay's `docs/REPOSITORIES.md` "Veel" table lists `veel_fmt` / `veel_print` where Veel now has `veel_print` alone | friction | Annilay | The table matches Veel's `docs/design/compiler.md` |

## For the integrator

Nothing here needs the app; every row is proven by `cargo test` in Veel. On the round's merge, in `work/integrate/Veel`:

```text
$ cargo test -p veel_check -p veel_print -p veel_edit -p veel_cli --locked   # every test result: ok
$ cargo build -p veel_cli && for d in tests/slice/*; do target/debug/veel check $d && target/debug/veel fmt --check $d; done; echo $?
0                                                                             # no other output
$ target/debug/veel check tests/negative/veel-overlap.veel; echo $?          # prints veel-overlap.stderr
1
```

For U1a.1 ("Veel's spec and `veel --help` alone"), the session needs the binary: `cargo build -p veel_cli` in Veel gives `target/debug/veel`.

## Unverified until the round's build

None: no row of this area needs the app.
