//! Media copy (MC, `CSI Ps i`) is recognised and ignored.
//!
//! Spec: ECMA-48 (MC), the VT510 manual ("MC: Media Copy") and xterm's ctlseqs ("CSI Ps i",
//! "CSI ? Ps i"). On a terminal with a printer, `CSI 5 i` turns the printer controller mode on: the
//! terminal then sends everything it receives to the printer, not the screen, until it reads
//! `CSI 4 i`. `CSI 0 i` prints the screen, `CSI ? 1 i` the cursor line, `CSI ? 5 i` and `CSI ? 4 i`
//! turn auto print on and off.
//!
//! The engine has no printer and **does none of it**: no mode, nothing swallowed, nothing kept,
//! nothing replied (DSR 15 says there is no printer). Decision: swallowing the data until `CSI 4 i`
//! would let a stray `CSI 5 i` (a `cat` of a binary file) silence the terminal for good, since
//! everything after it, a reset included, would be the printer's, and nothing a program sends could
//! bring it back. Modern terminals ignore media copy and keep drawing for the same reason. What a
//! program sent for the printer is shown, which is also what it gets from any terminal without one.
//! There is nothing here to build; the `.vtt` cases (`media-copy.vtt`) pin that it stays so.

#[cfg(test)]
mod tests {
    use crate::terminal::Terminal;
    use crate::types::Line;

    #[test]
    fn media_copy_changes_nothing_and_what_follows_is_drawn() {
        let mut term = Terminal::new(40, 2);
        term.feed(b"a\x1b[5ib\x1b[4ic\x1b[ id\x1b[?5ie\x1b[?1if\x1b[5ig\r\x1bch");
        assert_eq!(term.row_text(Line(0), true), "h");
        assert!(term.replies().is_empty());
        term.feed(b"\x1b[5i");
        for _ in 0..4 {
            term.feed(&vec![b'x'; 1 << 16]);
        }
        term.reset();
        term.feed(b"ok");
        assert_eq!(term.row_text(Line(0), true), "ok");
    }
}
