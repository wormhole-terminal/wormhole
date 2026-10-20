#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links, rustdoc::private_intra_doc_links, rustdoc::bare_urls)]

// The storage. Public only to our own tools, behind the `internals` feature; what a host needs
// from it is re-exported below, so the layout can change without a major release.
#[cfg(feature = "internals")]
#[doc(hidden)]
pub mod grid;
#[cfg(not(feature = "internals"))]
mod grid;
pub mod guide;
pub mod input;
pub mod parser;
pub mod passthrough;
mod terminal;
mod types;
pub mod unicode;

pub use grid::{
    BlockPart, CopiedLine, DropReason, DroppedHistory, HistoryLine, LinkSpan, Pin, RowMark, TextCopy,
    TextShape, ViewRow,
};
pub use terminal::{
    Builder, Cursor, DEFAULT_WORD_SEPARATORS, Damage, Event, Limits, LocatorButton, Pointer, Policy, Span,
    Terminal,
};
pub use types::{
    COLOR_COUNT, Cell, CellExtra, Color, ColorScheme, Colors, Column, CursorShape, Flags, Hyperlink, Line,
    LineSize, LinkId, Modes, NamedColor, Point, Rgb, RowExtras, Scroll, SelectionRange, SelectionType, Side,
};

/// What a program's sequences ask of the host, and the values that come with them.
pub mod event {
    pub use crate::terminal::{
        CWD_REPORTS_MAX, Event, Progress, ProgressState, PromptKind, ShellMark, Terminator,
    };
}

/// Pictures: kitty's graphics protocol, Sixel and iTerm2's inline images.
///
/// The engine parses them and keeps where each one is placed; the host decodes the pixels it is
/// handed ([`ImageUpload`](image::ImageUpload)) and draws the
/// [`ImagePlacement`](image::ImagePlacement)s.
pub mod image {
    pub use crate::terminal::{
        Compression, IMAGE_BUDGET_BYTES, ImageError, ImageFormat, ImageUpload, MAX_IMAGES, Placeholder,
        VirtualPlacement, placeholder_at, strip_placeholders,
    };
    pub use crate::types::{Extent, ImageHandle, ImagePlacement, PLACEHOLDER, SourceRect};
}

/// XTWINOPS: what a program may ask of the window (when the host's
/// [`Policy::window_ops`](crate::Policy::window_ops) allows it) and what the host tells the engine
/// about it, for the reports.
pub mod window {
    pub use crate::terminal::{Extent, Fullscreen, Maximize, WindowGeometry, WindowOp};
}

pub use terminal::repaint;

/// The whole state as named text, for our tests that compare two terminals.
#[cfg(feature = "internals")]
#[doc(hidden)]
pub use terminal::StateDump;
