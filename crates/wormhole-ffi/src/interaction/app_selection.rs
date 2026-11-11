//! Pasting over a selection a full-screen program drew itself (`wormhole_term::input::app_selection`): a program that
//! tracks the mouse selects text on its own and copies it with OSC 52 when the button goes up. ⌘V over it should replace
//! it, so the terminal remembers what the program copied and, on a paste, sends a Backspace ahead of it and pastes what
//! the clipboard held before the program's copy. The state is kept here; the view hands in the clipboard.

use std::time::Instant;

use wormhole_term::input::keys::{self, KeyInput};
use wormhole_term::input::mouse;

use crate::terminal::Terminal;

#[uniffi::export]
impl Terminal {
    /// ⌘V: `clipboard` pasted. Over a selection the program drew on the cursor's rows it is replaced (a Backspace
    /// first, and what the clipboard held before the program's copy when it still holds that copy).
    pub fn paste_clipboard(&self, clipboard: Option<String>) {
        let cursor_row =
            self.with_pty(|pty| mouse::wants_mouse(pty.mode()).then(|| pty.cursor_row()).flatten()).flatten();
        let plan = self.interaction.app_selection().paste(clipboard, cursor_row);
        if plan.replace_selection {
            let backspace = KeyInput { key: "backspace", ..KeyInput::default() };
            self.with_pty(|pty| pty.send_key(|mode| keys::encode(&backspace, mode)));
        }
        if let Some(text) = plan.text {
            self.paste(text);
        }
    }

    /// A program copied `text` (OSC 52) and the view put it on the clipboard, which held `clipboard_before`. Right
    /// after the program's own mouse selection that copy belongs to it.
    pub fn program_copied(&self, text: String, clipboard_before: Option<String>) {
        let mut selection = self.interaction.app_selection();
        if selection.copy_from_selection(Instant::now()) {
            selection.copied(text, clipboard_before);
        }
    }
}
