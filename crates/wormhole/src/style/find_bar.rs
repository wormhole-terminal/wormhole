//! The find bar: floats over the terminal's top right corner. A query field with its
//! count and the three toggles inside, then previous, next and close.

use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::component::Selectable as _;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::terminal::find::{FindOption, FindOptions};

use super::buttons::{icon, icon_button};
use super::scale::{CONTROL_H, Edges, TextRole};
use crate::settings::UiColors;

/// The bar's size.
pub const FIND_BAR_W: f32 = 440.0;
pub const FIND_BAR_H: f32 = 40.0;
/// Its shadow: it floats over the terminal.
const SHADOW_Y: f32 = 12.0;
const SHADOW_BLUR: f32 = 32.0;

/// What one of the bar's buttons does, given what it says (an option, a step, nothing).
pub type FindHandler<T> = Rc<dyn Fn(T, &mut Window, &mut App)>;

/// What the bar shows and what its buttons do.
pub struct FindBarParts {
    /// The query: a text field (the gallery shows plain text).
    pub field: AnyElement,
    /// The field has the keyboard: its outline is the focus color.
    pub focused: bool,
    /// "11 of 11", "No results", "Not a valid pattern", or nothing before anything is typed.
    pub count: SharedString,
    /// The count is an error (a pattern that doesn't parse).
    pub invalid: bool,
    pub options: FindOptions,
    pub on_option: FindHandler<FindOption>,
    /// −1 for the previous match, 1 for the next.
    pub on_step: FindHandler<isize>,
    pub on_close: FindHandler<()>,
}

/// The find bar. `id` keeps its buttons apart from another bar's.
#[track_caller]
pub fn find_bar(id: impl Into<SharedString>, ui: UiColors, parts: FindBarParts) -> Div {
    let id: SharedString = id.into();
    let FindBarParts { field, focused, count, invalid, options, on_option, on_step, on_close } = parts;
    let toggles = FindOption::ALL.map(|option| {
        let on_option = on_option.clone();
        icon_button(
            SharedString::from(format!("{id}-{}", option.name())),
            ui,
            option_icon(option),
            option.name(),
        )
        .in_row()
        .selected(options.get(option))
        .on_click(move |_, window, cx| on_option(option, window, cx))
    });
    let step = |name: &'static str, glyph, tooltip: &'static str, delta: isize| {
        let on_step = on_step.clone();
        icon_button(SharedString::from(format!("{id}-{name}")), ui, glyph, tooltip)
            .in_title_row()
            .on_click(move |_, window, cx| on_step(delta, window, cx))
    };
    div()
        .flex()
        .flex_none()
        .items_center()
        .gap_1()
        .w(px(FIND_BAR_W))
        .h(px(FIND_BAR_H))
        .pl_2()
        .pr_1()
        .rounded(ui.radius_lg)
        .outlined(ui, ui.border_strong)
        .bg(ui.elevated)
        .shadow(vec![BoxShadow {
            color: ui.shadow,
            offset: point(px(0.), px(SHADOW_Y)),
            blur_radius: px(SHADOW_BLUR),
            spread_radius: px(0.),
            inset: false,
        }])
        .cursor(CursorStyle::Arrow)
        // A click on the bar must not start a selection in the terminal under it.
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(
            div()
                .flex()
                .flex_1()
                .min_w_0()
                .items_center()
                .gap_2()
                .h(px(CONTROL_H))
                .pl_2()
                .rounded(ui.radius)
                .outlined(ui, if focused { ui.focus } else { ui.border })
                .bg(ui.input)
                .child(div().flex_none().text_color(ui.muted).child(icon(IconName::Search)))
                .child(div().flex_1().min_w_0().child(field))
                .child(
                    div()
                        .flex_none()
                        .max_w(px(FIND_BAR_W / 4.0))
                        .truncate()
                        .text_caption()
                        .text_color(if invalid { ui.danger_text } else { ui.muted })
                        .child(count),
                )
                .children(toggles),
        )
        .child(step("previous", IconName::ChevronUp, "Previous match (⇧⌘G)", -1))
        .child(step("next", IconName::ChevronDown, "Next match (⌘G)", 1))
        .child(
            icon_button(SharedString::from(format!("{id}-close")), ui, IconName::X, "Close find (esc)")
                .in_title_row()
                .on_click(move |_, window, cx| on_close((), window, cx)),
        )
}

fn option_icon(option: FindOption) -> IconName {
    match option {
        FindOption::MatchCase => IconName::CaseSensitive,
        FindOption::WholeWord => IconName::WholeWord,
        FindOption::Regex => IconName::Regex,
    }
}
