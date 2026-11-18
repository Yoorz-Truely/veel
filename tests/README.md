# Tests

The tests that hold the compiler to the [specification](../docs/spec/veel-v0.md), laid out as Scowr's (`../Scowr/tests/README.md`). The [compiler design](../docs/design/compiler.md) says which crate each kind of test proves. `cargo test --workspace` runs every one of them. The folders are created by the tasks that first need them; until then this page is the plan.

| Folder | Holds | Proves | Arrives with |
| --- | --- | --- | --- |
| `ui/` | One `.veel` file per case, with its expected diagnostics in a `.stderr` file beside it, checked with the slice's schema. `ui/syntax/` holds the syntax's own cases, run by `veel_syntax` with no schema: good files (named `good-…`, an empty `.stderr`) and broken ones | Diagnostics: code, place and wording | 1a-01 (`ui/syntax/`); the checker's codes have their cases in `negative/` since 1a-04 |
| `ui-spec/` | The same, checked with the whole schema generated from Kdenlive's assets | Diagnostics of every asset kind and parameter | 1b-04 |
| `negative/` | One case per diagnostic code, with its expected diagnostics beside it | Every code, message and fix the spec lists | 1a-04 |
| `roundtrip/` | Files that must print back byte for byte and lower to the same model. Until lowering exists (1a-04), `veel_syntax` checks that each prints back with no diagnostics | The lossless tree, the printing rule and lowering | 1a-01 |
| `slice/` | Files written only in the phase 1a slice | That the slice checks with no diagnostics at all, lowers, prints back and edits minimally | 1a-03 |
| `corpus/` | Hand-written projects, from one clip to a whole cut | Everything: every other test runs over them too | 1d-07 |
| `migrate/v0/` | The corpus's `.veel` files as they were when v0 was frozen | That `annilay migrate` turns every v0 project into the current format, and changes nothing the second time | 1d-02 |

## `ui/` and `ui-spec/`

Each case is a file that the checker reads as a whole project, and a `.stderr` file with exactly what `veel check` prints for it, as in Rust's `tests/ui`. The runner compares the two byte for byte. Run the tests with `VEEL_BLESS=1` to rewrite the `.stderr` files when a message changes on purpose, then read the diff.

- A diagnostic is one line, `file:line:column severity[code]: message`, then any secondary places, `note:` lines and `fix:` lines, each indented by two spaces. The file is named relative to its folder.
- Lines and columns count from 1, columns in Unicode scalar values.
- The first line of a case is a comment saying what it tests.

## `negative/`

One case per diagnostic code, named after the code: a file, or a project folder when the code needs several files. Each case covers every way the checker reports its code, and its `.stderr` file beside it is exactly what `veel check` prints. A test checks the goldens (`VEEL_BLESS=1` rewrites them), that every code has a case reporting it, and that every fix offered applies and mends what it was offered for.

## `roundtrip/`

Each file must come back from the lossless syntax tree byte for byte, comments, spacing and spelling included, and lower to the same model when printed in canonical form. Every corpus file and example is a round-trip case too.

## `slice/`

One folder per example, each a slice project as the spec's section 17 describes it: a folder of `.veel` files read as one project, the first line of each naming its path in the folder. `two-clips/` and `picture-in-picture/` are the section's worked examples, byte for byte. Since 1a-03, `veel_syntax`'s spec-example test (`compiler/veel_syntax/tests/spec_examples.rs`) parses every file with no diagnostic, and `veel_schema`'s corpus test (`compiler/veel_schema/tests/corpus.rs`) fails on a name the slice schema does not hold; from 1a-04, `veel check` checks each folder in full.

## `corpus/`

Each folder is a whole project laid out by Veelindy, with its manifest. The corpus is used by every other test, by the documentation and to teach AI, so it is kept correct to the letter. A coverage test fails when a kind or attribute of the schema has no example (1d-07).

## Fuzzing

`fuzz/` holds the fuzz targets once 1a-12 adds them; `tools/fuzz.sh` runs each for 60 seconds in `just verify`.
