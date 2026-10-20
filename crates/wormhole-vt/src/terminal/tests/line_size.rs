//! Double-width and double-height lines as a host sees them: the snapshot, the damage, the history
//! and the dropped lines. What the sequences do is in `tests/conformance/line-size.vtt`.

use crate::grid::ViewRow;
use crate::terminal::tests::Rig;
use crate::terminal::{Damage, Event, Terminal};
use crate::types::{Line, LineSize};

fn sizes(term: &mut Terminal) -> Vec<LineSize> {
    let mut rows: Vec<ViewRow> = Vec::new();
    term.copy_viewport(&mut rows, true);
    rows.iter().map(|row| row.size).collect()
}

#[test]
fn the_snapshot_says_how_big_each_row_is() {
    let mut rig = Rig::new(10, 4);
    rig.feed("\x1b[2;1H\x1b#3\x1b[3;1H\x1b#4\x1b[4;1H\x1b#6");
    assert_eq!(
        sizes(&mut rig.term),
        [LineSize::Single, LineSize::DoubleHeightTop, LineSize::DoubleHeightBottom, LineSize::DoubleWidth]
    );
    assert_eq!(rig.term.line_size(Line(1)), LineSize::DoubleHeightTop);
    assert_eq!(rig.term.line_size(Line(99)), LineSize::Single, "no such line");
}

#[test]
fn changing_a_size_damages_the_row() {
    let mut rig = Rig::new(10, 4);
    let mut damage = Damage::new();
    rig.term.take_damage(&mut damage);
    rig.term.take_damage(&mut damage);
    assert!(damage.is_empty());
    rig.feed("\x1b[3;1H\x1b#6");
    rig.term.take_damage(&mut damage);
    assert_eq!(damage.rows().collect::<Vec<_>>(), [2]);
    // The same size again is no change.
    rig.feed("\x1b#6");
    rig.term.take_damage(&mut damage);
    assert!(!damage.contains(2));
    rig.feed("\x1b#5");
    rig.term.take_damage(&mut damage);
    assert_eq!(damage.rows().collect::<Vec<_>>(), [2]);
}

#[test]
fn a_row_that_scrolls_into_the_history_keeps_its_size() {
    let mut rig = Rig::new(10, 3);
    rig.feed("\x1b#3top\r\n\x1b#4top\r\nplain\r\nmore");
    assert_eq!(rig.term.history_len(), 1);
    assert_eq!(rig.term.line_size(Line(-1)), LineSize::DoubleHeightTop);
    assert_eq!(rig.term.line_size(Line(0)), LineSize::DoubleHeightBottom);
    // The view scrolled back into the history shows the sizes the rows had.
    rig.term.scroll(crate::types::Scroll::Top);
    assert_eq!(
        sizes(&mut rig.term),
        [LineSize::DoubleHeightTop, LineSize::DoubleHeightBottom, LineSize::Single]
    );
}

#[test]
fn the_archive_keeps_the_size() {
    // Narrow rows, so that a few thousand lines make the grid drop its first pages.
    let mut term = Terminal::builder().size(20, 5).scrollback(20).archive_history(true).build();
    term.feed(b"\x1b#6wide\r\n");
    for i in 0..6000 {
        term.feed(format!("{i:05} line\r\n").as_bytes());
    }
    let lines: Vec<_> = term
        .events()
        .filter_map(|event| match event {
            Event::HistoryDropped(batch) => Some(batch),
            _ => None,
        })
        .flat_map(|batch| batch.lines)
        .collect();
    assert_eq!(lines.first().map(|line| line.size), Some(LineSize::DoubleWidth));
    assert!(lines.iter().skip(1).all(|line| line.size == LineSize::Single));
}

#[test]
fn text_is_the_cells_whatever_the_size() {
    let mut rig = Rig::new(10, 3);
    rig.feed("abcdefghij\x1b[1;1H\x1b#6");
    assert_eq!(rig.term.row_text(Line(0), true), "abcdefghij");
}

