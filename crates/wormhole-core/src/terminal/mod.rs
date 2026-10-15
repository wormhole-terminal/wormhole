//! The terminal core: a shell in a PTY and the screen it draws. Everything above this module talks
//! to [`Pty`] and to [`types`] (`wormhole_vt::types`: cells, colors, positions, modes); nothing
//! above it names the engine's internals.
//!
//! The engine is Wormhole's own (`wormhole_vt`). The PTY layer is core's own too: it opens the
//! pseudo-terminal, starts the program and runs the thread that reads it, which parses into the
//! screen it is given. The engine-specific files are:
//!
//! - `engine/`: the host side of the engine on the PTY thread: its parser and events (a title, a
//!   bell, a folder, a prompt mark, a notification, a picture) as what the UI hears, and the
//!   pictures' pixels;
//! - `screen`: the engine's terminal as the rest of core reads and drives it, in `wormhole_vt::types`;
//! - `osc`: what the sequences the engine hands over raw mean to Wormhole (the shell's marks,
//!   notifications, progress), cleaned and capped;
//! - `testing`: terminals for tests, in this crate's and the app's (feature `test-support`).
//!
//! The rest:
//!
//! - `pty`: the pseudo-terminal, the program on it and the reader thread (`open`, `spawn`,
//!   `reader`, `hangup`), and the fair lock the thread and the UI share the screen under;
//! - `process`: which program a terminal runs and with which environment;
//! - `ports`: the TCP ports the programs under the shell listen on (libproc);
//! - `events`: the bridge from the PTY thread to the UI;
//! - `restore`: replaying a previous session's text;
//! - `snapshot`: what the UI draws, copied under a short lock;
//! - `minimap`: the history drawn small, and where a click on it scrolls to;
//! - `scrollbar`: the scrollbar's thumb, what clicks and drags on it do, and when it shows and fades;
//! - `text`: the screen and scrollback as plain text;
//! - `find`: find in scrollback, on a copy of the grid's text;
//! - `program_status`: what programs report about themselves (OSC 7501), and the records kept;
//! - `marks`: where a command block's parts are, read back from the grid;
//! - `blocks`: each prompt with its command and output, and how the command went;
//! - `input`: keys, mouse and dropped files → bytes for the PTY;
//! - `links`: OSC 8 hyperlinks and URLs in the text, and which may be opened;
//! - `glyphs`: geometry of the characters the UI paints as shapes.

mod blocks;
mod engine;
mod events;
pub mod find;
pub mod glyphs;
pub mod images;
pub mod input;
mod links;
mod marks;
pub mod minimap;
mod osc;
mod ports;
mod process;
pub mod program_status;
mod pty;
mod restore;
mod screen;
pub mod scrollbar;
pub mod sequences;
mod snapshot;
#[cfg(any(test, feature = "test-support"))]
pub mod testing;
mod text;

/// The value types the screen is drawn and driven with: cells, colors, positions, modes.
pub use wormhole_vt::types;

/// What a program may ask of the window and what the engine needs to know of it to answer the
/// reports (XTWINOPS); see `allow_window_ops`.
pub use wormhole_vt::term::{Extent, Fullscreen, Maximize, WindowGeometry, WindowOp};

use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use wormhole_vt::types::{Colors, Column, Line, Point, Scroll, SelectionType, Side, TermMode};

pub use blocks::{Block, BlockState, Ended, readout_text, took_text};
use events::EventProxy;
use events::Shared;
pub use events::{PtyEvent, PtyEvents, Reply, TextFormatter};
pub use links::{Link, Openable};
pub use marks::{Direction, Part, RowMark};
pub(crate) use osc::percent_decode;
pub use osc::{
    BODY_MAX_CHARS, Mark, Notification, Progress, ProgressState, PromptKind, TITLE_MAX_CHARS, clean,
};
pub use ports::listening_ports;
pub use process::shell_name;
use pty::FairLock;
use screen::Screen;
pub use snapshot::{BlockSpan, ScreenSnapshot, SnapshotCursor, SnapshotRow};
pub use text::prompt_in;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridSize {
    pub cols: u16,
    pub rows: u16,
    pub cell_width: f32,
    pub cell_height: f32,
}

