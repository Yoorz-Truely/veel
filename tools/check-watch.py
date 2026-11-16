#!/usr/bin/env python3
"""What `just check` is waiting on, and its time against its budget (Annilay's board row B-24).

  check-watch.py watch <pid>       run in the background by `just check`: says when a compile of
                                   that check (a process below <pid>) waits for a slot of the build
                                   guard, and when it gets one; on SIGTERM, or once <pid> is gone,
                                   prints the time waited in all and ends
  check-watch.py time <seconds>    `check: <n> s, budget <b> s`, from tools/check-budget.txt (one
                                   line of whole seconds), with `OVER` past it; exits 0 either way
  check-watch.py selftest          both, against a fake compile and a fake budget

Beside a build the check used to wait on the guard with no sign of it (U0.1: 379 s and 480 s for
20.5 s and 9.1 s of CPU). A compile waiting for a slot is the build guard in wrapper mode
(`tools/build-guard.py <rustc> … --crate-name …`), which becomes the compiler once it has a slot;
the guard itself is Sau's file, byte for byte, so this watches it from outside. The guard's
thermal wait (`build-guard.py wait`) prints its own line when it starts; the total of both comes
at the end. The budget reports; it never fails a
check (Annilay's docs/agents/testing.md, "The check's budget"). The same file is in Veel and
Veelindy, each with its own budget file.
"""

import os
import pathlib
import signal
import subprocess
import sys
import tempfile
import time

HERE = pathlib.Path(__file__).resolve().parent
BUDGET = HERE / "check-budget.txt"
# A compile that waits this long is reported (the guard logs waits past 5 s); polled this often.
AFTER = float(os.environ.get("CHECK_WATCH_AFTER", "2"))
POLL = float(os.environ.get("CHECK_WATCH_POLL", "1"))


def say(line):
    print(f"check: {line}", file=sys.stderr, flush=True)


def waiting_below(pid):
    """PIDs of the guard below `pid`: in wrapper mode (compiles still waiting for a slot), and in
    `wait` (the check waiting for heavy thermal pressure to clear)."""
    out = subprocess.run(["ps", "-ax", "-o", "pid=,ppid=,command="], capture_output=True, text=True, check=False).stdout
    children, commands = {}, {}
    for line in out.splitlines():
        parts = line.split(None, 2)
        if len(parts) == 3 and parts[0].isdigit() and parts[1].isdigit():
            children.setdefault(int(parts[1]), []).append(int(parts[0]))
            commands[int(parts[0])] = parts[2]
    found, thermal, stack = [], [], list(children.get(pid, []))
    while stack:
        p = stack.pop()
        stack.extend(children.get(p, []))
        command = commands.get(p, "")
        if "build-guard.py" in command and "--crate-name" in command:
            found.append(p)
        elif "build-guard.py wait" in command:
            thermal.append(p)
    return found, thermal


def guard_status():
    """The guard's slots, thermal state and load, in one line (`just heat` has them all)."""
    r = subprocess.run([sys.executable, str(HERE / "build-guard.py"), "status"], capture_output=True, text=True, check=False)
    keep = ("thermal pressure", "load (1 min)", "compile slots")
    return "; ".join(" ".join(ln.split()) for ln in r.stdout.splitlines() if ln.startswith(keep))


def alive(pid):
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


def watch(pid):
    stop = []
    signal.signal(signal.SIGTERM, lambda *_: stop.append(1))
    signal.signal(signal.SIGINT, lambda *_: stop.append(1))
    since, total = None, 0.0  # when the current wait began; the waits before it
    reported, hot, last = False, 0.0, time.monotonic()
    while not stop and alive(pid):
        now = time.monotonic()
        waiting, thermal = waiting_below(pid)
        if thermal:  # the guard says when it starts; the total comes at the end
            hot += now - last
        last = now
        if waiting and since is None:
            since = now
        elif not waiting and since is not None:
            total += now - since
            if reported:
                say(f"no compile waiting for a slot any more, after {now - since:.0f} s")
            since, reported = None, False
        if since is not None and not reported and now - since >= AFTER:
            say(f"waiting for a compile slot of the build guard ({len(waiting)} compiles waiting; {guard_status()}; "
                "`just heat` shows the slots)")
            reported = True
        time.sleep(POLL)
    if since is not None:
        total += time.monotonic() - since
    if total >= AFTER:
        say(f"{total:.0f} s of it waiting for compile slots")
    if hot >= AFTER:
        say(f"{hot:.0f} s of it waiting for heavy thermal pressure to clear")
    return 0


def budget_line(took, budget_file=BUDGET):
    try:
        text = budget_file.read_text().strip()
    except OSError:
        text = ""
    if not text.isdigit():
        return f"check: {took} s; {budget_file.name} holds no budget in whole seconds"
    budget = int(text)
    if took > budget:
        return f"check: {took} s, budget {budget} s, OVER (report it with the load: uptime, just heat)"
    return f"check: {took} s, budget {budget} s"


def selftest():
    failures = []
    with tempfile.TemporaryDirectory() as tmp:
        b = pathlib.Path(tmp) / "check-budget.txt"
        b.write_text("7\n")
        for took, want in ((5, "check: 5 s, budget 7 s"), (7, "check: 7 s, budget 7 s"),
                           (9, "check: 9 s, budget 7 s, OVER (report it with the load: uptime, just heat)")):
            if budget_line(took, b) != want:
                failures.append(f"time {took}: {budget_line(took, b)!r}")
        b.write_text("seven\n")
        if "holds no budget" not in budget_line(3, b):
            failures.append(f"a budget that is not a number: {budget_line(3, b)!r}")
        # A fake compile waiting for a slot, below a fake check: the watcher names it, then says
        # when it ended, then prints the time waited once the check is gone.
        fake = f'"{sys.executable}" -c "import time; time.sleep(0.8)" build-guard.py --crate-name x'
        hot = f'"{sys.executable}" -c "import time; time.sleep(0.8)" build-guard.py wait'
        check = subprocess.Popen(["/bin/sh", "-c", f"{hot} & {fake}; wait; sleep 0.2"])
        env = dict(os.environ, CHECK_WATCH_AFTER="0.3", CHECK_WATCH_POLL="0.1")
        w = subprocess.Popen([sys.executable, __file__, "watch", str(check.pid)], env=env, stderr=subprocess.PIPE,
                             text=True)
        check.wait()  # reaped, so the watcher sees it gone
        try:
            _, out = w.communicate(timeout=30)
        except subprocess.TimeoutExpired:
            w.kill()
            _, out = w.communicate()
            failures.append("watch: still running 30 s after its check ended")
    for want in ("waiting for a compile slot of the build guard (1 compiles waiting", "no compile waiting for a slot any more, after",
                 "s of it waiting for compile slots", "s of it waiting for heavy thermal pressure to clear"):
        if want not in out:
            failures.append(f"watch: no {want!r} in {out!r}")
    for f in failures:
        print(f"check-watch selftest: FAILED: {f}", file=sys.stderr)
    if failures:
        return 1
    print("check-watch selftest: passed")
    return 0


def main():
    a = sys.argv[1:]
    if len(a) == 2 and a[0] == "watch" and a[1].isdigit():
        return watch(int(a[1]))
    if len(a) == 2 and a[0] == "time" and a[1].isdigit():
        print(budget_line(int(a[1])))
        return 0
    if a == ["selftest"]:
        return selftest()
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())
