#!/usr/bin/env python3
"""Run only the checks a change touches (`just affected`), against main: the commits since this
branch left main, staged and unstaged edits, and untracked files.

The crates are chosen from `cargo metadata`, not from a list, so a new member is picked up as
soon as the workspace names it. A changed file belongs to the member whose folder holds it; the
crates checked are those members and every member that depends on them, again and again. A
change to the workspace's own files (`Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`,
`deny.toml`, `.cargo/`) reaches every member. On them it runs `cargo clippy -p … --all-targets
-- -D warnings` and `cargo test -p …`, through the build guard as `just check` does, with
`cargo fmt --check -p …` on the crates whose files changed, Taplo on the changed TOML files, and
`cargo deny` when a manifest, the lock or `deny.toml` changed.

A crate of a sibling repository that depends on a chosen crate by path (Veelindy's
`veelindy_manifest` on Veel's `veel_syntax`) is checked too, Clippy and tests in that sibling,
when the sibling is a worktree made with this one (`just wt-new … --repos veel,veelindy` in
Annilay): a link to a main checkout, or a main checkout itself, is another agent's and only read
(board row B-14).

Files that make the gate itself (`justfile`, `lefthook.yml`, `tools/check-record.py`,
`tools/build-guard.py`, `tools/check-watch.py` and its budget, Veel's `tools/scowr.py` and its
record, this file), the fuzz targets and any other file a test may read
(`corpus/`, `templates/`) need `just check`: the command says so and records nothing.

A pass is recorded as an `affected` pass (check-record.py pass-affected), which lets a code commit
through only on an `agent/*` branch in a worktree under work/wt/; on `main` a code commit still
needs `just check`. This file is the same in Veel and Veelindy; Annilay's tests are chosen by
`annilay/tools/affected.py`.

Usage: tools/affected.py [--list]   (`just affected`; --list prints the commands and runs nothing)
"""

import argparse
import json
import os
import pathlib
import subprocess
import sys

GATE = {"justfile", "lefthook.yml", "tools/check-record.py", "tools/build-guard.py", "tools/affected.py",
        "tools/check-watch.py", "tools/check-budget.txt", "tools/scowr.py", "tools/scowr-commit"}
SIBLINGS = ("Veel", "Veelindy")
UNCOVERED = ("fuzz/", "corpus/", "templates/")
WHOLE = {"Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "deny.toml"}
IMAGES = (".png", ".jpg", ".jpeg", ".gif", ".svg", ".webp")


def git(root, *args):
    return subprocess.run(["git", *args], cwd=root, capture_output=True, text=True, check=True).stdout


def changed(root):
    """Every path that differs from where this branch left main, committed or not, and untracked."""
    base = git(root, "merge-base", "main", "HEAD").strip()
    paths = git(root, "diff", "--name-only", "-z", base).split("\0")
    paths += git(root, "ls-files", "-o", "--exclude-standard", "-z").split("\0")
    return sorted({p for p in paths if p})


def is_code(path):
    """As check-record.py counts it: not a document and, here, not an image."""
    p = path.lower()
    return not (p.endswith(".md") or p.startswith("docs/") or p.endswith(IMAGES))


def metadata(root):
    """The workspace members of `root`, from `cargo metadata`."""
    meta = json.loads(subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps", "--offline", "--locked"],
        cwd=root, capture_output=True, text=True, check=True).stdout)
    ids = set(meta["workspace_members"])
    return [p for p in meta["packages"] if p["id"] in ids]


def members(root):
    """Each workspace member's name, folder (relative to root) and the members it depends on."""
    pkgs = metadata(root)
    names = {p["name"] for p in pkgs}
    out = {}
    for p in pkgs:
        folder = pathlib.Path(p["manifest_path"]).parent.resolve().relative_to(root.resolve()).as_posix()
        out[p["name"]] = (folder, {d["name"] for d in p["dependencies"] if d.get("path") and d["name"] in names})
    return out


def worktree_pair(root, sib):
    """Whether `sib` is a sibling made with this checkout: a folder, not a link, that is a Git
    worktree rather than a main checkout (as Annilay's check.sh judges Veel and Veelindy)."""
    if sib.resolve() == root.resolve() or sib.is_symlink() or not (sib / "Cargo.toml").is_file():
        return False
    r = subprocess.run(["git", "rev-parse", "--absolute-git-dir", "--path-format=absolute", "--git-common-dir"],
                       cwd=sib, capture_output=True, text=True, check=False)
    dirs = r.stdout.split()
    return r.returncode == 0 and len(dirs) == 2 and dirs[0] != dirs[1]


