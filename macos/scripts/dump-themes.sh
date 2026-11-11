#!/usr/bin/env bash
# Regenerates the Swift UI's theme data from `wormhole-core`'s real token resolution:
#   1. the `dump_themes` example writes every bundled theme (normal and Increase Contrast) as JSON into
#      the DesignSystem resource `themes.json`;
#   2. `gen-token-accessors.py` writes the typed `ThemeTokens` accessors from its catalog.
# `--check` changes nothing and fails when either is out of date (the Rust test `cargo test -p wormhole-ffi
# --test theme_dump` is the first of these as a test). Honours CARGO_TARGET_DIR and CARGO_BUILD_JOBS.
set -euo pipefail

macos="$(cd "$(dirname "$0")/.." && pwd)"
resource="$macos/swift/Sources/DesignSystem/Resources/themes.json"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
# The dump is an example of the bridge crate, which is a cargo workspace of its own.
cd "$macos/../crates/wormhole-ffi"

if [[ "${1:-}" == "--check" ]]; then
  fresh="$(mktemp -d)/themes.json"
  cargo run --quiet -p wormhole-ffi --example dump_themes -- "$fresh" >/dev/null
  cmp -s "$fresh" "$resource" || { echo "themes.json is stale: run macos/scripts/dump-themes.sh" >&2; exit 1; }
  python3 -I scripts/gen-token-accessors.py --check || { echo "token accessors are stale" >&2; exit 1; }
  echo "themes.json and the token accessors are current"
else
  cargo run --quiet -p wormhole-ffi --example dump_themes -- "$resource"
  python3 -I scripts/gen-token-accessors.py
fi
