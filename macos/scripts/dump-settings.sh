#!/usr/bin/env bash
# Regenerates the Swift UI's settings schema from `wormhole-core`'s settings index: the `dump_settings` example
# writes every section, group and row (words, search words, config keys, control kind and options, defaults)
# as JSON into the SettingsModel resource `settings.json`. `--check` changes nothing and fails when it is out of
# date (the Rust test `cargo test -p wormhole-ffi --test settings_dump` is the same check as a test).
# Honours CARGO_TARGET_DIR and CARGO_BUILD_JOBS.
set -euo pipefail

macos="$(cd "$(dirname "$0")/.." && pwd)"
resource="$macos/swift/Sources/SettingsModel/Resources/settings.json"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
# The dump is an example of the bridge crate, which is a cargo workspace of its own.
cd "$macos/../crates/wormhole-ffi"

if [[ "${1:-}" == "--check" ]]; then
  fresh="$(mktemp -d)/settings.json"
  cargo run --quiet -p wormhole-ffi --example dump_settings -- "$fresh" >/dev/null
  cmp -s "$fresh" "$resource" || { echo "settings.json is stale: run macos/scripts/dump-settings.sh" >&2; exit 1; }
  echo "settings.json is current"
else
  cargo run --quiet -p wormhole-ffi --example dump_settings -- "$resource"
fi