def dependents_beside(root, crates, reached):
    """For each sibling repository beside `root`: its folder, the members that depend by path on a
    crate in `reached` (and those that depend on them), and whether it is a worktree pair."""
    folders = {(root / crates[n][0]).resolve() for n in reached}
    found = []
    for name in SIBLINGS:
        sib = root.parent / name
        if sib.resolve() == root.resolve() or not (sib / "Cargo.toml").is_file():
            continue
        pkgs = metadata(sib)
        names = {p["name"] for p in pkgs}
        deps = {p["name"]: {d["name"] for d in p["dependencies"] if d.get("path") and d["name"] in names} for p in pkgs}
        hit = {p["name"] for p in pkgs if any(d.get("path") and pathlib.Path(d["path"]).resolve() in folders
                                             for d in p["dependencies"])}
        while True:
            more = {n for n, ds in deps.items() if n not in hit and ds & hit}
            if not more:
                break
            hit |= more
        if hit:
            found.append((sib, sorted(hit), worktree_pair(root, sib)))
    return found


def choose(root, paths, crates, beside=None):
    """The commands for these changed paths, and the code paths `just affected` cannot cover. Each
    command is a list, run in `root`, or a (folder, list) pair, run in a sibling. `beside` finds
    the siblings' crates that depend on the chosen ones (dependents_beside); none when None."""
    code = [p for p in paths if is_code(p)]
    uncovered = [p for p in code if p in GATE or p.startswith(UNCOVERED)]
    whole = any(p in WHOLE or p.startswith(".cargo/") for p in code)
    touched = {n for n, (folder, _) in crates.items() if any(p.startswith(folder + "/") for p in code)}
    reached = set(crates) if whole else set(touched)
    while True:
        more = {n for n, (_, deps) in crates.items() if n not in reached and deps & reached}
        if not more:
            break
        reached |= more
    cmds = []
    if reached:
        some = [a for n in sorted(reached) for a in ("-p", n)]
        cmds.append([sys.executable, "tools/build-guard.py", "wait"])
        if touched:
            cmds.append(["cargo", "fmt", "--check", *[a for n in sorted(touched) for a in ("-p", n)]])
        cmds.append(["cargo", "clippy", *some, "--all-targets", "--locked", "--", "-D", "warnings"])
        cmds.append(["cargo", "test", *some, "--locked"])
        for sib, names, pair in (beside or (lambda *_: []))(root, crates, reached):
            if not pair:
                print(f"affected: not checked here: {', '.join(names)} in {sib}, which depend on this change, "
                      "because it is not a worktree made with this one; its own `just check` covers it", file=sys.stderr)
                continue
            theirs = [a for n in names for a in ("-p", n)]
            cmds.append((sib, ["cargo", "clippy", *theirs, "--all-targets", "--locked", "--", "-D", "warnings"]))
            cmds.append((sib, ["cargo", "test", *theirs, "--locked"]))
    toml = [p for p in code if p.endswith(".toml") and (root / p).is_file()]
    if toml:
        cmds.append(["taplo", "fmt", "--check", *toml])
    if any(p in ("Cargo.lock", "deny.toml") or p.endswith("Cargo.toml") for p in code):
        cmds.append(["cargo", "deny", "--offline", "check", "licenses", "bans", "sources"])
    return cmds, uncovered


def show(cmd):
    if isinstance(cmd, tuple):
        return f"(cd {cmd[0]} && {show(cmd[1])})"
    return " ".join(c if c != sys.executable else "python3" for c in cmd)


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--list", action="store_true", help="print the commands; run nothing")
    args = ap.parse_args()
    root = pathlib.Path(git(pathlib.Path.cwd(), "rev-parse", "--show-toplevel").strip())
    paths = changed(root)
    cmds, uncovered = choose(root, paths, members(root), dependents_beside)
    if args.list:
        for cmd in cmds:
            print(show(cmd))
        for p in uncovered:
            print(f"not covered, needs `just check`: {p}")
        return 0
    if uncovered:
        sys.exit("affected: these change the gate or files a test may read, which only `just check` covers: " + ", ".join(uncovered))
    key = f"affected-{os.getpid()}"
    subprocess.run([sys.executable, "tools/check-record.py", "start", key], cwd=root, check=True)
    env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1", RUST_LOG=os.environ.get("RUST_LOG", "warn"))
    for cmd in cmds:
        print(f"\n== {show(cmd)}", flush=True)
        cwd, cmd_ = cmd if isinstance(cmd, tuple) else (root, cmd)
        if subprocess.run(cmd_, cwd=cwd, env=env, check=False).returncode:
            sys.exit(f"affected: failed: {show(cmd)}")
    subprocess.run([sys.executable, "tools/check-record.py", "pass-affected", key], cwd=root, check=True)
    print(f"\naffected: passed ({len(cmds)} commands for {len(paths)} changed paths)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
