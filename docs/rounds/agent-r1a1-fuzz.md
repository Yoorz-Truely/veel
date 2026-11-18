# agent/r1a1-fuzz: recovery and fuzzing (round 1a.1, area B)

Area B of Annilay's round 1a.1, on branch `agent/r1a1-fuzz` in Veel only (worktree `Annilay/work/wt/r1a1-fuzz/Veel`, cut from Veel `4c3f3a3`). Row 1a-12 of Annilay's execution plan. Annilay (`eaa58b0`), Scowr (`399dbf9`), Null and Veelindy were read, never changed. Nothing of Annilay was built; no `just verify` was run.

**Before, on `main`** (Veel `4c3f3a3`), repeatable:

```text
$ ls fuzz                                        # No such file or directory
$ tools/fuzz.sh 1                                # fuzz: no fuzz targets yet
$ ls compiler/veel_syntax/tests                  # boundary.rs main.rs spec_examples.rs syntax.rs ui.rs: no recovery tests
```

## 1a-12 Error recovery and a fuzz target

### What works

- **`veel_syntax::invariants`** (new public module, `compiler/veel_syntax/src/invariants.rs`): `pub fn check(text: &str, start: usize, end: usize, replacement: &str)`, what must hold for any text, broken or not. It panics, naming the invariant, when the tree does not print the text back, its tokens are not the lexer's (contiguous, covering every byte), a lexer error is missing from the parse's errors, a message is not one of `messages::KNOWN`, a diagnostic's code is not in `codes::ALL`, a place or a fix's edit leaves the text or a character boundary, a diagnostic still carries Scowr's example, a syntax error goes unreported as `veel-syntax`, the diagnostics are out of text order, or an incremental reparse (`Parse::edit` of `start..end` to `replacement`) differs from a fresh parse in tree or errors. It also renders the diagnostics and the dump, and applies each fix, so a panic in any of them is caught. The recovery tests and the fuzz target share it, as Null's `null_syntax::invariants` does for Null.
- **The recovery tests** (`compiler/veel_syntax/tests/recovery.rs`, a module of `tests/main.rs`), four tests over the 11 clean `.veel` files of the tests (the slice corpus's 6 files, the 2 round-trip files, the 3 good UI cases; 6,129 bytes):
  - `every_truncation_recovers_with_diagnostics`: the file cut at **every** character boundary. Each cut keeps every invariant; every cut that leaves a `{` or `[` open, or a string or block comment running to the end, has a `veel-syntax` diagnostic saying which construct is never closed; and every node that ended on its own line (line feed or `;`) before the cut is still in the tree with the same name and the same span, so what came before a truncation is read as it was. More than half the cuts must leave something open, so the test is about broken text.
  - `garbled_slice_text_recovers_with_diagnostics`: 16 named garbles of slice text (unclosed block, stray `}`, unclosed list, stray `]`, reserved `(`, unclosed string, `from=` with no value, `rate=25` as a child line, `=25` with no key, a block with no name, text after a block, bad escapes, an unclosed block comment, a misplaced `/-`, a lone `=`, control bytes with a BOM and a CR): each keeps every invariant, says what it should (the message's start, Veel's wording where Veel rewords), and the node after the break is still found.
  - `garbled_slice_files_keep_every_invariant`: 3,000 deterministic mutations (a fixed-seed generator) of the clean files, 1 to 4 insertions, deletions or replacements each from 26 breaking pieces (`{ } [ ] ( ) " #" "# \ \⏎ // /* */ /- ; = ⏎ ⏎CR tab é U+2028 NUL .. @ :`), then an edit; each keeps every invariant; more than half must be broken.
  - `garbage_and_deep_nesting_recover`: 500 random byte strings read as UTF-8 with bad bytes replaced; 2,000-deep `{`, `[`, `a {`, `a [` (the shared parser stops at 256: "blocks/lists are nested too deeply", no stack overflow); a 64 KB line; 10,000 line feeds.
- **The fuzz workspace** `fuzz/` (its own manifest and `Cargo.lock`, `[workspace] members = ["."]`, not a member of the root workspace, which stays `compiler/*`): package `veel-fuzz` with `libfuzzer-sys` 0.4.13 and `veel_syntax` by path; one target, `fuzz/fuzz_targets/parse.rs`: the first three bytes choose an edit (start and end as fractions of the text, and one of 16 pieces to write), the rest read as UTF-8 with bad bytes replaced is the text, and `veel_syntax::invariants::check` runs on both. 20 seeds in `fuzz/seeds/parse/`, one per `.veel` file of `tests/` (clean and broken), each behind a three-byte header; `fuzz/.gitignore` keeps out `target/`, `corpus/`, `artifacts/` and `coverage/`. Veel's `just check` formats it (`cargo fmt --check --manifest-path fuzz/Cargo.toml`, Taplo) and licence-checks it (`cargo deny … --manifest-path fuzz/Cargo.toml`), as its recipe already did once the manifest exists; `libfuzzer-sys`'s NCSA licence was already allowed in `deny.toml`.
- **`just verify` runs fuzzing:** Veel's `verify` recipe already runs `tools/fuzz.sh 60` (justfile, recipe `verify`, third line), which runs every target in `fuzz/fuzz_targets/`; until now it printed "fuzz: no fuzz targets yet". Unchanged here.

### What was added to `veel_syntax`'s public API (for area A)

Only additions; no signature changed:

- `pub mod invariants` with `pub fn check(text: &str, start: usize, end: usize, replacement: &str)`.
- The crate doc names it. Nothing else in `src/` changed.

### Measured

- `cargo test -p veel_syntax`: 23 tests pass (19 before, 4 new). Timed alone, warm, under load 8 to 10: the truncation test 1.1 s, the 3,000 mutations 0.7 s, garbage and nesting 0.4 s (first written with 20,000-deep nesting and a 1 MB line it took 2.9 s; cut to the sizes above for the check's budget).
- Truncation: 11 files, every character boundary: 6,140 cuts; every one recovers. Mutations: 3,000, more than half broken (asserted).
- `tools/fuzz.sh 30` (smoke): 24,752 runs in 31 s, clean.
- **Control, planted fault** (the same one for the tests and the fuzzer): `diagnostics` made to drop the third syntax error when there are three or more (`if out.len() >= 3 { out.pop(); }` before the names were added). `cargo test -p veel_syntax recovery`: 3 of the 4 recovery tests fail (`every_truncation…`, `garbled_slice_files…`, `garbage_and_deep_nesting…`, each at `invariants.rs:79`, "every syntax error is reported"; the named garbles have fewer than three errors each). `cargo +nightly fuzz run parse <empty corpus> -- -max_total_time=120`: crash at about the 55th input, in under a second, "assertion `left == right` failed: every syntax error is reported, left: 2, right: 3", exit 1. Fault removed; all pass.
- `just check` on the committed tree: passed, 40 s against its 11 s budget (OVER), 29 s of it waiting for heavy thermal pressure to clear, so 11 s of work; load average 6.2 on 18 cores afterwards. An earlier run: 26 s, 5 s waiting for a slot, load 8.1.

### Found

- **The fixes for a truncated file do not always mend it** (Scowr's `fixes.rs`, which Veel passes through). Measured over every truncation of the slice and round-trip files (4,059 cuts with a "never closed" error): applying the first such diagnostic's fix alone leaves something never closed in 700; a "never closed" fix applied alone leaves its own error in 872 (for example a cut inside a trailing `// comment` gets `}` appended to the comment; a cut inside `size [19` gets `}` inside the open list); and applying every fix of the file together leaves errors in 3,294, since each block's fix closes every block up to its own (`}}}`, `}}`, `}`) and together they over-close. Proposed row 1. Not changed here: the fixes are Scowr's and the row asks for recovery with diagnostics, which holds.
- **Veel's `just check` warns that Scowr is `399dbf9`, not the recorded `6d674e7`.** Recording it (`just scowr-record`, a gate file) is not this area's; every test passed with `399dbf9`.
- **The check's budget.** 40 s against 11 s, of which 29 s waiting out heavy thermal pressure; the recovery tests add about 2 s of CPU in parallel with the rest. Reported with its load, not as a failure.

### Unverified

- `tools/fuzz.sh 600`: see "Measured" once it has run (below).
- Nothing in this row needs the app.

## Proposed board rows

| Issue | Seen | Severity | Repo | Done when |
| --- | --- | --- | --- | --- |
| 1. A "never closed" fix on a truncated file does not always mend it: a `}` lands inside a trailing line comment or an open `[`, and the fixes applied together over-close (700, 872 and 3,294 of 4,059 truncations of the slice and round-trip files, as "Found" counts them) | agent/r1a1-fuzz, round 1a.1 | friction (before `veel fix` applies fixes in bulk) | Scowr (its `fixes.rs`), or Veel rewording in `veel_syntax` | A test in `veel_syntax` over every truncation of the slice corpus: each "never closed" diagnostic's fix applied alone removes that diagnostic, and the fixes of a file applied together (as `veel fix` would) leave no syntax error |

## Unverified until the round's build

Nothing: row 1a-12 needs no app. The integrator's check of it on the round's merge, in Veel:

```text
$ cargo test -p veel_syntax --locked recovery     # 4 passed
$ tools/fuzz.sh 600                                # "fuzz: parse for 600s" … "Done N runs in 601 second(s)", exit 0
$ just verify                                      # includes "fuzz: parse for 60s", exit 0
```