impl GridSize {
    fn window_size(&self) -> WindowSize {
        WindowSize {
            num_lines: self.rows,
            num_cols: self.cols,
            cell_width: self.cell_width as u16,
            cell_height: self.cell_height as u16,
        }
    }
}

/// A grid's size in cells and in pixels: what a program that asks for the text area's size is told
/// and what an image is fitted to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WindowSize {
    pub num_lines: u16,
    pub num_cols: u16,
    pub cell_width: u16,
    pub cell_height: u16,
}

/// How to start a tab's shell.
#[derive(Clone, Debug, Default)]
pub struct SpawnOptions {
    pub cwd: Option<PathBuf>,
    /// The tab this shell belongs to, as `WORMHOLE_TAB_ID`: what its agent hooks report.
    pub tab_id: String,
    /// Previous session text, replayed dimmed before the shell starts.
    pub restore: Option<String>,
    pub scrollback_lines: usize,
    /// Answer DECRQCRA, a program's request for the checksum of a rectangle of the screen
    /// (`allow_screen_checksum`); the engine ignores it otherwise.
    pub allow_screen_checksum: bool,
    /// Let the program act on the window (XTWINOPS: move, resize, minimise, …; DECCOLM changing the
    /// width) and read its title back (`allow_window_ops`); the engine ignores both otherwise.
    pub allow_window_ops: bool,
    /// Extra environment, e.g. from a trusted project's `config.json`.
    pub env: std::collections::BTreeMap<String, String>,
    /// The program to run instead of the login shell: the tab's profile, a trusted project's
    /// shell or the default, decided by `shells::for_tab`.
    pub shell: Option<crate::shells::ShellCommand>,
    /// Start the program through `Wormhole welcome -- <program>`: the first-run welcome in this
    /// terminal, which becomes the program when it ends. The app also puts the welcome's token in
    /// `env` (`welcome::TOKEN_VAR`).
    pub welcome: bool,
    /// The theme's colors, which the engine answers a program's color queries (OSC 4, 10, 11, 12)
    /// with from its first byte; without them it answers with xterm's until
    /// [`Pty::set_default_colors`] is called, which a terminal that was never drawn has not yet.
    pub colors: Option<Box<[types::Rgb; types::COLOR_COUNT]>>,
}

/// A shell in a PTY with a grid and a parser on the PTY's own thread. The grid stays behind this
/// API (snapshots, text, selection, input), so the UI never holds the lock the parser needs for
/// longer than a copy.
pub struct Pty {
    term: Arc<FairLock<Screen>>,
    shared: Arc<Shared>,
    /// The PTY's reader thread and input; it owns the PTY and ends the shell (see `pty::hangup`).
    io: pty::Control,
    size: GridSize,
    pid: u32,
    /// How long the shell gets to exit after the hangup once the tab is closed (a field so a test
    /// can shorten it).
    hangup_grace: Duration,
    /// The folder the shell last reported (OSC 7), and where polling put it then (see `cwd`).
    reported_cwd: Mutex<Option<ReportedCwd>>,
}

/// An OSC 7 report, anchored to what polling the shell's process said when it was taken.
struct ReportedCwd {
    path: PathBuf,
    polled_then: Option<PathBuf>,
}

impl Pty {
    pub fn spawn(size: GridSize, options: &SpawnOptions) -> anyhow::Result<(Self, PtyEvents)> {
        Self::spawn_shell(size, options, process::shell(options))
    }

    /// `spawn` with an explicit program instead of the user's shell.
    #[cfg(test)]
    pub(crate) fn spawn_program(
        size: GridSize,
        options: &SpawnOptions,
        program: &str,
        args: &[&str],
    ) -> anyhow::Result<(Self, PtyEvents)> {
        let args = args.iter().map(|arg| arg.to_string()).collect();
        Self::spawn_shell(size, options, process::launch(program.into(), args))
    }

    fn spawn_shell(
        size: GridSize,
        options: &SpawnOptions,
        shell: pty::Shell,
    ) -> anyhow::Result<(Self, PtyEvents)> {
        let (proxy, events) = EventProxy::new();
        let shared = proxy.shared().clone();
        shared.set_size(size.window_size());
        let launch = process::launch_options(options, shell);
        let engine::Spawned { screen, io, pid } = engine::spawn(size, options, &launch, proxy, &shared)?;
        Ok((
            Self {
                term: screen,
                shared,
                io,
                size,
                pid,
                hangup_grace: pty::HANGUP_GRACE,
                reported_cwd: Mutex::new(None),
            },
            events,
        ))
    }

