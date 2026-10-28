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
    /// A sticky notice is up (its words may be under a copy's for a moment).
    pub(crate) sticky: bool,
    /// The chip takes a click now: it shows a sticky notice's own words.
    pub(crate) clickable: bool,
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

/// Say a sticky notice in `window`: the real warning about a terminal that lost its host when `words` is
/// `None`, otherwise a warning that says `words`. Once dismissed, the same one is not said again.
#[cfg(feature = "devctl")]
pub(crate) fn test_sticky(window: &Window, words: Option<&str>, cx: &mut App) {
    use std::hash::{DefaultHasher, Hash as _, Hasher as _};

    use wormhole_core::view::notice::{Notice, Warning};

    let Some(words) = words else {
        super::chips::post_notice(window, Notice::Warned(Warning::HostFallback), cx);
        return;
    };
    let mut hash = DefaultHasher::new();
    words.hash(&mut hash);
    super::chips::post_words(window, Notice::Warned(Warning::Test(hash.finish())), words.to_string(), cx);
}

/// Click `window`'s chip as a person does. `false`: it has none.
pub(crate) fn click(window: WindowId, cx: &App) -> bool {
    super::chips::press(window, cx)
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
            sticky: entry.rules.held().is_some(),
            clickable: entry.rules.clickable(),
            spot: entry.spot,
            #[cfg(feature = "devctl")]
            panel: entry.chip.as_ref().and_then(|chip| chip.info()),
        })
        .collect()
}
