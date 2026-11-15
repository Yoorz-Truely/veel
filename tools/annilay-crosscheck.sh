#!/bin/bash
# Builds Annilay's Rust crates (annilay/crates) against this checkout, so a change to an API
# Annilay uses fails here, in the repository that made it, before it merges, not later in
# Annilay's build (Sau's rule of its task 0-12, adopted by Annilay's blueprint 00).
# `just verify` runs it; it is skipped while ../Annilay has no committed annilay/crates/Cargo.toml
# (until Annilay's task 1a-06), and when there is no ../Annilay beside this checkout.
#
# Usage: tools/annilay-crosscheck.sh <Veel|Veelindy>, the name of this repository.
#
# Annilay's crates take Veel and Veelindy by path, and Cargo reads such paths as written, without
# resolving links. So the check builds them in a folder holding Annilay's committed
# `annilay/crates` (its HEAD, extracted with `git archive`, not its working tree, which may be
# mid-merge or hold someone's edits), with this checkout, under its own name, and the other
# repositories and Scowr beside it as links. Annilay's own Cargo.lock is used as it is
# (`--locked`): if this repository gained a dependency Annilay's lock does not list, that fails
# too, and the check prints the exact change Annilay's lock needs, as a diff to apply there. It
# never changes Annilay's own lock: the lead commits the change. The same file is in Veel and
# Veelindy (`tools/`), adapted from Null's sau-crosscheck.sh.
set -euo pipefail
name=${1:?usage: tools/annilay-crosscheck.sh <Veel|Veelindy>}
root=$(git rev-parse --show-toplevel)
app=$root/../Annilay
if ! git -C "$app" cat-file -e HEAD:annilay/crates/Cargo.toml 2>/dev/null; then
  echo "annilay-crosscheck: no committed annilay/crates in ../Annilay yet (task 1a-06); skipped"
  exit 0
fi
work=$root/target/annilay-crosscheck
tree=$work/tree
head=$(git -C "$app" rev-parse HEAD)
# Annilay's committed crates. If they come to read Kdenlive sources at compile time, 1a-06 adds
# those paths here, as Null's copy adds the Blender sources Sau's crates read.
paths=(annilay/crates)
# Extracted again only when Annilay's HEAD or that list changes, so later runs build only what
# changed. A real folder, not a link: the compiler resolves `include_str!` paths through links.
marker="$head ${paths[*]}"
if [ "$(cat "$tree/Annilay/.annilay-crosscheck-head" 2>/dev/null)" != "$marker" ]; then
  rm -rf "${tree:?}/Annilay"
  mkdir -p "$tree/Annilay"
  git -C "$app" archive HEAD "${paths[@]}" | tar -x -C "$tree/Annilay"
  echo "$marker" >"$tree/Annilay/.annilay-crosscheck-head"
fi
for other in Veel Veelindy Scowr; do
  rm -f "${tree:?}/${other:?}"
  if [ "$other" = "$name" ]; then
    ln -s "$root" "$tree/$other"
  elif [ -e "$root/../$other" ]; then
    ln -s "$(cd "$root/../$other" && pwd -P)" "$tree/$other"
  fi
done
echo "annilay-crosscheck: Annilay's crates at ${head:0:8} (committed) against this $name"

# Annilay's lock against this checkout's manifests. When it no longer fits, Cargo works out the
# smallest change to it in the extracted copy, offline; the copy is put back at once, and the
# change is printed as a diff to apply in Annilay and kept in $work/Cargo.lock.diff.
manifest=$tree/Annilay/annilay/crates/Cargo.toml
lock=$tree/Annilay/annilay/crates/Cargo.lock
if ! cargo metadata --locked --offline --format-version 1 --manifest-path "$manifest" >/dev/null 2>"$work/metadata.err"; then
  cp "$lock" "$work/Cargo.lock.committed"
  if cargo metadata --offline --format-version 1 --manifest-path "$manifest" >/dev/null 2>"$work/metadata.err"; then
    diff -u --label a/annilay/crates/Cargo.lock --label b/annilay/crates/Cargo.lock \
      "$work/Cargo.lock.committed" "$lock" >"$work/Cargo.lock.diff" || true
    cp "$work/Cargo.lock.committed" "$lock"
    if [ -s "$work/Cargo.lock.diff" ]; then
      {
        echo "annilay-crosscheck: this $name changes what Annilay's crates depend on, and Annilay's"
        echo "annilay-crosscheck: annilay/crates/Cargo.lock needs this change (the lead commits it in Annilay):"
        cat "$work/Cargo.lock.diff"
        echo "annilay-crosscheck: in Annilay: git apply $work/Cargo.lock.diff"
        echo "annilay-crosscheck: then cargo check --locked in annilay/crates passes against this $name"
      } >&2
      exit 1
    fi
  else
    cp "$work/Cargo.lock.committed" "$lock"
    {
      echo "annilay-crosscheck: Annilay's annilay/crates/Cargo.lock does not fit this $name, and Cargo"
      echo "annilay-crosscheck: cannot work out the change offline (a crate not yet downloaded?). Cargo says:"
      cat "$work/metadata.err"
      echo "annilay-crosscheck: in Annilay's annilay/crates, \`cargo metadata --format-version 1 >/dev/null\`"
      echo "annilay-crosscheck: online makes the change; the lead commits it."
    } >&2
    exit 1
  fi
fi
if ! RUSTC_WRAPPER="$root/tools/build-guard.py" CARGO_TARGET_DIR="$work/target" \
  cargo check --locked --workspace --all-targets --quiet \
  --manifest-path "$manifest"; then
  echo "annilay-crosscheck: Annilay's crates do not build against this $name. Either a change here" \
    "breaks an API Annilay uses (keep it, or say what Annilay must change), or this $name gained a" \
    "dependency Annilay's annilay/crates/Cargo.lock does not list. Annilay is the lead's to change." >&2
  exit 1
fi
echo "annilay-crosscheck: passed"
