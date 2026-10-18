//! A hosted terminal's host side: what `wormhole-host` runs a terminal it owns with
//! (`../repo-docs/docs/session-host.md`, sections 5 and 6).
//!
//! The PTY, its reader thread, the fair lock and the hangup are the local terminal's own (`pty`),
//! started with a feed that parses into the host's screen, the authoritative one, and then puts the
//! same bytes into a [`Stream`] that the connections read at their own pace with a [`Cursor`]. A
//! stream offset counts the PTY bytes parsed since the start; everything a client receives is
//! stamped with one. On top:
//!
//! - replies to the program's queries go straight back to it from the host's engine, as in a
//!   local terminal; its events (title, bell, folder, marks, notifications, progress, status,
//!   clipboard, window operations, exit) become [`HostEvent`]s in the stream, at the offset of the
//!   slice they came from (`tap`, `event`);
//! - an OSC 52 read is a ticket for the controller, answered "empty" when there is none or it
//!   does not answer in [`CLIPBOARD_TIMEOUT`];
//! - a resize is applied between two slices (the screen's lock keeps the reader out) and recorded
//!   in the stream at the offset it was applied at ([`HostedTerminal::resize`]);
//! - an attach is a repaint (`wormhole_vt`'s writer, at most [`ATTACH_HISTORY_LINES`] of history)
//!   made under the same lock as the registration of the client's cursor, so the raw bytes it
//!   then reads continue exactly where the repaint left off ([`HostedTerminal::repaint`]);
//! - [`meta`] is the file that says which host is which, [`paths`] where its socket goes.
//!
//! Lock order: the screen's, then the stream's. A [`Wake`] takes neither.

mod event;
pub mod meta;
pub mod paths;
mod stream;
mod tap;

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use anyhow::Context as _;
use wormhole_vt::parser::Parser;
use wormhole_vt::types::{COLOR_COUNT, NamedColor, Rgb};
use wormhole_wire as wire;

use self::tap::{Core, HostedFeed};
use crate::engine::OwnFeed;
use crate::events::EventProxy;
use crate::pty::{self, FairLock, Hooks};
use crate::screen::Screen;
use crate::{GridSize, Launch, Shell};

pub use self::tap::{CLIPBOARD_TIMEOUT, now_ms};
pub use event::{HostEvent, window_op_from_wire, window_op_to_wire};
pub use stream::{Cursor, FORWARD_CAP, Next, ResizeMark, Stream, Wake};
pub use wormhole_vt::term::repaint::{Gaps, NotReady};

/// The most history lines an attach's repaint carries (the design's `attach_history_lines`; until
/// history pages exist, a reattach restores at most this much).
pub const ATTACH_HISTORY_LINES: usize = 20_000;

/// The largest grid a client may ask for, in either direction.
pub const MAX_CELLS: u16 = 4096;

/// What an attach's repaint made, besides its bytes.
pub struct Repainted {
    /// Reads the raw bytes from [`offset`](Self::offset) on.
    pub cursor: Cursor,
    /// The stream offset the repaint is current as of.
    pub offset: u64,
    pub epoch: u32,
    /// The size the replica must have before it parses the repaint.
    pub size: wire::Size,
    /// The counters the client seeds its replica with first (`EventKind::RepaintSeeds`).
    pub block: u32,
    pub link_seq: u64,
    pub image_handle: u64,
    /// What the repaint could not say (logged; the bisimulation tests own its meaning).
    pub gaps: Gaps,
    pub history_lines: usize,
}

/// The program a host runs and the screen it draws.
pub struct HostedTerminal {
    screen: Arc<FairLock<Screen>>,
    shared: Arc<crate::events::Shared>,
    core: Arc<Core>,
    control: pty::Control,
    pid: u32,
    size: wire::Size,
    /// Bytes of the clients' input handed to the PTY, and bytes of the engine's own replies (which
    /// the PTY's count of written input includes).
    client_input: u64,
    replies: Arc<AtomicU64>,
    closed: bool,
}

