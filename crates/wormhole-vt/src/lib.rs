//! A terminal engine: bytes from a program in, screen state and events out.
//!
//! No PTY, no threads, no UI and no I/O: the host owns the process and the reader and draws what
//! the engine holds. Every escape sequence a program sends is read here, so nothing parses the
//! output a second time.
//!
//! ```
//! use wormhole_vt::{Line, Terminal};
//!
//! let mut term = Terminal::new(80, 24);
//! term.feed(b"hello \x1b[1mworld\x1b[0m\r\n");
//! assert_eq!(term.row_text(Line(0), true), "hello world");
//! ```
//!
//! # The front door
//!
//! [`Terminal`] (made with [`Terminal::new`] or [`Terminal::builder`]) is the engine: `feed` it a
//! program's output, read the screen back, send what [`replies`](Terminal::replies) holds to the
//! program and handle the [`events`](Terminal::events) (title, bell, clipboard, pictures, …). What
//! a program may do beyond printing is a [`Policy`], all off by default. The types it speaks in
//! (cells, colours, positions, modes) are at the root of the crate.
//!
//! Modules for the groups that need a namespace:
//!
//! - [`parser`]: the DEC/ANSI state machine on its own, for tools that want the sequences and not
//!   a screen;
//! - [`unicode`]: cell widths and grapheme cluster breaks from generated tables;
//! - [`event`]: what a program's sequences ask of the host;
//! - [`image`]: pictures (kitty graphics, Sixel, iTerm2): uploads to decode, placements to draw;
//! - [`window`]: what XTWINOPS lets a program ask of the window, and where the window is;
//! - [`repaint`]: the VT bytes that rebuild one terminal's state in another.
//!
//! # Guarantees
//!
//! Input never panics, and never makes the terminal grow without bound: every string is capped,
//! queues nobody drains are capped, counts are clamped to the screen. Replies are built from the
//! engine's state, never from the request's bytes, and a title is never reported back unless the
//! host's [`Policy`] says so.

#![forbid(unsafe_code)]

// The storage. Public only to our own tools, behind the `internals` feature; what a host needs
// from it is re-exported below, so the layout can change without a major release.
#[cfg(feature = "internals")]
#[doc(hidden)]
pub mod grid;
#[cfg(not(feature = "internals"))]
mod grid;
pub mod parser;
mod terminal;
mod types;
pub mod unicode;

pub use grid::{Content, CopiedLine, LinkSpan, PinId, RowMark, TextCopy, TextShape, ViewRow};
pub use terminal::{
    Builder, Cursor, DEFAULT_WORD_SEPARATORS, Event, LocatorButton, Modes, PinnedSpan, Pointer, Policy,
    Terminal,
};
pub use types::{
    COLOR_COUNT, Cell, CellExtra, Color, Colors, Column, CursorShape, Flags, Hyperlink, Line, NamedColor,
    Point, Rgb, RowExtras, Scroll, SelectionRange, SelectionType, Side, TermMode,
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
