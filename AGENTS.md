# Veel agent guide

Veel is the project language of Annilay ([README](README.md)). This file is the router for agents: the rules that always apply, then which guide to read for your task. The plan, the progress page, the board, the handoff and the guide to running the work are Annilay's, for all three repositories (`../Annilay/AGENTS.md`).

## Always

- **Read `../Annilay/HANDOFF.md` first, and check it against Git:** Claude and GPT both work in these repositories, and either may have moved on.
- **Work from Annilay's plan** (`../Annilay/docs/plans/2026-11-16-execution-plan.md`), in rounds by `../Annilay/docs/agents/running-the-plan.md`: each area in its own worktree (`just wt-new <name> --repos veel` in Annilay), one integrator merges. What is done is said only by `../Annilay/docs/PROGRESS.md`; every open issue is a row of `../Annilay/docs/BOARD.md`. A task behind an open gate in Annilay's `HANDOFF.md` waits; never infer RL's answer.
- Read the [specification](docs/spec/veel-v0.md) before changing anything the language means. Decisions RL settled are not reopened; propose a change through `docs/rfcs/`.
- **The freeze rule.** Once Veel v0 freezes (Annilay's task 1d-01), a change to the spec needs an RFC first ([`docs/rfcs/README.md`](docs/rfcs/README.md)): any new form, word, unit, default, diagnostic or rule, any change to what a file means, its canonical form or its write-back. Only a fix to wording that changes no meaning, and implementation that follows the spec as it is, go in without one. An accepted RFC names itself in the spec and in `CHANGELOG.md`. Until the freeze the draft changes with the slice (1a) and the schema (1b).
- Original code only, with permissive dependencies checked first (`deny.toml` lists the licences allowed; `just check` refuses others). **Never copy or translate GPL-family code, including Kdenlive's and MLT's:** Veel is MIT, and reads Kdenlive's assets only through the schema file Annilay generates.
- **Never change Scowr from here.** Veel uses Scowr's `scowr_syntax` and `scowr_errors` through `veel_syntax` only; the boundary is in [the compiler's design](docs/design/compiler.md#the-shared-front-end). Format with `cargo fmt` (`just fmt`), never `cargo fmt --all`, which would rewrite Scowr's files.
- **Commit every working change** as soon as `just check` exits 0 on its own (format, Clippy with warnings as errors, tests, the licence gate); an area agent commits on `just affected` once Annilay's task 0-43 lands. A commit of documents alone needs only the hooks. **Area agents run no `just verify`:** the integrator verifies once per round (V45).
- **Never push: committing is enough** (G1). Never bypass the hooks (`just hooks` installs them).
- **Capability claims** carry an evidence label in Annilay's `docs/CAPABILITIES.md` (V47).
- **The Mac is shared.** Every compile goes through the build guard (`tools/build-guard.py`, an identical copy of Sau's); do not bypass it or raise its limits, and kill nothing you did not start.
- Annilay (`../Annilay`), Veelindy (`../Veelindy`) and Scowr (`~/Projects/Scowr`) are separate repositories, as are Sau, Null and Cosmósis. Change them only when the task says so.

## Task routes

| Task | Read |
| --- | --- |
| What the language means | `docs/spec/veel-v0.md` |
| Proposing a language change | `docs/rfcs/README.md` |
| How the compiler is laid out; the front end shared with Scowr and its known edges | `docs/design/compiler.md` |
| Writing Veel for an AI or a person | `docs/guide/ai.md` |
| Project layout, packages and commands | Veelindy's `docs/spec/veelindy-v0.md` |
| What is planned, and in what order | Annilay's `docs/ROADMAP.md`; the steps under it, Annilay's `docs/plans/2026-11-16-execution-plan.md` |
| What is done, task by task | Annilay's `docs/PROGRESS.md` |
| Open issues | Annilay's `docs/BOARD.md` |
| The current state and RL's gates | Annilay's `HANDOFF.md` |
| Carrying out the plan in rounds | Annilay's `docs/agents/running-the-plan.md` |
| Commits, reports, code style | Annilay's `docs/agents/workflow.md` |
| Worktrees, machine limits, merging | Annilay's `docs/agents/parallel.md` |
| What each repository owns; licences | Annilay's `docs/agents/boundaries.md` |
| What to test, the check's budget, evidence labels | Annilay's `docs/agents/testing.md` |
| The syntax Veel may share | Scowr's `docs/spec/scowr-v0.md`, section 4 |
| The icons | `branding/README.md` |