    pub fn write(&self, bytes: impl Into<Cow<'static, [u8]>>) {
        if self.is_alive() {
            self.io.input(bytes.into());
        }
    }

    /// Whether the shell still runs. After it exits its screen stays, to read,
    /// select and save, but the process is gone: input goes nowhere and its pid
    /// is forgotten (it may already belong to another process).
    pub fn is_alive(&self) -> bool {
        !self.shared.exited()
    }

    fn pid(&self) -> Option<u32> {
        self.is_alive().then_some(self.pid)
    }

    /// The shell's process id while it runs: what a query that is done on another thread
    /// (`listening_ports`) is given, since the terminal itself stays with the UI.
    pub fn shell_pid(&self) -> Option<u32> {
        self.pid()
    }

    pub fn resize(&mut self, size: GridSize) {
        if size == self.size {
            return;
        }
        self.size = size;
        self.shared.set_size(size.window_size());
        self.term.lock().resize(size);
        self.io.resize(size.window_size());
    }

    pub fn size(&self) -> GridSize {
        self.size
    }

    /// Tell the engine where the window is: a program asks for it, its size and the screen's
    /// (XTWINOPS 11, 13, 14, 15, 19). The latest is kept.
    pub fn set_window(&self, geometry: WindowGeometry) {
        self.term.lock().set_window(geometry);
    }

    /// Whether programs in this terminal may act on the window (`allow_window_ops`).
    pub fn window_ops_allowed(&self) -> bool {
        self.term.lock().window_ops_allowed()
    }

    /// Take the engine back to the size this terminal has. A program that asked for another size
    /// (XTWINOPS 8, DECSLPP, DECCOLM) was given it at once; when the window could not follow, or
    /// finished following, this makes the two agree.
    pub fn sync_size(&self) {
        self.term.lock().resize(self.size);
    }

    pub fn mode(&self) -> TermMode {
        self.term.lock().mode()
    }

    /// Bring `out` up to date with the screen and `above` rows of history over it.
    pub fn snapshot(&self, out: &mut ScreenSnapshot, above: usize) {
        out.update(&mut self.term.lock(), above);
        if !out.block_spans().is_empty() {
            let log = self.shared.commands();
            out.set_block_states(|id| log.state(id));
        }
    }

    /// Bring the minimap's copy up to date for a minimap `fits` lines tall (read again only when
    /// the output or the view changed).
    pub fn minimap(&self, out: &mut minimap::MinimapSnapshot, fits: usize) {
        let revision = self.output_revision();
        out.update(&self.term.lock(), fits, revision);
    }

    /// Send a key press: `encode` turns it into bytes for the current keyboard
    /// modes. When it does, the selection goes and the view returns to the live
    /// screen, as typing does in any terminal. One lock for all of it, since the
    /// PTY thread may hold the lock for a whole read while output streams.
    pub fn send_key(&self, encode: impl FnOnce(TermMode) -> Option<Vec<u8>>) -> KeySent {
        let (bytes, view_changed) = {
            let mut term = self.term.lock();
            let Some(bytes) = encode(term.mode()) else { return KeySent::Nothing };
            let had_selection = term.take_selection();
            let scrolled = term.display_offset() != 0;
            if scrolled {
                term.scroll_display(Scroll::Bottom);
            }
            (bytes, had_selection || scrolled)
        };
        self.write(bytes);
        KeySent::Sent { view_changed }
    }

    /// The pointer for a program that asked for the DEC locator (`TermMode::LOCATOR`): a button
    /// went down or up (`Some`) or the pointer moved (`None`). What the program asked to be told
    /// goes to its input.
    pub fn locator(
        &self,
        button: Option<(input::mouse::MouseButton, bool)>,
        at: input::mouse::LocatorPointer,
    ) {
        use input::mouse::MouseButton;
        use wormhole_vt::term::LocatorButton;
        let button = match button {
            Some((MouseButton::Left, pressed)) => Some((LocatorButton::Left, pressed)),
            Some((MouseButton::Middle, pressed)) => Some((LocatorButton::Middle, pressed)),
            Some((MouseButton::Right, pressed)) => Some((LocatorButton::Right, pressed)),
            Some(_) => return,
            None => None,
        };
        let at = wormhole_vt::term::Pointer { col: at.col, row: at.row, x: at.x, y: at.y };
        let replies = self.term.lock().locator_event(button, at);
        if !replies.is_empty() && self.is_alive() {
            self.shared.reply(replies);
        }
    }

