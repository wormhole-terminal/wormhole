//! What each operating system command is, and the text helpers they share.
//!
//! Spec: xterm's ctlseqs ("Operating System Commands": 0, 1, 2 titles, 52 clipboard; the colors
//! are in `osc_colors`), the OSC 8 hyperlink specification, FinalTerm's OSC 133 (shell marks),
//! iTerm2's OSC 9 and 1337, ConEmu's OSC 9;4 progress, rxvt's OSC 777 `notify`, kitty's OSC 99.
//! The ones whose protocol is big and not the terminal's business are handed to the host raw: 99,
//! 7501 and what is not an inline image in 1337.
//!
//! Each command's meaning is in a file of its own: `title` (0, 1, 2), `osc_colors`, `osc_cwd` (7),
//! `osc_link` (8), `osc_notify` (9, 9;4, 99, 777), `osc_clipboard` (52), `osc_marks` (133); this
//! one only sends the string to them.

use super::Terminal;
use super::event::{Event, Terminator};
use super::osc::split_number;

/// The terminator a reply to an OSC ends with: the one the request used.
pub(super) fn terminator(end: Terminator) -> &'static str {
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

pub(super) fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Replace `text`'s contents with `bytes` (invalid UTF-8 as U+FFFD), cut to `max` bytes at a
/// character boundary. Reuses the string's buffer: a title or folder that changes with every
/// prompt costs no allocation.
pub(super) fn fill(text: &mut String, bytes: &[u8], max: usize) {
    text.clear();
    match std::str::from_utf8(bytes) {
        Ok(valid) => text.push_str(valid),
        Err(_) => text.push_str(&String::from_utf8_lossy(bytes)),
    }
    cut(text, max);
}

impl Terminal {
    /// The OSC string has ended.
    pub(super) fn finish_osc(&mut self, end: Terminator) {
        let Some(mut buf) = self.osc.take() else { return };
        if let Some((number, rest)) = split_number(&buf) {
            let skip = buf.len() - rest.len();
            self.run_osc(number, &mut buf, skip, end);
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
            1337 => {
                // Inline images are the engine's; the rest of iTerm2's protocol is the host's.
                if !self.iterm_image(rest) {
                    buf.drain(..skip);
                    self.push_event(Event::Iterm(std::mem::take(buf)));
                }
            }
            7501 => {
                buf.drain(..skip);
                self.push_event(Event::ProgramStatus { payload: std::mem::take(buf), terminator: end });
            }
            _ => {}
        }
    }
}
