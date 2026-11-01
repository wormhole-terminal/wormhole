//! Which one popup of a kind is open in a window, and how a press closes it.
//!
//! The (i)'s card and a menu control's menu are built only while open, and a window shows one of
//! each at a time. Both need the same small machine, so it is here once: the open one is named in a
//! global by window and by the id of its row, so opening another closes this one without either
//! knowing of the other. The kind `K` (a marker type) gives each its own global.
//!
//! A popup closes on the capture phase of any press outside it, before the trigger under the pointer
//! sees that press; without [`OpenOne::dismissed_outside`] the trigger would open it again at once and
//! could never be used to close it. The row says which popup, so a press on another trigger still opens
//! that one. Every trigger reads this at its own press ([`OpenOne::pressed`]) and settles whether the
//! click that follows is to be swallowed, so a dismissal left over by a press elsewhere, or by a press
//! that never became a click, matches nothing later.
//!
//! Tabbing away from the trigger closes its popup too (it would otherwise stay open with no key to
//! close it): [`OpenOne::toggle`] watches the focus the press or key gave the trigger.

use std::marker::PhantomData;

use gpui_kit::*;

/// The open popup of kind `K`, if any.
pub(crate) struct OpenOne<K: 'static> {
    shown: Option<(WindowId, ElementId)>,
    dismissed: Option<Dismissed>,
    /// Whether the click that comes of the press now going on is the one that closed the popup.
    swallow: bool,
    /// Closes the popup when the trigger it hangs from loses the keyboard; dropped with the popup.
    focus_out: Option<Subscription>,
    kind: PhantomData<fn() -> K>,
}

impl<K: 'static> Default for OpenOne<K> {
    fn default() -> Self {
        Self { shown: None, dismissed: None, swallow: false, focus_out: None, kind: PhantomData }
    }
}

impl<K: 'static> Global for OpenOne<K> {}

/// The press a popup closed on, and whose popup it was.
#[derive(Clone, PartialEq)]
struct Dismissed {
    row: ElementId,
    at: Point<Pixels>,
}

impl<K: 'static> OpenOne<K> {
    /// Whether the popup of `row` is the open one in `window`.
    pub(crate) fn is_open(window: WindowId, row: &ElementId, cx: &App) -> bool {
        Self::row_in(window, cx).as_ref() == Some(row)
    }

    /// The row whose popup is open in `window`.
    pub(crate) fn row_in(window: WindowId, cx: &App) -> Option<ElementId> {
        let open = cx.try_global::<Self>()?;
        open.shown.as_ref().filter(|(open, _)| *open == window).map(|(_, row)| row.clone())
    }

    /// The trigger of `row` is pressed at `at`: settles whether the click after it is to be swallowed.
    pub(crate) fn pressed(row: &ElementId, at: Point<Pixels>, cx: &mut App) {
        let open = cx.default_global::<Self>();
        let dismissed = open.dismissed.take();
        open.swallow = dismissed.is_some_and(|d| d.row == *row && d.at == at);
    }

    /// The trigger of `row` is clicked, by the pointer or as ↩ and Space click a focused element.
    pub(crate) fn clicked(row: &ElementId, event: &ClickEvent, window: &mut Window, cx: &mut App) {
        // A pointer's click is told apart from the press that closed the popup.
        if matches!(event, ClickEvent::Mouse(_)) && std::mem::take(&mut cx.default_global::<Self>().swallow) {
            return;
        }
        Self::toggle(row, window, cx);
    }

    /// A press at `at` outside the popup of `row`: it closes, and the trigger under that press is told
    /// by [`Self::pressed`] that the click which follows is the one that closed it.
    pub(crate) fn dismissed_outside(row: &ElementId, at: Point<Pixels>, window: &mut Window, cx: &mut App) {
        cx.default_global::<Self>().dismissed = Some(Dismissed { row: row.clone(), at });
        Self::close(window, cx);
    }

    /// Open the popup of `row`, or close it when it is the open one.
    pub(crate) fn toggle(row: &ElementId, window: &mut Window, cx: &mut App) {
        let this_window = window.window_handle().window_id();
        if Self::is_open(this_window, row, cx) {
            Self::close(window, cx);
            return;
        }
        // Whatever has the keyboard now is the trigger: a press focuses it, as ↩ and Space need it to.
        // When it loses the keyboard its popup closes; the check on the row keeps a popup that has since
        // moved to another trigger open.
        let focus_out = window.focused(cx).map(|handle| {
            let row = row.clone();
            window.on_focus_out(&handle, cx, move |_, window, cx| {
                if Self::is_open(window.window_handle().window_id(), &row, cx) {
                    Self::close(window, cx);
                }
            })
        });
        let open = cx.default_global::<Self>();
        open.shown = Some((this_window, row.clone()));
        open.focus_out = focus_out;
        window.refresh();
    }

    /// Close the open popup, if it is in `window`'s kind of global at all.
    pub(crate) fn close(window: &mut Window, cx: &mut App) {
        let open = cx.default_global::<Self>();
        open.shown = None;
        // Dropped after the borrow, not inside it: it may be the very listener that is running.
        let focus_out = open.focus_out.take();
        window.refresh();
        drop(focus_out);
    }
}
