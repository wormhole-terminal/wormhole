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
mod image;
mod index;
mod mode;
mod view;

pub use cell::{Cell, Flags};
pub use color::{COLOR_COUNT, Color, Colors, NamedColor, Rgb};
pub use extras::{CellExtra, Hyperlink, RowExtras};
pub use image::{Extent, ImageHandle, ImagePlacement, PLACEHOLDER, SourceRect};
pub use index::{Column, Line, Point, Side};
pub use mode::TermMode;
pub use view::{CursorShape, Scroll, SelectionRange, SelectionType};
