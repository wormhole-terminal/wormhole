//! The presentation state: DECRQPSR asks for it, DECRSPS puts it back.
//!
//! Spec: VT510 DECRQPSR (`CSI Ps $ w`: 1 the cursor information report, 2 the tab stop report),
//! DECCIR (`DCS 1 $ u Pr ; Pc ; Pp ; Srend ; Satt ; Sflag ; Pgl ; Pgr ; Scss ; Sdesig ST`, the
//! information DECSC saves), DECTABSR (`DCS 2 $ u 9/17/25 ST`) and DECRSPS (`DCS Ps $ t D…D ST`,
//! whose data is a report's, so a program can save the state and put it back). DECRQDE and DECRPDE
//! (the displayed extent), DECRQUPSS and DECAUPSS (the user-preferred supplemental set) are here
//! too.
//!
//! The page is always 1, and the window shows all of it: DECRQDE says the screen's size and the
//! window at the page's corner (xterm's reading of "user windows": one page, as big as the window).

use super::Terminal;
use super::charset::{Charset, Preferred};
use super::modes::Modes;
use crate::types::Flags;

/// The most data DECRSPS and DECRSTS read; a string longer than this is dropped whole.
pub(super) const RESTORE_MAX: usize = 32 * 1024;

/// `D…D` split at the character, parsed as numbers: `None` when any piece is not a number (the
/// manual: "if there is an invalid value, the terminal ignores the rest of the sequence").
pub(super) fn numbers(data: &[u8], separator: u8) -> Option<Vec<usize>> {
    if data.is_empty() {
        return Some(Vec::new());
    }
    data.split(|&b| b == separator)
        .map(|piece| {
            let text = std::str::from_utf8(piece).ok()?;
            text.parse::<usize>().ok()
        })
        .collect()
}

impl Terminal {
    /// DECRQPSR.
    pub(super) fn presentation_report(&mut self, ps: u16) {
        match ps {
            1 => {
                self.reply(format_args!("\x1bP1$u"));
                self.write_cursor_information();
                self.reply(format_args!("\x1b\\"));
            }
            2 => {
                self.reply(format_args!("\x1bP2$u"));
                self.write_tab_stops();
                self.reply(format_args!("\x1b\\"));
            }
            _ => {}
        }
    }

    /// The data string of DECCIR.
    pub(super) fn write_cursor_information(&mut self) {
        let (row, col) = (self.cursor.y + 1, self.cursor.x + 1);
        let attrs = self.pen.style.attrs;
        let rendition = 0x40
            | u8::from(attrs.contains(Flags::BOLD))
            | u8::from(attrs.intersects(Flags::ALL_UNDERLINES)) << 1
            | u8::from(attrs.contains(Flags::BLINK)) << 2
            | u8::from(attrs.contains(Flags::INVERSE)) << 3;
        let protection = 0x40 | u8::from(self.pen.protected);
        let single = self.charsets.single();
        let flags = 0x40
            | u8::from(self.modes.contains(Modes::ORIGIN))
            | u8::from(single == Some(2)) << 1
            | u8::from(single == Some(3)) << 2
            | u8::from(self.cursor.pending) << 3;
        let (gl, gr) = self.charsets.invoked();
        let sizes =
            0x40 | (0..4).map(|i| u8::from(self.charsets.slot(i).is_96()) << i).fold(0, |all, bit| all | bit);
        self.reply(format_args!(
            "{row};{col};1;{};{};{};{gl};{gr};{};",
            char::from(rendition),
            char::from(protection),
            char::from(flags),
            char::from(sizes)
        ));
        for i in 0..4 {
            let designator = self.charsets.slot(i).designator();
            self.reply(format_args!("{designator}"));
        }
    }

    /// The data string of DECTABSR: the columns of the tab stops, counted from 1, between slashes.
    pub(super) fn write_tab_stops(&mut self) {
        let columns: Vec<usize> = self.tabs.columns().map(|x| x + 1).collect();
        for (i, column) in columns.iter().enumerate() {
            if i > 0 {
                self.reply(format_args!("/"));
            }
            self.reply(format_args!("{column}"));
        }
    }

    /// DECRSPS (`DCS Ps $ t D…D ST`).
    pub(super) fn restore_presentation(&mut self, ps: u16, data: &[u8]) {
        match ps {
            1 => self.restore_cursor_information(data),
            2 => self.restore_tab_stops(data),
            _ => {}
        }
    }

