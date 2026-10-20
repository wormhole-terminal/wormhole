#!/usr/bin/env bash
# Fetch the outside conformance suites scripts/suite-compare.sh runs, at the commits in suites.pin,
# into a cache outside the repository (their code is never committed here):
#
#   tools/vt-compare/fetch-suites.sh            # clone both, make the Python venv (a minute, mostly pip), build vttest
#   tools/vt-compare/fetch-suites.sh --force    # fetch again, rebuild the venv and vttest
#   tools/vt-compare/fetch-suites.sh --path     # print the folder, fetch nothing
#   tools/vt-compare/fetch-suites.sh --env      # fetch what is missing, then print ESCTEST_DIR=, UCS_DETECT_DIR=, SUITES_PYTHON=, VTTEST_BIN=
#
# Everything lands in $WORMHOLE_VT_COMPARE_CACHE/suites (default ~/Library/Caches/wormhole/vt-compare
# on macOS, ${XDG_CACHE_HOME:-~/.cache}/wormhole/vt-compare elsewhere), the same cache as
# fetch-ghostty.sh:
#   esctest2-<rev12>/    Thomas Dickey's esctest2 (GPL-2.0)
#   ucs-detect-<rev12>/  Jeff Quast's ucs-detect (MIT)
#   venv-<hash>/         Python with ucs-detect's packages (esctest2 needs only the standard library)
#   vttest-<version>/    Thomas Dickey's vttest (MIT-X11): a tarball checked against the SHA-256 in suites.pin,
#                        then `./configure && make` (needs a C compiler and make; without them or the network
#                        this one is skipped with a warning and --env prints no VTTEST_BIN)
#
# Downloaded code is data until a run starts it: nothing here executes it (but vttest's own build, which
# is the one step that runs its configure script), and the venv holds only the packages named in suites.pin.
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
    -h|--help) sed -n '2,20p' "$0"; exit 0 ;;
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

# vttest_build: the pinned source tarball, checked and built in place. A failure (no network, no
# compiler, a checksum that does not match) is reported and leaves the other suites alone.
vttest_build() {
  local version url sha dir tgz got fast stamp
  version="$(pin vttest_version)" url="$(pin vttest_url)" sha="$(pin vttest_sha256)"
  dir="$out/vttest-$version"
  # Built with suites/vttest-fast.h: its pauses between a request and the answer are shortened (see the header).
  fast="$here/suites/vttest-fast.h"
  # The stamp names what the build was made of: the tarball, the recipe (bump r2 when it changes) and the header.
  stamp="$sha-r2-$(shasum -a 256 "$fast" | cut -c1-12)"
  [ -n "$version" ] && [ -n "$url" ] && [ -n "$sha" ] || { log "suites.pin has no vttest_* entries"; return 1; }
  if [ "$force" = 0 ] && [ -x "$dir/vttest" ] && [ -f "$dir/.built-$stamp" ]; then
    log "vttest $version is there: $dir"
    return 0
  fi
  command -v curl >/dev/null && command -v make >/dev/null && command -v cc >/dev/null \
    || { log "vttest needs curl, make and a C compiler"; return 1; }
  log "fetching vttest $version"
  tgz="$(mktemp "${TMPDIR:-/tmp}/vttest.XXXXXX")"
  curl -fsSL --retry 2 --max-time 120 -o "$tgz" "$url" || { rm -f "$tgz"; log "could not download $url"; return 1; }
  got="$(shasum -a 256 "$tgz" | cut -d' ' -f1)"
  if [ "$got" != "$sha" ]; then
    rm -f "$tgz"; log "vttest $version: SHA-256 is $got, suites.pin says $sha; not building it"; return 1
  fi
  rm -rf "$dir"; mkdir -p "$dir"
  tar -xzf "$tgz" -C "$dir" --strip-components 1 || { rm -f "$tgz"; log "could not unpack the vttest tarball"; return 1; }
  rm -f "$tgz"
  log "building vttest $version"
  # (The header goes into make's compiler, not configure's: configure would no longer find usleep.)
  ( cd "$dir" && ./configure >configure.log 2>&1 && make CC="cc -include $fast" >make.log 2>&1 ) \
    || { log "vttest did not build (see $dir/configure.log, make.log)"; return 1; }
  [ -x "$dir/vttest" ] || { log "vttest did not build"; return 1; }
  touch "$dir/.built-$stamp"
}
vttest_ok=1
vttest_build || vttest_ok=0

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
  [ "$vttest_ok" = 1 ] && echo "VTTEST_BIN=$out/vttest-$(pin vttest_version)/vttest"
fi
