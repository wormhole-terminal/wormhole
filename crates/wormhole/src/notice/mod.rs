//! The notice chip: a small capsule that drops from the top of a window to say what was just done ("Copied 3
//! lines"). Call [`post`] when something really happened. This module owns each window's chip: it runs core's
//! rules (`view::notice`: one chip per window, a new message swaps the words, the same one again is dropped),
//! turns the notice into words from the catalog, has the platform show it (`platform::notice`: a native panel
//! on Core Animation, so GPUI draws nothing) and sets the timer that takes it away again.
//!
//! The state is a global keyed by the window, not a field of the workspace. It must not use the windows, the
//! sidebar or the terminals: they call it. It is off, and every call here does nothing, unless the Notices
//! feature flag is on.
//!
//! The words are never the copied text, only what it was and how much (core's `Copied`).

mod chips;
#[cfg(any(test, feature = "devctl"))]
mod report;
mod words;

use gpui_kit::*;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::view::notice::Notice;

use crate::settings::settings;

pub(crate) use chips::{post, post_front};
#[cfg(any(test, feature = "devctl"))]
pub(crate) use report::notices;
#[cfg(feature = "devctl")]
pub(crate) use report::test_notice;

/// Whether the chip is on: its feature flag. Off, a copy says nothing (and a program's clipboard write
/// keeps the pane bar's notice, which the chip replaces when it is on).
pub(crate) fn enabled(cx: &App) -> bool {
    settings(cx).feature_enabled(FeatureFlag::Notices)
}

/// The words a notice is said in, `None` for one that has none.
pub(crate) fn words_of(notice: Notice) -> Option<String> {
    words::of(notice)
}
