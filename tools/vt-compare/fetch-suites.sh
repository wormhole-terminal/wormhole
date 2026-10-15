#!/usr/bin/env bash
# Fetch the outside conformance suites scripts/suite-compare.sh runs, at the commits in suites.pin,
# into a cache outside the repository (their code is never committed here):
#
#   tools/vt-compare/fetch-suites.sh            # clone both, make the Python venv (a minute, mostly pip)
#   tools/vt-compare/fetch-suites.sh --force    # fetch again and rebuild the venv
#   tools/vt-compare/fetch-suites.sh --path     # print the folder, fetch nothing
#   tools/vt-compare/fetch-suites.sh --env      # fetch what is missing, then print ESCTEST_DIR=, UCS_DETECT_DIR=, SUITES_PYTHON=
#
# Everything lands in $WORMHOLE_VT_COMPARE_CACHE/suites (default ~/Library/Caches/wormhole/vt-compare
# on macOS, ${XDG_CACHE_HOME:-~/.cache}/wormhole/vt-compare elsewhere), the same cache as
# fetch-ghostty.sh:
#   esctest2-<rev12>/    Thomas Dickey's esctest2 (GPL-2.0)
#   ucs-detect-<rev12>/  Jeff Quast's ucs-detect (MIT)
#   venv-<hash>/         Python with ucs-detect's packages (esctest2 needs only the standard library)
#
# Downloaded code is data until a run starts it: nothing here executes it, and the venv holds only the
# packages named in suites.pin.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
pin() { sed -n "s/^$1=//p" "$here/suites.pin" | head -n1 | sed 's/^"\(.*\)"$/\1/'; }
log() { printf '\033[1;34m==>\033[0m %s\n' "$*" >&2; }
die() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

if [ -n "${WORMHOLE_VT_COMPARE_CACHE:-}" ]; then
  cache="$WORMHOLE_VT_COMPARE_CACHE"
elif [ "$(uname -s)" = Darwin ]; then
  cache="$HOME/Library/Caches/wormhole/vt-compare"
else
  cache="${XDG_CACHE_HOME:-$HOME/.cache}/wormhole/vt-compare"
fi
out="$cache/suites"

force=0 env=0
for arg in "$@"; do
  case "$arg" in
    --force) force=1 ;;
    --path) echo "$out"; exit 0 ;;
    --env) env=1 ;;
    -h|--help) sed -n '2,16p' "$0"; exit 0 ;;
    *) die "unknown argument $arg (see --help)" ;;
  esac
done
command -v git >/dev/null || die "git is needed"

# fetch_rev <name> <repo> <rev>: a shallow checkout of exactly that commit (GitHub serves any hash).
fetch_rev() {
  local name="$1" repo="$2" rev="$3" dir="$out/$1-${3:0:12}"
  [ -n "$repo" ] && [ -n "$rev" ] || die "$here/suites.pin has no ${name//-/_}_repo or _rev"
  if [ "$force" = 0 ] && [ "$(git -C "$dir" rev-parse -q --verify HEAD 2>/dev/null || true)" = "$rev" ]; then
    log "$name ${rev:0:12} is there: $dir"
    return 0
  fi
  log "fetching $name ${rev:0:12}"
  mkdir -p "$dir"
  [ -d "$dir/.git" ] || { git init -q "$dir"; git -C "$dir" remote add origin "$repo"; }
  git -C "$dir" fetch -q --depth 1 origin "$rev"
  git -C "$dir" checkout -q --detach --force "$rev"
  [ "$(git -C "$dir" rev-parse HEAD)" = "$rev" ] || die "the checkout of $name is not at $rev"
}

mkdir -p "$out"
fetch_rev esctest2 "$(pin esctest2_repo)" "$(pin esctest2_rev)"
fetch_rev ucs-detect "$(pin ucs_detect_repo)" "$(pin ucs_detect_rev)"

# The Python: the newest of 3.14 to 3.10 that is installed (the system's 3.9 is too old for blessed 1.50).
py=""
for v in 3.14 3.13 3.12 3.11 3.10; do
  if command -v "python$v" >/dev/null; then py="$(command -v "python$v")"; break; fi
done
[ -n "$py" ] || die "no python3.10 or newer (brew install python)"
packages="$(pin py_packages)"
hash="$(printf '%s %s' "$packages" "$("$py" --version)" | shasum -a 256 | cut -c1-10)"
venv="$out/venv-$hash"
if [ "$force" = 1 ] || [ ! -x "$venv/bin/python" ] || [ ! -f "$venv/.ready" ]; then
  log "making the venv $venv ($("$py" --version))"
  rm -rf "$venv"
  "$py" -m venv "$venv"
  read -r -a pkgs <<<"$packages"
  "$venv/bin/pip" install -q --disable-pip-version-check "${pkgs[@]}"
  touch "$venv/.ready"
else
  log "venv is there: $venv"
fi
log "done: $out"
if [ "$env" = 1 ]; then
  echo "ESCTEST_DIR=$out/esctest2-$(pin esctest2_rev | cut -c1-12)"
  echo "UCS_DETECT_DIR=$out/ucs-detect-$(pin ucs_detect_rev | cut -c1-12)"
  echo "SUITES_PYTHON=$venv/bin/python"
fi
