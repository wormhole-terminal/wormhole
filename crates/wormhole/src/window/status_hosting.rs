//! The status bar's item that says where the selected terminal runs: an icon and a word, "Persistent"
//! (in a session host, which outlives the app), "Connecting…" or "In app" (it ends when Wormhole quits
//! or updates). Quiet when persistent, in the warning colour when it is not, since that is the one a
//! person would want to notice. Shown only while persistent terminals are on: without them every terminal
//! is in the app, which says nothing.

use gpui_kit::assets::IconName;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_text::{keys, text};

use crate::settings::UiColors;
use crate::style;
use crate::terminal_view::Hosting;

/// What the item says for `hosting`: its word and the sentence that explains it, or `None` for a
/// terminal that has ended with its host (nothing runs there to be persistent or not) and for one that
/// is in the app by design, the Welcome terminal (a warning there would be about nothing).
fn words(hosting: Hosting) -> Option<(&'static str, &'static str)> {
    match hosting {
        Hosting::Persistent => Some((
            text(keys::STATUS_BAR_HOSTING_PERSISTENT),
            text(keys::STATUS_BAR_HOSTING_PERSISTENT_TOOLTIP),
        )),
        Hosting::Connecting => Some((
            text(keys::STATUS_BAR_HOSTING_CONNECTING),
            text(keys::STATUS_BAR_HOSTING_CONNECTING_TOOLTIP),
        )),
        Hosting::InApp => {
            Some((text(keys::STATUS_BAR_HOSTING_IN_APP), text(keys::STATUS_BAR_HOSTING_IN_APP_TOOLTIP)))
        }
        Hosting::Lost | Hosting::AppOnly => None,
    }
}

/// Whether the bar has an item for `hosting` at all.
pub(super) fn shown(hosting: Hosting) -> bool {
    words(hosting).is_some()
}

/// The item for `hosting`, with its word when `word` (a narrow window keeps the icon, which the
/// tooltip and VoiceOver explain).
pub(super) fn item(ui: UiColors, hosting: Hosting, word: bool) -> Option<Stateful<Div>> {
    let (label, tooltip) = words(hosting)?;
    let (icon, color) = match hosting {
        Hosting::Persistent => (IconName::HardDrive, ui.muted),
        Hosting::Connecting => (IconName::RefreshCw, ui.muted),
        Hosting::InApp | Hosting::Lost | Hosting::AppOnly => (IconName::TriangleAlert, ui.warning_text),
    };
    Some(
        style::status_text(ui)
            .id("status-hosting")
            .role(Role::Label)
            // The sentence alone: it says the state and what follows from it, so VoiceOver needs no
            // second message built from the word and the sentence.
            .aria_label(tooltip)
            .text_color(color)
            .child(style::icon(icon))
            .when(word, |d| d.child(label))
            .tooltip(move |window, cx| Tooltip::new(tooltip).build(window, cx)),
    )
}

#[cfg(test)]
mod tests {
    use super::{Hosting, shown, words};

    #[test]
    fn every_state_with_a_program_has_a_word_and_a_sentence_and_a_lost_host_has_none() {
        for hosting in [Hosting::Persistent, Hosting::Connecting, Hosting::InApp] {
            let (label, tooltip) = words(hosting).expect("says something");
            assert!(!label.is_empty() && tooltip.ends_with('.'), "{hosting:?}: {label:?} {tooltip:?}");
        }
        assert!(!shown(Hosting::Lost));
        assert!(!shown(Hosting::AppOnly));
    }
}
