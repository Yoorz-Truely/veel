See [AGENTS.md](AGENTS.md).

For Claude Code specifically:

- Everyday commands are `just` recipes (`just` lists them): here `just check`, `just fmt`, `just hooks` and `just heat`; the board, worktrees and integration are Annilay's (`../Annilay`, `just` there). Prefer them over hand-typed command chains: they keep the checks, locks and cleanups consistent.
- Parallel subagents and workflows follow `../Annilay/docs/agents/parallel.md` and `../Annilay/docs/agents/batches.md`: workers build nothing, at most about 8 build-heavy agents at once, and `just integrate` runs in its own process session, never as a background command of yours.
- Check `../Annilay/HANDOFF.md` against `git log` before acting: GPT works in these repositories too.
