//! A menu control: the select's face on a settings row, with a menu that is built only while it is open.
//!
//! A [`super::SearchSelect`] builds a popover and its trigger for every row on every frame, and a page
//! has a dozen such rows. Here a closed control is one focusable element around the select's trigger and
//! one read of a global; the options, their closures and the card exist only for the open one, which a
//! window shows one at a time ([`OpenOne`]).
//!
//! The control is as wide as its value and sits at the right edge of its row; the card hangs from the
//! same edge and is as wide as its longest option (never narrower than the control: the value is one of
//! the options, and the card's check column is wider than the control's chevrons).
//!
//! The keyboard path is the one of the (i): ⇥ reaches the control, ↩ and Space open it, ↑ ↓ Home and End
//! move through the options, ↩ and Space choose, Esc closes and leaves the keyboard on the control. For
//! VoiceOver the control is a pop-up button with its current value, and the options are a list of which
//! the current one is selected; the highlighted option is the one the keyboard is on.

use std::rc::Rc;

use gpui_kit::accesskit::Role;
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::settings::UiColors;
use crate::style::{Edges as _, OpenOne};
use crate::ui::{Handler, Look, MenuFaceProps, MenuItemProps, MenuPanelProps};

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
/// The widest a control grows before its value truncates.
const FACE_MAX_W: f32 = 280.0;
/// The widest a menu grows before its words truncate, and how tall before it scrolls.
const CARD_MAX_W: f32 = 360.0;
const CARD_MAX_H: f32 = 280.0;

/// Which option a keyboard or pointer chose.
type Pick = Rc<dyn Fn(usize, &mut Window, &mut App)>;
/// The option the pointer or the keyboard is on now (`None`: none).
type Hover = Rc<dyn Fn(Option<usize>, &mut Window, &mut App)>;

/// One line of the open menu.
pub struct MenuLine {
    label: SharedString,
    detail: Option<SharedString>,
    icon: Option<AnyElement>,
    action: bool,
}

impl MenuLine {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self { label: label.into(), detail: None, icon: None, action: false }
    }
}

/// What the open menu offers. Built by the caller only when the menu opens.
pub struct MenuOptions {
    pub items: Vec<MenuLine>,
    /// The item that is the current value, checked; `None` when the value is none of them.
    pub current: Option<usize>,
    pub on_pick: Pick,
    /// Told which item is under the pointer or the keyboard, and `None` as it leaves and as the menu
    /// closes however it does (a page that previews each option).
    pub on_hover: Option<Hover>,
}

impl MenuOptions {
    /// Plain words, one per item.
    pub fn words(
        words: impl IntoIterator<Item = SharedString>,
        current: Option<usize>,
        on_pick: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            items: words.into_iter().map(MenuLine::new).collect(),
            current,
            on_pick: Rc::new(on_pick),
            on_hover: None,
        }
    }
}

/// The control of the row `row`: `value` in the select's face, a menu of what `options` builds under it.
#[derive(IntoElement)]
pub struct MenuSelect<B: FnOnce(&mut App) -> MenuOptions + 'static> {
    row: ElementId,
    ui: UiColors,
    /// The row's name, what VoiceOver calls the control.
    label: SharedString,
    value: SharedString,
    leading: Option<AnyElement>,
    options: B,
}

#[track_caller]
pub fn menu_select<B: FnOnce(&mut App) -> MenuOptions + 'static>(
    row: impl Into<ElementId>,
    ui: UiColors,
    label: impl Into<SharedString>,
    value: impl Into<SharedString>,
    options: B,
) -> MenuSelect<B> {
    MenuSelect { row: row.into(), ui, label: label.into(), value: value.into(), leading: None, options }
}

impl<B: FnOnce(&mut App) -> MenuOptions + 'static> MenuSelect<B> {}

