//! Input from the UI turned into bytes for the PTY: key presses, mouse reports,
//! pastes and dropped files, and pasting over a selection the program drew.
//!
//! The encodings are `wormhole_vt::input`; `keys` and `mouse` adapt the app's events to it.

use wormhole_vt::Modes;
use wormhole_vt::input::{InputModes, paste as encode_paste};

pub mod app_selection;
pub mod file_drop;
pub mod keys;
pub mod mouse;

/// The bytes for pasting `text`: wrapped in the bracketed-paste markers when the program turned
/// that on (with ESC and the C1 controls removed, so the text cannot end the bracket), and with
/// line breaks as Return in either case.
pub fn paste(text: &str, mode: Modes) -> Vec<u8> {
    let mut out = Vec::new();
    encode_paste(text, &InputModes::from(mode), &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_paste_follows_the_programs_bracketed_paste_mode() {
        assert_eq!(paste("a\r\nb", Modes::empty()), b"a\rb");
        assert_eq!(paste("a\nb\x1b[201~", Modes::BRACKETED_PASTE), b"\x1b[200~a\rb[201~\x1b[201~");
    }
}
