//! The terminal pane: owns a PTY + grid and renders it with GPUI.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::alacritty_terminal::grid::Scroll;
use wormhole_core::alacritty_terminal::index::{Column, Line, Point as GridPoint, Side};
use wormhole_core::alacritty_terminal::selection::SelectionType;
use wormhole_core::alacritty_terminal::term::TermMode;

use crate::settings::{SettingsStore, settings};
use crate::theme_bridge::token;
use wormhole_core::agents::integrations::{self, Signal};
use wormhole_core::agents::{AgentId, Foreground};
use wormhole_core::notify::Source;
use wormhole_core::terminal::input::app_selection::AppSelection;
use wormhole_core::terminal::input::{file_drop, keys, mouse};
use wormhole_core::terminal::program_status::Summary as Reported;
use wormhole_core::terminal::{GridSize, Mark, Notification, Pty, PtyEvent, SpawnOptions};

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

/// The grid's inset from the pane's sides, and below the title bar and above the bottom: one step
/// of the 4-pt scale, close to Terminal's, so the text gets the room. What
/// doesn't make a whole cell is left at the right and the bottom.
const PADDING_X: f32 = 4.0;
const PADDING_Y: f32 = 4.0;
/// Narrowest the terminal column may get (padding included); the layout rules in core own it.
pub const MIN_WIDTH: f32 = wormhole_core::layout::TERMINAL_MIN;
/// The font sizes a terminal draws at: what Settings allows, the window's ⌘+ / ⌘- included.
pub use wormhole_core::fonts::TERMINAL_FONT_SIZES as FONT_SIZES;
/// Height of the workspace's floating title bar; the grid starts below it and
/// earlier lines are drawn blurred underneath it. With several tabs the bar grows by their row
/// (`set_top_inset`).
pub const TOP_INSET: f32 = crate::style::TITLE_ROW_H;

/// Geometry of the last painted frame, shared with mouse handlers.
#[derive(Clone, Copy, Default)]
struct Layout {
    origin: Point<Pixels>,
    cell: Size<Pixels>,
    cols: usize,
    rows: usize,
}

pub struct TerminalView {
    pub focus_handle: FocusHandle,
    /// The PTY's event task, ahead of `pty` because fields drop in order: the task (and the waker
    /// it left with the channel) is gone before closing the PTY drops the channel's sender on its
    /// close thread. Otherwise that drop could wake the task from there, which GPUI's test
    /// scheduler reports as a non-deterministic test when a window closes.
    _tasks: Vec<Task<()>>,
    pty: Option<Pty>,
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
    mode: TermMode,
    /// History above the screen, as of the last frame.
    content_above: bool,
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
    /// Typed input since the last blink tick: the cursor stays on for a whole phase.
    blink_held: bool,
    /// Runs while `wants_blink`, then ends; `None` once it has.
    cursor_blink: Option<Task<()>>,
    /// The link under the pointer while ⌘ is held (see `links`).
    hovered_link: Option<wormhole_core::terminal::Link>,
    /// Points over the font size in Settings (⌘+ / ⌘- in this window; ⌘0 is 0).
    font_zoom: f32,
    /// How much of the pane's top the workspace's title bar covers: [`TOP_INSET`], or more
    /// while the tabs' row is under it.
    top_inset: f32,
    /// Where the pointer holds the minimap's slider, over its top, while it's dragged.
    minimap_grab: Option<f32>,
    /// The cell the start animation's ship docks on while the terminal welcome plays around this
    /// terminal: reported every frame, in window points, so a resize is followed (`set_dock`).
    dock: Option<wormhole_core::welcome::DockCell>,
    /// Find in scrollback.
    find: find::Find,
    /// Command blocks: the hovered one, and where ⌘↑ / ⌘↓ last landed.
    blocks: blocks::Blocks,
    _focus_watch: Vec<Subscription>,
}

impl TerminalView {
    pub fn new(mut options: SpawnOptions, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let initial = GridSize { cols: 100, rows: 30, cell_width: 8.0, cell_height: 18.0 };
        let shell = wormhole_core::terminal::shell_name(&options).into();
        options.scrollback_lines = settings(cx).scrollback_lines;
        let cwd = options.cwd.clone();
        let restored = options.restore.clone().unwrap_or_default();
        let (pty, events) = match Pty::spawn(initial, &options) {
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
            layout: Rc::default(),
            render_state: Rc::default(),
            mode: TermMode::empty(),
            content_above: false,
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
            blink_held: false,
            cursor_blink: None,
            hovered_link: None,
            font_zoom: 0.0,
            top_inset: TOP_INSET,
            minimap_grab: None,
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

    /// The program the tab runs ("zsh").
    pub fn shell(&self) -> SharedString {
        self.shell.clone()
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

    /// Whether earlier output sits above the visible screen (i.e. it scrolls),
    /// as of the last frame; a change is announced with a notify.
    pub fn has_content_above(&self) -> bool {
        self.content_above
    }

    /// How much of the pane's top the title bar covers (the find bar sits below it).
    pub fn top_inset(&self) -> f32 {
        self.top_inset
    }

    /// The title bar over the pane is `inset` tall now; the grid starts below it.
    pub fn set_top_inset(&mut self, inset: f32, cx: &mut Context<Self>) {
        if (self.top_inset - inset).abs() > 0.5 {
            self.top_inset = inset;
            // The view is cached: lay the grid out again.
            cx.notify();
        }
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
        self.pty.as_ref().is_some_and(|pty| pty.mode().contains(TermMode::BRACKETED_PASTE))
    }

    /// Paste text the way a real terminal does (bracketed if the app asked).
    pub fn paste(&mut self, text: &str) {
        let Some(pty) = &self.pty else { return };
        let text = text.replace("\r\n", "\n").replace('\n', "\r");
        let bytes = if pty.mode().contains(TermMode::BRACKETED_PASTE) {
            format!("\x1b[200~{}\x1b[201~", text.replace('\x1b', ""))
        } else {
            text
        };
        self.write_input(bytes.into_bytes());
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
