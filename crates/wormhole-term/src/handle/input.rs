//! Input: typed and pasted bytes, key presses and the pointer for a program that asked for it.

use crate::Pty;
use std::borrow::Cow;

use wormhole_vt::{Modes, Scroll};

use crate::{Backend, input};

impl Pty {
    pub fn write(&self, bytes: impl Into<Cow<'static, [u8]>>) {
        if self.is_alive() {
            match &self.io {
                Backend::Local(io) => io.input(bytes.into()),
                Backend::Hosted(host) => host.input(bytes.into()),
            }
        }
    }

    /// Write typed or pasted input; the view returns to the live screen.
    pub fn write_input(&self, bytes: impl Into<Cow<'static, [u8]>>) {
        {
            let mut term = self.term.lock();
            // Scrolling marks the whole screen changed; only do it when it moves.
            if term.display_offset() != 0 {
                term.scroll_display(Scroll::Bottom);
            }
        }
        self.write(bytes);
    }

    /// Send a key press: `encode` turns it into bytes for the current keyboard
    /// modes. When it does, the selection goes and the view returns to the live
    /// screen, as typing does in any terminal. One lock for all of it, since the
    /// PTY thread may hold the lock for a whole read while output streams.
    pub fn send_key(&self, encode: impl FnOnce(Modes) -> Option<Vec<u8>>) -> KeySent {
        let (bytes, view_changed) = {
            let mut term = self.term.lock();
            let Some(bytes) = encode(term.mode()) else { return KeySent::Nothing };
            let had_selection = term.take_selection();
            let scrolled = term.display_offset() != 0;
            if scrolled {
                term.scroll_display(Scroll::Bottom);
            }
            (bytes, had_selection || scrolled)
        };
        self.write(bytes);
        KeySent::Sent { view_changed }
    }

    /// Send what `encode` makes of the terminal's modes as they are now (a mouse report, a paste
    /// of the control line's), without clearing the selection or moving the view: input that is
    /// not the person typing. Whether anything was sent (`encode` may find the mode does not ask
    /// for it). The lock is let go of before the bytes are written.
    pub fn write_encoded(&self, encode: impl FnOnce(Modes) -> Option<Vec<u8>>) -> bool {
        let modes = self.term.lock().mode();
        match encode(modes) {
            Some(bytes) if !bytes.is_empty() => {
                self.write(bytes);
                true
            }
            _ => false,
        }
    }

    /// The pointer for a program that asked for the DEC locator (`Modes::LOCATOR`): a button
    /// went down or up (`Some`) or the pointer moved (`None`). What the program asked to be told
    /// goes to its input.
    pub fn locator(
        &self,
        button: Option<(input::mouse::MouseButton, bool)>,
        at: input::mouse::LocatorPointer,
    ) {
        use input::mouse::MouseButton;
        use wormhole_vt::LocatorButton;
        let button = match button {
            Some((MouseButton::Left, pressed)) => Some((LocatorButton::Left, pressed)),
            Some((MouseButton::Middle, pressed)) => Some((LocatorButton::Middle, pressed)),
            Some((MouseButton::Right, pressed)) => Some((LocatorButton::Right, pressed)),
            Some(_) => return,
            None => None,
        };
        let at = wormhole_vt::Pointer { col: at.col, row: at.row, x: at.x, y: at.y };
        let replies = self.term.lock().locator_event(button, at);
        if !replies.is_empty() && self.is_alive() {
            self.shared.reply(replies);
        }
    }
}

/// What a key press did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeySent {
    /// It means nothing in the current modes.
    Nothing,
    /// Its bytes went to the program; `view_changed` when that also cleared a
    /// selection or scrolled back to the live screen (a redraw is due).
    Sent { view_changed: bool },
}
