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