    /// The cursor's row on the screen as shown, unless the view is scrolled away from it.
    pub fn cursor_row(&self) -> Option<usize> {
        let term = self.term.lock();
        let row = term.grid_cursor().line.0 + term.display_offset() as i32;
        usize::try_from(row).ok().filter(|&row| row < term.screen_lines())
    }

    /// Colors programs set at runtime (OSC 4/10/11/12), by palette index.
    pub fn colors(&self) -> Colors {
        self.term.lock().colors()
    }

    /// Write typed or pasted input; the view returns to the live screen.
    pub fn write_input(&self, bytes: impl Into<Cow<'static, [u8]>>) {
        {
            let mut term = self.term.lock();
            // Scrolling marks the whole screen changed; only do it when it moves.
            if term.display_offset() != 0 {
                term.scroll_display(Scroll::Bottom);
            }
        }
        self.write(bytes);
    }

    /// Scroll the view through the history.
    pub fn scroll(&self, scroll: Scroll) {
        self.term.lock().scroll_display(scroll);
    }

    /// How many lines the view is scrolled up into the history.
    pub fn display_offset(&self) -> usize {
        self.term.lock().display_offset()
    }

    /// Scroll the view so `line` (a grid line: 0 is the live screen's top) is in it: a line out
    /// of view comes to the middle of the screen; one in view leaves the view where it is.
    pub fn reveal(&self, line: Line) {
        let mut term = self.term.lock();
        let offset = term.display_offset();
        let wanted = find::reveal_offset(line, term.screen_lines(), offset, term.history_size());
        if let Some(wanted) = wanted {
            term.scroll_display(Scroll::Delta(wanted as i32 - offset as i32));
        }
    }

    /// Scroll the view so `line` is its top row, as far as the history allows (a line on the live
    /// screen brings the view back to it).
    pub fn scroll_line_to_top(&self, line: Line) {
        let mut term = self.term.lock();
        let offset = term.display_offset() as i32;
        let wanted = (-line.0).clamp(0, term.history_size() as i32);
        if wanted != offset {
            term.scroll_display(Scroll::Delta(wanted - offset));
        }
    }

    /// Erase the history (the screen stays).
    pub fn clear_history(&self) {
        self.term.lock().clear_history();
    }

    /// Start a mouse selection at a grid point.
    pub fn start_selection(&self, kind: SelectionType, point: Point, side: Side) {
        self.term.lock().start_selection(kind, point, side);
    }

    /// Extend the selection being dragged.
    pub fn update_selection(&self, point: Point, side: Side) {
        self.term.lock().update_selection(point, side);
    }

    /// A click without a drag selects nothing.
    pub fn finish_selection(&self) {
        self.term.lock().drop_empty_selection();
    }

    /// Select the history and the screen.
    pub fn select_all(&self) {
        let mut term = self.term.lock();
        let start = Point::new(term.topmost_line(), Column(0));
        let end = Point::new(term.bottommost_line(), term.last_column());
        term.start_selection(SelectionType::Simple, start, Side::Left);
        term.update_selection(end, Side::Right);
    }

    /// Select whole lines `first..=last` (a block's output).
    pub fn select_lines(&self, first: Line, last: Line) {
        let mut term = self.term.lock();
        let first = first.max(term.topmost_line());
        let last = last.min(term.bottommost_line());
        if first > last {
            return;
        }
        term.start_selection(SelectionType::Lines, Point::new(first, Column(0)), Side::Left);
        let end = Point::new(last, term.last_column());
        term.update_selection(end, Side::Right);
    }

    /// The text of lines `first..=last`, trailing blanks trimmed (a block's output to copy).
    pub fn lines_text(&self, first: Line, last: Line) -> String {
        blocks::lines_text(&self.term.lock(), first, last)
    }

    /// Bumped whenever the program printed: compare two readings to tell
    /// whether the terminal changed in between.
    pub fn output_revision(&self) -> u64 {
        self.shared.revision()
    }

