//! What devctl and the tests ask of the chips: each window's chip as it stands, and (devctl's `notice`) a
//! notice posted by hand.

use gpui_kit::*;
use wormhole_core::view::notice::{Phase, Spot};

use super::chips::Chips;
#[cfg(feature = "devctl")]
use crate::platform::notice::Info;

/// One window's chip.
pub(crate) struct Report {
    pub(crate) window: WindowId,
    pub(crate) phase: Phase,
    /// What it says now, or said last.
    pub(crate) words: String,
    /// Where the chip was last put: the terminal it is about and the window.
    pub(crate) spot: Option<Spot>,
    /// The panel, while there is one.
    #[cfg(feature = "devctl")]
    pub(crate) panel: Option<Info>,
}

/// Say `words` in `window` as a notice of its own: the same words within two seconds are dropped, as any
/// message is.
#[cfg(feature = "devctl")]
pub(crate) fn test_notice(window: &Window, words: &str, cx: &mut App) {
    use std::hash::{DefaultHasher, Hash as _, Hasher as _};

    use wormhole_core::view::notice::Notice;

    let mut hash = DefaultHasher::new();
    words.hash(&mut hash);
    super::chips::post_words(window, Notice::Test(hash.finish()), words.to_string(), cx);
}

/// Every window that has had a chip.
pub(crate) fn notices(cx: &App) -> Vec<Report> {
    let Some(chips) = cx.try_global::<Chips>() else { return Vec::new() };
    chips
        .windows
        .iter()
        .map(|(window, entry)| Report {
            window: *window,
            phase: entry.rules.phase(),
            words: entry.words.clone(),
            spot: entry.spot,
            #[cfg(feature = "devctl")]
            panel: entry.chip.as_ref().and_then(|chip| chip.info()),
        })
        .collect()
}
