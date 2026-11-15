#!/bin/bash
# Tests tools/annilay-crosscheck.sh in throwaway clones of this checkout: a public signature
# Annilay's annilay_core calls, changed, fails the check with the compiler's error; a dependency
# added to a crate Annilay uses fails it with the change Annilay's Cargo.lock needs, which,
# applied, lets Annilay's crates build `--locked`, while Annilay's own lock is untouched; with no
# Annilay crates beside the clone, the check is skipped and passes. Slow (it builds Annilay's
# crates twice), so it is run by hand: tools/annilay-crosscheck-test.sh <Veel|Veelindy>. The
# same file is in Veel and Veelindy, adapted from Null's sau-crosscheck-test.sh.
#
# Until Annilay's task 1a-06 creates annilay/crates, only the skip is tested. 1a-06 chooses the
# signature below (a public function annilay_core calls) and the crate that gains a dependency.
set -euo pipefail
# Run from a hook or a rebase, git's GIT_* variables would send the clones' commands to the real
# repository: clear them all first.
while IFS= read -r v; do unset "$v"; done < <(compgen -e | grep '^GIT_' || true)
name=${1:?usage: tools/annilay-crosscheck-test.sh <Veel|Veelindy>}
root=$(git rev-parse --show-toplevel)
case $name in
# Chosen by task 1a-06: file=<a source file here>, fn=<a public function annilay_core calls>,
# crate=<a crate annilay_core uses>.
Veel) file="" fn="" crate="" ;;
Veelindy) file="" fn="" crate="" ;;
*) echo "usage: tools/annilay-crosscheck-test.sh <Veel|Veelindy>" >&2 && exit 2 ;;
esac
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT

if git -C "$root/../Annilay" cat-file -e HEAD:annilay/crates/Cargo.toml 2>/dev/null; then
  if [ -z "$fn" ]; then
    echo "annilay-crosscheck-test: annilay/crates exists, but no signature is chosen here (task 1a-06)" >&2
    exit 1
  fi
  # With Annilay beside it: the changed signature fails, with rustc's error.
  mkdir "$scratch/with"
  git clone -q --no-hardlinks "$root" "$scratch/with/$name"
  for other in Annilay Veel Veelindy Scowr; do
    if [ "$other" != "$name" ] && [ -e "$root/../$other" ]; then
      ln -s "$(cd "$root/../$other" && pwd -P)" "$scratch/with/$other"
    fi
  done
  grep -q "pub fn $fn(" "$scratch/with/$name/$file"
  # The unchanged clone passes first, so the failure below is the change's.
  (cd "$scratch/with/$name" && tools/annilay-crosscheck.sh "$name" >/dev/null)
  sed -i '' "s/pub fn $fn(/pub fn $fn(_extra: (), /" "$scratch/with/$name/$file"
  if out=$(cd "$scratch/with/$name" && tools/annilay-crosscheck.sh "$name" 2>&1); then
    echo "annilay-crosscheck-test: a changed signature passed:" >&2
    echo "$out" >&2
    exit 1
  fi
  if ! { grep -q "^error\[E0061\]" <<<"$out" && grep -q -e "--> annilay_core/src/" <<<"$out" && grep -q "$fn(" <<<"$out"; }; then
    echo "annilay-crosscheck-test: failed without the compiler's error in Annilay's annilay_core:" >&2
    echo "$out" >&2
    exit 1
  fi
  echo "annilay-crosscheck-test: a changed signature fails: $(grep -m1 '^error\[E' <<<"$out")"

  # A new dependency: the check fails with the lock diff and changes no lock of Annilay's; the
  # diff, applied to Annilay's committed lock, lets Annilay's crates build --locked.
  git -C "$scratch/with/$name" checkout -q -- "$file"
  app_lock=$(cd "$root/../Annilay/annilay/crates" && pwd -P)/Cargo.lock
  before=$(shasum "$app_lock")
  manifest=$scratch/with/$name/$crate/Cargo.toml
  grep -q '^serde_json' "$manifest" && {
    echo "annilay-crosscheck-test: $crate already depends on serde_json; pick another" >&2
    exit 1
  }
  sed -i '' 's/^\[dependencies\]$/[dependencies]\nserde_json = "1"/' "$manifest"
  grep -q '^serde_json = "1"$' "$manifest"
  if out=$(cd "$scratch/with/$name" && tools/annilay-crosscheck.sh "$name" 2>&1); then
    echo "annilay-crosscheck-test: a new dependency passed:" >&2
    echo "$out" >&2
    exit 1
  fi
  diff=$scratch/with/$name/target/annilay-crosscheck/Cargo.lock.diff
  if ! grep -q "needs this change" <<<"$out" || ! grep -q '^+ "serde_json",$' "$diff"; then
    echo "annilay-crosscheck-test: failed without the lock diff:" >&2
    echo "$out" >&2
    exit 1
  fi
  [ "$(shasum "$app_lock")" = "$before" ] || {
    echo "annilay-crosscheck-test: the check changed Annilay's own Cargo.lock" >&2
    exit 1
  }
  tree=$scratch/with/$name/target/annilay-crosscheck/tree
  (cd "$tree/Annilay" && git apply "$diff")
  RUSTC_WRAPPER="$root/tools/build-guard.py" CARGO_TARGET_DIR="$scratch/with/$name/target/annilay-crosscheck/target" \
    cargo check --locked --workspace --all-targets --quiet --manifest-path "$tree/Annilay/annilay/crates/Cargo.toml"
  echo "annilay-crosscheck-test: a new dependency fails with the lock diff ($(grep -c '^[+-] ' "$diff") line(s)), which, applied, builds --locked"
else
  echo "annilay-crosscheck-test: no annilay/crates in ../Annilay yet (task 1a-06): the signature and lock cases wait for it"
fi

# With no Annilay beside it: skipped, and passing.
mkdir "$scratch/without"
git clone -q --no-hardlinks "$root" "$scratch/without/$name"
out=$(cd "$scratch/without/$name" && tools/annilay-crosscheck.sh "$name")
grep -q "skipped" <<<"$out"
echo "annilay-crosscheck-test: with no Annilay crates beside it, skipped: $out"
echo "annilay-crosscheck-test: passed"
