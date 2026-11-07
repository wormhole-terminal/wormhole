//! A frame as bytes: the rows that changed since the last one, in the layout the renderer reads.
//!
//! Everything is little-endian `u32`s, so the Swift side reads it with plain loads and the layout
//! has no padding to agree on:
//!
//! ```text
//! header   HEADER_BYTES (48): magic, version, cols, rows, changed rows, flags,
//!                             cursor row (NONE if hidden), cursor column, cursor style,
//!                             display offset, history size, 1 reserved
//! per changed row, in order:
//!          row index, row flags (`LineSize`), then `cols` cells of CELL_BYTES (16):
//!          char (u32), fg, bg, flags (the engine's `Flags` bits)
//! colour   tag << 24 | value: tag 0 named slot (0..258: the 16 ANSI, then foreground,
//!          background, cursor), 1 indexed (0..255), 2 truecolor (r << 16 | g << 8 | b)
//! ```
//!
//! Colours stay unresolved, as the program set them: the renderer resolves them against its
//! palette, so a theme change rebuilds no row. Zero-width characters, underline colours and
//! hyperlinks (`SnapshotRow::extras`) are not carried yet (the spike draws neither).

use wormhole_term::ScreenSnapshot;
use wormhole_term::types::{Cell, Color, CursorShape, LineSize, NamedColor};

/// The layout's version; bumped by any change to what is above.
pub(crate) const VERSION: u32 = 1;
const MAGIC: u32 = u32::from_le_bytes(*b"WHF1");
pub(crate) const HEADER_BYTES: usize = 48;
pub(crate) const CELL_BYTES: usize = 16;
const ROW_HEADER_BYTES: usize = 8;
/// "No cursor" in the header (hidden, or scrolled out of view).
const NONE: u32 = u32::MAX;

/// The header words the placements section (`inline_images/section.rs`) patches: the flags, and the
/// reserved word, which then carries the placement count.
pub(crate) const FLAGS_WORD: usize = 5;
pub(crate) const IMAGES_WORD: usize = 11;

/// Set in the header's flags: every row is in the frame (the first frame, a resize, a scroll).
const FLAG_FULL: u32 = 1;

/// Pack the rows `snapshot` says changed (all of them when `all` is set).
pub(crate) fn pack(snapshot: &ScreenSnapshot, all: bool) -> Vec<u8> {
    let cols = snapshot.columns();
    let rows = snapshot.rows();
    let sent = |changed: bool| all || changed;
    let changed = rows.iter().filter(|row| sent(row.changed())).count();
    let mut out = Vec::with_capacity(HEADER_BYTES + changed * (ROW_HEADER_BYTES + cols * CELL_BYTES));

    let (cursor_row, cursor_col, style) = match snapshot.cursor {
        Some(cursor) => (cursor.row as u32, cursor.col as u32, cursor_style(cursor)),
        None => (NONE, 0, 0),
    };
    let full = changed == rows.len();
    for word in [
        MAGIC,
        VERSION,
        cols as u32,
        rows.len() as u32,
        changed as u32,
        if full { FLAG_FULL } else { 0 },
        cursor_row,
        cursor_col,
        style,
        snapshot.display_offset as u32,
        snapshot.history_size as u32,
        0,
    ] {
        out.extend_from_slice(&word.to_le_bytes());
    }
    debug_assert_eq!(out.len(), HEADER_BYTES);

    for (index, row) in rows.iter().enumerate() {
        if !sent(row.changed()) {
            continue;
        }
        out.extend_from_slice(&(index as u32).to_le_bytes());
        out.extend_from_slice(&line_size(row.size()).to_le_bytes());
        // A row copied under a wider grid than the snapshot's current one cannot happen (a resize
        // copies every row), but the layout promises `cols` cells: pad rather than trust it.
        let cells = row.cells();
        for cell in cells.iter().take(cols) {
            pack_cell(cell, &mut out);
        }
        for _ in cells.len()..cols {
            pack_cell(&Cell::default(), &mut out);
        }
    }
    out
}

