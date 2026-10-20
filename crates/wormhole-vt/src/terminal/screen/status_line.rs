//! The status line: DECSSDT (`CSI Ps $ ~`) and DECSASD (`CSI Ps $ }`).
//!
//! Spec: the VT510 manual and xterm's ctlseqs. The VT320 and later have a line below the screen,
//! apart from it: DECSSDT chooses what it is (0 none, 1 the terminal's own indicator, 2 a line the
//! host writes), and DECSASD chooses which display the cursor, text and editing controls act on,
//! the main one (0) or the status line (1). The selection is ignored unless the line is
//! host-writable, and the cursor stays in the status line until the main display is selected again.
//!
//! Decision: the engine has no status line to keep; it **discards what is written to it**. The
//! line is a 25th row of hardware, outside the grid every host reads and draws; a host that wants
//! text of its own beside the screen has its own status bar, and nothing common sends to this one.
//! Keeping the text would mean an API (a getter and an event) that no host asked for and that the
//! stability promise would then cover. What the engine must get right is the other half: while
//! the status line is the active display, nothing a program does *to the display* may touch the
//! main screen: text, cursor movement, erasing, editing, scrolling, tab stops, rendition and the
//! like are dropped (the main cursor and pen stay where the program left them, which is where
//! "selecting the main display moves the cursor back" puts them). Reports, modes, resets and
//! window operations are not about a display and work as always. Pictures are left alone.
//!
//! The VT52 has no status line: in VT52 mode the selection has no effect.

use crate::parser::Csi;

use crate::terminal::Terminal;
use crate::terminal::modes::Modes;

/// DECSSDT's number and whether the status line is the active display, as a program set them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::terminal) struct StatusLine {
    kind: u16,
    active: bool,
}

impl StatusLine {
    /// The number DECSSDT last set: 0 none, 1 indicator, 2 host-writable.
    pub(in crate::terminal) fn kind(&self) -> u16 {
        self.kind
    }

    /// Whether the status line is the active display (DECRQSS answers DECSASD with it).
    pub(in crate::terminal) fn active(&self) -> bool {
        self.active
    }

    /// Back to the main display, keeping the type.
    pub(in crate::terminal) fn deactivate(&mut self) {
        self.active = false;
    }
}

impl Terminal {
    /// DECSSDT: a type other than 0, 1 and 2 is ignored; a line that stops being host-writable
    /// cannot stay the active display.
    pub(in crate::terminal) fn select_status_line_type(&mut self, kind: u16) {
        if kind > 2 {
            return;
        }
        self.status.kind = kind;
        if kind != 2 {
            self.status.active = false;
        }
    }

    /// DECSASD: 1 selects the status line, and only a host-writable one; 0 the main display.
    pub(in crate::terminal) fn select_status_display(&mut self, display: u16) {
        match display {
            0 => self.status.active = false,
            1 if self.status.kind == 2 => self.status.active = true,
            _ => {}
        }
    }

    /// Whether what a program draws now goes to the status line, which has no place here.
    #[inline]
    pub(in crate::terminal) fn on_status_line(&self) -> bool {
        self.status.active && !self.modes.contains(Modes::VT52)
    }
}

/// C0 controls that move the cursor or the text: BS, HT, LF, VT, FF, CR.
pub(in crate::terminal) fn control_acts_on_display(byte: u8) -> bool {
    matches!(byte, 0x08..=0x0D)
}

/// Escape sequences that move the cursor, scroll, set a tab stop, save or restore the cursor, or
/// change a line's size or the whole screen: IND, NEL, RI, HTS, DECSC, DECRC, DECBI, DECFI, DECALN
/// and DECDHL, DECSWL, DECDWL.
pub(in crate::terminal) fn esc_acts_on_display(intermediates: &[u8], byte: u8) -> bool {
    match intermediates {
        [] => matches!(byte, b'D' | b'E' | b'M' | b'H' | b'7' | b'8' | b'6' | b'9'),
        [b'#'] => matches!(byte, b'3'..=b'6' | b'8'),
        _ => false,
    }
}

