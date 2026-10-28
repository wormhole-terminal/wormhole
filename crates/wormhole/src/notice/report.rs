//! What devctl and the tests ask of the chips: each window's chip as it stands, and (devctl's `notice`) a
//! notice posted by hand.

use gpui_kit::*;
use wormhole_core::view::notice::{Buttons, Phase, Press, Spot};

use super::chips::Chips;
#[cfg(feature = "devctl")]
use crate::platform::notice::Info;

/// One window's chip.
pub(crate) struct Report {
    pub(crate) window: WindowId,
    pub(crate) phase: Phase,
    /// What it says now, or said last.
    pub(crate) words: String,
    /// The buttons the chip shows now.
    pub(crate) buttons: Buttons,
    /// The explanation's popover is open.
    pub(crate) info_open: bool,
    /// A sticky notice is up (its words may be under a copy's for a moment).
    pub(crate) sticky: bool,
    /// The chip takes a click now: it shows a sticky notice's own words.
    pub(crate) clickable: bool,
    /// The look the chip was made or last restyled in: the theme's appearance, and a plain fill (the system
    /// asks for no transparency or more contrast).
    pub(crate) look: Option<(wormhole_core::theme::tokens::Appearance, bool)>,
    /// The panel is out of sight: its terminal is off screen or an overlay covers the window.
    pub(crate) hidden: bool,
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

/// The error text the real warning carries when devctl posts it: a sample of what a failed helper says.
#[cfg(feature = "devctl")]
const SAMPLE_REASON: &str = "couldn't start the helper process: No such file or directory (os error 2)"; // text: not shown (a made-up error for devctl's sample warning)

/// Say a sticky notice in `window`: the real warning about a terminal that lost its host when `words` is
/// `None` (with a sample error for its explanation; `again`: even if it was dismissed before), otherwise a
/// warning that says `words`, which has no explanation. Once dismissed, the same one is not said again.
#[cfg(feature = "devctl")]
pub(crate) fn test_sticky(window: &Window, words: Option<&str>, again: bool, cx: &mut App) {
    use std::hash::{DefaultHasher, Hash as _, Hasher as _};

    use wormhole_core::view::notice::{Notice, Warning};

    let Some(words) = words else {
        if again {
            super::chips::forget_dismissed(window.window_handle().window_id(), cx);
        }
        let detail = Some(SAMPLE_REASON.to_string());
        super::chips::post_notice(window, Notice::Warned(Warning::HostFallback), detail, cx);
        return;
    };
    let mut hash = DefaultHasher::new();
    words.hash(&mut hash);
    super::chips::post_words(window, Notice::Warned(Warning::Test(hash.finish())), words.to_string(), cx);
}

/// Click `window`'s chip at `press` as a person does. `false`: it has none.
pub(crate) fn click(window: WindowId, press: Press, cx: &App) -> bool {
    super::chips::press(window, press, cx)
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
            buttons: entry.rules.buttons(),
            info_open: entry.info.is_some(),
            hidden: entry.hidden,
            look: entry.look,
            spot: entry.spot,
            #[cfg(feature = "devctl")]
            panel: entry.chip.as_ref().and_then(|chip| chip.info()),
        })
        .collect()
}
