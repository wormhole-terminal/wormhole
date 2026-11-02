//! A colour control: a swatch and the colour's name in the select's face, and a card that is built only
//! while it is open, with the theme's own colour, eight presets and Custom… (an editor of saturation and
//! brightness, hue and a hex code, in `editor`).
//!
//! It is for the settings that are a colour a person may choose freely (the primary, the branch). It knows
//! nothing of them: the page says what the colour is now ([`Value`]) and what a choice does ([`Pick`]),
//! so the same control serves a setting that has one more way to be set than the others (the branch can
//! follow the primary, one of its `extras`). A colour is never capped: the editor reaches every colour of
//! the sRGB cube and a hex code written by hand is shown as it is, whatever it is.
//!
//! Like the menu control it shares its open and close with (`style::OpenOne`), a closed control is one
//! focusable element around the select's trigger; the card, its closures and the editor exist only for
//! the open one, which a window shows one at a time.
//!
//! The keyboard path is the menu's: ⇥ reaches the control, ↩ and Space open it, the arrows, Home and End
//! move over the choices in order (the theme's colour, the extras, the presets, Custom…), ↩ and Space
//! choose, Esc closes and leaves the keyboard on the control. Custom… moves the keyboard to the area,
//! where the arrows move the thumb (⇧ for bigger steps), ⇥ goes on to the hue bar, the field and Done.
//! For VoiceOver the control is a pop-up button with its colour's name, and the presets are a list of
//! which the current one is selected.

mod editor;
mod state;

use std::rc::Rc;

use gpui_kit::accesskit::Role;
use gpui_kit::component::Selectable as _;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::prelude::*;
use gpui_kit::*;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

pub use self::state::ColorPickState;
use super::{SelectTrigger, menu_bg, menu_shadow, option};
use crate::settings::UiColors;
use crate::style::{self, COLOR_DOT, COLOR_DOT_MENU, CONTROL_H, Edges as _, OpenOne};
use crate::theme_bridge;

/// Marks the colour controls' [`OpenOne`].
struct ColorMenu;

type Open = OpenOne<ColorMenu>;

/// The gap between the control and its card, and how near the card may come to the window's edge.
const GAP: f32 = 4.0;
const EDGE: f32 = 8.0;
/// The widest the face grows before its name truncates.
const FACE_MAX_W: f32 = 280.0;
/// The card's width.
const CARD_W: f32 = 264.0;

/// A colour with the name a person knows it by.
#[derive(Clone)]
pub struct Swatch {
    pub name: SharedString,
    pub color: Hsla,
}

/// What the setting is now.
#[derive(Clone)]
pub enum Value {
    /// The theme's own colour.
    Theme,
    /// The setting's other way of being set (the branch following the primary): an index into `extras`.
    Extra(usize),
    /// One of the presets.
    Preset(usize),
    /// A colour of the person's own, or a value that is none of the above. `text` is what is stored (a hex
    /// code, or what was written by hand in `config.json`); `color` is it as a colour, when it is one.
    Custom { text: SharedString, color: Option<Hsla> },
}

/// What a person chose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Picked {
    Theme,
    /// The extra at this place.
    Extra(usize),
    /// A colour, as an opaque `#rrggbb`: a preset's, or one made in the editor or typed.
    Hex(String),
}

/// What a choice does to the setting.
pub type Pick = Rc<dyn Fn(Picked, &mut App)>;

/// Everything the page tells the control.
pub struct ColorChoice {
    /// The theme's own colour, which "Theme's colour" shows.
    pub theme: Hsla,
    /// More ways to set it than the theme's, the presets and a colour of one's own, listed after the theme's.
    pub extras: Vec<Swatch>,
    /// The presets, as the theme draws them.
    pub presets: Vec<Swatch>,
    pub value: Value,
    pub pick: Pick,
}

impl ColorChoice {
    /// The place of the current value among the choices of the list (the theme's, the extras, the presets,
    /// Custom…).
    fn current(&self) -> usize {
        match &self.value {
            Value::Theme => 0,
            Value::Extra(at) => 1 + at,
            Value::Preset(at) => 1 + self.extras.len() + at,
            Value::Custom { .. } => self.lines() - 1,
        }
    }