impl HostedTerminal {
    /// Start what `spawn` says, in its environment only (the host adds nothing to it, and the
    /// process it runs in should have none of its own: see `wormhole-host`). `wake` is called
    /// whenever the stream, the clipboard or the program's end has news. `on_reader_start` runs
    /// first on the PTY thread.
    ///
    /// Returns the terminal and a cursor that reads the stream from its first byte, for the
    /// connection that asked for the spawn (dropped, the stream stops keeping bytes for it).
    pub fn start(
        spawn: &wire::Spawn<'_>,
        wake: Wake,
        on_reader_start: Option<Box<dyn FnOnce() + Send>>,
    ) -> anyhow::Result<(Self, Cursor)> {
        let size = clamp_size(spawn.size);
        let grid = grid_size(size);
        let core = Arc::new(Core::new(wake.clone(), size.rows));
        // The events side of the proxy goes unused: a tapped proxy queues nothing for a UI.
        let (proxy, _events) = EventProxy::tapped(core.clone());
        let shared = proxy.shared().clone();
        shared.set_size(grid.window_size());
        let mut screen = Screen::new(grid, spawn.options.scrollback_lines as usize);
        apply_options(&mut screen, &spawn.options);
        if let Some(colors) = &spawn.colors {
            let palette = palette_with(screen.term.default_colors(), colors);
            screen.set_default_colors(&palette);
        }
        let screen = Arc::new(FairLock::new(screen));
        let feed = HostedFeed::new(OwnFeed::new(proxy.clone(), screen.clone()), core.clone());
        // Before the program's first byte, so the spawning client misses none.
        let cursor = core.stream.register(true, Duration::ZERO, wake);
        let counted = core.clone();
        let hooks = Hooks {
            // The session acknowledges input with the count: woken to send it.
            on_input: Some(Box::new(move |total| {
                counted.input_written.store(total, Ordering::Release);
                counted.wake();
            })),
            on_thread_start: on_reader_start,
        };
        let launch = launch_of(spawn);
        let started = pty::start_with(&launch, grid.window_size(), screen.clone(), feed, proxy, hooks)
            .with_context(|| format!("could not start {}", launch.shell.program))?;
        let replies = Arc::new(AtomicU64::new(0));
        let writer = started.control.writer();
        let counted = replies.clone();
        shared.set_writer(Box::new(move |bytes| {
            counted.fetch_add(bytes.len() as u64, Ordering::AcqRel);
            writer(bytes);
        }));
        Ok((
            Self {
                screen,
                shared,
                core,
                control: started.control,
                pid: started.pid,
                size,
                client_input: 0,
                replies,
                closed: false,
            },
            cursor,
        ))
    }

    /// The program's process id.
    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn size(&self) -> wire::Size {
        self.size
    }

    pub fn stream(&self) -> &Arc<Stream> {
        &self.core.stream
    }

    /// Give the program `bytes` of input. Never waits; dropped once the program is gone.
    pub fn input(&mut self, bytes: &[u8]) {
        if bytes.is_empty() || self.finished() || self.closed {
            return;
        }
        self.client_input += bytes.len() as u64;
        self.control.input(Cow::Owned(bytes.to_vec()));
    }

    /// How many bytes of the clients' input the PTY has taken, at least (`InputAck`). The PTY counts
    /// the engine's replies with the input; they are taken off as soon as they are queued, so this
    /// can lag the truth by the replies still waiting, never run ahead of it.
    pub fn input_acked(&self) -> u64 {
        let written = self.core.input_written.load(Ordering::Acquire);
        let replies = self.replies.load(Ordering::Acquire);
        written.saturating_sub(replies).min(self.client_input)
    }

    /// Apply a resize between two slices of output, and record it in the stream at the offset it
    /// was applied at. A new grid size starts a new epoch.
    pub fn resize(&mut self, req: u32, size: wire::Size) -> ResizeMark {
        let size = clamp_size(size);
        let grid = grid_size(size);
        let mark = {
            let mut screen = self.screen.lock();
            let changed =
                (screen.columns(), screen.screen_lines()) != (usize::from(size.cols), usize::from(size.rows));
            screen.resize(grid);
            self.core.publish(&screen, self.core.ready.load(Ordering::Acquire));
            let epoch = if changed {
                self.core.epoch.fetch_add(1, Ordering::AcqRel).wrapping_add(1)
            } else {
                self.core.epoch.load(Ordering::Acquire)
            };
            let mark = ResizeMark { offset: self.core.stream.total(), epoch, req, size };
            self.core.stream.push_mark(mark);
            mark
        };
        self.shared.set_size(grid.window_size());
        if !self.finished() && !self.closed {
            self.control.resize(grid.window_size());
        }
        self.size = size;
        mark
    }

