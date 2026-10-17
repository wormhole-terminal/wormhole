//! Renaming a terminal in its row ("Rename…" in its menu): the name becomes a field with the
//! current name selected; ↩ or a click elsewhere keeps what was typed, ⎋ the old name, and an empty
//! name goes back to what runs in the terminal.
use gpui_kit::component::input::Input;

use super::*;

/// A terminal being renamed: which, and its name field.
pub(in crate::workspace) struct TerminalRename {
    pub(super) id: i64,
    pub(in crate::workspace) input: Entity<InputState>,
    _events: Subscription,
}

impl Workspace {
    pub(in crate::workspace) fn start_terminal_rename(
        &mut self,
        id: i64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(ix) = self.find_session(id) else { return };
        let name = self.sessions[ix].name(cx).to_string();
        let input = cx.new(|cx| InputState::new(window, cx).default_value(name));
        let events = cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| match event {
            InputEvent::PressEnter { .. } | InputEvent::Blur => this.finish_terminal_rename(true, window, cx),
            _ => {}
        });
        input.update(cx, |input, cx| {
            input.focus(window, cx);
            input.select_all(window, cx);
        });
        self.sidebar.renaming = Some(TerminalRename { id, input, _events: events });
        cx.notify();
    }

    /// Keep the typed name (`keep`) or the old one, and give the keyboard back to the terminal.
    pub(in crate::workspace) fn finish_terminal_rename(
        &mut self,
        keep: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(renaming) = self.sidebar.renaming.take() else { return };
        if keep && let Some(ix) = self.find_session(renaming.id) {
            let typed = renaming.input.read(cx).value().trim().to_string();
            // Emptied, it is called what runs in it again; left as it was, it keeps no name of its own.
            if typed.is_empty() {
                self.sessions[ix].custom_name = None;
            } else if typed != self.sessions[ix].name(cx).as_ref() {
                self.sessions[ix].custom_name = Some(typed);
            }
            self.mark_dirty(cx);
        }
        window.focus(&self.active_terminal().focus_handle(cx), cx);
        cx.notify();
    }

    /// The field that stands in the name's place on terminal `id`'s row, if it is being renamed.
    pub(super) fn rename_field(&self, id: i64, cx: &mut Context<Self>) -> Option<AnyElement> {
        let renaming = self.sidebar.renaming.as_ref().filter(|renaming| renaming.id == id)?;
        Some(
            div()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" {
                        cx.stop_propagation();
                        this.finish_terminal_rename(false, window, cx);
                    }
                }))
                .child(Input::new(&renaming.input).xsmall())
                .into_any_element(),
        )
    }
}
