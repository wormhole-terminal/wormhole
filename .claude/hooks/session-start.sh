#!/bin/bash
# SessionStart hook for Claude Code on the web: make the workspace buildable on Linux so cloud
# sessions can run fmt, clippy and the tests of BOTH crates (the app crate type-checks, lints and
# tests off macOS). Local macOS sessions need nothing and skip this.
# Idempotent and non-interactive; the container state is cached once it finishes.
set -euo pipefail

[ "${CLAUDE_CODE_REMOTE:-}" = "true" ] || exit 0
cd "${CLAUDE_PROJECT_DIR:-.}"

# GPUI's Linux backend needs these: fontconfig even for `cargo check` (its build script panics
# without it), xkbcommon to link the test binaries. Everything else is dlopen'd or vendored.
PACKAGES=(build-essential pkg-config git libfontconfig-dev libxkbcommon-dev libxkbcommon-x11-dev)

missing=()
for pkg in "${PACKAGES[@]}"; do
  dpkg -s "$pkg" >/dev/null 2>&1 || missing+=("$pkg")
done
if [ "${#missing[@]}" -gt 0 ]; then
  SUDO=""
  [ "$(id -u)" -eq 0 ] || SUDO="sudo"
  $SUDO apt-get update -qq
  $SUDO apt-get install -y -qq --no-install-recommends "${missing[@]}"
fi

rustup component add rustfmt clippy >/dev/null 2>&1 || true
# Warm the registry cache so the first build does not wait on downloads. Never without --locked:
# a plain fetch would rewrite Cargo.lock in the container.
cargo fetch --locked >/dev/null 2>&1 || true
