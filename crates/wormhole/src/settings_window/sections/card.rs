//! The card every page's preview stands in: Settings' side of [`style::preview_card`], which says
//! "Preview", what is previewed and the caption in the catalogs' words.

use gpui_kit::*;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::{Key, text};

use crate::settings::UiColors;
use crate::style::{self, PreviewCard};

/// What a card says besides its picture, as the keys of the catalogs' messages.
#[derive(Clone, Copy)]
pub(super) struct Words {
    /// The card's element id, per page.
    pub id: &'static str,
    /// What is previewed ("Window", "Fonts", ...).
    pub what: Key,
    /// The picture's name for VoiceOver.
    pub spoken: Key,
    /// The line under the picture.
    pub caption: Key,
}

/// `frame` (a [`style::preview_frame`] with the miniature in it) as the page's preview card.
pub(super) fn card(ui: UiColors, words: Words, frame: Div) -> AnyElement {
    style::preview_card(
        ui,
        PreviewCard {
            id: words.id,
            label: text(SETTINGS_WINDOW_PREVIEW),
            what: text(words.what),
            spoken: text(words.spoken),
            caption: text(words.caption),
        },
        frame,
    )
    .into_any_element()
}
