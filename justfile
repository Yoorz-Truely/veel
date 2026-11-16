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
    python3 tools/check-watch.py selftest
    python3 tools/scowr.py selftest
    # Warns, naming both commits, when ../Scowr is not the commit Veel last passed with (B-12).
    python3 tools/scowr.py check
    # Says when a compile waits for a slot of the build guard, and ends with the time waited (B-24).
    python3 tools/check-watch.py watch $$ &
    watcher=$!
    trap 'kill "$watcher" 2>/dev/null || true' EXIT
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
    python3 tools/scowr.py passed
    kill "$watcher" 2>/dev/null || true
    wait "$watcher" || true
    # The time against tools/check-budget.txt; over it prints OVER and still passes (B-24).
    python3 tools/check-watch.py time "$SECONDS"
    echo "check: passed"

# Only the crates a change touches and those that depend on them, against main (--list prints them); its pass lets a code commit through only on an agent/* branch in work/wt/
[positional-arguments]
affected *args:
    python3 tools/affected.py "$@"

# The integrator's gate, once per merged batch (V45), and nightly: check, deeper property tests,
# fuzzing, and Annilay's crates built against this checkout. Workers run `check` only.
verify:
    just check
    PROPTEST_CASES=4096 cargo test --workspace --locked --release
    tools/fuzz.sh 60
    # Annilay's crates built against this checkout, when they exist beside it (from 1a-06).
    tools/annilay-crosscheck.sh Veel
    @echo "verify: passed"

# Record the Scowr commit the last passing `just check` here used, in tools/scowr-commit (then check again and commit)
scowr-record:
    python3 tools/scowr.py record

# Format this workspace's crates (never `--all`, which would rewrite Scowr's files)
fmt:
    cargo fmt

# Install the git hooks for this clone and all its worktrees
hooks:
    lefthook install

# Thermal pressure, load and compile slots in use
heat:
    python3 tools/build-guard.py status