    /// A tab stop report read back: the stops it lists, and no other.
    pub(super) fn restore_tab_stops(&mut self, data: &[u8]) {
        let Some(columns) = numbers(data, b'/') else { return };
        if columns.iter().any(|&c| c == 0 || c > self.cols) {
            return;
        }
        self.tabs.clear_all();
        for column in columns {
            self.tabs.set(column - 1);
        }
    }

    /// A cursor information report read back. Anything wrong with it and nothing changes.
    pub(super) fn restore_cursor_information(&mut self, data: &[u8]) {
        let fields: Vec<&[u8]> = data.split(|&b| b == b';').collect();
        let [row, col, _page, rendition, protection, flags, gl, gr, sizes, designators] = fields[..] else {
            return;
        };
        let one =
            |field: &[u8]| field.first().copied().filter(|b| (0x40..=0x7f).contains(b)).map(|b| b & 0x3f);
        let slot = |field: &[u8]| match field {
            [d @ b'0'..=b'3'] => Some(d - b'0'),
            _ => None,
        };
        let number = |field: &[u8]| std::str::from_utf8(field).ok()?.parse::<usize>().ok();
        let (Some(row), Some(col)) = (number(row), number(col)) else { return };
        let (Some(rendition), Some(protection), Some(flags), Some(sizes)) =
            (one(rendition), one(protection), one(flags), one(sizes))
        else {
            return;
        };
        let (Some(gl), Some(gr)) = (slot(gl), slot(gr)) else { return };
        let Some(sets) = read_designators(designators, sizes) else { return };

        self.modes.set(Modes::ORIGIN, flags & 1 != 0);
        self.place(col.clamp(1, self.cols) - 1, row.clamp(1, self.rows) - 1);
        // A wrap is only ever due on the last column.
        self.cursor.pending = flags & 8 != 0 && self.cursor.x == self.right_limit();
        let attrs = &mut self.pen.style.attrs;
        attrs.set(Flags::BOLD, rendition & 1 != 0);
        attrs.set(Flags::UNDERLINE, rendition & 2 != 0);
        attrs.set(Flags::BLINK, rendition & 4 != 0);
        attrs.set(Flags::INVERSE, rendition & 8 != 0);
        self.pen.protected = protection & 1 != 0;
        for (i, set) in sets.into_iter().enumerate() {
            self.charsets.designate(i, set);
        }
        self.charsets.invoke(gl);
        self.charsets.invoke_right(gr);
        match (flags & 2 != 0, flags & 4 != 0) {
            (true, _) => self.charsets.single_shift(2),
            (_, true) => self.charsets.single_shift(3),
            _ => self.charsets.clear_single_shift(),
        }
    }

    /// DECRQUPSS (`CSI & u`): the user-preferred supplemental set, as DECAUPSS says it.
    pub(super) fn report_preferred_set(&mut self) {
        let report = self.preferred.report();
        self.reply(format_args!("\x1bP{report}\x1b\\"));
    }

    /// DECAUPSS (`DCS Pn ! u Dscs ST`): choose the user-preferred supplemental set. Only the two
    /// a VT320 has are known: DEC Supplemental Graphic (0, `%5`) and ISO Latin-1 (1, `A`).
    pub(super) fn assign_preferred_set(&mut self, size: u16, name: &[u8]) {
        if let Some(set) = Preferred::from_assignment(size, name) {
            self.preferred = set;
        }
    }

    /// DECRQDE (`CSI " v`): how much of the page shows, as DECRPDE says it.
    pub(super) fn report_displayed_extent(&mut self) {
        let (rows, cols) = (self.rows, self.cols);
        self.reply(format_args!("\x1b[{rows};{cols};1;1;1\"w"));
    }
}

/// The four designators of a DECCIR (`B0%5%5`), with the sizes `Scss` gave, one bit per slot.
fn read_designators(mut text: &[u8], sizes: u8) -> Option<[Charset; 4]> {
    let mut sets = [Charset::Ascii; 4];
    for (i, slot) in sets.iter_mut().enumerate() {
        let wide = sizes >> i & 1 != 0;
        let found = [2usize, 1].into_iter().find_map(|len| {
            let name = text.get(..len)?;
            let set = Charset::ALL
                .into_iter()
                .find(|set| set.designator().as_bytes() == name && set.is_96() == wide)?;
            Some((set, len))
        })?;
        *slot = found.0;
        text = &text[found.1..];
    }
    text.is_empty().then_some(sets)
}
