# Veel agent guide

Veel is the project language of Annilay ([README](README.md)). This file is the router for agents. Building starts with Annilay's phase 1a; the shared workflow, task board and limits are Annilay's (`../Annilay/AGENTS.md`).

## Always

- **Read `../Annilay/HANDOFF.md` first, and check it against Git:** Claude and GPT both work in these repositories, and either may have moved on.
- Read the [specification](docs/spec/veel-v0.md) before changing anything the language means.
- Decisions RL settled are not reopened; propose a change through `docs/rfcs/`.
- **The freeze rule.** Once Veel v0 freezes (Annilay's task 1d-01), a change to the spec needs an RFC first ([`docs/rfcs/README.md`](docs/rfcs/README.md)): any new form, word, unit, default, diagnostic or rule, any change to what a file means, its canonical form or its write-back. Only a fix to wording that changes no meaning, and implementation that follows the spec as it is, go in without one. An accepted RFC names itself in the spec and in `CHANGELOG.md`. Until the freeze the draft changes with the slice (1a) and the schema (1b).
- Original code only, with permissive dependencies checked first. **Never copy or translate GPL-family code, including Kdenlive's and MLT's:** Veel is MIT, and reads Kdenlive's assets only through the schema file Annilay generates.
- Work from Annilay's task board (`../Annilay/docs/TASKS.md`, `just tasks` in Annilay): claim a task before starting it, hold at most three, and move each through `review` and `done` (`../Annilay/docs/agents/workflow.md`). A lead running a batch follows `../Annilay/docs/agents/batches.md`.
- **Open issues are rows** on Annilay's board with a checkable "Done when", added in the commit that finds them, never prose in a report or `HANDOFF.md`.
- **A task behind an open gate** in Annilay's `HANDOFF.md` waits. Never infer RL's answer.
- The only agent writing Veel works in this main checkout on `main`; a second agent writing Veel at the same time works in its own worktree and branch, `just wt-new <name> --repos veel` in Annilay (V40, `../Annilay/docs/agents/parallel.md`).
- **Commit every working change** as soon as it works and `just check` exits 0 on its own (format, Clippy with warnings as errors, tests, the licence gate); a commit of documents alone needs only the git hooks. Before a task goes to review: `just sync` in Annilay's worktree, then `just check`. **A worker runs `just check` only;** the integrator runs `just verify` once per merged batch (V45): property tests, fuzzing, and Annilay's crates built against this checkout once they exist, so a change to an API Annilay uses fails here first.
- **Never push: committing is enough** (G1). Never bypass the hooks (`just hooks` installs them).
- **Capability claims** carry an evidence label in Annilay's `docs/CAPABILITIES.md` (V47).
- Only permissive dependencies: `deny.toml` lists the licences allowed, and `just check` refuses others.
- Every compile goes through the build guard (`tools/build-guard.py`, an identical copy of Sau's), which keeps the shared Mac cool. Do not bypass it.
- Annilay (`../Annilay`), Veelindy (`../Veelindy`) and Scowr (`~/Projects/Scowr`) are separate repositories, as are Sau, Null and Cosmósis. Change them only when the task says so.

## The shared front end

Veel writes Scowr's node syntax and uses Scowr's front-end crates for it, by path from `../Scowr` (V17), as Null does. The boundary:

| Shared, Scowr's (`../Scowr/compiler/`) | Veel's own (`compiler/`) |
| --- | --- |
| `scowr_syntax`: lexer, lossless syntax tree, parser with error recovery, incremental reparse, the typed view (`ast`) | `veel_syntax`: the only crate that names Scowr's; re-exports the tree, gives syntax diagnostics Veel's code (`veel-syntax`) and checks that names are kebab-case (`veel-name`) |
| `scowr_errors`: spans, files, diagnostics, fixes, their rendering, `closest` for suggestions | Everything a word means: frames, rates and units, handles, kinds, the schema from Kdenlive's assets, the checker, the printer and write-back, the language server |

- Every other Veel crate depends on `veel_syntax`, never on a `scowr_*` crate, so when the shared crates move to their own repository only `veel_syntax`'s manifest changes. The paths are in the root `Cargo.toml`, `[workspace.dependencies]`.
- Scowr's other crates (`scowr_value`, `scowr_schema`, `scowr_check`, `scowr_fmt` and the rest) are Scowr's language, not shared syntax; Veel does not use them.
- Never change Scowr from here. A change Veel needs in the shared syntax is agreed with RL first, made in Scowr, and passes Scowr's own tests (`cargo test --workspace` in `../Scowr`). Record such a need on Annilay's board as `blocked` with the exact change.
- Format with `cargo fmt` (what `just fmt` and `just check` run), never `cargo fmt --all`: `--all` also formats local path dependencies, which would rewrite Scowr's files.
- Known edges of the shared syntax, for Veel's checker to handle (found by Null): `at`, `to` and `len` followed by a space and a value are read as Scowr's keyword attributes (`NodeKind::Keyword`), not arguments; and a node ends only at a line feed, `;` or `}`, so `{ width 4mm segments 3 }` is one child, `width`, with three arguments. Add any edge Veel finds. Found by Veel (1a-01): a drop-frame timecode written with `;` (`00:00:10;00`) ends its node at the `;`, so its frames start a new node; `veel_syntax` adds a note saying timecode is written with colons. A list may run over lines, so an unclosed `[` runs on to the end of the file.

## Task routes

| Task | Read |
| --- | --- |
| What the language means | `docs/spec/veel-v0.md` |
| Proposing a language change | `docs/rfcs/README.md` |
| How the compiler is laid out | `docs/design/compiler.md` |
| Project layout, packages and commands | Veelindy's `docs/spec/veelindy-v0.md` |
| Where this fits in the product; the plan for this repository | Annilay's `docs/ROADMAP.md` and `docs/REPOSITORIES.md` |
| The current state and RL's gates | Annilay's `HANDOFF.md` |
| The syntax Veel may share | Scowr's `docs/spec/scowr-v0.md`, section 4 |
| Taking and finishing work; commits; reports | Annilay's `docs/agents/workflow.md` and `docs/TASKS.md` |
| Leading a batch | Annilay's `docs/agents/batches.md` |
| Worktrees, machine limits, merging | Annilay's `docs/agents/parallel.md` |
| What each repository owns; licences | Annilay's `docs/agents/boundaries.md` |
| What to test, the gates, evidence labels | Annilay's `docs/agents/testing.md` |
| The icons | `branding/README.md` |
