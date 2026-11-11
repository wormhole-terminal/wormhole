//! What a cell carries beyond its character, for the rows a frame carries (`FFI-CONTRACT.md`, "Cell
//! extras"): the zero-width characters that make one picture of several (an accent, a variation
//! selector, the joiners of a family emoji, a flag's second letter, a keycap) and an underline colour
//! of its own (SGR 58). Additive to layout version 1, like the placements: a reader that does not know
//! the section reads the rows and stops.
//!
//! After the rows and the placements, when the header's flags carry [`FLAG_EXTRAS`]: a count of records,
//! then each record as little-endian words
//!
//! ```text
//! row, column, underline colour (the frame's packed colour, NONE when the text's own),
//! number of characters n, then n characters (u32 scalar values)
//! ```
//!
//! A changed row with no record has none; the reader forgets what it kept for every row the frame names.
//! A placeholder cell's marks name a tile (`placeholders.rs` turns them into placements), not text.

use wormhole_term::ScreenSnapshot;
use wormhole_term::images::PLACEHOLDER;

use crate::frame::{FLAGS_WORD, pack_color};

pub(crate) const FLAG_EXTRAS: u32 = 4;
/// "No colour of its own" in a record.
const NONE: u32 = u32::MAX;

/// Append the section for the rows `frame` carries (all of them when `all`), if any cell has something.
pub(crate) fn append(frame: &mut Vec<u8>, snapshot: &ScreenSnapshot, all: bool) {
    let start = frame.len();
    frame.extend_from_slice(&0u32.to_le_bytes());
    let mut records = 0u32;
    for (index, row) in snapshot.rows().iter().enumerate() {
        if !(all || row.changed()) {
            continue;
        }
        for (col, extra) in row.extras().cells() {
            let placeholder = row.cells().get(col).is_some_and(|cell| cell.c == PLACEHOLDER);
            let marks: &[char] = if placeholder { &[] } else { extra.zerowidth };
            if marks.is_empty() && extra.underline_color.is_none() {
                continue;
            }
            let underline = extra.underline_color.map_or(NONE, pack_color);
            for word in [index as u32, col as u32, underline, marks.len() as u32] {
                frame.extend_from_slice(&word.to_le_bytes());
            }
            for mark in marks {
                frame.extend_from_slice(&u32::from(*mark).to_le_bytes());
            }
            records += 1;
        }
    }
    if records == 0 {
        frame.truncate(start);
        return;
    }
    frame[start..start + 4].copy_from_slice(&records.to_le_bytes());
    let at = FLAGS_WORD * 4;
    let flags = u32::from_le_bytes([frame[at], frame[at + 1], frame[at + 2], frame[at + 3]]) | FLAG_EXTRAS;
    frame[at..at + 4].copy_from_slice(&flags.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use wormhole_term::ScreenSnapshot;
    use wormhole_term::testing::TestTerm;

    use super::{FLAG_EXTRAS, append};
    use crate::frame::{FLAGS_WORD, pack};

    fn words(bytes: &[u8]) -> Vec<u32> {
        bytes.chunks(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect()
    }

    #[test]
    fn marks_and_underline_colours_ride_after_the_rows() {
        let mut term = TestTerm::new(10, 3, 100);
        // An e with an acute accent, a family emoji (joiners) and text underlined in colour 1 of the palette.
        term.feed("e\u{301} \u{1F468}\u{200D}\u{1F469}\r\n\x1b[4:1m\x1b[58;5;1mu\x1b[0m".as_bytes());
        let mut snapshot = ScreenSnapshot::default();
        term.snapshot(&mut snapshot);
        let mut frame = pack(&snapshot, true);
        let rows_end = frame.len();
        append(&mut frame, &snapshot, true);
        assert_ne!(words(&frame)[FLAGS_WORD] & FLAG_EXTRAS, 0);
        let section = words(&frame[rows_end..]);
        assert_eq!(section[0], 3, "three cells have something");
        // Row 0 column 0: the accent; column 2: the man's joiner and the woman.
        assert_eq!(section[1..6], [0, 0, u32::MAX, 1, 0x301]);
        assert_eq!(section[6..12], [0, 2, u32::MAX, 2, 0x200D, 0x1F469]);
        // Row 1 column 0: an underline colour of its own (indexed 1) and no marks.
        assert_eq!(section[12..], [1, 0, 1 << 24 | 1, 0]);
    }

    #[test]
    fn a_screen_without_any_has_no_section() {
        let mut term = TestTerm::new(10, 3, 100);
        term.feed(b"plain");
        let mut snapshot = ScreenSnapshot::default();
        term.snapshot(&mut snapshot);
        let mut frame = pack(&snapshot, true);
        let before = frame.len();
        append(&mut frame, &snapshot, true);
        assert_eq!(frame.len(), before);
        assert_eq!(words(&frame)[FLAGS_WORD] & FLAG_EXTRAS, 0);
    }
}
