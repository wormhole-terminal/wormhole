//! The text field: the one look of every single-line field in Settings (and the box for
//! several lines), so a field beside a button, a select or a stepper has their height, corners
//! and fill. It is flat, as System Settings draws a field: a filled rounded box with no outline,
//! the focus a soft ring round it. GPUI Kit's `Input` brings the editing; its own frame is
//! switched off (`appearance(false)`) and drawn here, because the Kit's frame fills from its
//! `input` colour (which is `input.border` here, a line colour), rings the focus in its one
//! `ring` and cannot tell an empty field's hint from a filled one's text.

use gpui_kit::assets::IconName;
use gpui_kit::component::input::{Input, InputState, Textarea, TextareaState};
use gpui_kit::component::{Sizable, Theme as KitTheme};
use gpui_kit::prelude::*;
use gpui_kit::*;

use wormhole_text::{keys, text};

use super::buttons::icon;
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

/// A flat text field on `control.background` (a little more under the pointer, `control.hover`),
/// [`CONTROL_H`] tall (a box of lines is as tall as its lines need), with the `ui.radius` corners
/// of the controls round it and no outline. While it has the keyboard it wears a soft ring of
/// `input.border.focus` (the primary) a few points wide; Increase Contrast draws an outline of
/// `input.border` at rest and the ring solid and thicker (`metrics.focus_ring_width`). An empty
/// field shows its hint in `input.placeholder`, a value is `text.primary`.
#[derive(IntoElement)]
pub struct TextField {
    ui: UiColors,
    body: Body,
    width: Option<f32>,
    focused: Option<bool>,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
    cleanable: bool,
    /// Where the field was built, so one input drawn twice in a page (the Components page does) gives
    /// each a node of its own in the accessibility tree.
    place: u64,
}

/// The round clear glyph's size: a 4-pt multiple that sits in the field's height with room round it.
const CLEAR_SIZE: f32 = 16.0;
/// How much of the primary the focus ring shows: half, which keeps the field flat and still reads as a
/// focus indicator on a light card (35 % was too soft there) as well as on a dark one.
pub(crate) const FLAT_RING_ALPHA: f32 = 0.5;
/// The height of a box of several lines before it holds any: three lines of text and the field's padding,
/// on the 4-pt grid, so an empty one is a box to click into, not a strip.
const MULTILINE_MIN_H: f32 = 76.0;
/// The ring's width beyond `metrics.focus_ring_width`'s one point inside the edge: a field has no
/// outline to hide it in, so it is a point wider than a select's.
const RING_EXTRA: f32 = 1.0;

/// A single-line field for `input`, as wide as the box it is put in; give it a width with
/// [`TextField::w`] ([`FIELD_W`] and its siblings).
#[track_caller]
pub fn text_field(ui: UiColors, input: &Entity<InputState>) -> TextField {
    TextField::new(ui, Body::Line(input.clone()))
}

impl TextField {
    #[track_caller]
    fn new(ui: UiColors, body: Body) -> Self {
        let at = std::panic::Location::caller();
        let mut hasher = std::hash::DefaultHasher::new();
        std::hash::Hash::hash(&(at.file(), at.line(), at.column()), &mut hasher);
        let place = std::hash::Hasher::finish(&hasher);
        Self { ui, body, width: None, focused: None, prefix: None, suffix: None, cleanable: false, place }
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

    /// Something after the text, a count of hits (one line only), after the clear button.
    #[must_use]
    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }

    /// A button that empties the field while it holds text (one line only): a small round glyph.
    #[must_use]
    pub fn cleanable(mut self, cleanable: bool) -> Self {
        self.cleanable = cleanable;
        self
    }
}