    /// How many choices the list has.
    fn lines(&self) -> usize {
        self.extras.len() + self.presets.len() + 2
    }

    /// The swatch and the words of the face: the colour's name, or the code of a colour of one's own.
    fn face(&self) -> (Option<Hsla>, SharedString) {
        match &self.value {
            Value::Theme => (Some(self.theme), text(COLOR_PICK_THEME).into()),
            Value::Extra(at) => self
                .extras
                .get(*at)
                .map_or((None, SharedString::default()), |s| (Some(s.color), s.name.clone())),
            Value::Preset(at) => self
                .presets
                .get(*at)
                .map_or((None, SharedString::default()), |s| (Some(s.color), s.name.clone())),
            Value::Custom { text: written, color } => (
                *color,
                if written.is_empty() { text(COLOR_PICK_CUSTOM_FACE).into() } else { written.clone() },
            ),
        }
    }
}

/// A colour's mark: its dot, or an empty outline where there is none to show.
#[track_caller]
fn dot(ui: UiColors, color: Option<Hsla>, size: f32) -> Div {
    match color {
        Some(color) => style::color_dot(ui, color, size),
        None => div().flex_none().size(px(size)).rounded(ui.radius_sm).outlined_always(ui, ui.border_strong),
    }
}

/// The control of the row `row`.
#[derive(IntoElement)]
pub struct ColorSelect {
    row: ElementId,
    ui: UiColors,
    /// The row's name, what VoiceOver calls the control.
    label: SharedString,
    state: Entity<ColorPickState>,
    choice: ColorChoice,
}

#[track_caller]
pub fn color_select(
    row: impl Into<ElementId>,
    ui: UiColors,
    label: impl Into<SharedString>,
    state: &Entity<ColorPickState>,
    choice: ColorChoice,
) -> ColorSelect {
    ColorSelect { row: row.into(), ui, label: label.into(), state: state.clone(), choice }
}

impl RenderOnce for ColorSelect {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self { row, ui, label, state, choice } = self;
        let this_window = window.window_handle().window_id();
        let open = Open::is_open(this_window, &row, cx);
        let (swatch, name) = choice.face();
        let trigger = state.read(cx).trigger.clone();
        let face = div()
            .flex()
            .items_center()
            .gap_2()
            .child(dot(ui, swatch, COLOR_DOT))
            .child(div().min_w_0().truncate().child(name.clone()));
        let face = SelectTrigger::new("face", ui, face).selected(open);
        let (on_press, on_down, starts_editing) =
            (row.clone(), row.clone(), matches!(choice.value, Value::Custom { .. }));
        let opening = state.clone();
        div()
            .id("color-select")
            .role(Role::ComboBox)
            .aria_label(label)
            .aria_value(name)
            .aria_expanded(open)
            .track_focus(&trigger)
            .focus_visible(move |d| d.ringed(ui, ui.focus, ui.focus_ring_width))
            .relative()
            .flex()
            .flex_none()
            .max_w(px(FACE_MAX_W))
            .rounded(ui.radius)
            .child(face)
            .on_mouse_down(MouseButton::Left, move |event, _, cx| Open::pressed(&on_down, event.position, cx))
            .on_click(move |event, window, cx| {
                if !Open::is_open(window.window_handle().window_id(), &on_press, cx) {
                    // The editor is open at once for a colour of one's own; the keyboard starts at the value.
                    opening.update(cx, |state, _| {
                        state.editing = starts_editing;
                        state.highlight = None;
                    });
                }
                Open::clicked(&on_press, event, window, cx);
            })
            .when(open, |d| {
                let at = state.read(cx).highlight.unwrap_or_else(|| choice.current());
                let choice = Rc::new(choice);
                {
                    let (state, trigger) = (state.clone(), trigger.clone());
                    let closed = Rc::new(move |window: &mut Window, cx: &mut App| {
                        state.update(cx, |state, _| state.editing = false);
                        // A press elsewhere or ⇥ has put the keyboard where it goes: only a close that
                        // left it nowhere takes it back to the control.
                        if window.focused(cx).is_none() {
                            window.focus(&trigger, cx);
                        }
                    });
                    Open::on_close(this_window, &row, closed, cx);
                }
                let keys = Keys { row: row.clone(), state: state.clone(), choice: choice.clone() };
                d.capture_key_down(move |event, window, cx| keys.handle(&event.keystroke, window, cx))
                    .child(card(ui, row, &state, &choice, at, cx))
            })
    }
}

