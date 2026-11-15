# agent/veel-1a: Veel's syntax adapter and minimal time (batch 2)

Worker `veel-1a` in Annilay's batch 2, on branch `agent/veel-1a` in Veel only (worktree `Annilay/work/wt/veel-1a/`). Rows 1a-01 and 1a-02 on Annilay's board. Scowr (`554dd9a`) and Null (`1a22fa1`) were read, never changed.

**Before, on `main`** (Veel `711715d`, the 0-01 scaffold), repeatable from Annilay's main checkout:

```text
$ ls ../Veel/compiler                                         # veel_syntax only
$ grep -c "pub fn diagnostics" ../Veel/compiler/veel_syntax/src/lib.rs   # 0: no adapter, no codes
$ ls ../Veel/tests                                            # README.md only: no ui/, no roundtrip/
$ ls ../Veel/compiler/veel_time                               # No such file or directory
```

The batch probe's line `1a-0x veel crates:` is empty on `main` and stays empty after this branch, because it lists `../Veel/crates` and Veel's crates live in `compiler/` (see "Found").

## 1a-01 `veel_syntax`

### What works

- `veel_syntax` re-exports Scowr's tree, typed AST, lexer and diagnostics (`scowr_syntax`, `scowr_errors` by path through the `Scowr` link, as `null_syntax` does), and gives `parse`, `diagnostics` and `dump`.
- Every syntax error carries `veel-syntax`, never Scowr's code. Scowr's musical examples are reworded (`vel=96` becomes `rate=25`, `\u{266A}` becomes `\u{e9}`), and `rate=25` on a line of its own or `size = …` gets Null's child-node message and fix. `messages::KNOWN` lists the 22 messages Scowr's parser gives (read against Scowr `554dd9a`: the same set Null read), and a test fails on any other.
- `veel-name`: node names and property keys must be kebab-case. A handle may stand as a node name. A string name is an error. Every error offers the kebab-case spelling as its fix where one exists. Lines inside `raw` (at any depth) and nodes commented out with `/-` are exempt. A drop-frame timecode written with `;` (`00:00:10;00`) gets a note saying timecode is written with colons.
- UI tests: `tests/ui/syntax/` (workspace root), 12 cases, 3 good (empty `.stderr`) and 9 broken, compared byte for byte with `.stderr` goldens in Scowr's `file:line:col error[code]: message` form (`VEEL_BLESS=1` rewrites them). Between them the cases report both codes, every message is a known one, and none carries Scowr's examples.
- Only this crate names Scowr. A test walks every crate under `compiler/` and fails if any other crate's `.rs` or `.toml` mentions Scowr. It also fails if the root manifest names Scowr anywhere except the two path dependencies.
- **Does the shared syntax express every slice construct? Yes, with no gap found.**
  - The blueprint's slice (`tests/roundtrip/blueprint-slice.veel`, verbatim from 02-slice.md) reads as it means: the range `00:00:10:00..+5s` is one word, `at 0` and `at 5s` are keyword attributes, `size [1920 1080]` is a list, and `rect { key 0 [...]; key 2s [...] }` gives two `key` children, each with a time and a list.
  - All 29 value forms of spec section 5 are one word each: timecode, both range forms, `30000/1001`, `1:02.250`, `-3dB`, `48kHz`, `#ff880080`, `@pkg:lt-name`, `@a/@grade-c`, `sha256:…` and the rest.
  - All 6 `veel` blocks in `docs/spec/veel-v0.md` and both round-trip files print back byte for byte, with no diagnostics.
  - No `blocked` row is needed.

### Measured

- `just check` exit 0 (check record on tree `06605e61244e` before this report was added; re-run on the committed tree, below). `cargo test -p veel_syntax`: **19 tests**: 3 unit, 15 integration (`tests/main.rs`: 1 module check, 1 boundary, 9 syntax, 4 UI) and 1 doc test, 0 failed.
- **Broken on purpose:** `is_kebab_case` changed to accept `_` (`|| b == b'_'`). **4 of the 15 integration tests failed**: `kebab_case_is_the_rule_for_names`, `a_raw_blocks_lines_keep_kdenlives_names_and_nothing_else_does`, `the_fixes_make_the_file_kebab_case` and the UI test `every_case_prints_its_golden` (`field_order` lost its error). The first run of this break showed the UI goldens missed it, since every bad name there also had a capital, so `field_order` was added to `not-kebab-case.veel`. Restored: 19 of 19 pass.

### Found

- The batch probe (`work/batch2/probe.sh`, last line) lists `../Veel/crates`, but Veel's crates are in `compiler/` (`docs/design/compiler.md`), so the line is always empty. For the lead: `ls ../Veel/compiler` would show the change.
- Spec section 4 says "Outside `raw`, a name that is not a word … is a string". That could be read as allowing string names, while Null makes every string name a `null-name` error. I followed Null, the smaller choice and the one that keeps every name a word: a string name is `veel-name` with the kebab-case fix. 1b-05 decides how Kdenlive's non-kebab names map, and may relax this.
- Handles in arguments (`@Bad` in `from=@Bad`) are not checked by `veel-name`: as in Null, they are the checker's (1a-04).
- `tests/ui/syntax/` is run by `veel_syntax` without a schema. 1a-04's checker runs `tests/ui/` with the slice's schema, and should run or take over these cases (the README says so).

### Unverified

Nothing here needs the build: `veel_syntax` is pure Rust and runs fully in `just check`. The fuzz target and the recovery tests are 1a-12.

