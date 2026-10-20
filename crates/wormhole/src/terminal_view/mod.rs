//! The terminal pane: owns a PTY + grid and renders it with GPUI.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::terminal::types::{
    Column, Line, Modes, MouseTracking, Point as GridPoint, Scroll, SelectionType, Side,
};

use wormhole_core::profile::ProfileId;

use crate::settings::{settings_in, store_in};
use crate::theme_bridge::token;
use wormhole_core::agents::integrations::{self, Signal};
use wormhole_core::agents::{AgentId, Foreground};
use wormhole_core::notify::Source;
use wormhole_core::terminal::input::app_selection::AppSelection;
use wormhole_core::terminal::input::{file_drop, keys, mouse};
use wormhole_core::terminal::program_status::Summary as Reported;
use wormhole_core::terminal::{
    GridSize, Mark, Notification, Pty, PtyEvent, SpawnOptions, WindowGeometry, WindowOp,
};

mod blocks;
mod failed;
mod find;
mod find_bar;
mod input;
mod links;
mod render;
mod reported;
mod status;

pub use failed::Failure;
pub use input::{TerminalKeys, keystroke_spec};
#[cfg(feature = "devctl")]
pub(crate) use render::PAINTS;
use status::RedrawGate;
pub use status::{ProgramNotice, Status, TerminalEvent, TerminalSummary};

// Edit → Clear Buffer (⌘K) and the command blocks' actions (⌘↑ / ⌘↓ between prompts, the
// rest on the current block); the terminal answers Edit → Copy, Paste and Select All too.
actions!(
    terminal,
    [ClearBuffer, PreviousPrompt, NextPrompt, SelectOutput, CopyOutput, CopyCommand, RerunCommand]
);

/// The grid's least inset from the pane's sides, and below the title bar and above the bottom; with
/// the surface's own inset it makes the window's standard edge. What doesn't make a whole cell is split evenly between the two sides
/// (the grid is centred), and the edge cells' backgrounds run out to the pane's edges.
const PADDING_X: f32 = crate::style::TERMINAL_PAD;
const PADDING_Y: f32 = crate::style::TERMINAL_PAD;
/// Narrowest the terminal column may get (padding included); the layout rules in core own it.
pub const MIN_WIDTH: f32 = wormhole_core::layout::TERMINAL_MIN;
/// The font sizes a terminal draws at: what Settings allows, the window's ⌘+ / ⌘- included.
pub use wormhole_core::fonts::TERMINAL_FONT_SIZES as FONT_SIZES;
/// Geometry of the last painted frame, shared with mouse handlers.
#[derive(Clone, Copy, Default)]
struct Layout {
    origin: Point<Pixels>,
    cell: Size<Pixels>,
    cols: usize,
    rows: usize,
    /// How far the grid's left edge is from the pane's: the padding and half the leftover width.
    gutter: f32,
    /// What the window has around the grid (the content less the grid's area), for window operations.
    frame: (f32, f32),
}

