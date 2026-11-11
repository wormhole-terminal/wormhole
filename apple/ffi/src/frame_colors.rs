//! The colours programs set for themselves (OSC 4 an indexed entry, OSC 10, 11 and 12 the foreground, background and
//! cursor; OSC 104, 110, 111 and 112 back to the theme's), for the renderer to draw with, as the GPUI app resolves
//! every cell, the padding and the cursor against them (`palette.rs` `resolve`). Layout version 2 of the frame: after
//! the fold markers, when the header's flags carry [`FLAG_COLORS`], the whole table of overrides as it is now,
//!
//! ```text
//! count n, then n records of two words: slot (0..255 indexed, 256 foreground, 257 background, 258 cursor),
//! colour (r << 16 | g << 8 | b)
//! ```
//!
//! sent when it changed since the last frame (a count of 0: every colour is the theme's again) and with every frame
//! that carries all the rows while any is set, so a renderer that lost its picture gets them back. A frame without
//! the section leaves the table as it was. Cells keep their unresolved colours: a change rebuilds no row.

use wormhole_term::types::{COLOR_COUNT, Colors};

use crate::frame::FLAGS_WORD;

pub(crate) const FLAG_COLORS: u32 = 32;

/// The overrides the renderer was last sent.
#[derive(Default)]
pub(crate) struct ColorsSent(Colors);

impl ColorsSent {
    /// Append the section to `frame` when `colors` differ from what was sent, or `all` rows go and some are set.
    pub(crate) fn append(&mut self, frame: &mut Vec<u8>, colors: &Colors, all: bool) {
        let set = || (0..COLOR_COUNT).filter_map(|slot| Some((slot, colors.get(slot)?)));
        if self.0 == *colors && !(all && set().next().is_some()) {
            return;
        }
        self.0 = *colors;
        let count = set().count() as u32;
        frame.extend_from_slice(&count.to_le_bytes());
        for (slot, rgb) in set() {
            let packed = u32::from(rgb.r) << 16 | u32::from(rgb.g) << 8 | u32::from(rgb.b);
            frame.extend_from_slice(&(slot as u32).to_le_bytes());
            frame.extend_from_slice(&packed.to_le_bytes());
        }
        let at = FLAGS_WORD * 4;
        let flags =
            u32::from_le_bytes([frame[at], frame[at + 1], frame[at + 2], frame[at + 3]]) | FLAG_COLORS;
        frame[at..at + 4].copy_from_slice(&flags.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use wormhole_term::ScreenSnapshot;
    use wormhole_term::testing::TestTerm;

    use super::*;
    use crate::frame::pack;

    fn word(bytes: &[u8], at: usize) -> u32 {
        u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
    }

    /// The section's records, `None` when the frame has none.
    fn section(
        term: &mut TestTerm,
        snapshot: &mut ScreenSnapshot,
        sent: &mut ColorsSent,
        all: bool,
    ) -> Option<Vec<(u32, u32)>> {
        term.snapshot(snapshot);
        let mut frame = pack(snapshot, all);
        let start = frame.len();
        sent.append(&mut frame, &snapshot.colors, all);
        if word(&frame, FLAGS_WORD * 4) & FLAG_COLORS == 0 {
            return None;
        }
        let count = word(&frame, start) as usize;
        Some((0..count).map(|i| (word(&frame, start + 4 + 8 * i), word(&frame, start + 8 + 8 * i))).collect())
    }

    #[test]
    fn what_a_program_sets_goes_once_and_its_reset_goes_too() {
        let mut term = TestTerm::new(10, 4, 100);
        let (mut snapshot, mut sent) = (ScreenSnapshot::default(), ColorsSent::default());
        assert_eq!(section(&mut term, &mut snapshot, &mut sent, true), None, "nothing set: no section");
        // Background (OSC 11), cursor (OSC 12) and ANSI red (OSC 4;1).
        term.feed(b"\x1b]11;rgb:10/20/30\x07\x1b]12;#ff8000\x07\x1b]4;1;rgb:ff/00/00\x07");
        let table = section(&mut term, &mut snapshot, &mut sent, false).expect("the change is sent");
        assert_eq!(table, [(1, 0xff0000), (257, 0x102030), (258, 0xff8000)]);
        assert_eq!(section(&mut term, &mut snapshot, &mut sent, false), None, "unchanged: not again");
        assert_eq!(
            section(&mut term, &mut snapshot, &mut sent, true).map(|t| t.len()),
            Some(3),
            "a full frame carries it"
        );
        term.feed(b"\x1b]111\x07\x1b]104\x07");
        assert_eq!(section(&mut term, &mut snapshot, &mut sent, false), Some(vec![(258, 0xff8000)]));
        term.feed(b"\x1b]112\x07");
        assert_eq!(
            section(&mut term, &mut snapshot, &mut sent, false),
            Some(vec![]),
            "all back to the theme's"
        );
    }
}