Unverified until the batch build: in the integrator's `work/integrate/Veel`, `cargo test -p veel_syntax --locked` prints `test result: ok. 3 passed`, `test result: ok. 15 passed` and `test result: ok. 1 passed`, and `just verify` passes (`tools/fuzz.sh` prints `fuzz: no fuzz targets yet` until 1a-12).

## 1a-02 `veel_time` (minimal)

### What works

- `compiler/veel_time`, with no dependencies and proptest as its only dev-dependency.
  - `Rate`: an exact rational. `25` and `30000/1001` are read. A decimal is `veel-time-inexact` with the exact rate as its fix: `29.97 is not exact; write 30000/1001`, the spec's own wording. Likewise `23.976` and `23.98` become `24000/1001`, `59.94` becomes `60000/1001`, and `12.5` becomes `25/2`.
  - `Time`: the words `120`, `-12`, `2s`, `1.5s`, `250ms` and `01:00:10:12`.
  - `Timecode`: non-drop, 30 frames to a timecode second at `30000/1001`, with each field checked against the rate.
  - `Clock`: what a frame is (the sequence's rate), plus the timecode's rate and the timecode of frame 0. A sequence's clock uses its own rate and `start=`; a clip's source range uses the source's rate and `start-timecode`.
  - `Range`: half-open `[start, end)`, read from `in..out` and `in..+duration`. Its length is `end - start`; touching ranges do not overlap.
- **An off-frame value is an error, never rounded** (V20, corrected). `Inexact` carries the exact frames and the whole frames on either side. Each choice is written in the word's own unit where that is exact (`0.03s` at 25 gives `0s` or `0.04s`; `12s` at 24000/1001 gives `287` or `12.012s`), and in frames otherwise. Nothing in the crate rounds. Scowr's time crate was read but not used or copied, and its halves-up rounding is the one thing the tests break on purpose.
- `Clock::write(frame, unit)` gives the word in a unit that names exactly that frame, if there is one, for write-back to keep a line's unit (1a-04).
- `TimeError::code()` gives the checker's code for each error: `veel-time-inexact`, `veel-unit`, `veel-range` or `veel-type`.

### Measured

- `cargo test -p veel_time`: **21 tests**: 9 unit, 11 integration (1 module check and 10 property tests at proptest's default of 256 cases each) and 1 doc test, 0 failed. At `just verify`'s depth (`PROPTEST_CASES=4096`, `--release`) the same 21 pass.
- The `30000/1001` properties:
  - `n ms` is a whole frame exactly when 1001 divides n, and is then 30n/1001 frames (n below 10⁷). `n s` is a whole frame exactly when 1001 divides n (n below 10⁹).
  - `k × 1001 ms` is frame 30k, and so is the same time written as `k.kkk s`. One millisecond either side is inexact, with 30k and its neighbour as the choices (k below 10⁷).
  - A timecode label names its count's frame, and `:30` is refused.
  - A source at 24000/1001 in a sequence at 30000/1001 gives source frame d as 5d/4 sequence frames, whole exactly when 4 divides d.
  - Unit test: `01:00:00:00` is 108,000 frames, which is 3,603.6 s of media.
- Other properties: at any rate (8 common ones and arbitrary rationals up to 240000/1001), frames to seconds and back is the identity. A seconds or milliseconds decimal is either whole frames or an inexact error, and its choices convert back to exactly the frames on either side. Timecode counts round-trip and carry correctly. Ranges are half-open.
- **Broken on purpose:** `Clock::convert` changed to round halves up instead of returning `Inexact`. **8 of the 21 tests failed**: unit tests `the_specs_examples` and `the_choices_keep_the_unit_where_it_is_exact`; properties `seconds_and_millis_are_exact_or_errors`, `ntsc_millis_land_on_frames_only_at_multiples_of_1001`, `ntsc_seconds_land_on_frames_only_at_multiples_of_1001`, `ntsc_boundaries_either_side_of_a_whole_frame` and `a_source_timecode_at_another_rate_is_exact_or_an_error`; and the doc test. Restored: 21 of 21 pass. The broken run's `proptest-regressions/` file was deleted, not committed.
- The boundary test from 1a-01 now covers `veel_time` and passes. It was narrowed in this commit to crate identifiers (`scowr_…`) outside comments, so prose may mention Scowr.

### Found

- Annilay's blueprint 02-slice.md still lists "one rounding to nearest even" for 1a-02, which V20 (corrected) replaced. Added as board row 1a-15.
- Smallest choices the spec does not settle, for the lead:
  - A timecode used as a duration (`in..+00:00:05:00`) counts from `00:00:00:00`, not from the clock's start.
  - Negative positions are read (`-12`, `-2s`); whether an element may sit there is the checker's.
  - An empty range (`5..5`) is a range; whether a clip may be empty is the checker's.
  - A number in a time word has at most 18 digits, so every conversion fits in `i128`. A longer one is a `veel-range` error ("too large").
- The codes from `TimeError::code()` are listed again in the checker's `codes::ALL` by 1a-04. A malformed time word and a timecode field out of range are both `veel-type`.
- Not in the minimal time, as the row says: `smp`, `min`, `h`, `m:ss.fff`, drop-frame (V42) and VFR maps. These are row 1c-01, and until then the words are refused as `veel-unit` or `veel-type`.

### Unverified

Nothing here needs the build: `veel_time` is pure Rust and runs fully in `just check`. No other crate uses it yet (1a-04).

Unverified until the batch build: in the integrator's `work/integrate/Veel`, `PROPTEST_CASES=4096 cargo test -p veel_time --locked --release` prints `test result: ok. 9 passed`, `test result: ok. 11 passed` and `test result: ok. 1 passed`, and `just verify` passes.