pub struct TerminalView {
    pub focus_handle: FocusHandle,
    /// The PTY's event task, ahead of `pty` because fields drop in order: the task (and the waker
    /// it left with the channel) is gone before closing the PTY drops the channel's sender on its
    /// close thread. Otherwise that drop could wake the task from there, which GPUI's test
    /// scheduler reports as a non-deterministic test when a window closes.
    _tasks: Vec<Task<()>>,
    pty: Option<Pty>,
    /// The theme palette the terminal's engine was last given (the own engine answers a program's
    /// color queries itself, so it needs to know them).
    palette_given: Option<Rc<crate::palette::Palette>>,
    /// Where the window was when the engine was last told, for the reports a program asks for
    /// (XTWINOPS); told again when it changes.
    window_given: Option<WindowGeometry>,
    /// A program has asked for the window to go to the Dock, or to come back (and when): until
    /// AppKit says so too, which is after its animation, the reports say what was asked, not the
    /// frames drawn meanwhile.
    minimise_asked: Option<(bool, Instant)>,
    /// The size a program's last resize asked the window for, until the layout shows it (or a
    /// second has passed): the next operation, which a program sends at once, is about that size.
    /// With the layout's size when it was made, to tell when the layout has moved.
    size_asked: Option<(crate::window_ops::SizeGoal, (usize, usize), Instant)>,
    /// After a window operation: brings the engine back to the pane's size once the window has
    /// had its say.
    window_sync: Option<Task<()>>,
    /// The title the program set (OSC 0/2), as the tab shows it (core `names::title_name`).
    pub title: Option<String>,
    /// Renames from the title, held to one per [`status::TITLE_EVERY`].
    title_gate: status::TitleGate,
    pub cwd: Option<PathBuf>,
    /// The coding agent in the foreground (process detection or its hooks).
    pub agent: Option<AgentId>,
    /// Set while an agent's hooks report for this tab; disables the
    /// output-activity heuristic.
    hook_driven: bool,
    /// Latest notification text from the agent ("Claude needs permission…").
    pub agent_message: Option<String>,
    /// What the programs in the terminal report about themselves (OSC 7501), as last read from the
    /// terminal: the record that needs you most. `Some` while any program holds a record, which
    /// makes the status the program's own (see `reported`).
    program: Option<Reported>,
    /// The status and message last came from `program` (rather than from hooks or the heuristics),
    /// so the program saying it is done reporting hands them back.
    program_driven: bool,
    status: Status,
    /// When `status` last changed (how long an agent has worked, or waited for you).
    status_since: Instant,
    /// The program the tab runs ("zsh"), as the status bar names it.
    shell: SharedString,
    last_input: Instant,
    last_resize: Instant,
    /// When the program was last told the terminal gained or lost focus.
    last_focus_report: Instant,
    last_output: Instant,
    layout: Rc<Cell<Layout>>,
    render_state: Rc<RefCell<render::RenderState>>,
    /// Terminal modes as of the last frame, for mouse and scroll handling
    /// (asking the terminal would mean waiting on its lock during output).
    mode: Modes,
    selecting: bool,
    /// Button held while the app receives mouse reports.
    mouse_button: Option<mouse::MouseButton>,
    last_mouse_cell: Option<(usize, usize)>,
    /// What the program selected with the mouse and copied, for ⌘V to replace.
    app_selection: AppSelection,
    scroll_remainder: f32,
    scroll_remainder_x: f32,
    /// A folder check waiting for a burst of output to settle.
    cwd_check: Option<Task<()>>,
    /// A program other than the shell held the terminal when last checked, so
    /// output can't come with a folder change.
    program_in_foreground: bool,
    /// That program's command line and start, for a row without an agent.
    foreground: Option<Foreground>,
    /// The command that last left the foreground: the tab keeps its name while its result is
    /// unseen ("cargo test ✓").
    finished_command: Option<String>,
    /// The bell rang in a terminal without an agent and nobody has looked since.
    bell_rang: bool,
    /// When a bell last counted as "needs you" (at most once a minute).
    bell_counted: Option<Instant>,
    /// What the shell's own marks (OSC 133) say: `Some(true)` at its prompt, `Some(false)`
    /// running a command, `None` for a shell without integration (the heuristics decide alone).
    at_prompt: Option<bool>,
    /// The shell said a command finished (OSC 133 `D`): the status settles on the next tick
    /// instead of waiting for the output to go quiet.
    finished_mark: bool,
    /// The last command's failure (a failing exit code), until the next command starts: the
    /// row says it even after the status has been seen.
    failure: Option<Failure>,
    /// How many failures this terminal has had (numbers them: [`Failure::serial`]).
    failures: u64,
    /// The program's last notification of its own (OSC 9/99/777), for the notification sweep.
    notice: Option<ProgramNotice>,
    /// The summary as last announced with `TerminalEvent::SummaryChanged`.
    announced: TerminalSummary,
    /// The scrollback as last saved: what the tab was restored with, then what
    /// each save got. Saved again while a full-screen program hides the history.
    saved_scrollback: RefCell<String>,
    redraw: RedrawGate,
    /// Focus as last reported to the program (DECSET 1004).
    focus_reported: bool,
    /// The shell's exit code, once it has exited and the system said.
    exit_code: Option<i32>,
    /// The visual bell's glow is on (`bell_visual`); the task turns it off again.
    bell_glow: bool,
    bell_glow_off: Option<Task<()>>,
    /// A focused cursor the program asked to blink, as of the last frame.
    wants_blink: bool,
    /// The blink is in its off phase.
    cursor_off: bool,
    /// Blinking text is on screen in a window in front (see `render::blink`).
    wants_text_blink: bool,
    /// The clock that repaints it twice a second; `None` when nothing blinks.
    text_blink: Option<Task<()>>,
    /// Typed input since the last blink tick: the cursor stays on for a whole phase.
    blink_held: bool,
    /// Runs while `wants_blink`, then ends; `None` once it has.
    cursor_blink: Option<Task<()>>,
    /// The link under the pointer while ⌘ is held (see `links`).
    hovered_link: Option<wormhole_core::terminal::Link>,
    /// Points over the font size in Settings (⌘+ / ⌘- in this window; ⌘0 is 0).
    font_zoom: f32,
    /// Where the pointer holds the minimap's slider, over its top, while it's dragged.
    minimap_grab: Option<f32>,
    /// The scrollbar: its idle time, the pointer on it and the thumb held.
    scrollbar: render::ScrollbarState,
    /// The cell the start animation's ship docks on while the terminal welcome plays around this
    /// terminal: reported every frame, in window points, so a resize is followed (`set_dock`).
    dock: Option<wormhole_core::welcome::DockCell>,
    /// Find in scrollback.
    find: find::Find,
    /// Command blocks: the hovered one, and where ⌘↑ / ⌘↓ last landed.
    blocks: blocks::Blocks,
    _focus_watch: Vec<Subscription>,
    /// The profile of the window this terminal is in: whose look (colours, fonts, settings) it
    /// draws with, which is not the shown profile's in an odd window.
    profile: ProfileId,
}