impl RenderOnce for TextField {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let TextField { ui, body, width, focused, prefix, suffix, cleanable, place } = self;
        let handle = match &body {
            Body::Line(input) => input.read(cx).focus_handle(cx),
            Body::Lines(input) => input.read(cx).focus_handle(cx),
        };
        let focused = focused.unwrap_or_else(|| handle.contains_focused(window, cx));
        let one_line = matches!(body, Body::Line(_));
        let id = match &body {
            Body::Line(input) => input.entity_id(),
            Body::Lines(input) => input.entity_id(),
        };
        // The Kit's own clear button is a ghost button with an × that cannot be restyled; the round
        // glyph is ours, and shows only while the field holds text.
        let clear = match &body {
            Body::Line(input) if cleanable && !input.read(cx).value().is_empty() => {
                Some(clear_button(ui, input.clone(), place))
            }
            _ => None,
        };
        let suffix = match (clear, suffix) {
            (None, suffix) => suffix,
            (Some(clear), None) => Some(clear.into_any_element()),
            (Some(clear), Some(suffix)) => {
                Some(div().flex().items_center().gap_1().child(clear).child(suffix).into_any_element())
            }
        };

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
                    .h_full()
                    .when_some(prefix, |field, prefix| field.prefix(prefix))
                    .when_some(suffix, |field, suffix| field.suffix(suffix));
                RenderOnce::render(field, window, cx).into_any_element()
            }
            Body::Lines(input) => {
                // Fills the box, which is at least `MULTILINE_MIN_H` tall, so a click below the lines focuses it.
                let field = Textarea::new(&input).small().appearance(false).h_full();
                RenderOnce::render(field, window, cx).into_any_element()
            }
        };
        KitTheme::global_mut(cx).muted_foreground = kit_hint;

        own_menu(
            div()
                .id(ElementId::NamedInteger("text-field".into(), id.as_u64() ^ place))
                .flex()
                .flex_none()
                .when_some(width, |well, width| well.w(px(width)))
                .when(width.is_none(), |well| well.w_full())
                .when(one_line, |well| well.h(px(CONTROL_H)))
                .when(!one_line, |well| well.min_h(px(MULTILINE_MIN_H)))
                .rounded(ui.radius)
                .bg(ui.control)
                .text_color(ui.text)
                .when(!focused, |well| well.hover(move |d| d.bg(ui.control_hover)))
                .map(|well| match (focused, ui.increase_contrast) {
                    // A faint fill alone is no edge, so Increase Contrast keeps the field's outline.
                    (false, true) => well.outlined_always(ui, ui.input_border),
                    (false, false) => well,
                    (true, true) => well.relative().child(flat_ring(ui, ui.input_border_focus, 1.0)),
                    (true, false) => {
                        well.relative().child(flat_ring(ui, ui.input_border_focus, FLAT_RING_ALPHA))
                    }
                })
                .child(inner),
        )
    }
}

/// The ring of a focused flat control: a band of `color` at `alpha` ([`FLAT_RING_ALPHA`], or 1 under Increase
/// Contrast), `metrics.focus_ring_width` and a point wide just outside the control's edge, for the control to
/// hold as a child (it must be `relative`). It is drawn as a border of its own over nothing, not as the
/// control's shadow, because a flat control's fill is a translucent overlay and a shadow shows through it as
/// a solid block. It takes no clicks and moves nothing. The text field, the shortcut recorder and an open
/// select wear it.
#[track_caller]
pub(crate) fn flat_ring(ui: UiColors, color: Hsla, alpha: f32) -> Div {
    let w = ui.focus_ring_width + px(RING_EXTRA);
    let mut ring = div()
        .absolute()
        .top(-w)
        .right(-w)
        .bottom(-w)
        .left(-w)
        .rounded(ui.radius + w)
        .border_color(Hsla { a: alpha, ..color });
    let widths = &mut ring.style().border_widths;
    (widths.top, widths.right, widths.bottom, widths.left) =
        (Some(w.into()), Some(w.into()), Some(w.into()), Some(w.into()));
    ring
}

/// The × of a cleanable field: a [`CLEAR_SIZE`] round glyph, quiet until the pointer is on it. It
/// leaves the keyboard where it was (the field), so it is no tab stop.
#[track_caller]
fn clear_button(ui: UiColors, input: Entity<InputState>, place: u64) -> impl IntoElement {
    div()
        .id(ElementId::NamedInteger("text-field-clear".into(), input.entity_id().as_u64() ^ place))
        .role(Role::Button)
        .aria_label(text(keys::SETTINGS_FIELD_CLEAR))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .size(px(CLEAR_SIZE))
        .rounded_full()
        .bg(ui.control_hover)
        .text_color(ui.muted)
        .cursor_pointer()
        .hover(move |d| d.text_color(ui.text))
        .child(icon(IconName::X).xsmall())
        .on_click(move |_, window, cx| {
            input.update(cx, |input, cx| {
                input.clean(window, cx);
                input.focus(window, cx);
            });
        })
}
