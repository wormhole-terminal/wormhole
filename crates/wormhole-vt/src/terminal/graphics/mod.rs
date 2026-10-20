//! Pictures in the terminal: kitty's graphics protocol, Sixel and iTerm2's inline images.
//!
//! The engine reads the three protocols, owns where every picture sits (placements in the grid,
//! see [`crate::grid`]'s `images`) and what a program may ask about them, and leaves the pixels to
//! the host: it has no image decoder and does no I/O. A picture reaches the host as an
//! [`Event::ImageData`](crate::terminal::Event::ImageData) with the bytes still encoded (PNG, a JPEG, zlib
//! raw pixels), the host decodes it and calls [`Terminal::image_decoded`](crate::Terminal::image_decoded), and frees it when
//! it hears [`Event::ImageFree`](crate::terminal::Event::ImageFree). Sixel needs no decoder: the engine
//! turns it into RGBA itself. What to draw, and where, is [`Terminal::placements`](crate::Terminal::placements).
//!
//! # Where each part is
//!
//! | file | what |
//! |------|------|
//! | `apc` | reading one kitty command as it streams in: control data, base64 payload |
//! | `command` | the control keys (`a=T,i=3,c=10,…`) |
//! | `transmit` | storing a picture: chunks, formats, limits, ids and numbers, eviction |
//! | `put` | placing a stored picture at the cursor, moving the cursor, virtual placements |
//! | `delete` | `a=d`, every selector |
//! | `reply` | the answers, and keeping them in the order asked |
//! | `geometry` | cells covered, from pixels, cell size and the keys |
//! | `sniff` | a picture's size from its header |
//! | `sixel`, `sixel_run` | the decoder, and drawing one |
//! | `iterm` | `OSC 1337 ; File=` and its multipart form |
//! | `placeholder` | kitty's Unicode placeholders (`U=1`) |
//! | `images` | the table of what a screen knows |
//!
//! # Choices
//!
//! - **Placements** (spec: kitty, "Display images on screen"): a picture is placed with its top
//!   left cell at the cursor and covers the cells its size needs; the cursor then goes right of
//!   its last row (`C=0`), scrolling the screen as far as the picture needs, or stays (`C=1`),
//!   in which case nothing scrolls and the picture is cut at the bottom. A picture wider than
//!   the rest of the line is cut at the screen's edge, not scaled. Text printed over a picture
//!   leaves it; the erases that cover a cell of it delete it, as does a delete by position.
//! - **Each screen has its own pictures** (kitty: "the alternate screen has its own set"): ids
//!   and numbers mean something on one screen only, and leaving the alternate screen forgets its
//!   pictures.
//! - **Memory**: a command's payload is capped at 32 MiB decoded, a screen keeps at most 1,024
//!   pictures in 128 MiB (as RGBA), the least recently used going first (those nothing is
//!   placed from before those that something is), at most 4,096 placements are kept per
//!   screen, and the uploads waiting for the host are capped at 256 MiB. The host can lower each
//!   of these (`Limits`); lowering the pictures' count or bytes evicts at once.
//! - **Not done**: animation frames (`a=f`, `a=a`, `a=c`), files and shared memory as the medium
//!   (`t=f`, `t=t`, `t=s`: the engine does no I/O, and every one is refused alike with `EBADF`),
//!   relative placements (`P`, `Q`, `H`, `V`), a compressed PNG, Sixel's private palette
//!   (mode 1070 is not read; every picture starts with the VT340's colors) and ReGIS.

mod apc;
mod command;
mod delete;
mod geometry;
mod images;
mod iterm;
mod placeholder;
mod put;
mod reply;
mod sixel;
mod sixel_run;
mod sniff;
#[cfg(test)]
mod tests;
mod transmit;
mod upload;

use crate::types::ImageHandle;

pub use images::{IMAGE_BUDGET_BYTES, MAX_IMAGES};
pub use placeholder::{Placeholder, placeholder_at, strip_placeholders};
pub use upload::{Compression, ImageError, ImageFormat, ImageUpload, VirtualPlacement};

pub(super) use apc::PAYLOAD_MAX;

use apc::ApcReader;
use command::Command;
use images::ImageTable;
use reply::Gate;

/// Most bytes of uploads the host may leave undrained before new pictures are refused.
pub(super) const QUEUED_BYTES_MAX: usize = 256 * 1024 * 1024;

/// A transmission waiting for its next chunk.
#[derive(Debug)]
pub(super) struct Transfer {
    /// The first chunk's keys, which rule the whole transmission.
    first: Command,
    /// The decoded payload so far.
    data: Vec<u8>,
}

/// Everything the terminal keeps for pictures.
#[derive(Debug, Default)]
pub(super) struct Graphics {
    /// The kitty command being read.
    apc: ApcReader,
    transfer: Option<Transfer>,
    /// The pictures of the main and the alternate screen.
    tables: [ImageTable; 2],
    /// The last handle given out.
    last_handle: u64,
    /// Replies held behind an answer that waits for the host.
    gate: Gate,
    /// iTerm2's multipart image under way.
    multipart: Option<iterm::Multipart>,
}

impl Graphics {
    /// See [`Terminal::check_invariants`](crate::terminal::Terminal::check_invariants).
    #[cfg(any(test, feature = "internals"))]
    pub(super) fn check(&self, max_images: usize, max_bytes: u64) -> Result<(), String> {
        for (slot, table) in self.tables.iter().enumerate() {
            table.check(max_images, max_bytes).map_err(|e| format!("pictures of screen {slot}: {e}"))?;
        }
        Ok(())
    }

    /// Everything the pictures hold as text, for the state dump (the transfer under way and the
    /// held replies included: a repaint has to cut at a point where there are none).
    #[cfg(any(test, feature = "internals"))]
    pub(super) fn describe(&self) -> Vec<(&'static str, String)> {
        vec![
            ("a/graphics/last_handle", self.last_handle.to_string()),
            ("a/graphics/transfer", format!("{} {}", self.transfer.is_some(), self.multipart.is_some())),
            ("a/graphics/gate", format!("{:?}", self.gate)),
            ("a/graphics/tables", format!("{:?}", self.tables)),
        ]
    }

    /// Whether a picture is half-received: a chunk of a kitty transmission or of an iTerm2 image
    /// has been read and the rest has not.
    pub(super) fn in_transfer(&self) -> bool {
        self.transfer.is_some() || self.multipart.is_some()
    }

    /// Whether either screen holds a picture.
    pub(super) fn has_images(&self) -> bool {
        self.tables.iter().any(|table| !table.is_empty())
    }

    /// The last handle given out.
    pub(super) fn last_handle(&self) -> u64 {
        self.last_handle
    }

    /// Start numbering handles after `n`.
    pub(super) fn seed_handle(&mut self, n: u64) {
        self.last_handle = n;
    }

    fn next_handle(&mut self) -> ImageHandle {
        self.last_handle += 1;
        ImageHandle(self.last_handle)
    }

    /// Hold a reply behind the answers that wait for the host, if any do; false when none does
    /// and the reply can go out.
    pub(super) fn hold_reply(&mut self, args: std::fmt::Arguments<'_>) -> bool {
        let closed = self.gate.is_closed();
        if closed {
            self.gate.hold(args);
        }
        closed
    }

    /// [`hold_reply`](Self::hold_reply) for bytes that are already made.
    pub(super) fn hold_reply_bytes(&mut self, bytes: &[u8]) -> bool {
        let closed = self.gate.is_closed();
        if closed {
            self.gate.hold_bytes(bytes);
        }
        closed
    }
}
