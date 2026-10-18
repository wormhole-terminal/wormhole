//! The terminal state report: DECRQTSR asks for it, DECRSTS puts it back.
//!
//! Spec: VT510 DECRQTSR (`CSI 1 $ u`), DECTSR (`DCS 1 $ s D…D ST`) and DECRSTS (`DCS 1 $ p D…D ST`).
//! The manual leaves what `D…D` holds to the terminal ("software should not expect the format of
//! the terminal state report to be the same for all terminals"), only that restoring a report
//! returns the terminal to the state it was in.
//!
//! Wormhole's report has five parts joined by `!`, each in the format of a report the manual does
//! define:
//!
//! 1. the cursor information (DECCIR's data: position, rendition, protection, flags, character sets),
//! 2. the tab stops (DECTABSR's data),
//! 3. the margins, `top;bottom;left;right`, counted from 1,
//! 4. the ANSI modes that are set, among IRM (4) and LNM (20), separated by `;`,
//! 5. the DEC private modes that are set, among DECCKM (1), DECSCNM (5), DECAWM (7), DECTCEM (25),
//!    DECNRCM (42), reverse wraparound (45), DECNKM (66), DECLRMM (69) and DECNCSM (95).
//!
//! The color table report (DECCTR, `CSI 2 ; Pu $ u`) is a VT525 function and is not answered.

use super::Terminal;
use super::modes::{Status, Switch};
use super::presentation::{CursorInformation, numbers};

/// The ANSI modes a state report carries.
const ANSI_MODES: [u16; 2] = [4, 20];
/// The DEC private modes a state report carries.
const PRIVATE_MODES: [u16; 9] = [1, 5, 7, 25, 42, 45, 66, 69, 95];

impl Terminal {
    /// DECRQTSR.
    pub(super) fn state_report(&mut self, ps: u16) {
        if ps != 1 {
            return;
        }
        self.reply(format_args!("\x1bP1$s"));
        self.write_cursor_information();
        self.reply(format_args!("!"));
        self.write_tab_stops();
        let m = self.margins;
        self.reply(format_args!("!{};{};{};{}!", m.top + 1, m.bottom + 1, m.left + 1, m.right + 1));
        self.write_mode_list(&ANSI_MODES, false);
        self.reply(format_args!("!"));
        self.write_mode_list(&PRIVATE_MODES, true);
        self.reply(format_args!("\x1b\\"));
    }

    fn write_mode_list(&mut self, modes: &[u16], private: bool) {
        let mut first = true;
        for &n in modes {
            let status = if private { self.private_status(n) } else { self.ansi_status(n) };
            if status == Status::Set {
                self.reply(format_args!("{}{n}", if first { "" } else { ";" }));
                first = false;
            }
        }
    }

    /// DECRSTS (`DCS Ps $ p D…D ST`). A report that does not parse changes nothing: every part is
    /// read and checked before any is put back.
    pub(super) fn restore_state(&mut self, ps: u16, data: &[u8]) {
        if ps != 1 {
            return;
        }
        let parts: Vec<&[u8]> = data.split(|&b| b == b'!').collect();
        let [cursor, tabs, margins, ansi, private] = parts[..] else { return };
        let (Some(margins), Some(ansi), Some(private)) =
            (numbers(margins, b';'), numbers(ansi, b';'), numbers(private, b';'))
        else {
            return;
        };
        let [top, bottom, left, right] = margins[..] else { return };
        // As DECSTBM and DECSLRM: a region is at least two lines and two columns.
        if top == 0 || top >= bottom || bottom > self.rows || left == 0 || left >= right || right > self.cols
        {
            return;
        }
        let (Some(tabs), Some(cursor)) = (self.read_tab_stops(tabs), CursorInformation::read(cursor)) else {
            return;
        };
        let switch = |on: bool| if on { Switch::Set } else { Switch::Reset };
        for n in ANSI_MODES {
            self.set_ansi_mode(n, switch(ansi.contains(&usize::from(n))));
        }
        for n in PRIVATE_MODES {
            self.set_private_mode(n, switch(private.contains(&usize::from(n))));
        }
        self.margins.top = top - 1;
        self.margins.bottom = bottom - 1;
        // Without left and right margins mode the columns are the screen's.
        let columns = private.contains(&69);
        self.margins.left = if columns { left - 1 } else { 0 };
        self.margins.right = if columns { right - 1 } else { self.cols - 1 };
        self.set_tab_stops(&tabs);
        self.set_cursor_information(cursor);
    }
}