#[test]
fn a_resize_keeps_the_cells_and_invariants_of_sized_rows() {
    // A wide character on the cut: the crop must not leave half of it.
    let mut rig = Rig::new(10, 4);
    rig.feed("abcd\u{4e16}\u{4e16}\u{4e16}\x1b[1;1H\x1b#6\x1b[3;1Hline\x1b[4;1H\x1b#3");
    for (cols, rows) in [(5, 4), (9, 3), (6, 6), (3, 2), (30, 5), (2, 2), (10, 4)] {
        rig.term.resize(cols, rows);
        rig.term.check_invariants().unwrap_or_else(|why| panic!("{cols}x{rows}: {why}"));
    }
}

#[test]
fn sized_rows_pulled_back_from_the_history_by_a_resize_are_still_sized() {
    let mut rig = Rig::new(10, 3);
    rig.feed("\x1b#6first\r\nsecond\r\nthird\r\nfourth");
    assert_eq!(rig.term.line_size(Line(-1)), LineSize::DoubleWidth);
    rig.term.resize(10, 5);
    assert!(rig.term.grid.has_line_sizes(), "the screen may have sized rows again");
    assert_eq!(sizes(&mut rig.term)[0], LineSize::DoubleWidth);
    // The cursor on it is limited to the half it shows.
    rig.feed("\x1b[1;1H\x1b[99C");
    assert_eq!(rig.cursor(), (0, 4));
}

#[test]
fn a_screen_that_never_used_sizes_is_not_asked() {
    let mut rig = Rig::new(10, 3);
    rig.feed("plain text\r\nmore");
    assert!(!rig.term.grid.has_line_sizes());
    rig.feed("\x1b#6");
    assert!(rig.term.grid.has_line_sizes());
    // Blanking the whole screen clears the hint.
    rig.feed("\x1b[2J");
    assert!(!rig.term.grid.has_line_sizes());
}

#[test]
fn sized_rows_on_the_alternate_screen_stay_there() {
    let mut rig = Rig::new(10, 3);
    rig.feed("\x1b[?1049h\x1b#6alt");
    assert_eq!(rig.term.line_size(Line(0)), LineSize::DoubleWidth);
    rig.feed("\x1b[?1049l");
    assert_eq!(rig.term.line_size(Line(0)), LineSize::Single);
}

#[test]
fn a_hostile_mix_of_sizes_scrolls_and_resizes_keeps_the_invariants() {
    for seed in 1..=8u64 {
        hostile(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15));
    }
}

fn hostile(mut seed: u64) {
    let mut rig = Rig::new(12, 5);
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let pieces: [&[u8]; 16] = [
        b"\x1b#3",
        b"\x1b#4",
        b"\x1b#5",
        b"\x1b#6",
        b"abc",
        "\u{4e16}\u{4e16}".as_bytes(),
        b"\r\n",
        b"\x1bM",
        b"\x1b[L",
        b"\x1b[M",
        b"\x1b[3;4r",
        b"\x1b[r",
        b"\x1b[2J",
        b"\x1b[3@",
        b"\x1b[99C",
        b"\x1b[?7l",
    ];
    for step in 0..3000 {
        let piece = pieces[(next() % pieces.len() as u64) as usize];
        rig.feed_bytes(piece);
        if step % 97 == 0 {
            let (cols, rows) = (2 + (next() % 20) as usize, 1 + (next() % 8) as usize);
            rig.term.resize(cols, rows);
        }
        rig.term.check_invariants().unwrap_or_else(|why| panic!("step {step}: {why}"));
        let point = rig.term.cursor_point();
        let size = rig.term.line_size(point.line);
        assert!(
            point.column.0 < size.visible_columns(rig.term.cols()),
            "step {step}: the cursor is on column {} of a row that shows {} ({size:?})",
            point.column.0,
            size.visible_columns(rig.term.cols())
        );
    }
}
