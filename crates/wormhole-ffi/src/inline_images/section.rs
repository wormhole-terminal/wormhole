//! The placements section of a packed frame (`FFI-CONTRACT.md`, "Inline images"): additive to
//! layout version 1, so a reader that does not know it reads the rows and stops.
//!
//! After the rows, when the header's flags carry [`FLAG_PLACEMENTS`]: the header's last word says how
//! many records follow, each [`RECORD_BYTES`] of little-endian words:
//!
//! ```text
//! key (the picture's key for `inline_image`), z (i32), line (i32: grid line, 0 the live screen's
//! top row, negative history), column, cols, rows, source x, y, width, height (pixels of the
//! picture), offset x, y (pixels from the first cell's corner), extent kind (0 fill, 1 fit,
//! 2 pixels), extent width, extent height (pixels, kind 2 only)
//! ```
//!
//! The list is sent when it differs from the one the last frame carried (a scroll that brings one
//! into view, a reflow, a new picture, an erase), and always in a full frame. A frame without the
//! flag says "as before".

use wormhole_term::images::key_of;
use wormhole_term::types::{Extent, ImagePlacement};

use crate::frame::{FLAGS_WORD, IMAGES_WORD};

pub(crate) const FLAG_PLACEMENTS: u32 = 2;
pub(crate) const RECORD_BYTES: usize = 64;

/// What the last frame said about the placements.
#[derive(Default)]
pub(crate) struct PlacementsSent(Vec<ImagePlacement>);

impl PlacementsSent {
    /// Append the section to `frame` (a packed frame) when the list changed or `all` asks for it.
    /// Returns whether a section was written.
    pub(crate) fn append(&mut self, frame: &mut Vec<u8>, placements: &[ImagePlacement], all: bool) -> bool {
        if !all && self.0 == placements {
            return false;
        }
        if frame.len() < crate::frame::HEADER_BYTES {
            return false;
        }
        self.0.clear();
        self.0.extend_from_slice(placements);
        frame.reserve(placements.len() * RECORD_BYTES);
        for placement in placements {
            record(placement, frame);
        }
        let flags = word(frame, FLAGS_WORD) | FLAG_PLACEMENTS;
        set_word(frame, FLAGS_WORD, flags);
        set_word(frame, IMAGES_WORD, placements.len() as u32);
        true
    }
}

fn word(frame: &[u8], index: usize) -> u32 {
    let at = index * 4;
    u32::from_le_bytes([frame[at], frame[at + 1], frame[at + 2], frame[at + 3]])
}

fn set_word(frame: &mut [u8], index: usize, value: u32) {
    frame[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
}

fn record(p: &ImagePlacement, out: &mut Vec<u8>) {
    let (kind, extent_w, extent_h) = match p.extent {
        Extent::Fit => (1, 0, 0),
        Extent::Pixels { width, height } => (2, width, height),
        // `Fill`, and any extent the engine adds later: stretched over the cells.
        _ => (0, 0, 0),
    };
    for value in [
        key_of(p.image),
        p.z as u32,
        p.line.0 as u32,
        p.column as u32,
        p.cols as u32,
        p.rows as u32,
        p.source.x,
        p.source.y,
        p.source.width,
        p.source.height,
        p.offset_x,
        p.offset_y,
        kind,
        extent_w,
        extent_h,
    ] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    // Sixteenth word reserved: records stay a power of two long.
    out.extend_from_slice(&0u32.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use wormhole_term::types::{Extent, ImageHandle, ImagePlacement, Line, SourceRect};

    use super::{FLAG_PLACEMENTS, PlacementsSent, RECORD_BYTES, word};
    use crate::frame::{FLAGS_WORD, HEADER_BYTES, IMAGES_WORD};

    fn placement(line: i32) -> ImagePlacement {
        ImagePlacement {
            image: ImageHandle::from_raw(3),
            image_id: 0,
            placement_id: 0,
            line: Line(line),
            column: 2,
            cols: 4,
            rows: 2,
            source: SourceRect { x: 0, y: 0, width: 16, height: 8 },
            offset_x: 1,
            offset_y: 2,
            extent: Extent::Pixels { width: 30, height: 12 },
            z: -1,
        }
    }

    fn header() -> Vec<u8> {
        vec![0; HEADER_BYTES]
    }

    #[test]
    fn a_changed_list_is_sent_once_and_always_in_a_full_frame() {
        let mut sent = PlacementsSent::default();
        let list = [placement(-1)];
        let mut frame = header();
        assert!(sent.append(&mut frame, &list, false));
        assert_eq!(word(&frame, FLAGS_WORD) & FLAG_PLACEMENTS, FLAG_PLACEMENTS);
        assert_eq!(word(&frame, IMAGES_WORD), 1);
        assert_eq!(frame.len(), HEADER_BYTES + RECORD_BYTES);
        let words: Vec<u32> =
            frame[HEADER_BYTES..].chunks(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect();
        assert_eq!(words[..15], [3, (-1i32) as u32, (-1i32) as u32, 2, 4, 2, 0, 0, 16, 8, 1, 2, 2, 30, 12]);
        // Nothing changed: no section. A full frame repeats it.
        let mut next = header();
        assert!(!sent.append(&mut next, &list, false));
        assert_eq!(next.len(), HEADER_BYTES);
        assert!(sent.append(&mut header(), &list, true));
        // A scroll moved the line: sent again. The list going empty is also a change.
        assert!(sent.append(&mut header(), &[placement(0)], false));
        let mut empty = header();
        assert!(sent.append(&mut empty, &[], false));
        assert_eq!(word(&empty, IMAGES_WORD), 0);
        assert!(!sent.append(&mut header(), &[], false));
    }
}