impl TerminalView {
    pub fn profile(&self) -> &ProfileId {
        &self.profile
    }

    /// The window this terminal is in was moved to another profile.
    pub fn set_profile(&mut self, profile: ProfileId, cx: &mut Context<Self>) {
        if self.profile != profile {
            self.profile = profile;
            cx.notify();
        }
    }

    pub fn new(
        mut options: SpawnOptions,
        profile: ProfileId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let initial = GridSize { cols: 100, rows: 30, cell_width: 8.0, cell_height: 18.0 };
        let shell = wormhole_core::terminal::shell_name(&options).into();
        let store = store_in(&profile, cx);
        options.scrollback_lines = store.settings.scrollback_lines;
        options.allow_screen_checksum = store.settings.allow_screen_checksum;
        options.allow_window_ops = store.settings.allow_window_ops;
        options.allow_title_report = store.settings.allow_title_report;
        let palette = store.palette.clone();
        options.colors = Some(palette.engine_colors());
        options.color_scheme = Some(palette.color_scheme());
        let cwd = options.cwd.clone();
        let restored = options.restore.clone().unwrap_or_default();
        let (pty, events) = match wormhole_core::terminal::spawn(initial, &options) {
            Ok((pty, events)) => (Some(pty), Some(events)),
            Err(err) => {
                tracing::error!("failed to spawn shell: {err:#}");
                (None, None)
            }
        };

        let mut tasks = Vec::new();
        if let Some(events) = events {
            tasks.push(cx.spawn(async move |this, cx| {
                while let Some(event) = events.recv().await {
                    // Everything that's already queued goes into the same update.
                    let alive = this.update(cx, |this, cx| {
                        this.handle_event(event, cx);
                        while let Some(event) = events.try_recv() {
                            this.handle_event(event, cx);
                        }
                        this.after_events(cx);
                    });
                    if alive.is_err() {
                        break;
                    }
                }
            }));
        }

        input::TerminalKeys::register(cx.entity().downgrade(), cx);

        // Programs that asked for focus events (DECSET 1004) hear about the pane
        // and the window gaining or losing focus.
        let focus_handle = cx.focus_handle();
        let focus_watch = vec![
            cx.on_focus(&focus_handle, window, |this, window, cx| this.report_focus(window, cx)),
            cx.on_blur(&focus_handle, window, |this, window, cx| this.report_focus(window, cx)),
            cx.observe_window_activation(window, |this, window, cx| this.report_focus(window, cx)),
        ];

        let now = Instant::now();
        let announced = TerminalSummary {
            status: Status::Idle,
            title: None,
            cwd: cwd.clone(),
            agent: None,
            agent_message: None,
            source: Source::Plain,
            failed: None,
        };
        Self {
            focus_handle,
            pty,
            title: None,
            title_gate: status::TitleGate::default(),
            cwd,
            agent: None,
            hook_driven: false,
            agent_message: None,
            program: None,
            program_driven: false,
            status: Status::Idle,
            status_since: now,
            shell,
            last_input: now,
            last_resize: now,
            last_focus_report: now,
            last_output: now,
            profile,
            palette_given: Some(palette),
            window_given: None,
            minimise_asked: None,
            size_asked: None,
            window_sync: None,
            layout: Rc::default(),
            render_state: Rc::default(),
            mode: Modes::empty(),
            selecting: false,
            mouse_button: None,
            last_mouse_cell: None,
            app_selection: AppSelection::default(),
            scroll_remainder: 0.0,
            scroll_remainder_x: 0.0,
            cwd_check: None,
            program_in_foreground: false,
            foreground: None,
            finished_command: None,
            bell_rang: false,
            bell_counted: None,
            at_prompt: None,
            finished_mark: false,
            failure: None,
            failures: 0,
            notice: None,
            announced,
            saved_scrollback: RefCell::new(restored),
            redraw: RedrawGate::default(),
            focus_reported: false,
            exit_code: None,
            bell_glow: false,
            bell_glow_off: None,
            wants_blink: false,
            cursor_off: false,
            wants_text_blink: false,
            text_blink: None,
            blink_held: false,
            cursor_blink: None,
            hovered_link: None,
            font_zoom: 0.0,
            minimap_grab: None,
            scrollbar: Default::default(),
            dock: None,
            find: find::Find::default(),
            blocks: blocks::Blocks::default(),
            _focus_watch: focus_watch,
            _tasks: tasks,
        }
    }

