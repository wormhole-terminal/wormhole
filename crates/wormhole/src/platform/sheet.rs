//! The system's own alert as a sheet on a window, for the one question GPUI's prompt can't ask:
//! one with a checkbox (the question before quitting). The app falls back to GPUI's prompt where
//! there is no such sheet (`dialog`).

use gpui_kit::Window;
use smol::channel::Receiver;

use super::os::sheet as os;

/// What a sheet asks: the closing button first (Return presses it), then Cancel (Esc), and a
/// checkbox, ticked to begin with.
pub struct Question<'a> {
    pub title: &'a str,
    pub detail: &'a str,
    pub confirm: &'a str,
    pub checkbox: &'a str,
}

/// Show `question` as a sheet on `window`. Its answer arrives on the channel: `None` for Cancel
/// (or the window going), else whether the box was ticked when the closing button was pressed.
/// `None` from the call: no sheet can go on this window (off macOS, or a window without a native
/// one).
pub fn ask_with_checkbox(window: &Window, question: &Question) -> Option<Receiver<Option<bool>>> {
    os::ask_with_checkbox(window, question)
}
