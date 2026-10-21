#!/bin/bash
# The public API of wormhole-vt as a list in crates/wormhole-vt/api.txt, checked against the code.
# A change of the API (a function, a field, a variant, a derived trait) shows up in review as a
# diff of that file, and the check fails until the file says what the code says.
#
#   scripts/vt-api.sh              # compare the code's API with api.txt; fail on a difference
#   scripts/vt-api.sh --update     # rewrite api.txt (review the diff; a removal is a breaking change)
#   scripts/vt-api.sh --diff       # print the difference, change nothing
#
# It lists the API with cargo-public-api, which reads the crate's rustdoc JSON, and that needs a
# nightly toolchain (rustup toolchain install nightly) and the tool (cargo install --locked
# cargo-public-api). Without either the check is skipped with a warning, never failed, so the gate
# works on a machine that has neither; CI installs both and sets API_REQUIRED=1, which makes a skip a
# failure. The listing is of the
# crate as a user builds it (default features), without blanket and auto trait impls.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1
. scripts/lib.sh

mode=check
for arg in "$@"; do
  case "$arg" in
    --update) mode=update ;;
    --diff) mode=diff ;;
    -h|--help) sed -n '2,13p' "$0"; exit 0 ;;
    *) die "unknown argument: $arg (see --help)" ;;
  esac
done

snapshot=crates/wormhole-vt/api.txt

skip() {
  [ "${API_REQUIRED:-}" = 1 ] && die "wormhole-vt's public API cannot be checked: $*"
  warn "wormhole-vt's public API is unchecked: $*"
  exit 0
}

cargo public-api --version >/dev/null 2>&1 || skip "cargo-public-api is not installed (cargo install --locked cargo-public-api)."
cargo +nightly --version >/dev/null 2>&1 || skip "no nightly toolchain (rustup toolchain install nightly), which rustdoc JSON needs."

work="$(mktemp -d "${TMPDIR:-/tmp}/vt-api.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# The tool's own failure (a nightly newer than it understands) is not the code's: skip, with its words.
if ! cargo public-api -p wormhole-vt -ss > "$work/api.txt" 2> "$work/err"; then
  tail -5 "$work/err" >&2
  skip "cargo-public-api could not list the API (see above); update the tool or the nightly."
fi
[ -s "$work/api.txt" ] || skip "cargo-public-api listed nothing."

case "$mode" in
  update)
    cp "$work/api.txt" "$snapshot"
    log "wrote $snapshot ($(wc -l < "$snapshot" | tr -d ' ') lines)"
    ;;
  diff)
    diff -u "$snapshot" "$work/api.txt" || true
    ;;
  check)
    if [ ! -f "$snapshot" ]; then
      die "$snapshot is missing; create it with scripts/vt-api.sh --update"
    fi
    if ! diff -u "$snapshot" "$work/api.txt" > "$work/diff"; then
      cat "$work/diff" >&2
      die "wormhole-vt's public API differs from $snapshot (above). If the change is meant, run scripts/vt-api.sh --update and commit the file; a removed or changed line is a breaking change."
    fi
    log "the public API matches $snapshot"
    ;;
esac
