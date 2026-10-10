//! Inline images: kitty's graphics protocol, iTerm2's `OSC 1337 ; File` and Sixel, as pictures in
//! the grid that scroll and reflow with the text.
//!
//! alacritty has no place for images in its cells, so the pictures live beside the grid and the
//! grid says where they go, the way kitty's own Unicode placeholders do: `pty_filter::images`
//! lifts each image sequence out of the PTY's bytes before either parser sees it, and prints in
//! its place a rectangle of placeholder cells (`U+10EEEE`, the image id in the foreground color,
//! the placement id in the underline color, the row and column in combining marks). Those cells
//! scroll, reflow, drop with the history and go when a row is overwritten, like any text, and the
//! view draws the image's tile at every one it meets (`placeholder`). The pixels themselves are in
//! a per-terminal [`ImageStore`] under a global byte budget, decoded off the PTY thread.
//!
//! Everything here is hostile input: a program may send a 2 GB image, ten thousand of them, a
//! Sixel that never ends or a PNG that decompresses to a terabyte. Payloads, dimensions, pixel
//! counts and the store are capped, decoding runs under those limits and stops on the first
//! problem, and a failure drops that one image and logs it; the PTY thread never waits on it.

pub(super) mod decode;
pub mod iterm;
pub mod kitty;
mod placeholder;
pub mod sixel;
mod store;

pub use decode::{DecodeError, Decoded, Format, Limits, decode, dimensions};
pub use placeholder::{PLACEHOLDER, Placeholder, placeholder_at, placeholder_bytes, strip_placeholders};
pub use store::{GLOBAL_BUDGET, Image, ImageStore, PlacementSpec, STORE_BUDGET, global_bytes};

/// How many images may be decoding at once per terminal; more are dropped (a program flooding
/// the terminal with pictures gets the first few).
pub const DECODES_MAX: usize = 4;

/// Most an image may be on either side, in pixels (kitty's own limit).
pub const MAX_DIMENSION: u32 = 10_000;
/// Most pixels one image may have: 64 MiB of RGBA (a 4K screenshot is 8.3 million).
pub const MAX_PIXELS: u64 = 16 * 1024 * 1024;
/// Most one image's encoded data may be (after base64), whatever the format says.
pub const MAX_PAYLOAD_BYTES: usize = 32 * 1024 * 1024;
/// Most one Sixel stream may be; it is decoded on the PTY thread, so it stays small.
pub const MAX_SIXEL_BYTES: usize = 4 * 1024 * 1024;
/// How many images a terminal keeps at most, whatever their size.
pub const MAX_IMAGES: usize = 1024;
/// Most rows or columns a placement may take: what the placeholders' marks can count.
pub const MAX_CELLS: u32 = placeholder::DIACRITICS_LEN as u32;
