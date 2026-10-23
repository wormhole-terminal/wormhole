//! What the window's shortcuts do beyond calling one method (⇧⌘E, stepping terminals), and
//! Esc, which means something different in each place. The keys themselves are bound from
//! core's keymap table (`actions`).
use gpui_kit::*;

use crate::window::Workspace;

impl Workspace {
    /// ⇧⌘E: from anywhere to the terminal, and from the terminal to the feature that takes the keys
    /// (Notes: the note being worked on, a new one if there is none).
    pub(crate) fn toggle_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let terminal_focus = self.active_terminal().focus_handle(cx);
        if !terminal_focus.is_focused(window) {
            window.focus(&terminal_focus, cx);
            return;
        }
        self.focus_feature(window, cx);
    }

    /// The next (`1`) or previous (`-1`) terminal the sidebar shows, top to bottom, wrapping
    /// around (folded and filtered-out terminals are skipped).
    pub(super) fn step_session(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(ix) = self.session_listing(cx).step(self.active, delta) {
            self.activate(ix, window, cx);
        }
    }

    /// Esc, one layer at a time: a panel drag, then the ladder (`focus`).
    pub(super) fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        if keystroke.key != "escape" {
            return;
        }
        if keystroke.modifiers.modified() && self.resizing.is_none() {
            return;
        }
        if self.escape_key(window, cx) {
            cx.stop_propagation();
        }
    }
}
