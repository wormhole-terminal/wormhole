//! The notice chip: a capsule that drops onto a terminal's top edge to say what was just done ("Copied 3
//! lines"). Call [`post`] when something really happened. This module owns each window's chip: it runs core's
//! rules (`view::notice`: one chip per window, a new message swaps the words, the same one again is dropped),
//! turns the notice into words from the catalog, has the platform show it (`platform::notice`: a native panel
//! on Core Animation, so GPUI draws nothing) and sets the timer that takes it away again.
//!
//! The state is a global keyed by the window, not a field of the workspace. It must not use the windows, the
//! sidebar or the terminals: they call it, each naming its own window and, for a notice about a terminal, where
//! that terminal is (`Anchor`). A window tells the module who it is once (`register`). It does nothing unless
//! the Notices feature flag is on in the window's profile.
//!
//! A warning (core's `Notice::Warned`) is sticky: the chip stays until the person clicks it, and the click
//! comes back from the panel over a channel this module reads.
//!
//! The words are never the copied text, only what it was and how much (core's `Copied`).

mod chips;
mod explain;
mod follow;
mod info;
#[cfg(any(test, feature = "devctl"))]
mod report;
mod words;

pub(crate) use chips::{Anchor, Facts, Where, enabled, post, post_to, post_to_with, register};
#[cfg(test)]
pub(crate) use follow::EVERY as FOLLOW_EVERY;
#[cfg(any(test, feature = "devctl"))]
pub(crate) use report::{click, notices};
#[cfg(feature = "devctl")]
pub(crate) use report::{test_notice, test_sticky};
pub(crate) use words::host_fallback_detail;
