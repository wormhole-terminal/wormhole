//! The terminal layer: a program in a PTY and the screen it draws. Everything above this crate talks
//! to [`Pty`] and to [`types`] (`wormhole_vt::types`: cells, colors, positions, modes); nothing
//! above it names the engine's internals. It knows neither the UI nor the rest of the app (its
//! settings, its sockets, its database): the program, its folder and its environment come in as a
//! [`Launch`]; the app uses it directly, and core's `terminal` starts its programs.
//!
//! The engine is Wormhole's own (`wormhole_vt`). The PTY layer is this crate's own too: it opens the
//! pseudo-terminal, starts the program and runs the thread that reads it, which parses into the
//! screen it is given. The engine-specific files are:
//!
//! - `engine/`: the host side of the engine on the PTY thread: its parser and events (a title, a
//!   bell, a folder, a prompt mark, a notification, a picture) as what the UI hears, and the
//!   pictures' pixels;
//! - `screen`: the engine's terminal as the rest of the crate reads and drives it, in `wormhole_vt::types`;
//! - `osc`: what the sequences the engine hands over raw mean to Wormhole (the shell's marks,
//!   notifications, progress), cleaned and capped;
//! - `testing`: terminals for tests, in this crate's and the app's (feature `test-support`).
//!
//! The rest:
//!
//! - `pty`: the pseudo-terminal, the program on it and the reader thread (`open`, `spawn`,
//!   `reader`, `hangup`), and the fair lock the thread and the UI share the screen under;
//! - `sys`: the OS edge besides the PTY: users, sockets and peers, file modes, a process's folder, the
//!   machine's name (`procinfo` keeps the check that an OSC 7 host is this machine);
//! - `agents`: the coding agents' registry and which one runs in the foreground of a terminal;
//! - `ports`: the TCP ports the programs under the shell listen on;
//! - `events`: the bridge from the PTY thread to the UI;
//! - `hosted`: a terminal run by a session host (`wormhole-host`): the stream its clients read,
//!   the attach repaint, its meta file and socket path;
//! - `client`: the app's side of a hosted terminal: starting a host, and the connection that
//!   keeps a [`Pty`]'s screen a replica of the host's;
//! - `handle`: the methods of [`Pty`], one file for each concern; `options`: the sizes and settings a
//!   terminal starts with;
//! - `restore`: replaying a previous session's text;
//! - `snapshot`: what the UI draws, copied under a short lock;
//! - `minimap`: the history drawn small, and where a click on it scrolls to;
//! - `scrollbar`: the scrollbar's thumb, what clicks and drags on it do, and when it shows and fades;
//! - `contents`: the screen and scrollback as plain text;
//! - `find`: find in scrollback, on a copy of the grid's text;
//! - `program_status`: what programs report about themselves (OSC 7501), and the records kept;
//! - `row_mark`: what a row of the grid carries of a command block; `marks`: where a block's parts are,
//!   read back from the grid;
//! - `blocks`: each prompt with its command and output, and how the command went;
//! - `input`: keys, mouse and dropped files → bytes for the PTY;
//! - `links`: OSC 8 hyperlinks and URLs in the text, and which may be opened;
//! - `glyphs`: geometry of the characters the UI paints as shapes.

#![cfg_attr(test, allow(clippy::disallowed_methods, reason = "tests build their own files and processes"))]

pub mod agents;
mod blocks;
pub mod client;
mod command_to_prompt;
mod contents;
mod engine;
mod events;
pub mod find;
pub mod glyphs;
mod handle;
pub mod hosted;
pub mod images;
pub mod input;
mod links;
mod marks;
pub mod minimap;
mod options;
mod osc;
pub mod perf;
mod ports;
mod procinfo;
pub mod program_status;
mod pty;
mod restore;
mod row_mark;
mod screen;
pub mod scrollbar;
pub mod sequences;
mod snapshot;
pub mod sys;
#[cfg(any(test, feature = "test-support"))]
pub mod testing;

/// The value types the screen is drawn and driven with: cells, colors, positions, modes.
pub mod types {
    pub use wormhole_vt::image::{Extent, ImageHandle, ImagePlacement, PLACEHOLDER, SourceRect};
    pub use wormhole_vt::input::{MouseEncoding, MouseTracking};
    pub use wormhole_vt::{
        COLOR_COUNT, Cell, CellExtra, Color, ColorScheme, Colors, Column, CursorShape, Flags, Hyperlink,
        Line, LineSize, Modes, NamedColor, Point, Rgb, RowExtras, Scroll, SelectionRange, SelectionType,
        Side,
    };
}

/// What a program may ask of the window and what the engine needs to know of it to answer the
/// reports (XTWINOPS); see `allow_window_ops`.
pub use wormhole_vt::window::{Extent, Fullscreen, Maximize, WindowGeometry, WindowOp};

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub use blocks::{Block, BlockState, Ended, readout_text, took_text};
pub use command_to_prompt::{Refusal, command_to_prompt};
pub use contents::prompt_in;
use events::Shared;
pub use events::{PtyEvent, PtyEvents, Reply, TextFormatter};
pub use links::{Link, Openable};
pub use marks::Direction;
pub use osc::percent_decode;
pub use osc::{
    BODY_MAX_CHARS, Mark, Notification, Progress, ProgressState, PromptKind, TITLE_MAX_CHARS, clean,
    is_invisible,
};
pub use ports::listening_ports;
use pty::FairLock;
pub use pty::{Launch, Shell};
pub use row_mark::{Part, RowMark};
use screen::Screen;
pub use snapshot::{BlockSpan, ScreenSnapshot, SnapshotCursor, SnapshotRow};

/// A shell in a PTY with a grid and a parser on the PTY's own thread. The grid stays behind this
/// API (snapshots, text, selection, input), so the UI never holds the lock the parser needs for
/// longer than a copy.
pub struct Pty {
    term: Arc<FairLock<Screen>>,
    shared: Arc<Shared>,
    /// Where the program runs: here, or in a session host.
    io: Backend,
    size: GridSize,
    pid: u32,
    /// How long the shell gets to exit after the hangup once the tab is closed (a field so a test
    /// can shorten it).
    hangup_grace: Duration,
    /// The folder the shell last reported (OSC 7), and where polling put it then (see `cwd`).
    reported_cwd: Mutex<Option<ReportedCwd>>,
}

/// Where a terminal's program runs.
enum Backend {
    /// The PTY's reader thread and input; it owns the PTY and ends the shell (see `pty::hangup`).
    Local(pty::Control),
    /// A session host owns the PTY; this is the connection to it, whose thread keeps the screen
    /// a replica of the host's (`client`).
    Hosted(client::Hosted),
}

/// An OSC 7 report, anchored to what polling the shell's process said when it was taken.
struct ReportedCwd {
    path: PathBuf,
    polled_then: Option<PathBuf>,
}

pub use handle::{KeySent, ScrollbackSource};
pub(crate) use options::WindowSize;
pub use options::{GridSize, StartOptions};

#[cfg(test)]
mod measure;
#[cfg(test)]
mod tests;