/// Block 0, underline 1, beam 2, hollow block 3 in the low byte; bit 8 a wide character under it;
/// bit 9 blinking.
fn cursor_style(cursor: wormhole_term::SnapshotCursor) -> u32 {
    let shape = match cursor.shape {
        CursorShape::Underline => 1,
        CursorShape::Beam => 2,
        CursorShape::HollowBlock => 3,
        _ => 0,
    };
    shape | u32::from(cursor.wide) << 8 | u32::from(cursor.blinking) << 9
}

/// Normal 0, double-width 1, double-height top half 2, bottom half 3.
fn line_size(size: LineSize) -> u32 {
    match size {
        LineSize::DoubleWidth => 1,
        LineSize::DoubleHeightTop => 2,
        LineSize::DoubleHeightBottom => 3,
        // `Single`, and any size the engine adds later: drawn as a normal row until the reader learns it.
        _ => 0,
    }
}

#[inline]
fn pack_cell(cell: &Cell, out: &mut Vec<u8>) {
    out.extend_from_slice(&u32::from(cell.c).to_le_bytes());
    out.extend_from_slice(&pack_color(cell.fg).to_le_bytes());
    out.extend_from_slice(&pack_color(cell.bg).to_le_bytes());
    out.extend_from_slice(&pack_flags(cell).to_le_bytes());
}

/// The cell's flags as the frame carries them (the engine's bits).
#[inline]
pub(crate) fn pack_flags(cell: &Cell) -> u32 {
    cell.flags.bits()
}

#[inline]
pub(crate) fn pack_color(color: Color) -> u32 {
    match color {
        Color::Named(named) => named_slot(named),
        Color::Indexed(index) => 1 << 24 | u32::from(index),
        Color::Rgb(rgb) => 2 << 24 | u32::from(rgb.r) << 16 | u32::from(rgb.g) << 8 | u32::from(rgb.b),
    }
}

/// The slot of a named colour in the engine's palette (its discriminant).
#[inline]
fn named_slot(named: NamedColor) -> u32 {
    named as u16 as u32
}

#[cfg(test)]
mod tests {
    use wormhole_term::ScreenSnapshot;
    use wormhole_term::testing::TestTerm;

    use super::{CELL_BYTES, HEADER_BYTES, MAGIC, pack};

    fn word(bytes: &[u8], at: usize) -> u32 {
        u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
    }

    #[test]
    fn a_first_frame_is_every_row_and_the_next_only_what_changed() {
        let mut term = TestTerm::new(10, 4, 100);
        term.feed(b"\x1b[38;2;1;2;3mhi\x1b[0m\r\n\x1b[41mx");
        let mut snapshot = ScreenSnapshot::default();
        term.snapshot(&mut snapshot);
        let first = pack(&snapshot, false);
        assert_eq!(word(&first, 0), MAGIC);
        assert_eq!(word(&first, 8), 10, "columns");
        assert_eq!(word(&first, 12), 4, "rows");
        assert_eq!(word(&first, 16), 4, "all rows changed the first time");
        assert_eq!(first.len(), HEADER_BYTES + 4 * (8 + 10 * CELL_BYTES));
        // Row 0, cell 0: 'h' with truecolor foreground 1,2,3 and the default background.
        let cell = HEADER_BYTES + 8;
        assert_eq!(word(&first, cell), u32::from('h'));
        assert_eq!(word(&first, cell + 4), 2 << 24 | 1 << 16 | 2 << 8 | 3);
        assert_eq!(word(&first, cell + 8), 257, "the default background is the named slot 257");
        // Row 1, cell 0: 'x' on ANSI red (named slot 1).
        let cell = HEADER_BYTES + (8 + 10 * CELL_BYTES) + 8;
        assert_eq!(word(&first, cell), u32::from('x'));
        assert_eq!(word(&first, cell + 8), 1);

        term.snapshot(&mut snapshot);
        term.feed(b"y");
        term.snapshot(&mut snapshot);
        let next = pack(&snapshot, false);
        assert_eq!(word(&next, 16), 1, "one row changed");
        assert_eq!(word(&next, HEADER_BYTES), 1, "and it is row 1");
        assert_eq!(pack(&snapshot, true).len(), first.len(), "`all` sends every row");
    }
}