    /// The visible screen as plain text, one line per row.
    pub fn screen_text(&self) -> String {
        text::screen_text(&self.term.lock())
    }

    /// What's typed in an AI CLI's input box on the screen.
    pub fn grab_prompt(&self) -> Option<String> {
        text::grab_prompt(&self.term.lock())
    }

    /// The link at a grid point: a program's hyperlink (OSC 8), else a URL in the text.
    pub fn link_at(&self, point: Point) -> Option<Link> {
        links::link_at(&self.term.lock(), point)
    }

    /// The selected text, if anything is selected (an image's placeholders aren't text).
    pub fn selection_text(&self) -> Option<String> {
        let text = self.term.lock().selection_text()?;
        let text = images::strip_placeholders(&text).into_owned();
        (!text.trim().is_empty()).then_some(text)
    }

    /// A handle that builds the scrollback text, also from another thread.
    pub fn scrollback(&self) -> ScrollbackSource {
        ScrollbackSource { term: self.term.clone(), shared: self.shared.clone() }
    }

    /// The shell's current working directory.
    ///
    /// A shell with integration says where it is (OSC 7) at every prompt, and that is the truth
    /// even where polling its process can't see it (a nested shell's own folder, a container).
    /// The report wins for as long as polling says what it said when the report came: a prompt
    /// is the only time the shell's folder changes, and the next prompt brings the next report.
    /// When polling moves without a report, a shell without integration has taken over (`exec
    /// bash`), and polling is the truth again until the next report. Reports from another host
    /// (ssh) or of a folder that isn't here never get this far (`EventProxy::osc`), so those
    /// terminals show the local shell's folder, as before.
    pub fn cwd(&self) -> Option<PathBuf> {
        let polled = process::process_cwd(self.pid()?);
        let mut reported = self.reported_cwd.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(path) = self.shared.take_cwd_report() {
            *reported = Some(ReportedCwd { path, polled_then: polled.clone() });
        }
        match &*reported {
            Some(report) if report.polled_then == polled => Some(report.path.clone()),
            _ => polled,
        }
    }

    /// Whether the shell is at its prompt, as its OSC 133 marks say; `None` for a shell without
    /// integration (nothing has marked anything yet). Exact where `shell_in_foreground` guesses.
    pub fn at_prompt(&self) -> Option<bool> {
        self.shared.at_prompt()
    }

    /// The exit code of the last command the shell reported finished (OSC 133 `D`).
    pub fn last_exit_code(&self) -> Option<i32> {
        self.shared.last_exit_code()
    }

    /// The progress a program reports (OSC 9;4), until it removes it.
    pub fn progress(&self) -> Option<Progress> {
        self.shared.progress()
    }

