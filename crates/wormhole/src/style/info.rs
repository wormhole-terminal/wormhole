//! The (i) after a setting's label: a light, focusable trigger whose card is built only while it is
//! open, and of which a window shows one at a time.
//!
//! A `gpui_kit` `Popover` around a button builds a popover and its trigger for every row on every
//! frame, and a page has dozens of rows. Here a closed (i) is one small element and one read of a
//! global; the card, the highlight and the anchoring exist only for the open one. The
//! open one is named in a global by window and row, so opening another closes this one without
//! either knowing of the other.
//!
//! ↩ and Space open it, as they click any focused element; Esc closes it and the keyboard stays on
//! the (i), and so does moving the keyboard away from it (Tab), which would otherwise leave the card
//! open with no key to close it. A press outside the card closes it, and the press that lands on the
//! (i) itself only closes (see [`Dismissed`]).

use std::ops::Range;

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_text::{keys, text};

use super::buttons::icon;
use super::scale::{Edges as _, TextRole as _};
use crate::settings::UiColors;

/// The (i)'s face, on the 4 pt grid; it is a hit target, so no smaller than the glyph with room.
const FACE: f32 = 24.0;
/// The gap between the face and the card.
const GAP: f32 = 4.0;
/// A paragraph's readable width inside the card.
const CARD_W: f32 = 360.0;
/// How near the card may come to the window's edge.
const EDGE: f32 = 8.0;

/// Which (i) is open, if any: its window and the id of its row, which is unique in the window.
#[derive(Default)]
struct Open {
    shown: Option<(WindowId, ElementId)>,
    dismissed: Option<Dismissed>,
    /// Whether the click that comes of the press now going on is the one that closed the card.
    swallow: bool,
    /// Closes the card when the (i) it hangs from loses the keyboard; dropped with the card.
    focus_out: Option<Subscription>,
}

impl Global for Open {}

/// The press a card closed on, and whose card it was. A card closes on the capture phase of any
/// press outside it, before the (i) under the pointer sees that press; without this the (i) would
/// open the card again at once, and could never be used to close it. The row says which card, so a
/// press on another (i) still opens that one. Every (i) reads this at its own press and settles
/// whether the click that follows is to be swallowed (`Open::swallow`), so a dismissal left over by a
/// press elsewhere, or by a press that never became a click, matches nothing later.
#[derive(Clone, PartialEq)]
struct Dismissed {
    row: ElementId,
    at: Point<Pixels>,
}

impl Open {
    fn is(&self, window: WindowId, id: &ElementId) -> bool {
        self.shown.as_ref().is_some_and(|(open, shown)| *open == window && shown == id)
    }
}

/// Whether a card is open in `window`, for the tests of the pages that show (i)s.
#[cfg(test)]
pub(crate) fn is_open_in(window: WindowId, cx: &App) -> bool {
    open_row_in(window, cx).is_some()
}

/// The row whose card is open in `window`, for the same tests.
#[cfg(test)]
pub(crate) fn open_row_in(window: WindowId, cx: &App) -> Option<ElementId> {
    let open = cx.try_global::<Open>()?;
    open.shown.as_ref().filter(|(open, _)| *open == window).map(|(_, row)| row.clone())
}

// What the last card was built with: its words and the part a search marked.
#[cfg(test)]
thread_local! {
    pub(crate) static LAST_CARD: std::cell::RefCell<Option<(SharedString, Option<Range<usize>>)>> =
        const { std::cell::RefCell::new(None) };
}

/// `text` with `found` marked, when it lies on character boundaries of it.
#[track_caller]
pub(super) fn marked(ui: UiColors, text: SharedString, found: Option<Range<usize>>) -> AnyElement {
    match found {
        Some(found)
            if found.end <= text.len()
                && text.is_char_boundary(found.start)
                && text.is_char_boundary(found.end) =>
        {
            let mark = HighlightStyle { background_color: Some(ui.highlight), ..Default::default() };
            StyledText::new(text).with_highlights([(found, mark)]).into_any_element()
        }
        _ => text.into_any_element(),
    }
}

/// The (i) of the row `row`, whose `description` it opens, with `found` marked in it.
#[derive(IntoElement)]
pub(super) struct SettingInfo {
    row: ElementId,
    ui: UiColors,
    description: SharedString,
    found: Option<Range<usize>>,
}

#[track_caller]
pub(super) fn setting_info(
    row: ElementId,
    ui: UiColors,
    description: SharedString,
    found: Option<Range<usize>>,
) -> SettingInfo {
    SettingInfo { row, ui, description, found }
}

