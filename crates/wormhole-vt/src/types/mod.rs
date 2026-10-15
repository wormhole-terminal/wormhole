//! The value types the app draws and talks with, whatever engine is behind them.
//!
//! A cell, its colors and attributes, the positions and ranges of the grid, scrolling and
//! selecting, the terminal modes the app reads and the palette overrides programs set. They name
//! no engine: `wormhole-core` converts its engine's cells to these while it copies the screen, and
//! every public API above it speaks only these. Swapping the engine changes the converting code
//! and nothing that draws.
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
