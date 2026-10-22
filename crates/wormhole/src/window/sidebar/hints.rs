//! The ⌘1–9 keycaps: ⌘ held on its own for a moment shows them; letting go, any other
//! key, or the window losing focus hides them. Core's `CmdHold` keeps the time; this feeds it
//! the window's modifier and key events and redraws when the answer changes.
use std::time::Instant;

use gpui_kit::*;

use crate::window::Workspace;

impl Workspace {
    /// The modifier keys changed (from the root's `on_modifiers_changed`).
    pub(in crate::window) fn on_modifiers(
        &mut self,
        event: &ModifiersChangedEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let before = self.hints_showing(cx);
        self.sidebar.hold.modifiers(event.modifiers == Modifiers::command(), Instant::now());
        // Ask again when the hold is long enough.
        self.sidebar.hold_timer = self.sidebar.hold.due_in(Instant::now()).map(|due| {
            cx.spawn_in(window, async move |this, cx| {
                cx.background_executor().timer(due).await;
                this.update(cx, |_, cx| cx.notify()).ok();
            })
        });
        if before != self.hints_showing(cx) {
            cx.notify();
        }
    }

    /// A key other than a modifier went down while ⌘ may be held: no hints for this press.
    fn on_key(&mut self, cx: &mut Context<Self>) {
        let before = self.hints_showing(cx);
        self.sidebar.hold.key();
        self.sidebar.hold_timer = None;
        if before {
            cx.notify();
        }
    }

    /// The window lost focus: whatever is held is not for this window.
    pub(in crate::window) fn hints_blur(&mut self, cx: &mut Context<Self>) {
        let before = self.hints_showing(cx);
        self.sidebar.hold.blur();
        self.sidebar.hold_timer = None;
        if before {
            cx.notify();
        }
    }
}

/// Hear every key pressed in `window`, including the ones the terminal takes for itself (⌘C):
/// a keystroke interceptor sees them all before anything handles them.
pub(super) fn watch_keys(window: &mut Window, cx: &mut Context<Workspace>) -> Subscription {
    let handle = window.window_handle();
    let this = cx.weak_entity();
    cx.intercept_keystrokes(move |event, window, cx| {
        if window.window_handle() != handle || is_modifier(&event.keystroke.key) {
            return;
        }
        this.update(cx, |this, cx| this.on_key(cx)).ok();
    })
}

/// The keystroke GPUI reports when a lone modifier is let go ("platform" for ⌘).
fn is_modifier(key: &str) -> bool {
    matches!(key, "platform" | "shift" | "control" | "alt" | "function")
}
