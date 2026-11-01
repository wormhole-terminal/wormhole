//! A menu control: the select's face on a settings row, with a menu that is built only while it is open.
//!
//! A [`super::SearchSelect`] builds a popover and its trigger for every row on every frame, and a page
//! has a dozen such rows. Here a closed control is one focusable element around the select's trigger and
//! one read of a global; the options, their closures and the card exist only for the open one, which a
//! window shows one at a time ([`OpenOne`]).
//!
//! The keyboard path is the one of the (i): ⇥ reaches the control, ↩ and Space open it, ↑ ↓ Home and End
//! move through the options, ↩ and Space choose, Esc closes and leaves the keyboard on the control. For
//! VoiceOver the control is a pop-up button with its current value, and the options are a list of which
//! the current one is selected; the highlighted option is the one the keyboard is on.

use std::rc::Rc;

use gpui_kit::accesskit::Role;
use gpui_kit::component::Selectable as _;
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::{SelectTrigger, menu_bg, menu_shadow, option};
use crate::settings::UiColors;
use crate::style::{Edges as _, OpenOne};

/// Marks the menu controls' [`OpenOne`].
struct Menu;

type Open = OpenOne<Menu>;

/// What a window's open menu has highlighted: the option the keyboard is on.
#[derive(Default)]
struct Highlight {
    row: Option<(WindowId, ElementId, usize)>,
}

impl Global for Highlight {}

/// The gap between the control and its menu.
const GAP: f32 = 4.0;
/// How near the menu may come to the window's edge.
const EDGE: f32 = 8.0;

/// Which pick of the options a keyboard or pointer chose.
type Pick = Rc<dyn Fn(usize, &mut Window, &mut App)>;
/// The option the pointer or the keyboard is on now (`None`: none).
type Hover = Rc<dyn Fn(Option<usize>, &mut Window, &mut App)>;

/// What the open menu offers. Built by the caller only when the menu opens.
pub struct MenuOptions {
    /// The options' words, in order.
    pub labels: Vec<SharedString>,
    /// The option that is the current value, checked; `None` when the value is none of them.
    pub current: Option<usize>,
    pub on_pick: Pick,
    /// Told which option is under the pointer or the keyboard, and `None` as it leaves or the menu closes
    /// (a page that previews each option).
    pub on_hover: Option<Hover>,
}

/// The control of the row `row`: `value` in the select's face, a menu of what `options` builds under it.
#[derive(IntoElement)]
pub struct MenuSelect<B: FnOnce() -> MenuOptions + 'static> {
    row: ElementId,
    ui: UiColors,
    /// The row's name, what VoiceOver calls the control.
    label: SharedString,
    value: SharedString,
    width: Pixels,
    options: B,
}

#[track_caller]
pub fn menu_select<B: FnOnce() -> MenuOptions + 'static>(
    row: impl Into<ElementId>,
    ui: UiColors,
    label: impl Into<SharedString>,
    value: impl Into<SharedString>,
    width: f32,
    options: B,
) -> MenuSelect<B> {
    MenuSelect { row: row.into(), ui, label: label.into(), value: value.into(), width: px(width), options }
}

impl<B: FnOnce() -> MenuOptions + 'static> RenderOnce for MenuSelect<B> {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self { row, ui, label, value, width, options } = self;
        let this_window = window.window_handle().window_id();
        let open = Open::is_open(this_window, &row, cx);
        let (on_press, on_down) = (row.clone(), row.clone());
        let face = SelectTrigger::new("face", ui, value.clone()).width(width).selected(open);
        div()
            // Scoped by its row's id, so one name does for all.
            .id("menu-select")
            .role(Role::ComboBox)
            .aria_label(label)
            .aria_value(value)
            .aria_expanded(open)
            .focusable()
            .tab_stop(true)
            .focus_visible(move |d| d.ringed(ui, ui.focus, ui.focus_ring_width))
            .relative()
            .flex_none()
            .rounded(ui.radius)
            .child(face)
            .on_mouse_down(MouseButton::Left, move |event, _, cx| Open::pressed(&on_down, event.position, cx))
            .on_click(move |event, window, cx| {
                if !Open::is_open(window.window_handle().window_id(), &on_press, cx) {
                    cx.default_global::<Highlight>().row = None;
                }
                Open::clicked(&on_press, event, window, cx);
            })
            .when(open, |d| {
                let menu = options();
                let current = menu.current;
                let at = highlighted(this_window, &row, current, menu.labels.len(), cx);
                let keys = Keys {
                    row: row.clone(),
                    current,
                    pick: menu.on_pick.clone(),
                    hover: menu.on_hover.clone(),
                };
                let count = menu.labels.len();
                d.capture_key_down(move |event, window, cx| {
                    keys.handle(&event.keystroke.key, count, window, cx)
                })
                .child(card(ui, row, width, menu, at, cx))
            })
    }
}

