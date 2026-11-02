//! The notice: the one inline message of the Settings window. A solid fill in the kind's
//! color with the words on it, an icon, and room at the right for a button that sits on the
//! same fill.

use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonCustomVariant, ButtonVariants};
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::scale::{BANNER_MIN_H, CONTROL_H, TITLE_GLYPH, TextRole};
use crate::settings::UiColors;

/// What a [`notice`] says, which picks its fill and icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeKind {
    /// Something to know.
    Info,
    /// Something is off, or about to be (dark words on the amber).
    Warning,
    /// Something failed, or a question that destroys something.
    Danger,
}

impl NoticeKind {
    /// The fill (`banner.<kind>.solid`) and the color of what is written on it.
    fn colors(self, ui: UiColors) -> (Hsla, Hsla) {
        match self {
            Self::Info => (ui.notice_info, ui.notice_info_text),
            Self::Warning => (ui.notice_warning, ui.notice_warning_text),
            Self::Danger => (ui.notice_danger, ui.notice_danger_text),
        }
    }

    fn icon(self) -> IconName {
        match self {
            Self::Info => IconName::Info,
            Self::Warning | Self::Danger => IconName::TriangleAlert,
        }
    }
}

/// An inline message next to what it is about: the kind's icon and one or two lines of
/// text on the kind's solid fill. Add a [`notice_button`] with `.child(…)`; it sits at the
/// right.
#[track_caller]
pub fn notice(ui: UiColors, kind: NoticeKind, text: impl Into<SharedString>) -> Div {
    let (fill, ink) = kind.colors(ui);
    div()
        .flex()
        .items_center()
        .gap_3()
        .min_h(px(BANNER_MIN_H))
        .py(px(NOTICE_PAD_Y))
        .pl_3()
        .pr_2()
        .rounded(ui.radius)
        .bg(fill)
        .text_color(ink)
        .child(Icon::new(kind.icon()).with_size(px(TITLE_GLYPH)).flex_none())
        .child(div().flex_1().min_w_0().text_label().font_weight(FontWeight::NORMAL).child(text.into()))
}

/// A notice's padding above and below its text: a one-line notice is [`BANNER_MIN_H`], with a
/// [`CONTROL_H`] button centered in it.
const NOTICE_PAD_Y: f32 = 6.0;

/// A button on a notice's fill, for the action the notice asks for: the words' color as its
/// fill and the notice's fill as its text, a [`CONTROL_H`] tall button like every other in
/// Settings. Add the label, an icon and `.on_click(…)`.
///
/// GPUI Kit's custom variant draws its fill at a fifth of its alpha, so the resting fill is
/// set on the button itself; the variant only gives the hover and the press.
#[track_caller]
pub fn notice_button(id: impl Into<ElementId>, ui: UiColors, kind: NoticeKind, cx: &App) -> Button {
    let (fill, ink) = kind.colors(ui);
    Button::new(id)
        .custom(
            ButtonCustomVariant::new(cx)
                .color(ink)
                .foreground(fill)
                .hover(ink.opacity(HOVER_OPACITY))
                .active(ink.opacity(ACTIVE_OPACITY)),
        )
        .small()
        .h(px(CONTROL_H))
        .bg(ink)
}

/// A [`notice_button`] for the way out ("Cancel") beside a main one: no fill, the notice's
/// words as its text, a wash of them under the pointer.
#[track_caller]
pub fn notice_button_quiet(id: impl Into<ElementId>, ui: UiColors, kind: NoticeKind, cx: &App) -> Button {
    let (_, ink) = kind.colors(ui);
    Button::new(id)
        .custom(
            ButtonCustomVariant::new(cx)
                .foreground(ink)
                .hover(ink.opacity(WASH_OPACITY))
                .active(ink.opacity(2.0 * WASH_OPACITY)),
        )
        .small()
        .h(px(CONTROL_H))
}

/// How much of the filled button's color stays under the pointer and while pressed.
const HOVER_OPACITY: f32 = 0.85;
const ACTIVE_OPACITY: f32 = 0.7;
/// The quiet button's wash under the pointer.
const WASH_OPACITY: f32 = 0.16;