/// Control sequences that move the cursor or change what the screen shows or the pen: cursor
/// movement and addressing, erasing, inserting, deleting and scrolling (characters, lines, columns,
/// rectangles), tab stops, REP, SGR, protection, the saved cursor and the margins (DECSTBM and
/// DECSLRM home the cursor).
pub(in crate::terminal) fn csi_acts_on_display(csi: &Csi) -> bool {
    let end = csi.final_byte();
    match (csi.marker(), csi.intermediates()) {
        (None, []) => {
            matches!(end, b'@'..=b'M' | b'P' | b'S' | b'T' | b'X' | b'Z' | b'^'..=b'b')
                || matches!(end, b'd' | b'e' | b'f' | b'g' | b'm' | b'r' | b's' | b'u')
        }
        (Some(b'?'), []) => matches!(end, b'J' | b'K'),
        (None, [b'$']) => matches!(end, b'r' | b't' | b'v' | b'x' | b'z' | b'{'),
        (None, [b'\'']) => matches!(end, b'}' | b'~'),
        (None, [b'"']) => end == b'q',
        (None, [b' ']) => matches!(end, b'@' | b'A'),
        (None, [b'#']) => matches!(end, b'{' | b'}'),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use crate::terminal::Terminal;
    use crate::types::Line;

    fn row(term: &Terminal, y: i32) -> String {
        term.row_text(Line(y), true)
    }

    #[test]
    fn text_for_the_status_line_is_not_drawn() {
        let mut term = Terminal::new(30, 3);
        term.feed(b"main\x1b[2$~\x1b[1$}status\r\n\x1b[1;1H\x1b[2K\x1b[1m\x1b[0$}!");
        assert_eq!(row(&term, 0), "main!");
        assert_eq!(row(&term, 1), "");
        assert!(!term.status.active());
    }

    #[test]
    fn the_main_cursor_and_pen_stay_where_they_were() {
        let mut term = Terminal::new(30, 3);
        term.feed(b"\x1b[2;5H\x1b[31m\x1b[2$~\x1b[1$}\x1b[3;9H\x1bM\x1b[44mx\x1b[7m\x1b[0$}");
        let cursor = term.cursor();
        assert_eq!((cursor.point.line.0, cursor.point.column.0), (1, 4));
        term.feed(b"y");
        assert_eq!(row(&term, 1), "    y");
        let cell = term.cell(crate::types::Point::new(Line(1), crate::types::Column(4)));
        let pen = cell.map(|cell| (cell.fg, cell.bg, cell.flags));
        assert_eq!(pen.map(|(fg, ..)| fg), Some(crate::types::Color::Named(crate::types::NamedColor::Red)));
        assert_eq!(
            pen.map(|(_, bg, _)| bg),
            Some(crate::types::Color::Named(crate::types::NamedColor::Background))
        );
    }

    #[test]
    fn the_selection_is_ignored_unless_the_line_is_host_writable() {
        let mut term = Terminal::new(30, 3);
        term.feed(b"\x1b[1$}a");
        assert_eq!(row(&term, 0), "a");
        term.feed(b"\x1b[1$~\x1b[1$}b");
        assert_eq!(row(&term, 0), "ab");
        term.feed(b"\x1b[2$~\x1b[1$}c\x1b[1$~d");
        assert_eq!(row(&term, 0), "abd", "a line that stops being host-writable is not the active display");
    }

    #[test]
    fn reports_and_modes_still_work_in_the_status_line() {
        let mut term = Terminal::new(30, 3);
        term.feed(b"\x1b[2$~\x1b[1$}\x1b[?7l\x1b[5n\x1b[0$}");
        assert_eq!(term.replies(), b"\x1b[0n");
        assert!(!term.modes().contains(crate::terminal::modes::Modes::AUTOWRAP));
    }

    #[test]
    fn decrqss_tells_the_type_and_the_display() {
        let mut term = Terminal::new(30, 3);
        term.feed(b"\x1b[2$~\x1b[1$}\x1bP$q$}\x1b\\\x1bP$q$~\x1b\\");
        assert_eq!(term.replies(), b"\x1bP1$r1$}\x1b\\\x1bP1$r2$~\x1b\\");
    }

    #[test]
    fn a_soft_reset_and_a_full_reset_return_to_the_main_display() {
        let mut term = Terminal::new(30, 3);
        term.feed(b"\x1b[2$~\x1b[1$}\x1b[!pa");
        assert_eq!(row(&term, 0), "a");
        term.feed(b"\x1b[2$~\x1b[1$}");
        term.reset();
        term.feed(b"b");
        assert_eq!(row(&term, 0), "b");
    }
}