impl<B: FnOnce(&mut App) -> MenuOptions + 'static> RenderOnce for MenuSelect<B> {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self { row, ui, label, value, leading, options } = self;
        let this_window = window.window_handle().window_id();
        let open = Open::is_open(this_window, &row, cx);
        let (on_press, on_down) = (row.clone(), row.clone());
        let face = MenuFaceProps { value: value.clone(), lead: leading, open, disabled: false }
            .render(&Look::of(cx));
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
            .flex()
            .flex_none()
            .max_w(px(FACE_MAX_W))
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
                let menu = options(cx);
                let count = menu.items.len();
                let at = highlighted(this_window, &row, menu.current, count, cx);
                if let Some(hover) = menu.on_hover.clone() {
                    // However the menu closes, what it previewed goes with it.
                    Open::on_close(this_window, &row, Rc::new(move |window, cx| hover(None, window, cx)), cx);
                }
                let keys = Keys {
                    row: row.clone(),
                    current: menu.current,
                    pick: menu.on_pick.clone(),
                    hover: menu.on_hover.clone(),
                };
                d.capture_key_down(move |event, window, cx| {
                    keys.handle(&event.keystroke.key, count, window, cx)
                })
                .child(card(ui, row, menu, at, cx))
            })
    }
}

/// The highlighted item of the open menu of `row`: where the keyboard moved it, else the current value.
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
                Open::close(window, cx);
            }
            "escape" => {
                cx.stop_propagation();
                Open::close(window, cx);
            }
            _ => {}
        }
    }
}

/// The menu, under the control's right edge and kept inside the window: the `MenuPanel` component with a
/// `MenuItem` for each option, and the pointer's and keyboard's behaviour added to them.
#[track_caller]
fn card(ui: UiColors, row: ElementId, menu: MenuOptions, at: usize, cx: &App) -> Div {
    let look = Look::of(cx);
    let MenuOptions { items, current, on_pick, on_hover } = menu;
    let mut was_action = false;
    let mut rows = Vec::with_capacity(items.len() + 1);
    for (ix, item) in items.into_iter().enumerate() {
        let MenuLine { label, detail, icon, action } = item;
        if action && !was_action && !rows.is_empty() {
            rows.push(crate::style::popover_rule(ui).into_any_element());
        }
        was_action = action;
        let (pick, hover) = (on_pick.clone(), on_hover.clone());
        let chosen = current == Some(ix) && !action;
        let click: Handler = Rc::new(move |_, window, cx| {
            pick(ix, window, cx);
            Open::close(window, cx);
        });
        rows.push(
            MenuItemProps {
                id: ElementId::from(("option", ix)),
                name: label,
                hint: detail,
                lead: icon,
                chosen,
                active: ix == at,
                muted: action,
                destructive: false,
                bare: false,
                on_click: Some(click),
            }
            .render(&look)
            // The option the keyboard is on is the one the control's focus describes.
            .when(ix == at, |d| d.aria_active_descendant())
            .on_hover(move |hovered, window, cx| {
                if let Some(hover) = &hover {
                    hover((*hovered).then_some(ix), window, cx);
                }
            })
            .into_any_element(),
        );
    }
    let dismissed = row;
    let list = MenuPanelProps {
        id: ElementId::from("menu-select-card"),
        label: SharedString::default(),
        compact: false,
        rows: div().flex().flex_col().children(rows).into_any_element(),
    }
    .render(&look)
    // Over the page: it takes the pointer, the clicks and the wheel from it.
    .occlude()
    .max_w(px(CARD_MAX_W))
    .max_h(px(CARD_MAX_H))
    .overflow_y_scroll()
    .cursor(CursorStyle::Arrow)
    .on_mouse_down_out(move |event, window, cx| {
        Open::dismissed_outside(&dismissed, event.position, window, cx);
    });
    // The holder gives the anchor a corner to hang from: below the control, at its right edge.
    div().absolute().top(px(crate::style::CONTROL_H + GAP)).right_0().child(
        deferred(anchored().anchor(Anchor::TopRight).snap_to_window_with_margin(px(EDGE)).child(list))
            .with_priority(gpui_kit::base::POPUP_PRIORITY),
    )
}
