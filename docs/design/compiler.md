# The compiler

How Veel's compiler is laid out, crate by crate, under `compiler/`. Each crate is added in the phase that needs it. The table is Annilay's (`../Annilay/docs/REPOSITORIES.md`, "Veel"), where the plan changes first; Null's `compiler/` is the model.

| Crate | Role | Phase |
| --- | --- | --- |
| `veel_syntax` | The only crate that names Scowr. It wraps `scowr_syntax` and `scowr_errors`, re-exports the tree, typed AST and diagnostics, maps Scowr's syntax errors to `veel-syntax`, and adds the kebab-case naming check `veel-name` (below, "The shared front end") | Scaffold 0-01; 1a |
| `veel_time` | Rational rates, frames, samples, timecode, drop-frame display, half-open ranges, VFR maps, exact conversions | 1a (minimal), 1c |
| `veel_value` | Typed values, units, exact float printing, colours, keyframe curves | 1a |
| `veel_schema` | The element schema: the hand-written editorial core, plus the asset schema Annilay's `just schema` generates from Kdenlive's assets and commits in `schema/`. Veel reads only that file | 1a, 1b |
| `veel_check` | Resolution of handles across files, types, overlaps, range validity, references | 1a |
| `veel_fmt` / `veel_print` | Lossless formatter and canonical printer | 1a, 1d |
| `veel_edit` | Tree patches that keep comments, units and abstract forms | 1a, 3 |
| `veel_live` | Diff of two revisions by stable ID into native requests, and the inverse | 1a, 2, 3 |
| `veel_kdenlive` | Lossless `.kdenlive` ↔ Veel conversion, with the `raw` fallback | 1d |
| `veel_otio` | OpenTimelineIO, SRT, ASS and VTT | 4b |
| `veel_lift` | Lifting raw imports into patterns and components | 4b |
| `veel_migrate` | `veel-format` migrations | 1d |
| `veel_lsp` | Language server | 3 |
| `veel_cli` | The `veel` command for the language alone: `check`, `fmt`, `print` | 1a |

Each crate keeps its integration tests in one test binary (`tests/main.rs`, `autotests = false`), as Null's do. Where each kind of test lives is in [`tests/README.md`](../../tests/README.md).

## The shared front end

Moved here from `AGENTS.md` on November 16, 2026. Veel writes Scowr's node syntax and uses Scowr's front-end crates for it, by path from `../Scowr` (V17), as Null does. The boundary:

| Shared, Scowr's (`../Scowr/compiler/`) | Veel's own (`compiler/`) |
| --- | --- |
| `scowr_syntax`: lexer, lossless syntax tree, parser with error recovery, incremental reparse, the typed view (`ast`) | `veel_syntax`: the only crate that names Scowr's; re-exports the tree, gives syntax diagnostics Veel's code (`veel-syntax`) and checks that names are kebab-case (`veel-name`) |
| `scowr_errors`: spans, files, diagnostics, fixes, their rendering, `closest` for suggestions | Everything a word means: frames, rates and units, handles, kinds, the schema from Kdenlive's assets, the checker, the printer and write-back, the language server |

- Every other Veel crate depends on `veel_syntax`, never on a `scowr_*` crate, so when the shared crates move to their own repository only `veel_syntax`'s manifest changes. The paths are in the root `Cargo.toml`, `[workspace.dependencies]`.
- Scowr's other crates (`scowr_value`, `scowr_schema`, `scowr_check`, `scowr_fmt` and the rest) are Scowr's language, not shared syntax; Veel does not use them.
- Never change Scowr from here. A change Veel needs in the shared syntax is agreed with RL first, made in Scowr, and passes Scowr's own tests (`cargo test --workspace` in `../Scowr`). Record such a need as a row on Annilay's board (`../Annilay/docs/BOARD.md`), severity decision, with the exact change.
- Format with `cargo fmt` (what `just fmt` and `just check` run), never `cargo fmt --all`: `--all` also formats local path dependencies, which would rewrite Scowr's files.
- Known edges of the shared syntax, for Veel's checker to handle (found by Null): `at`, `to` and `len` followed by a space and a value are read as Scowr's keyword attributes (`NodeKind::Keyword`), not arguments; and a node ends only at a line feed, `;` or `}`, so `{ width 4mm segments 3 }` is one child, `width`, with three arguments. Add any edge Veel finds. Found by Veel (1a-01): a drop-frame timecode written with `;` (`00:00:10;00`) ends its node at the `;`, so its frames start a new node; `veel_syntax` adds a note saying timecode is written with colons. A list may run over lines, so an unclosed `[` runs on to the end of the file.
