//! What each operating system command is: the table that sends a string to the file that reads it.
//!
//! Spec: xterm's ctlseqs ("Operating System Commands": 0, 1, 2 titles, 52 clipboard; the colors
//! are in `colors`), the OSC 8 hyperlink specification, FinalTerm's OSC 133 (shell marks),
//! iTerm2's OSC 9 and 1337, ConEmu's OSC 9;4 progress, rxvt's OSC 777 `notify`, kitty's OSC 99.
//! The ones whose protocol is big and not the terminal's business are handed to the host raw: 99,
//! 7501 (the program status protocol) and what is not an inline image in 1337. A number the host
//! asked for with a `Passthrough` is handed over raw too, ahead of this table.
//!
//! Each command's meaning is in a file of its own: `reports/title` (0, 1, 2), `colors`, `cwd` (7),
//! `link` (8), `notify` (9, 9;4, 99, 777), `clipboard` (52), `marks` (133), `pointer` (22),
//! `uservar` (1337's SetUserVar); this one only sends the string to them.

use crate::terminal::Terminal;
use crate::terminal::event::{Event, Terminator};
use crate::terminal::osc::{USER_VAR_KIND, split_number};

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
            22 => self.osc_pointer_shape(rest),
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

    /// Inline images and user variables are the engine's; the rest of iTerm2's protocol is the
    /// host's, up to `Limits::osc_iterm` bytes (a picture keeps the picture limit).
    fn osc_1337(&mut self, buf: &mut Vec<u8>, skip: usize) {
        if self.iterm_image(&buf[skip..]) {
            return;
        }
        if let Some(pair) = buf[skip..].strip_prefix(USER_VAR_KIND) {
            self.osc_user_var(pair);
        } else if buf.len() <= self.limits.osc_iterm {
            buf.drain(..skip);
            self.push_event(Event::Iterm(std::mem::take(buf)));
        }
    }
}
