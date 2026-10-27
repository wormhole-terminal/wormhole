#!/usr/bin/env bash
# P1a of the agent permissions: the code-signature module moves from the session host to
# `wormhole_term::sys`, so the app can read a caller's signature too. A move, no behaviour change.
# Rerunnable: it does nothing once the files are where they should be.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
H=crates/wormhole-host/src/sys
T=crates/wormhole-term/src/sys
[ -f "$H/codesign.rs" ] || { echo "already moved"; exit 0; }

git mv "$H/codesign.rs" "$T/codesign.rs"
git mv "$H/macos/codesign.rs" "$T/macos/codesign.rs"
git mv "$H/linux/codesign.rs" "$T/linux/codesign.rs"

# Paths and visibility: what was the host's crate-private API is the term crate's public one.
sed -i '' -e 's/^pub(crate) /pub /' -e 's/use super::linux::codesign as os;/use super::linux::codesign as os;/' \
  -e 's/pub use os::Requirement;/pub use os::Requirement;/' "$T/codesign.rs"
sed -i '' -e 's/use crate::sys::codesign::/use crate::sys::codesign::/' -e 's/^pub(crate) /pub /' \
  -e 's/^pub(crate) struct Requirement/pub struct Requirement/' "$T/macos/codesign.rs"
sed -i '' -e 's/^pub(crate) /pub /' "$T/linux/codesign.rs"
sed -i '' -e 's/use wormhole_term::sys::socket::Stream;/use crate::sys::socket::Stream;/' \
  "$T/codesign.rs" "$T/macos/codesign.rs" "$T/linux/codesign.rs"

# Module lists.
sed -i '' -e '/^pub(crate) mod codesign;$/d' "$H/mod.rs"
sed -i '' -e '/^pub(super) mod codesign;$/d' "$H/macos/mod.rs" "$H/linux/mod.rs"
sed -i '' -e 's/^pub mod detached;$/pub mod codesign;\npub mod detached;/' "$T/mod.rs"
sed -i '' -e 's/^mod ports;$/pub(super) mod codesign;\nmod ports;/' "$T/macos/mod.rs"
sed -i '' -e 's/^pub(super) mod process;$/pub(super) mod codesign;\npub(super) mod process;/' "$T/linux/mod.rs"

# The host's callers.
sed -i '' -e 's/use crate::sys::codesign::{self, Refusal, Requirement};/use wormhole_term::sys::codesign::{self, Refusal, Requirement};/' \
  -e 's/use crate::sys::codesign::{self, Stage};/use wormhole_term::sys::codesign::{self, Stage};/' crates/wormhole-host/src/peer.rs
