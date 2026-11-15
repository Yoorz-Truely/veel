# Everyday commands. Run `just` to list them. The workflow, board and limits are Annilay's:
# ../Annilay/AGENTS.md and ../Annilay/docs/agents/.

# List the recipes
default:
    @just --list

# The commit gate for code: format, lint, test and the licence gate, through the build guard
check:
    #!/usr/bin/env bash
    # One script, so the check's start and its pass share a key ($$). Only a pass that runs to
    # the end is recorded; a code commit needs one on its tree (tools/check-record.py).
    set -euo pipefail
    export PYTHONDONTWRITEBYTECODE=1
    python3 tools/check-record.py start $$
    python3 tools/check-record.py selftest
    python3 tools/build-guard.py wait
    cargo fmt --check
    if [ -f fuzz/Cargo.toml ]; then
      cargo fmt --check --manifest-path fuzz/Cargo.toml
    fi
    # The pre-commit fixers' formatters in check mode, so a tree that passes is one they leave alone.
    git ls-files -co --exclude-standard -z -- '*.toml' | RUST_LOG=warn xargs -0 taplo fmt --check
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo deny --offline check licenses bans sources
    if [ -f fuzz/Cargo.toml ]; then
      cargo deny --offline --manifest-path fuzz/Cargo.toml check licenses bans sources
    fi
    python3 tools/check-record.py pass $$
    echo "check: passed"

# The integrator's gate, once per merged batch (V45), and nightly: check, deeper property tests,
# fuzzing, and Annilay's crates built against this checkout. Workers run `check` only.
verify:
    just check
    PROPTEST_CASES=4096 cargo test --workspace --locked --release
    tools/fuzz.sh 60
    # Annilay's crates built against this checkout, when they exist beside it (from 1a-06).
    tools/annilay-crosscheck.sh Veel
    @echo "verify: passed"

# Format this workspace's crates (never `--all`, which would rewrite Scowr's files)
fmt:
    cargo fmt

# Install the git hooks for this clone and all its worktrees
hooks:
    lefthook install

# Thermal pressure, load and compile slots in use
heat:
    python3 tools/build-guard.py status
