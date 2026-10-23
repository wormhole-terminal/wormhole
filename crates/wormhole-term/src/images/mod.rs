//! Inline images: the pixels of the pictures the engine places in the grid (kitty's graphics
//! protocol, iTerm2's `OSC 1337 ; File` and Sixel).
//!
//! The engine (`wormhole_vt`) parses the sequences and owns where a picture is: a placement in
//! the grid, which scrolls and reflows with the text, or kitty's Unicode placeholder cells
//! (`U=1`), whose picture the cells name. It has no decoder, so the host decodes what it is sent
//! (`engine::images`) and keeps the pixels in a per-terminal [`ImageStore`] under a global byte
//! budget, decoded off the PTY thread.
//!
//! Everything here is hostile input: a program may send a 2 GB image, ten thousand of them or a
//! PNG that decompresses to a terabyte. Dimensions, pixel counts and the store are capped,
//! decoding runs under those limits and stops on the first problem, and a failure drops that one
//! image and logs it; the PTY thread never waits on it.

pub(super) mod decode;
mod store;

pub use decode::{DecodeError, Decoded, Format, Limits, decode, dimensions};
pub use store::{GLOBAL_BUDGET, Image, ImageStore, STORE_BUDGET, global_bytes};
pub use wormhole_vt::image::{PLACEHOLDER, Placeholder, placeholder_at, strip_placeholders};

/// Where a placeholder cell's picture is in the store, and how many tiles it spans (kitty's
/// virtual placements; see `Pty::virtual_placement`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tiles {
    /// The picture's key in the [`ImageStore`].
    pub key: u32,
    /// Tiles across and down.
    pub cols: u32,
    pub rows: u32,
}

/// The key of a picture's pixels in the [`ImageStore`]: its handle, which counts up from 1 and is never
/// reused. The store's keys are 32 bits; a terminal that sent four billion pictures would have
/// had the first evicted long ago.
pub fn key_of(handle: wormhole_vt::image::ImageHandle) -> u32 {
    handle.get() as u32
}

/// How many images may be decoding at once per terminal; more are dropped (a program flooding
/// the terminal with pictures gets the first few).
pub const DECODES_MAX: usize = 4;

/// Most an image may be on either side, in pixels (kitty's own limit).
pub const MAX_DIMENSION: u32 = 10_000;
/// Most pixels one image may have: 64 MiB of RGBA (a 4K screenshot is 8.3 million).
pub const MAX_PIXELS: u64 = 16 * 1024 * 1024;
/// How many images a terminal keeps at most, whatever their size.
pub const MAX_IMAGES: usize = 1024;