    /// Tell the program when the terminal gained or lost focus, if it asked, and draw the cursor
    /// for it.
    fn report_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // The cursor is drawn filled or hollow by focus, and the view is cached: draw it again.
        cx.notify();
        let focused = window.is_window_active() && self.focus_handle.is_focused(window);
        if focused == self.focus_reported {
            return;
        }
        self.focus_reported = focused;
        if let Some(pty) = &self.pty
            && let Some(bytes) = keys::focus_report(focused, pty.mode())
        {
            self.last_focus_report = Instant::now();
            pty.write(bytes);
        }
    }

    /// Re-read the folder shortly after output: a `cd` is followed by the new
    /// prompt, so this catches it at once instead of on the next tick. Bursts of
    /// output share one check.
    fn check_cwd_soon(&mut self, cx: &mut Context<Self>) {
        if self.cwd_check.is_some() || self.program_in_foreground {
            return;
        }
        self.cwd_check = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(30)).await;
            let _ = this.update(cx, |this, cx| {
                this.cwd_check = None;
                if this.refresh_cwd() {
                    this.announce_summary(cx);
                }
            });
        }));
    }

    pub fn refresh_cwd(&mut self) -> bool {
        let cwd = self.pty.as_ref().and_then(|pty| pty.cwd());
        if cwd.is_some() && cwd != self.cwd {
            self.cwd = cwd;
            return true;
        }
        false
    }

    /// [`Self::shell`], borrowed: names are worked out on every frame.
    pub fn shell_name(&self) -> &str {
        &self.shell
    }

    /// The grid's columns and rows as of the last frame.
    pub fn grid_size(&self) -> (usize, usize) {
        let layout = self.layout.get();
        (layout.cols, layout.rows)
    }

    pub fn write(&mut self, bytes: Vec<u8>) {
        if let Some(pty) = &self.pty {
            pty.write(bytes);
        }
    }

    pub fn write_input(&mut self, bytes: Vec<u8>) {
        self.last_input = Instant::now();
        self.app_selection.forget();
        self.forget_block_cursor();
        if let Some(pty) = &self.pty {
            // Typing snaps the view back to the live screen.
            pty.write_input(bytes);
        }
    }

    /// Draw the text this many points over the size in Settings (⌘+, ⌘-, ⌘0).
    pub fn set_font_zoom(&mut self, zoom: f32, cx: &mut Context<Self>) {
        if self.font_zoom != zoom {
            self.font_zoom = zoom;
            cx.notify();
        }
    }

    /// The start animation's ship docks on this cell of the terminal's grid (the welcome program
    /// draws its mark there), or on none. The cell's place in the window is reported as the grid is
    /// laid out, so the ship follows a window that is resized or a font that changes.
    pub fn set_dock(&mut self, dock: Option<wormhole_core::welcome::DockCell>, cx: &mut Context<Self>) {
        if self.dock != dock {
            self.dock = dock;
            cx.notify();
        }
    }

    /// Where the ship docks, from the last frame drawn (devctl's `welcome-dock` draws it there).
    #[cfg(feature = "devctl")]
    pub fn dock_landing(&self) -> Option<wormhole_core::intro::voyage::Landing> {
        let layout = self.layout.get();
        self.dock.map(|dock| render::dock_landing(layout.origin, layout.cell, dock))
    }

    /// Whether the program in front takes a paste as text (it turned bracketed paste on): without
    /// it, every line of a paste is typed as if Return were pressed after it.
    pub fn bracketed_paste(&self) -> bool {
        self.pty.as_ref().is_some_and(|pty| pty.mode().contains(Modes::BRACKETED_PASTE))
    }

    /// Paste text the way a real terminal does (bracketed if the app asked).
    pub fn paste(&mut self, text: &str) {
        let Some(pty) = &self.pty else { return };
        let bytes = wormhole_core::terminal::input::paste(text, pty.mode());
        self.write_input(bytes);
    }

    /// The selected text, if anything is selected.
    pub fn selection_text(&self) -> Option<String> {
        self.pty.as_ref()?.selection_text()
    }

    /// The visible screen as plain text (for debugging and devctl).
    #[cfg(feature = "devctl")]
    pub fn screen_text(&self) -> String {
        self.pty.as_ref().map(|pty| pty.screen_text()).unwrap_or_default()
    }

    /// The last `max_lines` lines of history + screen, for restoring later.
    /// While vim, less or htop has the screen the history is out of reach, and
    /// this is the last copy that was saved, so a save never replaces the
    /// history with a full-screen program's screen.
    pub fn scrollback_text(&self, max_lines: usize) -> String {
        let fresh = self.pty.as_ref().and_then(|pty| pty.scrollback().text(max_lines));
        keep_last(&mut self.saved_scrollback.borrow_mut(), fresh)
    }

    /// Grab what's typed in an AI CLI's input box (or the selection).
    pub fn grab_prompt(&self) -> Option<String> {
        if let Some(text) = self.selection_text() {
            return Some(text);
        }
        self.pty.as_ref()?.grab_prompt()
    }

    /// Tell the engine the theme's colors when they changed since it was last told: it answers a
    /// program's color queries (OSC 4, 10, 11, 12) itself. A new terminal gets them with its
    /// spawn options (a terminal nobody has drawn yet is asked too); this keeps up with theme
    /// changes, and with a palette that was not the one at spawn.
    fn share_palette(&mut self, palette: &Rc<crate::palette::Palette>) {
        if self.palette_given.as_ref().is_some_and(|given| Rc::ptr_eq(given, palette)) {
            return;
        }
        self.palette_given = Some(palette.clone());
        let Some(pty) = self.pty.as_ref() else { return };
        pty.set_default_colors(&palette.engine_colors());
        pty.set_color_scheme(palette.color_scheme());
    }

    /// The pane's size, which window operations are sums on: what the last resize asked for while
    /// the layout has not caught up with it.
    pub fn pane_size(&self) -> crate::window_ops::PaneSize {
        let layout = self.layout.get();
        let mut pane = crate::window_ops::PaneSize {
            cell: (f32::from(layout.cell.width), f32::from(layout.cell.height)),
            cols: layout.cols,
            rows: layout.rows,
            frame: layout.frame,
        };
        if let Some((cols, rows)) = self.size_pending() {
            (pane.cols, pane.rows) = (cols, rows);
        }
        pane
    }

    /// The size the window was last asked to give the pane, while the layout has not shown it yet.
    fn size_pending(&self) -> Option<(usize, usize)> {
        let layout = self.layout.get();
        match self.size_asked {
            Some((crate::window_ops::SizeGoal::Set { cols, rows }, before, at))
                if before == (layout.cols, layout.rows) && at.elapsed() < Duration::from_secs(1) =>
            {
                Some((cols, rows))
            }
            _ => None,
        }
    }

    /// Tell the engine where the window is when that changed since it was last told.
    fn share_window(&mut self, geometry: Option<WindowGeometry>) {
        /// How long AppKit gets to follow a minimise or a restore before it is believed over the
        /// request (a window that cannot be minimised stays as it is).
        const SETTLE: Duration = Duration::from_secs(3);
        let Some(mut geometry) = geometry else { return };
        // The engine has worked out the window's size from the resize (it grows by what the text
        // area does); the window and its frames still show the size before, which would undo that.
        if self.size_pending().is_some() {
            return;
        }
        if let Some((want, at)) = self.minimise_asked {
            if geometry.iconified == want || at.elapsed() > SETTLE {
                self.minimise_asked = None;
            } else {
                geometry.iconified = want;
            }
        }
        if self.window_given == Some(geometry) {
            return;
        }
        self.window_given = Some(geometry);
        if let Some(pty) = &self.pty {
            pty.set_window(geometry);
        }
    }

    /// The workspace has carried out a program's window operation (`minimised` when it says the
    /// window is, or is no longer, in the Dock). The window takes a moment to follow a resize, and
    /// may not at all: after it has had that, the engine is brought back to the pane's size.
    pub fn window_op_done(&mut self, done: crate::window_ops::Applied, cx: &mut Context<Self>) {
        let minimised = done.minimised;
        match done.size {
            crate::window_ops::SizeGoal::Unchanged => {}
            goal => {
                let layout = self.layout.get();
                self.size_asked = Some((goal, (layout.cols, layout.rows), Instant::now()));
            }
        }
        if let Some(minimised) = minimised {
            self.minimise_asked = Some((minimised, Instant::now()));
        }
        if let (Some(minimised), Some(given)) = (minimised, self.window_given) {
            // No frame is drawn while the window is minimised, so nothing else would say so.
            let told = WindowGeometry { iconified: minimised, ..given };
            self.window_given = Some(told);
            if let Some(pty) = &self.pty {
                pty.set_window(told);
            }
        }
        self.window_sync = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(400)).await;
            this.update(cx, |this, _| {
                if let Some(pty) = &this.pty {
                    pty.sync_size();
                }
            })
            .ok();
        }));
    }

    /// Recompute the grid size for the available space and resize the PTY.
    fn resize(&mut self, size: GridSize) {
        if let Some(pty) = &mut self.pty
            && pty.size() != size
        {
            self.last_resize = Instant::now();
            pty.resize(size);
        }
    }
}

/// `fresh` when there is one (remembered as `last`), else `last`.
fn keep_last(last: &mut String, fresh: Option<String>) -> String {
    if let Some(fresh) = fresh {
        last.clone_from(&fresh);
        return fresh;
    }
    last.clone()
}

impl EventEmitter<TerminalEvent> for TerminalView {}

impl Focusable for TerminalView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::keep_last;

    #[test]
    fn a_save_without_history_gives_the_last_copy_again() {
        let mut last = "restored history".to_string();
        assert_eq!(keep_last(&mut last, None), "restored history");
        assert_eq!(keep_last(&mut last, Some("new history".into())), "new history");
        // vim opened: the alternate screen has no history.
        assert_eq!(keep_last(&mut last, None), "new history");
    }
}