    /// A cursor from the stream's end on: the bytes too (`bytes`), or only the resizes and events.
    /// `gap` limits how often output wakes it. Registered between two slices.
    pub fn follow(&self, bytes: bool, gap: Duration, wake: Wake) -> Cursor {
        let _screen = self.screen.lock();
        self.core.stream.register(bytes, gap, wake)
    }

    /// Append to `out` the repaint that brings a fresh replica of [`Repainted::size`] to the screen
    /// as it is now, with at most `max_history` lines of history (and never more than
    /// [`ATTACH_HISTORY_LINES`]), and a cursor that reads the bytes from that point on. `Err` while
    /// the parser is inside a sequence: try again after the next slice.
    pub fn repaint(&self, max_history: usize, out: &mut Vec<u8>, wake: Wake) -> Result<Repainted, NotReady> {
        let screen = self.screen.lock();
        if !self.core.ready.load(Ordering::Acquire) {
            return Err(NotReady::Sequence);
        }
        // The feed's own parser is between sequences (`ready`, set under this lock), which is all
        // the writer asks a parser about.
        let made = screen.term.repaint_capped(&Parser::new(), out, max_history.min(ATTACH_HISTORY_LINES))?;
        let offset = self.core.stream.total();
        let cursor = self.core.stream.register(true, Duration::ZERO, wake);
        Ok(Repainted {
            cursor,
            offset,
            epoch: self.core.epoch.load(Ordering::Acquire),
            size: self.size,
            block: made.seeds.block,
            link_seq: made.seeds.link_seq,
            image_handle: made.seeds.image_handle,
            gaps: made.gaps,
            history_lines: made.history_lines,
        })
    }

    /// What the scrollbar and the sidebar read.
    pub fn counters(&self) -> wire::Counters {
        let core = &self.core;
        wire::Counters {
            epoch: core.epoch.load(Ordering::Acquire),
            history_lines: core.history_lines.load(Ordering::Acquire),
            trimmed_total: 0,
            screen_rows: core.screen_rows.load(Ordering::Acquire).min(u32::from(u16::MAX)) as u16,
            alt_screen: core.alt_screen.load(Ordering::Acquire),
            revision: core.stream.total(),
        }
    }

    /// Bumped by every slice the PTY thread parses (`Pong.pty_progress`).
    pub fn progress(&self) -> u64 {
        self.core.progress.load(Ordering::Acquire)
    }

