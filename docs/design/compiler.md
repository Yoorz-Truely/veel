# The compiler

How Veel's compiler is laid out, crate by crate, under `compiler/`. Each crate is added in the phase that needs it. The table is Annilay's (`../Annilay/docs/REPOSITORIES.md`, "Veel"), where the plan changes first; Null's `compiler/` is the model.

| Crate | Role | Phase |
| --- | --- | --- |
| `veel_syntax` | The only crate that names Scowr. It wraps `scowr_syntax` and `scowr_errors`, re-exports the tree, typed AST and diagnostics, maps Scowr's syntax errors to `veel-syntax`, and adds the kebab-case naming check `veel-name` (`AGENTS.md`, "The shared front end") | Scaffold 0-01; 1a |
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
