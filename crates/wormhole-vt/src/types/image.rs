//! Pictures in the grid, as the host draws them.
//!
//! The engine owns where a picture goes (the placements, which scroll, reflow and are erased with
//! the text) and the host owns the pixels: it decodes what an
//! [`Event::ImageData`](crate::terminal::Event::ImageData) carries, keeps it under the picture's
//! [`ImageHandle`], and draws each [`ImagePlacement`] it is given over the cells it names.

use super::index::Line;

/// The placeholder character of kitty's Unicode placeholders (a private-use code point; no font
/// shows it): a cell of a picture a program printed as text. Copies of the text leave it out.
pub const PLACEHOLDER: char = '\u{10EEEE}';

/// The key of one picture's pixels, unique within a terminal for as long as it runs and never
/// reused: a program that sends a new picture under an old kitty id gets a new handle, so a host
/// cache keyed by it can't show stale pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ImageHandle(pub(crate) u64);

impl ImageHandle {
    /// A handle by its number, for a host's tests of what it draws: the engine hands out its own,
    /// counting from 1.
    pub const fn from_raw(raw: u64) -> ImageHandle {
        ImageHandle(raw)
    }

    /// The number behind the handle, for hosts that key a map by an integer.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// A part of the picture, in its pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceRect {
    /// The left edge.
    pub x: u32,
    /// The top edge.
    pub y: u32,
    /// How many pixels wide.
    pub width: u32,
    /// How many pixels tall.
    pub height: u32,
}

/// How large the picture is drawn, inside the cells a placement covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Extent {
    /// Stretched over the covered cells, from the pixel offset on (a program that gave both a
    /// width and a height in cells, as kitty's `c` and `r` do).
    Fill,
    /// Scaled to fit inside the covered cells keeping its shape, centred (iTerm2's
    /// `preserveAspectRatio`).
    Fit,
    /// At this size in pixels from the pixel offset on, scaled from the part of the picture
    /// shown. The covered cells are the ones it touches.
    Pixels {
        /// Drawn width.
        width: u32,
        /// Drawn height.
        height: u32,
    },
}

/// One placement of a picture, as it is now. The cells are `column..column + cols` of the rows
/// from `line` on; only some of them may be on screen (a taller picture has scrolled partly into
/// the history), which the host clips to its view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImagePlacement {
    /// The picture's pixels.
    pub image: ImageHandle,
    /// The id the program gave the picture (kitty's `i`), 0 for none.
    pub image_id: u32,
    /// The id the program gave this placement (kitty's `p`), 0 for none.
    pub placement_id: u32,
    /// The top row. 0 is the live screen's top row, as everywhere; a negative line is history.
    pub line: Line,
    /// The left column.
    pub column: usize,
    /// Cells covered across, already cut to the screen's width.
    pub cols: usize,
    /// Cells covered down.
    pub rows: usize,
    /// The part of the picture shown, already cut where the placement meets the screen's edge.
    pub source: SourceRect,
    /// Pixels from the left of the first cell to the picture's left edge.
    pub offset_x: u32,
    /// Pixels from the top of the first cell to the picture's top edge.
    pub offset_y: u32,
    /// How the part shown is sized.
    pub extent: Extent,
    /// Stacking order: negative is drawn under the text, and the lower of two is drawn first.
    pub z: i32,
}
