#!/usr/bin/env python3
"""Which Scowr commit Veel last passed `just check` with (Annilay's board row B-12).

Veel builds `scowr_syntax` and `scowr_errors` from whatever checkout `../Scowr` names, by path
(Cargo.toml). `tools/scowr-commit` holds, on one line, the Scowr commit Veel last passed its check
with; it is code to the check record, so a new one is committed only on a tree `just check`
passed on.

  scowr.py check      start of `just check`: warns, naming both commits, when ../Scowr is not at
                      the recorded commit or has uncommitted changes; remembers the commit it saw
  scowr.py passed     end of a passing `just check`: notes, in this checkout's Git folder, that the
                      check passed with that commit, if ../Scowr did not move meanwhile
  scowr.py record     `just scowr-record`: writes that commit to tools/scowr-commit, only if the
                      last passing check here saw ../Scowr exactly as it is now
  scowr.py selftest   all three against throwaway repositories

A warning, not a failure: Scowr moves with its own project's work, and a check that failed for
that alone would stop every area. Once a check passes with the new Scowr, `just scowr-record` and
`just check` again (it then passes without the warning), and commit the record with the tree.
"""

import os
import pathlib
import subprocess
import sys
import tempfile

RECORD = "tools/scowr-commit"


def git(cwd, *args):
    r = subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True, check=False)
    return r.stdout.strip() if r.returncode == 0 else None


def root():
    return pathlib.Path(git(pathlib.Path.cwd(), "rev-parse", "--show-toplevel") or ".")


def linked(veel):
    """(commit, dirty, subject) of the Scowr checkout ../Scowr names, or None when there is none."""
    scowr = veel.parent / "Scowr"
    commit = git(scowr, "rev-parse", "HEAD") if scowr.is_dir() else None
    if not commit:
        return None
    dirty = bool(git(scowr, "status", "--porcelain", "--untracked-files=no"))
    return commit, dirty, git(scowr, "log", "-1", "--format=%s") or ""


def recorded(veel):
    try:
        return (veel / RECORD).read_text().strip()
    except OSError:
        return ""


def state(veel, name):
    return pathlib.Path(git(veel, "rev-parse", "--path-format=absolute", "--git-path", name))


def say(line):
    print(f"check: {line}", file=sys.stderr, flush=True)


def check(veel):
    now, old = linked(veel), recorded(veel)
    seen = state(veel, "scowr-checking")
    if now is None:
        say(f"no Scowr checkout at {veel.parent / 'Scowr'}; Veel builds scowr_syntax and scowr_errors from there")
        seen.unlink(missing_ok=True)
        return 1
    commit, dirty, subject = now
    seen.write_text(f"{commit}{' dirty' if dirty else ''}\n")
    if dirty:
        say(f"WARNING: ../Scowr ({commit[:12]}) has uncommitted changes; Veel last passed its check with Scowr "
            f"{old[:12] or '(none recorded)'} ({RECORD}), and this check proves nothing about a commit")
    elif commit != old:
        say(f"WARNING: ../Scowr is at {commit[:12]} ({subject}), but Veel last passed its check with Scowr "
            f"{old[:12] or '(none recorded)'} ({RECORD}); if this check passes, `just scowr-record` records "
            f"{commit[:12]}, to commit with the tree after `just check` passes on it")
    return 0


def passed(veel):
    seen = state(veel, "scowr-checking")
    try:
        at_start = seen.read_text().strip()
    except OSError:
        at_start = ""
    now = linked(veel)
    if now is None or at_start != now[0]:
        say(f"../Scowr changed during the check, or had uncommitted changes ({at_start[:12] or 'none'}); "
            "no Scowr commit is noted as passed")
        state(veel, "scowr-passed").unlink(missing_ok=True)
        return 0
    state(veel, "scowr-passed").write_text(now[0] + "\n")
    if now[0] != recorded(veel):
        say(f"passed with Scowr {now[0][:12]}, not the recorded {recorded(veel)[:12] or '(none)'}: "
            "`just scowr-record`, then `just check`, and commit tools/scowr-commit")
    return 0


def record(veel):
    try:
        ok = state(veel, "scowr-passed").read_text().strip()
    except OSError:
        ok = ""
    now = linked(veel)
    if now is None or now[1] or ok != now[0]:
        print(f"scowr: no `just check` has passed here with ../Scowr as it is now "
              f"({now[0][:12] + (' with uncommitted changes' if now[1] else '') if now else 'missing'}); run it first",
              file=sys.stderr)
        return 1
    (veel / RECORD).write_text(now[0] + "\n")
    print(f"scowr: {RECORD} now holds {now[0]}; run `just check` and commit it with the tree")
    return 0


def selftest():
    failures = []

    def run(veel, *args):
        return subprocess.run([sys.executable, __file__, *args], cwd=veel, capture_output=True, text=True, check=False)

    env_keep = {k: os.environ.pop(k) for k in [k for k in os.environ if k.startswith("GIT_")]}
    try:
        with tempfile.TemporaryDirectory() as tmp:
            tmp = pathlib.Path(tmp)
            ident = ["-c", "user.name=t", "-c", "user.email=t@example.invalid", "-c", "commit.gpgsign=false",
                     "-c", "core.hooksPath=/dev/null"]
            commits = []
            for name in ("Veel", "Scowr"):
                (tmp / name / "tools").mkdir(parents=True)
                git(tmp / name, "init", "-q", "-b", "main")
                (tmp / name / "f").write_text("1\n")
                git(tmp / name, "add", "-A")
                git(tmp / name, *ident, "commit", "-q", "-m", "one")
            veel, scowr = tmp / "Veel", tmp / "Scowr"
            commits.append(git(scowr, "rev-parse", "HEAD"))
            (veel / RECORD).write_text(commits[0] + "\n")
            r = run(veel, "check")
            if r.returncode or r.stderr:
                failures.append(f"at the recorded commit: exit {r.returncode}, {r.stderr!r}")
            (scowr / "f").write_text("2\n")
            git(scowr, *ident, "commit", "-q", "-am", "two")
            commits.append(git(scowr, "rev-parse", "HEAD"))
            r = run(veel, "check")
            if r.returncode or commits[0][:12] not in r.stderr or commits[1][:12] not in r.stderr:
                failures.append(f"moved: no warning naming both commits (exit {r.returncode}, {r.stderr!r})")
            r = run(veel, "record")
            if r.returncode == 0:
                failures.append("record before a passing check: accepted")
            run(veel, "passed")
            r = run(veel, "record")
            if r.returncode or recorded(veel) != commits[1]:
                failures.append(f"record after a passing check: {r.returncode}, {recorded(veel)!r}")
            r = run(veel, "check")
            if r.stderr:
                failures.append(f"once recorded, still warns: {r.stderr!r}")
            (scowr / "f").write_text("3\n")
            r = run(veel, "check")
            if "uncommitted" not in r.stderr:
                failures.append(f"a dirty Scowr: no warning ({r.stderr!r})")
            run(veel, "passed")
            if run(veel, "record").returncode == 0:
                failures.append("record with a dirty Scowr: accepted")
    finally:
        os.environ.update(env_keep)
    for f in failures:
        print(f"scowr selftest: FAILED: {f}", file=sys.stderr)
    if failures:
        return 1
    print("scowr selftest: passed")
    return 0


def main():
    a = sys.argv[1:]
    commands = {"check": check, "passed": passed, "record": record}
    if a == ["selftest"]:
        return selftest()
    if len(a) == 1 and a[0] in commands:
        return commands[a[0]](root())
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())
