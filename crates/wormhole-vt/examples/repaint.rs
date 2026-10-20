//! Serialize a terminal with the repaint writer and rebuild it in a fresh one.
//!
//! ```sh
//! cargo run -p wormhole-vt --example repaint
//! ```
//!
//! `Terminal::repaint` writes the escape sequences that, parsed by another terminal of the same
//! size, leave it in the same state: the screens and the history, the colours and attributes of
//! every cell, hyperlinks, modes, tab stops, titles and the rest. It is what a host needs to attach
//! a second view to a running terminal, to resync a client that fell behind, or to checkpoint a
//! session. The program here makes a busy terminal, repaints it into a new one, and checks that
//! the two are the same cell by cell.
#![allow(clippy::print_stdout)] // an example: what it prints is what it shows

use std::error::Error;

use wormhole_vt::repaint::NotReady;
use wormhole_vt::{Cell, Line, RowExtras, Terminal};

pub fn main() -> Result<(), Box<dyn Error>> {
    let original = busy_terminal();

    let mut bytes = Vec::new();
    let painted = original.repaint(&mut bytes)?;
    println!(
        "{} bytes rebuild a {} x {} terminal with {} lines of history; left out: {:?}",
        bytes.len(),
        original.cols(),
        original.rows(),
        painted.history_lines,
        painted.gaps,
    );
    assert!(painted.gaps.is_empty(), "nothing in this terminal is beyond what VT can say");

    let replica = restore(&original)?;
    assert_same(&original, &replica);
    println!("the replica is the same, cell by cell");
    Ok(())
}

/// A terminal with something of everything the repaint has to carry.
pub fn busy_terminal() -> Terminal {
    let mut term = Terminal::builder().size(40, 8).scrollback(100).build();
    term.feed(b"\x1b]2;a title\x07\x1b[?2004h"); // a title, bracketed paste
    term.feed(b"\x1b[1;38;2;255;128;0mbold orange\x1b[0m, \x1b[3;4:3;58;5;9mcurly\x1b[0m\r\n");
    term.feed("wide: \u{4e16}\u{754c}, accents: e\u{301}, flag: \u{1f1e9}\u{1f1ea}\r\n".as_bytes());
    term.feed(b"see \x1b]8;id=x;https://example.org\x07a link\x1b]8;;\x07\r\n");
    term.feed(b"\x1b#6double width\r\n");
    term.feed(b"\x1b[3 q\x1b[2;30r"); // cursor style, scrolling region
    for n in 0..20 {
        term.feed(format!("\x1b[33mline {n}\x1b[m\r\n").as_bytes()); // fills the history
    }
    term.feed(b"\x1b[5;9H"); // the cursor ends somewhere
    term
}

/// A terminal of the same size that the repaint of `original` has rebuilt.
pub fn restore(original: &Terminal) -> Result<Terminal, NotReady> {
    let mut bytes = Vec::new();
    let painted = original.repaint(&mut bytes)?;

    // The receiving side picks its own size and history limit before the bytes arrive.
    let (cols, rows) = original.size();
    let mut replica = Terminal::builder().size(cols, rows).scrollback(100).build();
    replica.seed(&painted.seeds); // the counters no sequence can set
    replica.feed(&bytes);
    Ok(replica)
}

/// Panics unless the two terminals show the same: every cell of the history and the screen with
/// its extras (links, marks, underline colours), the cursor, the modes and the title.
pub fn assert_same(a: &Terminal, b: &Terminal) {
    assert_eq!(a.size(), b.size());
    assert_eq!(a.history_len(), b.history_len());
    assert_eq!(a.cursor(), b.cursor());
    assert_eq!(a.modes(), b.modes());
    assert_eq!(a.title(), b.title());

    let (mut cells_a, mut extras_a) = (Vec::<Cell>::new(), RowExtras::default());
    let (mut cells_b, mut extras_b) = (Vec::<Cell>::new(), RowExtras::default());
    for line in -(a.history_len() as i32)..a.rows() as i32 {
        let line = Line(line);
        let mark_a = a.copy_row(line, &mut cells_a, Some(&mut extras_a));
        let mark_b = b.copy_row(line, &mut cells_b, Some(&mut extras_b));
        assert_eq!(cells_a, cells_b, "the cells of {line:?}");
        assert_eq!(extras_a, extras_b, "the extras of {line:?}");
        assert_eq!(mark_a, mark_b, "the shell mark of {line:?}");
        assert_eq!(a.line_size(line), b.line_size(line), "the size of {line:?}");
    }
}
