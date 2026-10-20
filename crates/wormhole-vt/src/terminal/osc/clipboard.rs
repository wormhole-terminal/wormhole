//! OSC 52: the clipboard, written to or asked for. The host owns the clipboard: a store and a load
//! reach it as events, and its answer comes back through [`Terminal::clipboard_reply`].

use crate::terminal::Terminal;
use crate::terminal::event::{Event, Terminator};
use crate::terminal::osc::base64;
use crate::terminal::osc::dispatch::{lossy, terminator};

/// The selections OSC 52 may name: clipboard, primary, secondary, select and cut buffers 0 to 7.
const CLIPBOARD_TARGETS: &[u8] = b"cpqs01234567";

/// The selection an OSC 52 names, as xterm reads it: only the letters it knows count, and none is
/// the primary one and cut buffer 0 (`s0`). The reply to a read repeats it into the program's
/// input, so whatever else a hostile program put there must not come back.
fn clipboard_selection(bytes: &[u8]) -> String {
    let named: String = bytes
        .iter()
        .filter(|b| CLIPBOARD_TARGETS.contains(b))
        .take(CLIPBOARD_TARGETS.len())
        .map(|&b| char::from(b))
        .collect();
    if named.is_empty() { "s0".to_string() } else { named }
}

impl Terminal {
    /// OSC 52 `selection ; data`: `data` is base64 text to copy, or `?` to ask for the clipboard.
    pub(in crate::terminal) fn osc_clipboard(&mut self, rest: &[u8], end: Terminator) {
        let (selection, data) = match rest.iter().position(|&b| b == b';') {
            Some(at) => (&rest[..at], &rest[at + 1..]),
            None => return,
        };
        let selection = clipboard_selection(selection);
        if data == b"?" {
            let replies_before = self.replies.len();
            self.push_event(Event::ClipboardLoad { selection, terminator: end, replies_before });
        } else if let Some(bytes) = base64::decode(data) {
            let text = String::from_utf8(bytes).unwrap_or_else(|error| lossy(error.as_bytes()));
            self.push_event(Event::ClipboardStore { selection, text });
        }
    }

    /// The reply to an [`Event::ClipboardLoad`]: the clipboard's `text`, for `selection`, ended
    /// as the request was.
    pub fn clipboard_reply(&mut self, selection: &str, text: &str, end: Terminator) {
        let data = base64::encode(text.as_bytes());
        self.reply(format_args!("\x1b]52;{selection};{data}{}", terminator(end)));
    }
}