/// Make the choice at `at` in the list: close the card, or open the editor (Custom…).
fn choose(
    at: usize,
    row: &ElementId,
    state: &Entity<ColorPickState>,
    choice: &ColorChoice,
    window: &mut Window,
    cx: &mut App,
) {
    let extras = choice.extras.len();
    if at == choice.lines() - 1 {
        // Custom…: the editor under the presets, the keyboard on its area.
        let focus = state.update(cx, |state, _| {
            state.editing = true;
            state.area_focus.clone()
        });
        window.focus(&focus, cx);
        window.refresh();
        return;
    }
    let picked = match at {
        0 => Picked::Theme,
        at if at <= extras => Picked::Extra(at - 1),
        at => match choice.presets.get(at - 1 - extras) {
            Some(preset) => Picked::Hex(theme_bridge::color(preset.color).with_alpha(1.0).to_hex()),
            None => return,
        },
    };
    (choice.pick)(picked, cx);
    let _ = row;
    Open::close(window, cx);
}

/// What the open card's keys do.
struct Keys {
    row: ElementId,
    state: Entity<ColorPickState>,
    choice: Rc<ColorChoice>,
}

impl Keys {
    fn handle(&self, keystroke: &Keystroke, window: &mut Window, cx: &mut App) {
        let key = keystroke.key.as_str();
        if key == "escape" {
            cx.stop_propagation();
            let trigger = self.state.read(cx).trigger.clone();
            Open::close(window, cx);
            window.focus(&trigger, cx);
            return;
        }
        if self.state.read(cx).editing {
            self.in_editor(key, window, cx);
        } else {
            self.in_list(key, window, cx);
        }
    }

    /// The editor has the keyboard, in the area, the bar or the field: they read their own keys. Only the
    /// field's ↩ is read here, as it is a field and not a control that knows what it edits.
    fn in_editor(&self, key: &str, window: &mut Window, cx: &mut App) {
        let hex = self.state.read(cx).hex.read(cx).focus_handle(cx);
        if !hex.is_focused(window) {
            return;
        }
        if key == "enter" {
            cx.stop_propagation();
            if let Some(hex) = self.state.update(cx, |state, cx| state.typed(window, cx)) {
                (self.choice.pick)(Picked::Hex(hex), cx);
            }
        }
    }

    /// The list has the keyboard: the arrows walk it, ↩ and Space choose.
    fn in_list(&self, key: &str, window: &mut Window, cx: &mut App) {
        let last = self.choice.lines() - 1;
        let at = self.state.read(cx).highlight.unwrap_or_else(|| self.choice.current()).min(last);
        let moved = match key {
            "down" | "right" => Some((at + 1).min(last)),
            "up" | "left" => Some(at.saturating_sub(1)),
            "home" => Some(0),
            "end" => Some(last),
            _ => None,
        };
        if let Some(to) = moved {
            cx.stop_propagation();
            self.state.update(cx, |state, _| state.highlight = Some(to));
            window.refresh();
        } else if matches!(key, "enter" | "space") {
            // Stopping the key keeps the control from taking its release for a click that opens the card again.
            cx.stop_propagation();
            choose(at, &self.row, &self.state, &self.choice, window, cx);
        }
    }
}