    /// What programs in this terminal report about themselves (OSC 7501). The PTY thread keeps it
    /// (a hostile program's reports pile up in 256 records, not in a queue), and a
    /// [`PtyEvent::ProgramStatus`] says it changed. The lock is held for a lookup or an update,
    /// never across a wait.
    pub fn program_status(&self) -> std::sync::MutexGuard<'_, program_status::ProgramStatus> {
        self.shared.program_status()
    }

    /// The lines where prompts start (OSC 133 `A`), oldest first: negative lines are in the
    /// history. A scan of the grid under the lock, for a key press rather than every frame.
    pub fn prompt_lines(&self) -> Vec<Line> {
        marks::prompt_lines(&self.term.lock())
    }

    /// The nearest prompt start above or below `from` (a grid line; the top of the view is
    /// `-display_offset`), for jumping between prompts.
    pub fn prompt_line(&self, from: Line, direction: Direction) -> Option<Line> {
        marks::prompt_line(&self.term.lock(), from, direction)
    }

    /// Whether the shell has integration (a prompt mark has arrived), so there are blocks.
    pub fn has_blocks(&self) -> bool {
        self.shared.at_prompt().is_some()
    }

    /// Every command block on the grid, oldest first: a scan of the whole grid under the lock,
    /// for a key press or a menu, not for every frame.
    pub fn blocks(&self) -> Vec<Block> {
        let term = self.term.lock();
        blocks::blocks(&term, &self.shared.commands())
    }

    /// The block whose text `line` is in.
    pub fn block_at(&self, line: Line) -> Option<Block> {
        let term = self.term.lock();
        blocks::block_at(&term, line, &self.shared.commands())
    }

    /// The block with this id (a `BlockSpan`'s), while its text is on the grid.
    pub fn block(&self, id: u64) -> Option<Block> {
        let term = self.term.lock();
        blocks::block(&term, id, &self.shared.commands())
    }

    /// The command that most recently finished with exit code `code`: what it was, how long it ran
    /// and what it last printed. Looks only at the newest blocks, so it costs a few lines of the
    /// grid, not a scan of the history; for the moment a command is found to have failed.
    pub fn ended(&self, code: i32) -> Option<Ended> {
        let term = self.term.lock();
        blocks::ended(&term, &self.shared.commands(), code)
    }

    /// The grid's last line (the live screen's bottom row).
    pub fn bottommost_line(&self) -> Line {
        self.term.lock().bottommost_line()
    }

    /// The inline images (`images`): what the placeholder cells in a snapshot draw. The lock is
    /// for a frame's lookups; nothing waits on it.
    pub fn images(&self) -> std::sync::MutexGuard<'_, images::ImageStore> {
        self.shared.images()
    }

    /// Give the engine the theme's colors, for the programs that ask what they are (OSC 4, 10,
    /// 11, 12): it answers those itself.
    pub fn set_default_colors(&self, palette: &[types::Rgb; types::COLOR_COUNT]) {
        self.term.lock().set_default_colors(palette);
    }

    /// The virtual placement (kitty's `U=1`) the placeholder cells with this picture id and
    /// placement id belong to. Pictures placed in the grid are drawn from
    /// [`ScreenSnapshot::placements`]; these are the ones drawn from placeholder cells.
    pub fn virtual_placement(&self, image_id: u32, placement_id: u32) -> Option<images::Tiles> {
        let placement = self.term.lock().virtual_placement(image_id, placement_id)?;
        Some(images::Tiles {
            key: engine::image_key(placement.image),
            cols: placement.cols,
            rows: placement.rows,
        })
    }

    /// The coding agent running in the foreground, if any.
    pub fn foreground_agent(&self) -> Option<crate::agents::AgentId> {
        crate::agents::foreground_agent(self.pid()?)
    }

    /// Whether the shell is back at its prompt (nothing else in the foreground).
    pub fn shell_in_foreground(&self) -> bool {
        self.pid().is_some_and(crate::agents::shell_in_foreground)
    }

    /// Whether a program other than the shell holds the terminal; `None` where
    /// the platform can't tell. While one does, the shell's folder can't change.
    pub fn program_in_foreground(&self) -> Option<bool> {
        crate::agents::shell_holds_terminal(self.pid()?).map(|shell| !shell)
    }

    /// The program in the foreground and since when, unless it's the shell.
    pub fn foreground_command(&self) -> Option<crate::agents::Foreground> {
        crate::agents::foreground_command(self.pid()?)
    }
}

/// What a key press did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeySent {
    /// It means nothing in the current modes.
    Nothing,
    /// Its bytes went to the program; `view_changed` when that also cleared a
    /// selection or scrolled back to the live screen (a redraw is due).
    Sent { view_changed: bool },
}

/// Builds a terminal's saved scrollback; `Send`, so a save can run on a
/// background thread (it locks the terminal only while copying text).
#[derive(Clone)]
pub struct ScrollbackSource {
    term: Arc<FairLock<Screen>>,
    shared: Arc<Shared>,
}

impl ScrollbackSource {
    /// See [`Pty::output_revision`].
    pub fn output_revision(&self) -> u64 {
        self.shared.revision()
    }

    /// The last `max_lines` lines of history + screen.
    pub fn text(&self, max_lines: usize) -> Option<String> {
        text::scrollback_text(&self.term.lock(), max_lines)
    }

    /// The history and the screen copied for a search ([`find`]), with the output revision it
    /// is as of.
    pub fn grid_text(&self) -> (find::GridText, u64) {
        let term = self.term.lock();
        (find::GridText::copy(&term), self.shared.revision())
    }
}

#[cfg(test)]
mod measure;
#[cfg(test)]
mod tests;

impl Drop for Pty {
    fn drop(&mut self) {
        // A shell that has exited is gone, and so is its thread (its pid may be someone else's).
        if self.pid().is_some() {
            self.io.close(self.hangup_grace);
        }
    }
}