/// The highlighted option of the open menu of `row`: where the keyboard moved it, else the current value.
fn highlighted(window: WindowId, row: &ElementId, current: Option<usize>, count: usize, cx: &App) -> usize {
    let kept = cx
        .try_global::<Highlight>()
        .and_then(|h| h.row.as_ref())
        .filter(|(w, r, _)| *w == window && r == row)
        .map(|(_, _, at)| *at);
    kept.or(current).unwrap_or(0).min(count.saturating_sub(1))
}

/// What the open menu's keys do, the row's own copy of what the menu offers.
struct Keys {
    row: ElementId,
    current: Option<usize>,
    pick: Pick,
    hover: Option<Hover>,
}

impl Keys {
    fn handle(&self, key: &str, count: usize, window: &mut Window, cx: &mut App) {
        let this_window = window.window_handle().window_id();
        let at = highlighted(this_window, &self.row, self.current, count, cx);
        let last = count.saturating_sub(1);
        let moved = match key {
            "down" => Some((at + 1).min(last)),
            "up" => Some(at.saturating_sub(1)),
            "home" => Some(0),
            "end" => Some(last),
            _ => None,
        };
        if let Some(to) = moved {
            cx.stop_propagation();
            cx.default_global::<Highlight>().row = Some((this_window, self.row.clone(), to));
            if let Some(hover) = &self.hover {
                hover(Some(to), window, cx);
            }
            window.refresh();
            return;
        }
        match key {
            // Enter and Space choose the highlighted one; stopping the key keeps the control from
            // taking its release for a click that would open the menu again.
            "enter" | "space" => {
                cx.stop_propagation();
                (self.pick)(at, window, cx);
                self.finish(window, cx);
            }
            "escape" => {
                cx.stop_propagation();
                self.finish(window, cx);
            }
            _ => {}
        }
    }

    fn finish(&self, window: &mut Window, cx: &mut App) {
        if let Some(hover) = &self.hover {
            hover(None, window, cx);
        }
        Open::close(window, cx);
    }
}

/// The menu, under the control's right edge and kept inside the window.
#[track_caller]
fn card(ui: UiColors, row: ElementId, width: Pixels, menu: MenuOptions, at: usize, cx: &App) -> Div {
    let MenuOptions { labels, current, on_pick, on_hover } = menu;
    let dismissed = row;
    let list = div()
        .id("menu-select-card")
        .role(Role::ListBox)
        // Over the page: it takes the pointer, the clicks and the wheel from it.
        .occlude()
        .flex()
        .flex_col()
        .p_1()
        .min_w(width)
        .rounded(ui.radius_lg)
        .bg(menu_bg(ui, cx))
        .shadow(menu_shadow(ui))
        .cursor(CursorStyle::Arrow)
        .on_mouse_down_out(move |event, window, cx| {
            Open::dismissed_outside(&dismissed, event.position, window, cx);
        })
        .children(labels.into_iter().enumerate().map(|(ix, label)| {
            let (pick, hover) = (on_pick.clone(), on_hover.clone());
            let chosen = current == Some(ix);
            option(("option", ix), ui, chosen, false)
                .role(Role::ListBoxOption)
                .aria_label(label.clone())
                .aria_selected(chosen)
                .when(ix == at, |d| {
                    // The option the keyboard is on is the one the control's focus describes.
                    d.bg(ui.hover).aria_active_descendant()
                })
                .child(div().flex_1().min_w_0().truncate().child(label))
                .on_hover(move |hovered, window, cx| {
                    if let Some(hover) = &hover {
                        hover((*hovered).then_some(ix), window, cx);
                    }
                })
                .on_click(move |_, window, cx| {
                    pick(ix, window, cx);
                    Open::close(window, cx);
                })
        }));
    // The holder gives the anchor a corner to hang from: below the control, at its right edge.
    div().absolute().top(px(crate::style::CONTROL_H + GAP)).right_0().child(
        deferred(anchored().anchor(Anchor::TopRight).snap_to_window_with_margin(px(EDGE)).child(list))
            .with_priority(gpui_kit::base::POPUP_PRIORITY),
    )
}

/// devctl `settings-select`: open the menu of the row `row`, as a click on it does.
#[cfg(feature = "devctl")]
pub fn devctl_open(row: &'static str, window: &mut Window, cx: &mut App) {
    cx.default_global::<Highlight>().row = None;
    Open::toggle(&ElementId::from(row), window, cx);
}