/// The card, under the control's right edge and kept inside the window.
#[track_caller]
fn card(
    ui: UiColors,
    row: ElementId,
    state: &Entity<ColorPickState>,
    choice: &Rc<ColorChoice>,
    at: usize,
    cx: &App,
) -> Div {
    let editing = state.read(cx).editing;
    let extras = choice.extras.len();
    let current = choice.current();
    // The theme's colour and the extras, one to a line.
    let line = |ix: usize, swatch: Option<Hsla>, name: SharedString| {
        let (row, state, choice) = (row.clone(), state.clone(), choice.clone());
        option(("line", ix), ui, current == ix, false)
            .role(Role::ListBoxOption)
            .aria_label(name.clone())
            .aria_selected(current == ix)
            .when(at == ix, |d| d.bg(ui.hover).aria_active_descendant())
            .child(dot(ui, swatch, COLOR_DOT_MENU))
            .child(div().flex_1().min_w_0().truncate().child(name))
            .on_click(move |_, window, cx| choose(ix, &row, &state, &choice, window, cx))
    };
    let mut lines = vec![line(0, Some(choice.theme), text(COLOR_PICK_THEME).into()).into_any_element()];
    lines.extend(
        choice
            .extras
            .iter()
            .enumerate()
            .map(|(i, s)| line(1 + i, Some(s.color), s.name.clone()).into_any_element()),
    );
    let presets = div().flex().items_center().justify_between().px_2().py_1().children(
        choice.presets.iter().enumerate().map(|(i, preset)| {
            let ix = 1 + extras + i;
            let (row, state, choice) = (row.clone(), state.clone(), choice.clone());
            style::color_choice(
                ("preset", i),
                ui,
                preset.name.clone(),
                Some(preset.color),
                current == ix,
                true,
            )
            .when(at == ix, |d| d.bg(ui.hover).aria_active_descendant())
            .tooltip({
                let name = preset.name.clone();
                move |window, cx| Tooltip::new(name.clone()).build(window, cx)
            })
            .on_click(move |_, window, cx| choose(ix, &row, &state, &choice, window, cx))
        }),
    );
    let custom_at = choice.lines() - 1;
    let custom_swatch = match &choice.value {
        Value::Custom { color, .. } => *color,
        _ => None,
    };
    let custom = {
        let (row, state, choice) = (row.clone(), state.clone(), choice.clone());
        option("custom", ui, current == custom_at, false)
            .role(Role::ListBoxOption)
            .aria_label(text(COLOR_PICK_CUSTOM))
            .aria_selected(current == custom_at)
            .when(at == custom_at, |d| d.bg(ui.hover).aria_active_descendant())
            .child(dot(ui, custom_swatch, COLOR_DOT_MENU))
            .child(div().flex_1().min_w_0().truncate().child(text(COLOR_PICK_CUSTOM)))
            .on_click(move |_, window, cx| choose(custom_at, &row, &state, &choice, window, cx))
    };
    let dismissed = row.clone();
    let done = {
        let trigger = state.read(cx).trigger.clone();
        move |window: &mut Window, cx: &mut App| {
            Open::close(window, cx);
            window.focus(&trigger, cx);
        }
    };
    let editor =
        editing.then(|| div().px_2().pt_1().pb_2().child(editor::editor(ui, state, &choice.pick, done, cx)));
    let list = div()
        .id("color-select-card")
        .role(Role::ListBox)
        .occlude()
        .flex()
        .flex_col()
        .p_1()
        .w(px(CARD_W))
        .rounded(ui.radius_lg)
        .bg(menu_bg(ui, cx))
        .shadow(menu_shadow(ui))
        .cursor(CursorStyle::Arrow)
        .on_mouse_down_out(move |event, window, cx| {
            Open::dismissed_outside(&dismissed, event.position, window, cx);
        })
        .children(lines)
        .child(presets)
        .child(style::popover_rule(ui))
        .child(custom)
        .children(editor);
    div().absolute().top(px(CONTROL_H + GAP)).right_0().child(
        deferred(anchored().anchor(Anchor::TopRight).snap_to_window_with_margin(px(EDGE)).child(list))
            .with_priority(gpui_kit::base::POPUP_PRIORITY),
    )
}

/// devctl `settings-select`: open the card of the row `row`, as a click on it does.
#[cfg(feature = "devctl")]
pub fn devctl_open(row: &'static str, state: &Entity<ColorPickState>, window: &mut Window, cx: &mut App) {
    state.update(cx, |state, _| state.highlight = None);
    Open::toggle(&ElementId::from(row), window, cx);
}
