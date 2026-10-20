//! What each operating system command is, and the text helpers they share.
//!
//! Spec: xterm's ctlseqs ("Operating System Commands": 0, 1, 2 titles, 52 clipboard; the colors
//! are in `osc_colors`), the OSC 8 hyperlink specification, FinalTerm's OSC 133 (shell marks),
//! iTerm2's OSC 9 and 1337, ConEmu's OSC 9;4 progress, rxvt's OSC 777 `notify`, kitty's OSC 99.
//! The ones whose protocol is big and not the terminal's business are handed to the host raw: 99,
//! 7501 (the program status protocol) and what is not an inline image in 1337. A number the host
//! asked for with a `Passthrough` is handed over raw too, ahead of this table.
//!
//! Each command's meaning is in a file of its own: `title` (0, 1, 2), `osc_colors`, `osc_cwd` (7),
//! `osc_link` (8), `osc_notify` (9, 9;4, 99, 777), `osc_clipboard` (52), `osc_marks` (133); this
//! one only sends the string to them.

use crate::terminal::Terminal;
use crate::terminal::event::{Event, Terminator};
use crate::terminal::osc::{ITERM_OTHER, split_number};

/// The terminator a reply to an OSC ends with: the one the request used.
pub(in crate::terminal) fn terminator(end: Terminator) -> &'static str {
    match end {
        Terminator::Bel => "\x07",
        Terminator::St => "\x1b\\",
    }
}

/// Cut `text` to at most `max` bytes, at a character boundary.
fn cut(text: &mut String, max: usize) {
    if text.len() > max {
        let mut at = max;
        while !text.is_char_boundary(at) {
            at -= 1;
        }
        text.truncate(at);
    }
}

pub(in crate::terminal) fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Replace `text`'s contents with `bytes` (invalid UTF-8 as U+FFFD), cut to `max` bytes at a
/// character boundary. Reuses the string's buffer: a title or folder that changes with every
/// prompt costs no allocation.
pub(in crate::terminal) fn fill(text: &mut String, bytes: &[u8], max: usize) {
    text.clear();
    match std::str::from_utf8(bytes) {
        Ok(valid) => text.push_str(valid),
        Err(_) => text.push_str(&String::from_utf8_lossy(bytes)),
    }
    cut(text, max);
}

impl Terminal {
    /// The OSC string has ended.
    pub(in crate::terminal) fn finish_osc(&mut self, end: Terminator) {
        let Some(mut buf) = self.osc.take() else { return };
        if let Some((number, rest)) = split_number(&buf) {
            let skip = buf.len() - rest.len();
            if self.passthrough.osc_caps().iter().any(|claim| claim.0 == number) {
                // The host asked for this number (never one the engine acts on): the payload goes
                // to it whole, after the replies the program was owed.
                buf.drain(..skip);
                let replies_before = self.replies.len();
                let payload = std::mem::take(&mut buf);
                self.push_event(Event::Osc { number, payload, terminator: end, replies_before });
            } else {
                self.run_osc(number, &mut buf, skip, end);
            }
        }
        self.osc.recycle(buf);
    }

    fn run_osc(&mut self, number: u32, buf: &mut Vec<u8>, skip: usize, end: Terminator) {
        let rest = &buf[skip.min(buf.len())..];
        match number {
            0 => self.set_titles(true, true, rest),
            1 => self.set_titles(false, true, rest),
            2 => self.set_titles(true, false, rest),
            4 | 5 => self.osc_palette(number, rest, end),
            7 => self.set_cwd(rest),
            8 => self.osc_hyperlink(rest),
            9 => self.osc_9(rest),
            10..=19 => self.osc_dynamic_colors(number, rest, end),
            52 => self.osc_clipboard(rest, end),
            99 => self.osc_99(rest),
            104 => self.osc_reset_palette(rest),
            105 => self.osc_reset_special(rest),
            110..=119 => self.osc_reset_dynamic(number),
            133 => self.osc_shell_mark(rest),
            777 => self.osc_777(rest),
            1337 => self.osc_1337(buf, skip),
            7501 => {
                buf.drain(..skip);
                self.push_event(Event::ProgramStatus { payload: std::mem::take(buf), terminator: end });
            }
            _ => {}
        }
    }

    /// Inline images are the engine's; the rest of iTerm2's protocol is the host's, up to
    /// [`ITERM_OTHER`] bytes (a picture keeps the picture limit).
    fn osc_1337(&mut self, buf: &mut Vec<u8>, skip: usize) {
        if !self.iterm_image(&buf[skip..]) && buf.len() <= ITERM_OTHER {
            buf.drain(..skip);
            self.push_event(Event::Iterm(std::mem::take(buf)));
        }
    }
}
