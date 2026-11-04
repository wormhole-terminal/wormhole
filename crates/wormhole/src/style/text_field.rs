//! The text field: the one look of every single-line field in Settings (and the box for
//! several lines), so a field beside a button, a select or a stepper has their height, corners
//! and outline. GPUI Kit's `Input` brings the editing; its own frame is switched off
//! (`appearance(false)`) and drawn here from the `input.*` tokens, because the Kit's frame fills
//! from its `input` colour (which is `input.border` here, a line colour), rings the focus in
//! its one `ring` and cannot tell an empty field's hint from a filled one's text.

use gpui_kit::component::input::{Input, InputState, Textarea, TextareaState};
use gpui_kit::component::{Sizable, Theme as KitTheme};
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::row_keys::own_menu;
use super::scale::{CONTROL_H, Edges};
use crate::settings::UiColors;

/// A field for one wide value, a folder or a file: every path field in a column has it.
pub const FIELD_W: f32 = 240.0;
/// One of two fields side by side (a name and an email); the pair is as wide as a [`FIELD_W_BOX`].
pub const FIELD_W_PAIR: f32 = 168.0;
/// A field for a single character (a profile's initial).
pub const FIELD_W_LETTER: f32 = 48.0;
/// A box of several lines: two [`FIELD_W_PAIR`] fields and the `_2` gap between them.
pub const FIELD_W_BOX: f32 = 2.0 * FIELD_W_PAIR + 8.0;

/// What the field edits.
enum Body {
    Line(Entity<InputState>),
    Lines(Entity<TextareaState>),
}

/// A text field in `input.background`, [`CONTROL_H`] tall (a box of lines is as tall as its
/// lines need), with the `ui.radius` corners of the controls round it and an outline of
/// `input.border`; while it has the keyboard the outline is `input.border.focus` (the primary)
/// drawn as a ring, the way a picked theme card's is. An empty field shows its hint in
/// `input.placeholder`, a value is `text.primary`.
#[derive(IntoElement)]
pub struct TextField {
    ui: UiColors,
    body: Body,
    width: Option<f32>,
    focused: Option<bool>,
    prefix: Option<AnyElement>,
    cleanable: bool,
}

/// A single-line field for `input`, as wide as the box it is put in; give it a width with
/// [`TextField::w`] ([`FIELD_W`] and its siblings).
#[track_caller]
pub fn text_field(ui: UiColors, input: &Entity<InputState>) -> TextField {
    TextField::new(ui, Body::Line(input.clone()))
}

impl TextField {
    fn new(ui: UiColors, body: Body) -> Self {
        Self { ui, body, width: None, focused: None, prefix: None, cleanable: false }
    }

    /// A box of several lines for `input` (an environment, a list), framed like the one-line field.
    #[track_caller]
    pub fn multiline(ui: UiColors, input: &Entity<TextareaState>) -> Self {
        Self::new(ui, Body::Lines(input.clone()))
    }

    /// The field's width in points.
    #[must_use]
    pub fn w(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Draw the focus outline whatever the focus is: for the Theme Studio's gallery, which cannot
    /// give a specimen the keyboard.
    #[must_use]
    pub fn show_focus(mut self, shown: bool) -> Self {
        self.focused = Some(shown);
        self
    }

    /// Something before the text, a search glyph (one line only).
    #[must_use]
    pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
        self.prefix = Some(prefix.into_any_element());
        self
    }

    /// A button that empties the field while it holds text (one line only).
    #[must_use]
    pub fn cleanable(mut self, cleanable: bool) -> Self {
        self.cleanable = cleanable;
        self
    }
}

impl RenderOnce for TextField {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let TextField { ui, body, width, focused, prefix, cleanable } = self;
        let handle = match &body {
            Body::Line(input) => input.read(cx).focus_handle(cx),
            Body::Lines(input) => input.read(cx).focus_handle(cx),
        };
        let focused = focused.unwrap_or_else(|| handle.contains_focused(window, cx));
        let one_line = matches!(body, Body::Line(_));

        // The Kit's editor paints an empty field's hint in `muted_foreground`, which it reads from
        // its theme while it renders and which is `text.muted` here (the colour of labels and
        // second lines, too near a value's). It reads it once per render, so the hint colour is
        // the Kit's for as long as this one field renders, and the other Kit widgets keep theirs.
        let kit_hint = KitTheme::global_mut(cx).muted_foreground;
        KitTheme::global_mut(cx).muted_foreground = ui.input_placeholder;
        let inner = match body {
            Body::Line(input) => {
                let field = Input::new(&input)
                    .small()
                    .appearance(false)
                    .cleanable(cleanable)
                    .h_full()
                    .when_some(prefix, |field, prefix| field.prefix(prefix));
                RenderOnce::render(field, window, cx).into_any_element()
            }
            Body::Lines(input) => {
                let field = Textarea::new(&input).small().appearance(false);
                RenderOnce::render(field, window, cx).into_any_element()
            }
        };
        KitTheme::global_mut(cx).muted_foreground = kit_hint;

        own_menu(
            div()
                .flex()
                .flex_none()
                .when_some(width, |well, width| well.w(px(width)))
                .when(width.is_none(), |well| well.w_full())
                .when(one_line, |well| well.h(px(CONTROL_H)))
                .rounded(ui.radius)
                .bg(ui.input)
                .text_color(ui.text)
                .map(|well| {
                    if focused {
                        well.outlined_always(ui, ui.input_border_focus)
                    } else {
                        well.outlined(ui, ui.input_border)
                    }
                })
                .child(inner),
        )
    }
}
