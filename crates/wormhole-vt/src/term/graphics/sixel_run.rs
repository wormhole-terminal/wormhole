//! Drawing a Sixel picture: decode it, hand the pixels to the host and put it at the cursor.
//!
//! Spec: DEC VT330/VT340 Programmer Reference (chapter 14: DECSDM, "sixel display mode") and
//! xterm's ctlseqs (mode 80, mode 8452). A Sixel starts at the cursor and covers the cells its
//! pixels touch. Where the cursor ends depends on two modes:
//!
//! - DECSDM reset (the default, sixel scrolling on): the screen scrolls to make room, and the
//!   cursor goes to the row under the picture, at the picture's left edge.
//! - Mode 8452 set: the cursor goes right of the picture's last row instead, as kitty's does.
//! - DECSDM set (scrolling off): the picture is drawn at the top left of the screen whatever the
//!   cursor is, is cut at the bottom, and the cursor does not move.

use crate::term::modes::Modes;
use crate::term::{Event, Terminal};

use super::geometry;
use super::put::{Motion, Placing};
use super::sixel;
use super::upload::{Compression, ImageFormat, ImageUpload};

impl Terminal {
    /// The data of a `DCS … q` string, with the numbers before the `q`.
    pub(in crate::term) fn run_sixel(&mut self, params: &[u16], data: &[u8]) {
        let decoded = match sixel::decode(data, params.first().copied().unwrap_or(0)) {
            Ok(decoded) => decoded,
            // Nobody to tell: a program that sends a Sixel expects no answer.
            Err(_) => return,
        };
        let (width, height) = (decoded.width, decoded.height);
        let Ok((handle, _)) = self.add_image(0, 0, width, height, decoded.rgba.len()) else { return };
        self.push_event(Event::ImageData(ImageUpload {
            image: handle,
            format: ImageFormat::Rgba,
            compression: Compression::None,
            width,
            height,
            data: decoded.rgba,
        }));
        // Raw pixels need no decoding that could fail, so nothing waits for the host.
        let cell = self.cell_size();
        let motion = if self.modes.contains(Modes::SIXEL_DISPLAY) {
            Motion::Home
        } else if self.modes.contains(Modes::SIXEL_CURSOR_RIGHT) {
            Motion::Right
        } else {
            Motion::Below
        };
        self.put_image(Placing {
            image: handle,
            image_id: 0,
            placement_id: 0,
            geometry: geometry::native(width, height, cell),
            z: 0,
            motion,
        });
    }
}