impl RenderOnce for SettingInfo {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self { row, ui, description, found } = self;
        let this_window = window.window_handle().window_id();
        let open = cx.try_global::<Open>().is_some_and(|open| open.is(this_window, &row));
        let on_press = row.clone();
        let on_down = row.clone();
        div()
            // Scoped by its row's id, so one name does for all.
            .id("info")
            .role(Role::Button)
            .aria_label(text(keys::SETTINGS_MORE_INFORMATION))
            .aria_expanded(open)
            .focusable()
            .tab_stop(true)
            .focus_visible(move |d| d.ringed(ui, ui.focus, ui.focus_ring_width))
            .relative()
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .size(px(FACE))
            .rounded(ui.radius_chip)
            .text_color(if open { ui.text } else { ui.faint })
            .cursor_pointer()
            .hover(move |d| d.bg(ui.hover).text_color(ui.text))
            .child(icon(IconName::Info))
            .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                // Settled at every press, so the answer is never an older press's.
                let open = cx.default_global::<Open>();
                let dismissed = open.dismissed.take();
                open.swallow = dismissed.is_some_and(|d| d.row == on_down && d.at == event.position);
            })
            .on_click(move |event, window, cx| {
                // A pointer's click is told apart from the press that closed the card (see
                // [`Dismissed`]); ↩ and Space reach here as a keyboard click.
                if matches!(event, ClickEvent::Mouse(_))
                    && std::mem::take(&mut cx.default_global::<Open>().swallow)
                {
                    return;
                }
                toggle(&on_press, window, cx);
            })
            // Only the open one listens for Esc, which keeps the closed ones lighter.
            .when(open, |d| {
                d.capture_key_down(move |event, window, cx| {
                    if event.keystroke.key == "escape" {
                        cx.stop_propagation();
                        close(window, cx);
                    }
                })
                .child(card(ui, row, description, found))
            })
    }
}

/// Open the card of `row`, or close it when it is the open one.
fn toggle(row: &ElementId, window: &mut Window, cx: &mut App) {
    let this_window = window.window_handle().window_id();
    if cx.default_global::<Open>().is(this_window, row) {
        close(window, cx);
        return;
    }
    // Whatever has the keyboard now is the (i): a press focuses it, as ↩ and Space need it to. When
    // it loses the keyboard its card closes; the check on the row keeps a card that has since moved
    // to another (i) open.
    let focus_out = window.focused(cx).map(|handle| {
        let row = row.clone();
        window.on_focus_out(&handle, cx, move |_, window, cx| {
            if cx.default_global::<Open>().is(window.window_handle().window_id(), &row) {
                close(window, cx);
            }
        })
    });
    let open = cx.default_global::<Open>();
    open.shown = Some((this_window, row.clone()));
    open.focus_out = focus_out;
    window.refresh();
}

fn close(window: &mut Window, cx: &mut App) {
    let open = cx.default_global::<Open>();
    open.shown = None;
    // Dropped after the borrow, not inside it: it may be the very listener that is running.
    let focus_out = open.focus_out.take();
    window.refresh();
    drop(focus_out);
}

/// The card, under the (i) and kept inside the window.
#[track_caller]
fn card(ui: UiColors, row: ElementId, description: SharedString, found: Option<Range<usize>>) -> Div {
    #[cfg(test)]
    LAST_CARD.with(|last| *last.borrow_mut() = Some((description.clone(), found.clone())));
    let words = div()
        .id("info-card")
        // Over the page: it takes the pointer, the clicks and the wheel from it.
        .occlude()
        .p_3()
        .max_w(px(CARD_W))
        .rounded(ui.radius_lg)
        .outlined(ui, ui.border)
        .bg(ui.elevated)
        .text_body()
        .text_color(ui.text)
        .cursor(CursorStyle::Arrow)
        .on_mouse_down_out(move |event, window, cx| {
            cx.default_global::<Open>().dismissed = Some(Dismissed { row: row.clone(), at: event.position });
            close(window, cx);
        })
        .child(marked(ui, description, found));
    // The holder gives the anchor a corner to hang from: below the face, at its left.
    div().absolute().top(px(FACE + GAP)).left_0().child(
        deferred(anchored().snap_to_window_with_margin(px(EDGE)).child(words))
            .with_priority(gpui_kit::base::POPUP_PRIORITY),
    )
}
