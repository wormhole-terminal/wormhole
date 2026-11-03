//! The notice: the one inline message of the Settings window. A solid fill in the kind's
//! color with the words on it, an icon, and room at the right for a button that sits on the
//! same fill.

use gpui_kit::component::button::{Button, ButtonCustomVariant, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::*;

use gpui_kit::component::Sizable;

use super::scale::CONTROL_H_SM;
use crate::settings::UiColors;
use crate::ui::{Look, NoticeBarProps};

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
}

/// An inline message next to what it is about: the kind's icon and one or two lines of text on the kind's
/// solid fill, drawn by the `NoticeBar` component. Add a [`notice_button`] with `.child(…)`; it sits at
/// the right.
#[derive(IntoElement)]
pub struct Notice {
    kind: NoticeKind,
    text: SharedString,
    actions: Vec<AnyElement>,
}

pub fn notice(_: UiColors, kind: NoticeKind, text: impl Into<SharedString>) -> Notice {
    Notice { kind, text: text.into(), actions: Vec::new() }
}

impl ParentElement for Notice {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.actions.extend(elements);
    }
}

impl RenderOnce for Notice {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self { kind, text, actions } = self;
        NoticeBarProps {
            id: ElementId::from(("notice", {
                use std::hash::{Hash as _, Hasher as _};
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                text.hash(&mut hasher);
                hasher.finish() as usize
            })),
            text,
            warning: kind == NoticeKind::Warning,
            danger: kind == NoticeKind::Danger,
            actions: (!actions.is_empty())
                .then(|| div().flex().items_center().gap_2().children(actions).into_any_element()),
        }
        .render(&Look::of(cx))
    }
}

/// A button on a notice's fill, for the action the notice asks for: the words' color as its
/// fill and the notice's fill as its text, a [`CONTROL_H_SM`] tall button like every other in
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
        .h(px(CONTROL_H_SM))
        .px(px(10.0))
        .text_size(px(12.0))
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
        .h(px(CONTROL_H_SM))
        .px(px(10.0))
        .text_size(px(12.0))
}

/// How much of the filled button's color stays under the pointer and while pressed.
const HOVER_OPACITY: f32 = 0.85;
const ACTIVE_OPACITY: f32 = 0.7;
/// The quiet button's wash under the pointer.
const WASH_OPACITY: f32 = 0.16;
