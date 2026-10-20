//! The value types a host draws and talks with: they are re-exported at the crate root.
//!
//! A cell, its colors and attributes, the positions and ranges of the grid, scrolling and
//! selecting, the terminal modes a host reads and the palette overrides programs set. They name
//! no storage: the grid copies its cells into these (`grid/copy.rs`), so how it keeps them can
//! change without anything that draws noticing.
//!
//! A [`Cell`] is `Copy` and 16 bytes. What is rare on a cell lives beside its row
//! ([`RowExtras`]).

mod cell;
mod color;
mod extras;
mod hyperlink;
mod image;
mod index;
mod line_size;
mod mode;
mod view;

pub use cell::{Cell, Flags};
pub use color::{COLOR_COUNT, Color, ColorScheme, Colors, NamedColor, Rgb};
pub use extras::{CellExtra, RowExtras};
pub use hyperlink::{Hyperlink, LinkId};
pub use image::{Extent, ImageHandle, ImagePlacement, PLACEHOLDER, SourceRect};
pub use index::{Column, Line, Point, Side};
pub use line_size::LineSize;
pub use mode::{KittyFlags, Modes, MouseEncoding, MouseTracking};
pub use view::{CursorShape, Scroll, SelectionRange, SelectionType};