    /// How the program ended, once it has.
    pub fn exit(&self) -> Option<wire::ExitInfo> {
        *self.core.exit.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The PTY thread is over: the program exited (and was reported), or it was hung up.
    pub fn finished(&self) -> bool {
        self.core.finished.load(Ordering::Acquire)
    }

    /// End the program as closing a local terminal does (`pty::hangup`): a hangup, and a kill after
    /// `grace`. Once; not for a program that is gone already.
    pub fn close(&mut self, grace: Duration) {
        if !self.closed && !self.finished() {
            self.closed = true;
            self.control.close(grace);
        }
    }

    /// The colors the engine answers color queries with, on top of the ones it has.
    pub fn set_colors(&self, colors: &wire::Colors<'_>) {
        let mut screen = self.screen.lock();
        let palette = palette_with(screen.term.default_colors(), colors);
        screen.set_default_colors(&palette);
    }

    pub fn set_options(&self, options: &wire::Options) {
        let mut screen = self.screen.lock();
        apply_options(&mut screen, options);
        screen.term.set_scrollback_lines(options.scrollback_lines as usize);
    }

    /// Whether a controller is there to answer the clipboard; without one a read is answered
    /// "empty" at once, and the tickets out are answered so when it leaves.
    pub fn set_controller(&self, attached: bool) {
        self.core.set_controller_attached(attached);
    }

    /// The controller's answer to `ClipboardLoad{ticket}`.
    pub fn clipboard_reply(&self, ticket: u32, data: &[u8]) {
        self.core.clipboard_reply(ticket, data);
    }

    /// Answer "empty" to the clipboard reads whose time is up; when the next one is.
    pub fn clipboard_expire(&self, now: Instant) -> Option<Instant> {
        self.core.clipboard_expire(now)
    }

    /// The oldest command block the command log holds (`Welcome.first_block_id`), 0 when none.
    pub fn first_block_id(&self) -> u64 {
        self.shared.commands().oldest().unwrap_or(0)
    }
}

/// A size the engine can take: at least one cell, at most [`MAX_CELLS`] each way.
pub fn clamp_size(size: wire::Size) -> wire::Size {
    wire::Size { cols: size.cols.clamp(1, MAX_CELLS), rows: size.rows.clamp(1, MAX_CELLS), ..size }
}

/// The wire's size (cells in 1/16 pixel) as the terminal's.
fn grid_size(size: wire::Size) -> GridSize {
    GridSize {
        cols: size.cols,
        rows: size.rows,
        cell_width: f32::from(size.cell_w) / 16.0,
        cell_height: f32::from(size.cell_h) / 16.0,
    }
}

/// What `spawn` runs: its program, arguments, folder and whole environment.
fn launch_of(spawn: &wire::Spawn<'_>) -> Launch {
    let shell = Shell::new(spawn.program.to_owned(), spawn.args.iter().map(|a| (*a).to_owned()).collect());
    let cwd = (!spawn.cwd.is_empty()).then(|| PathBuf::from(spawn.cwd));
    let env: HashMap<String, String> =
        spawn.env.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
    Launch { shell, cwd, env }
}

fn apply_options(screen: &mut Screen, options: &wire::Options) {
    screen.set_window_ops(options.flags & wire::option_flags::WINDOW_OPS != 0);
    screen.set_screen_checksum(options.flags & wire::option_flags::CHECKSUM != 0);
}

/// `base` with the entries `colors` names.
fn palette_with(base: &[Rgb; COLOR_COUNT], colors: &wire::Colors<'_>) -> [Rgb; COLOR_COUNT] {
    let rgb = |[r, g, b]: wire::Rgb| Rgb { r, g, b };
    let mut palette = *base;
    for (slot, color) in palette.iter_mut().zip(colors.palette.iter()) {
        *slot = rgb(*color);
    }
    palette[NamedColor::Foreground as usize] = rgb(colors.fg);
    palette[NamedColor::Background as usize] = rgb(colors.bg);
    if let Some(cursor) = colors.cursor {
        palette[NamedColor::Cursor as usize] = rgb(cursor);
    }
    palette
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_are_clamped_and_cells_keep_their_fraction() {
        let size = clamp_size(wire::Size { cols: 0, rows: 9000, cell_w: 134, cell_h: 272 });
        assert_eq!((size.cols, size.rows), (1, MAX_CELLS));
        let grid = grid_size(size);
        assert_eq!((grid.cell_width, grid.cell_height), (8.375, 17.0));
    }

    #[test]
    fn colors_land_on_their_slots() {
        let base = [Rgb { r: 1, g: 1, b: 1 }; COLOR_COUNT];
        let colors = wire::Colors {
            fg: [10, 11, 12],
            bg: [20, 21, 22],
            cursor: None,
            palette: wire::List::Owned(vec![[5, 5, 5]]),
        };
        let palette = palette_with(&base, &colors);
        assert_eq!(palette[0], Rgb { r: 5, g: 5, b: 5 });
        assert_eq!(palette[1], Rgb { r: 1, g: 1, b: 1 });
        assert_eq!(palette[NamedColor::Foreground as usize], Rgb { r: 10, g: 11, b: 12 });
        assert_eq!(palette[NamedColor::Background as usize], Rgb { r: 20, g: 21, b: 22 });
        assert_eq!(palette[NamedColor::Cursor as usize], Rgb { r: 1, g: 1, b: 1 });
    }

    #[test]
    fn a_spawn_becomes_its_launch_with_only_its_environment() {
        let spawn = wire::Spawn {
            size: wire::Size { cols: 80, rows: 24, cell_w: 128, cell_h: 256 },
            options: wire::Options { scrollback_lines: 1000, flags: 0 },
            program: "/bin/sh",
            cwd: "",
            args: wire::List::Owned(vec!["-c", "true"]),
            env: wire::List::Owned(vec![("TERM", "xterm-256color")]),
            colors: None,
        };
        let launch = launch_of(&spawn);
        assert_eq!(launch.shell, Shell::new("/bin/sh".into(), vec!["-c".into(), "true".into()]));
        assert_eq!(launch.cwd, None);
        assert_eq!(launch.env.len(), 1);
    }
}
