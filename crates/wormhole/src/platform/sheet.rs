//! The system's own alert as a sheet on a window, for the questions GPUI's prompt can't ask: one with a
//! checkbox (the question before quitting, the one about a group's folder) and one with several buttons and
//! a block of monospace details (a program asking to control Wormhole). The app falls back where there is
//! no such sheet (`dialog`).

use gpui_kit::Window;
use smol::channel::Receiver;

use super::os::sheet as os;

/// How an alert's badge reads: a warning for a question about losing something, none for one that asks
/// about a choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Warning,
    Informational,
}

/// What a sheet asks: the closing button first (Return presses it), then the one that backs out
/// (Esc), and a checkbox.
pub struct Question<'a> {
    pub style: Style,
    pub title: &'a str,
    pub detail: &'a str,
    pub confirm: &'a str,
    /// The button Esc presses ("Cancel", "Not Now").
    pub cancel: &'a str,
    pub checkbox: &'a str,
    /// Whether the box is ticked when the sheet comes up.
    pub checked: bool,
}

/// Show `question` as a sheet on `window`. Its answer arrives on the channel: `None` for the backing-out
/// button, else whether the box was ticked when the closing button was pressed. A sheet that goes away
/// without either (its window closing, the app quitting) sends nothing, and the channel closes.
/// `None` from the call: no sheet can go on this window (off macOS, or a window without a native
/// one).
pub fn ask_with_checkbox(window: &Window, question: &Question) -> Option<Receiver<Option<bool>>> {
    os::ask_with_checkbox(window, question)
}

/// Answer the sheet that is up on `window`, as if its button number `button` (0 is the closing one, in the
/// order they were added) had been pressed: whatever asked reads the answer as it reads a click. A number
/// no button has is the one that backs out (Cancel). `tick` sets the checkbox first, as a click on it would (`None`: as it stands).
/// Where a person is not there to click (a test instance, driven by devctl). `false`: no sheet is up.
#[cfg(feature = "devctl")]
pub fn answer(window: &Window, button: isize, tick: Option<bool>) -> bool {
    os::answer(window, button, tick)
}

/// A question with up to four buttons and details in a block of their own.
pub struct Choice<'a> {
    pub title: &'a str,
    pub detail: &'a str,
    /// Read-only monospace text under the detail, selectable (a path, a signer): data, never interface text.
    pub accessory: Option<&'a str>,
    /// The first is the default button (Return). The answer is a button's place in this list.
    pub buttons: &'a [&'a str],
}

/// Show `choice` as a sheet on `window`. The answer arrives on the channel: the pressed button's place, or
/// `None` when the sheet was ended without one ([`dismiss`], or the window going). `None` from the call:
/// no sheet can go on this window.
pub fn choose(window: &Window, choice: &Choice) -> Option<Receiver<Option<usize>>> {
    os::choose(window, choice)
}

/// End the sheet on `window` without an answer: the question was withdrawn (it timed out). Nothing if
/// there is none.
pub fn dismiss(window: &Window) {
    os::dismiss(window);
}
