//! The frame round a live preview in Settings, and the stage inside it: the card a page puts its
//! miniature of the app in, so every preview sits in the same box. The miniature is made of the
//! app's own components, never a drawing of them.

use gpui_kit::*;

use crate::settings::UiColors;
use crate::style::Edges;
use crate::ui::{Look, PreviewFrameProps};

/// What a preview card says around its picture, all of it words from the catalogs.
#[derive(Clone, Copy)]
pub struct PreviewCard<'a> {
    /// The card's own name, so it is one element for the keyboard and the tests.
    pub id: &'static str,
    /// The heading's first word, "Preview".
    pub label: &'a str,
    /// What is previewed: "Window", "Fonts", ... shown after the label.
    pub what: &'a str,
    /// The picture's name for VoiceOver: a picture of the settings, never a control.
    pub spoken: &'a str,
    /// The line under the picture, saying what it shows.
    pub caption: &'a str,
}

/// A page's preview as a card: the label and what is previewed above `frame` (a [`preview_frame`] holding
/// the miniature), the caption under it. Drawn by the `PreviewFrame` component. One group for
/// VoiceOver, named by `spoken`; the picture holds nothing to operate, so the card takes no focus.
#[derive(IntoElement)]
pub struct PreviewCardView {
    id: &'static str,
    label: SharedString,
    what: SharedString,
    spoken: SharedString,
    caption: SharedString,
    frame: AnyElement,
}

pub fn preview_card(_: UiColors, card: PreviewCard<'_>, frame: Div) -> PreviewCardView {
    PreviewCardView {
        id: card.id,
        label: card.label.to_uppercase().into(),
        what: card.what.to_owned().into(),
        spoken: card.spoken.to_owned().into(),
        caption: card.caption.to_owned().into(),
        frame: frame.into_any_element(),
    }
}

impl RenderOnce for PreviewCardView {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        PreviewFrameProps {
            id: ElementId::from(self.id),
            spoken: self.spoken,
            label: self.label,
            what: self.what,
            caption: self.caption,
            picture: self.frame,
        }
        .render(&Look::of(cx))
    }
}

/// The picture area of a preview card: the panel's shade comes from the card, so this holds the
/// miniature with the room round it the pictures have, and clips what moves inside it.
#[track_caller]
pub fn preview_frame(_: UiColors) -> Div {
    div().p(px(12.0)).overflow_hidden()
}

/// The sidebar's panel as a preview draws it on its own: the surface a list stands on, with the corner
/// a list has, clipped.
#[track_caller]
pub fn preview_surface(ui: UiColors) -> Div {
    div().rounded(ui.radius).overflow_hidden().outlined(ui, ui.border).bg(ui.panel)
}

/// [`preview_surface`] `height` points tall for a preview's parts to be placed on (absolutely, by
/// their distance from its top).
#[track_caller]
pub fn preview_stage(ui: UiColors, height: f32) -> Div {
    preview_surface(ui).relative().w_full().h(px(height))
}
