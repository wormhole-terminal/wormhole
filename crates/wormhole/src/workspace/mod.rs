use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::color_picker::{ColorPickerEvent, ColorPickerState};
use gpui_kit::component::input::{Input, InputEvent, InputState, Textarea, TextareaState};
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::settings::{SettingsStore, UiColors, settings, settings_in, store_in, ui, ui_in};
use crate::style::{self, Edges, TextRole};
use crate::vibrancy;
#[cfg(feature = "devctl")]
pub(crate) use about::devctl_command as devctl_about;
#[cfg(feature = "devctl")]
pub(crate) use actions::NewWindow;
pub(crate) use actions::{CheckForUpdates, init as init_actions};
pub(crate) use app::app_mut;
pub(crate) use app::{AppState, app};
#[cfg(feature = "devctl")]
pub(crate) use app_menu::dump as menu_dump;
pub(crate) use default_terminal::{listen as listen_for_opened, serve as serve_opened};
#[cfg(feature = "devctl")]
pub(crate) use focus::set_front;
use gpui_kit::assets::IconName;
#[cfg(feature = "devctl")]
pub(crate) use project::devctl_trust_shot;
#[cfg(feature = "devctl")]
pub(crate) use updates::devctl_installed as devctl_update_installed;
pub(crate) use windows::request_quit;
use wormhole_core::agents::AgentId;
use wormhole_core::agents::integrations;
use wormhole_core::ipc;
use wormhole_core::keymap::{self, ActionId};
use wormhole_core::profile::ProfileId;
use wormhole_core::sidebar::WindowSidebar;

mod about;
mod actions;
pub(crate) mod ambient;
mod app;
mod app_menu;
mod aux_window;
mod breadcrumb;
mod chrome;
mod command_palette;
pub(crate) mod control;
mod default_terminal;
#[cfg(feature = "devctl")]
mod devctl;
mod dialog;
#[cfg(test)]
mod drag_tests;
#[cfg(test)]
mod failed_tests;
mod focus;
mod folder_color;
#[cfg(test)]
mod folder_color_tests;
mod git_panel;
mod glass;
#[cfg(test)]
mod group_tests;
mod items;
#[cfg(test)]
mod key_tests;
mod layout;
pub(crate) mod menu_bar;
#[cfg(test)]
mod menu_tests;
#[cfg(test)]
mod note_tests;
mod notes;
pub(crate) mod notifications;
mod onboarding;
pub(crate) mod open_in;
mod overlay;
mod pane_drop;
#[cfg(test)]
mod pane_memory_tests;
#[cfg(test)]
mod pane_tests;
mod panel_list;
mod panes;
mod path_card;
mod profile_chip;
#[cfg(test)]
mod profile_look_tests;
mod profiles;
mod project;
pub(crate) mod quick_panel;
#[cfg(test)]
mod reported_tests;
mod resize;
mod session;
mod settings_page;
mod shells;
mod shortcut_sheet;
mod shortcuts;
mod sidebar;
mod slide;
mod spawn;
mod splits;
#[cfg(test)]
mod stable_list_tests;
mod status_bar;
mod studio;
mod tasks;
mod terminal_drag;
mod text;
mod title_chips;
mod title_extras;
mod title_row;
#[cfg(test)]
mod title_tests;
mod update_restart;
#[cfg(test)]
mod update_restart_tests;
mod updates;
mod usage;
#[cfg(test)]
mod view_tests;
mod welcome;
mod window_bounds;
mod windows;
mod worktree;

use git_panel::{changes, diff_view, history, picker};
use session::{Pane, Session};

/// What the right panel shows.
#[derive(Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum RightTab {
    #[default]
    Changes,
    History,
    Notes,
    Usage,
    Project,
}
use crate::terminal_view::{Status, TerminalEvent, TerminalView};
use wormhole_core::store::{NoteRecord, SessionRecord, TaskRecord, WindowRecord};

use wormhole_core::layout::{Columns, Edge, Panes, WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH};
/// A window's layout as saved with it.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct Layout {
    sidebar_width: f32,
    notes_width: f32,
    sidebar_open: bool,
    notes_open: bool,
    right_tab: RightTab,
    /// The sidebar's grouping, filter and search, and its folded bands.
    sidebar: WindowSidebar,
}

impl Default for Layout {
    fn default() -> Self {
        Self::new(Panes::default(), RightTab::default(), WindowSidebar::default())
    }
}

impl Layout {
    fn new(panes: Panes, right_tab: RightTab, sidebar: WindowSidebar) -> Self {
        Self {
            sidebar_width: panes.sidebar_width,
            notes_width: panes.panel_width,
            sidebar_open: panes.sidebar_open,
            notes_open: panes.panel_wanted,
            right_tab,
            sidebar,
        }
    }

    fn panes(&self) -> Panes {
        Panes {
            sidebar_open: self.sidebar_open,
            sidebar_width: self.sidebar_width,
            panel_wanted: self.notes_open,
            panel_width: self.notes_width,
        }
        .restored()
    }
}
/// How many lines of each terminal's scrollback are saved and restored. Fixed, whatever the
/// `scrollback_lines` setting (that one is how much a running terminal keeps).
const SCROLLBACK_LINES: usize = 2000;
const SCROLLBACK_SAVE_INTERVAL: Duration = Duration::from_secs(60);
const NOTE_PLACEHOLDER: &str = "Write a note or a prompt draft…\n⌘↵ sends the paragraph under the cursor (or the selection); ⌘⇧↵ also submits.";

struct Note {
    state: Entity<TextareaState>,
    collapsed: bool,
    /// "Show all" was clicked: the open note isn't cut at `NOTE_OPEN_MAX_LINES` (until folded).
    show_all: bool,
    _subscription: Subscription,
}

#[derive(Clone)]
struct TaskItem {
    text: SharedString,
    done: bool,
}

pub struct Workspace {
    /// The window's terminals, one session each, in the order the sidebar's manual order keeps.
    sessions: Vec<Session>,
    /// The splits some of them are shown in (a split's members are adjacent in `sessions`).
    splits: Vec<splits::Split>,
    /// The root of the `.wormhole/` project that governs each folder a band names (the terminals'
    /// and the pinned ones), as of the last look (`refresh_projects`): the band, the title row and
    /// the Project tab ask it where a folder's colour lives without touching the disk. A folder in
    /// no project isn't in it.
    folder_projects: std::collections::HashMap<PathBuf, PathBuf>,
    /// Which saved window this is.
    window_id: i64,
    /// The profile this window belongs to: its sessions are saved in that profile's database and its
    /// terminals start with that profile's environment.
    profile: ProfileId,
    /// The selected terminal. When it is in a split, the split is on screen with its pane focused.
    active: usize,
    next_number: usize,
    /// The side panels as the user left them (persisted). Whether the right panel is drawn
    /// also depends on the room left, see `notes_visible`.
    panes: Panes,
    /// The window's width as of the last render, for layout decisions made
    /// outside `render` (a shortcut that reveals the panel). Unbounded until the
    /// first frame so nothing collapses on a guess.
    window_width: f32,
    /// How much of the window the last frame gave to something beside the workspace: the debug
    /// Inspector's panel, which GPUI lays out in the same window (`layout::column_probe`).
    beside: f32,
    /// The terminal list: its saved view (grouping, filter, folded bands) and its state.
    sidebar: sidebar::SidebarState,
    right_tab: RightTab,
    /// Scrolls the panel's short tabs (Project, Usage; the lists keep their own), from the top
    /// again whenever another tab is shown.
    panel_scroll: ScrollHandle,
    /// Open branch picker (Changes "compare with" / History branch).
    picker: Option<picker::Picker>,
    branches: Vec<String>,
    /// The branch picker's search box (one for both tabs: only one picker is open at a time).
    picker_search: Entity<InputState>,
    _picker_subscription: Subscription,
    /// The Project tab's colour picker for the active terminal's folder (`folder_color`).
    folder_picker: Entity<ColorPickerState>,
    /// The folder whose band's "Custom colour…" has the picker open (it hangs off that band), else
    /// the picker is the Project tab's and means the active terminal's folder.
    picking_folder: Option<PathBuf>,
    _folder_picker_subscription: Subscription,
    diff_view: Option<diff_view::DiffView>,
    /// Folders collapsed in the Changes tree view.
    collapsed_dirs: std::collections::HashSet<String>,
    /// A drag on a panel's edge, while the button is down.
    resizing: Option<resize::ResizeDrag>,
    /// A drag on a divider between two panes, while the button is down.
    pane_drag: Option<panes::PaneDrag>,
    /// A terminal being dragged by its row or its pane's header, and what the pointer is over
    /// (`terminal_drag`); gone once the drag is.
    terminal_drag: Option<terminal_drag::TerminalDrag>,
    /// The group being dragged by its head, while it is.
    group_drag: Option<u32>,
    /// The divider the pointer has rested on (its line shows), and the timer that sets it.
    divider_hot: Option<wormhole_core::panes::DividerId>,
    divider_hover: Option<Task<()>>,
    /// The area the selected split's panes share, as of the last frame (`panes::pane_probe`).
    pane_area: Bounds<Pixels>,
    /// The resize handle the pointer has rested on (its line shows), and the timer that sets it.
    resize_hot: Option<Edge>,
    resize_hover: Option<Task<()>>,
    /// The status bar as last drawn (`None` while it is off), to notice when it is stale.
    status_bar_drawn: Option<status_bar::StatusBar>,
    /// The breadcrumb's status words as last drawn ("working · 4m"), to notice the minute.
    crumb_drawn: std::cell::RefCell<Option<String>>,
    /// The "+n" menu of the title row's ports is open.
    ports_menu: bool,
    /// The hover card under the title row's folder.
    path_card: path_card::PathCard,
    /// The other windows' terminals as the bar counts them, by window id, read on the tick.
    status_others: Vec<(i64, Vec<status_bar::Counted>)>,
    /// Index into the active session's notes that ⌘↵ / Stash act on.
    focused_note: Option<usize>,
    /// The note whose text has the keyboard right now (drawn as the active item).
    editing_note: Option<EntityId>,
    blur: vibrancy::Blur,
    /// Liquid Glass behind the sidebar or the tab bar (macOS 26).
    glass: std::rc::Rc<vibrancy::Glass>,
    /// Where the window was last seen, as saved.
    bounds: Option<String>,
    bounds_watch: window_bounds::BoundsWatch,
    /// The window title last set (the session's name).
    title: SharedString,
    /// What F6 moves the keyboard to besides the terminal.
    areas: focus::FocusAreas,
    /// Points over the terminal font size in Settings, for this window (⌘+ / ⌘-; not saved).
    font_zoom: f32,
    /// A "close this window?" question is on screen.
    closing_prompt: bool,
    /// Whether the close button shows its dot: closing would ask first (`should_close`).
    close_dot: bool,
    /// A "close this terminal?" question is on screen.
    closing_session_prompt: bool,
    /// The New Terminal in New Worktree dialog, while it is up.
    worktree_ask: Option<worktree::Asking>,
    /// The comment dialog of a diff line, while it is up.
    commenting: Option<git_panel::diff::Commenting>,
    /// The window was closed on purpose: nothing of it is saved any more.
    discarded: bool,
    dirty: bool,
    /// The first-run sheet while it is open, and what it leaves behind.
    first_run: onboarding::FirstRun,
    /// The terminal about to be made for a welcome (its session id and token): its shell is started
    /// through `Wormhole welcome` with the token in its environment (`make_pane`).
    pending_welcome: Option<(i64, String)>,
    /// The command palette or the shortcut sheet, while one is open over the window.
    overlay: Option<overlay::Overlay>,
    /// The sidebar or the right panel sliding in or out.
    slide: Option<slide::Slide>,
    last_scrollback_save: Instant,
    ticks: u64,
    _tick: Task<()>,
}

impl Workspace {
    pub fn new(
        record: WindowRecord,
        profile: ProfileId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let blur_wanted = aux_window::wants_blur(settings_in(&profile, cx), true);
        let tick = cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_millis(400)).await;
                let alive = this.update_in(cx, |this, window, cx| this.tick(window, cx));
                if alive.is_err() {
                    break;
                }
            }
        });
        cx.observe_window_activation(window, |this, window, cx| {
            if window.is_window_active() {
                this.became_front(cx);
                // macOS isn't relied on to say when Reduce Motion changes.
                cx.defer(app::sync_system_look);
            } else {
                this.hints_blur(cx);
                this.path_card.reset();
            }
        })
        .detach();
        // The close button asks first when a session in the window would be lost.
        let weak = cx.weak_entity();
        window.on_window_should_close(cx, move |window, cx| {
            weak.update(cx, |this, cx| this.should_close(window, cx)).unwrap_or(true)
        });

        let saved: Option<Layout> = record.layout.as_deref().and_then(|json| serde_json::from_str(json).ok());
        let mut layout = saved.unwrap_or_default();
        let sidebar = sidebar::SidebarState::new(std::mem::take(&mut layout.sidebar), window, cx);

        let picker_search = cx.new(|cx| InputState::new(window, cx).placeholder("Search branches"));
        let picker_subscription =
            cx.subscribe_in(&picker_search, window, |this, _, event: &InputEvent, _, cx| match event {
                InputEvent::Change => cx.notify(),
                InputEvent::PressEnter { .. } => this.pick_first_match(cx),
                _ => {}
            });

        let folder_picker = cx.new(|cx| ColorPickerState::new(window, cx));
        let folder_picker_subscription =
            cx.subscribe_in(&folder_picker, window, |this, _, event: &ColorPickerEvent, _, cx| {
                let ColorPickerEvent::Change(Some(color)) = event else { return };
                this.pick_folder_color(*color, cx);
            });

        let mut this = Self {
            sessions: Vec::new(),
            splits: Vec::new(),
            window_id: record.id,
            profile,
            active: 0,
            next_number: 1,
            panes: layout.panes(),
            window_width: f32::MAX,
            beside: 0.0,
            sidebar,
            right_tab: RightTab::default(),
            panel_scroll: ScrollHandle::new(),
            picker: None,
            branches: Vec::new(),
            picker_search,
            _picker_subscription: picker_subscription,
            folder_picker,
            picking_folder: None,
            _folder_picker_subscription: folder_picker_subscription,
            diff_view: None,
            collapsed_dirs: Default::default(),
            resizing: None,
            pane_drag: None,
            terminal_drag: None,
            group_drag: None,
            divider_hot: None,
            divider_hover: None,
            pane_area: Bounds::default(),
            resize_hot: None,
            resize_hover: None,
            status_bar_drawn: None,
            crumb_drawn: Default::default(),
            ports_menu: false,
            path_card: Default::default(),
            status_others: Vec::new(),
            focused_note: None,
            editing_note: None,
            blur: vibrancy::Blur::install(window, blur_wanted),
            glass: std::rc::Rc::new(vibrancy::Glass::install(window)),
            bounds: record.bounds,
            bounds_watch: Default::default(),
            title: SharedString::default(),
            areas: focus::FocusAreas::new(cx),
            font_zoom: 0.0,
            closing_prompt: false,
            close_dot: false,
            folder_projects: Default::default(),
            closing_session_prompt: false,
            worktree_ask: None,
            commenting: None,
            discarded: false,
            dirty: false,
            first_run: Default::default(),
            pending_welcome: None,
            overlay: None,
            slide: None,
            last_scrollback_save: Instant::now(),
            ticks: 0,
            _tick: tick,
        };
        this.right_tab = layout.right_tab;
        for session in record.sessions {
            this.add_session(Some(session), window, cx);
        }
        this.restore_splits(&record.splits);
        if this.sessions.is_empty() {
            // The very first launch, with the terminal welcome on: the first terminal is the welcome.
            match app_mut(cx).first_welcome.take() {
                Some(plan) => this.open_first_welcome(plan, window, cx),
                None => this.new_session(window, cx),
            }
        } else {
            this.activate(record.active.min(this.sessions.len() - 1), window, cx);
        }
        // Saved on the first tick, so a window opened just now survives a quit.
        this.dirty = true;
        this
    }

    /// Apply settings that live outside GPUI's render tree.
    fn sync_appearance(&mut self, cx: &mut Context<Self>) {
        let blur = aux_window::wants_blur(settings_in(&self.profile, cx), true);
        self.blur.set_enabled(blur);
        // Without the blur the window stands on the panel's colour: the gaps round the terminal's
        // surface and between a split's panes show the window, glass or not, and would otherwise
        // show the desktop through it.
        self.blur.set_backing((!blur).then(|| ui_in(&self.profile, cx).panel));
    }

    fn tick(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.ticks += 1;
        self.sync_appearance(cx);
        self.track_window(window, cx);
        self.sync_diff(cx);
        // Folders are also re-read when a terminal prints (see `TerminalView`); this
        // is the backstop. Agent detection walks the process tree, so less often.
        let refresh_agent = self.ticks % 5 == 1;
        let window_active = window.is_window_active();
        if window_active && !self.is_front(cx) {
            self.became_front(cx);
        }
        let mut changed = false;
        // Only the selected terminal and the panes shown with it are on screen.
        let on_screen: Vec<bool> = (0..self.sessions.len()).map(|ix| self.on_screen(ix)).collect();
        for (session, is_active) in self.sessions.iter().zip(on_screen) {
            let pane = &session.pane;
            changed |= pane.terminal.update(cx, |terminal, _| {
                let before = terminal.status();
                let mut changed = terminal.tick(is_active, window_active);
                if is_active && window_active {
                    changed |= terminal.acknowledge();
                }
                if terminal.refresh_cwd() {
                    changed = true;
                    self.dirty = true;
                }
                if refresh_agent {
                    changed |= terminal.refresh_agent();
                }
                changed || before != terminal.status()
            });
            if is_active && window_active {
                notifications::dismiss(pane.id, cx);
            }
        }
        // The Usage tab's times count down while it shows.
        let usage_on_screen = self.notes_visible() && self.right_tab == RightTab::Usage;
        if changed || (usage_on_screen && self.ticks.is_multiple_of(usage::REDRAW_TICKS)) {
            cx.notify();
        }
        self.sync_close_dot(window, cx);
        if self.notes_visible()
            && self.right_tab == RightTab::History
            && (self.ticks.is_multiple_of(12) || self.history_unanswered(cx))
            && !self.active_session().history.waits_for_retry()
        {
            let ix = self.active;
            self.refresh_history(ix, cx);
        }
        for ix in 0..self.sessions.len() {
            if self.sessions[ix].git.waits_for_retry() {
                continue;
            }
            let every = if ix == self.active { 5 } else { 20 };
            let cwd_changed = self.sessions[ix].git.cwd != self.sessions[ix].terminal().read(cx).cwd;
            if cwd_changed || (self.ticks + ix as u64).is_multiple_of(every) {
                self.refresh_git(ix, cx);
            }
        }
        if self.ticks % 5 == 1 {
            // A `git init` in the folder a session is already in changes its root without a `cd`.
            for session in &self.sessions {
                session.repo_root.forget();
            }
            self.refresh_projects(cx);
        }
        // The backstop for a new note that never got the focus it would be thrown away on
        // losing (see `prune_blank_notes`).
        self.prune_blank_notes(window, cx);
        if self.ticks.is_multiple_of(3) {
            self.sync_project_notes(window, cx);
        }
        if self.is_front(cx) {
            self.remember_project(cx);
        }
        self.tick_extras(cx);
        self.refresh_status_bar(cx);
        self.crumb_tick(cx);
        self.sidebar_tick(window_active, cx);
        if self.dirty && !self.discarded {
            self.save(cx);
        }
        if self.last_scrollback_save.elapsed() > SCROLLBACK_SAVE_INTERVAL {
            self.save_scrollback(cx);
        }

        #[cfg(feature = "devctl")]
        crate::devctl::poll(self, window, cx);
    }

    // ---------------------------------------------------------------- agent hooks

    /// An agent's hook event for one of this window's terminals (`tab`: its id, what the hooks
    /// call `WORMHOLE_TAB_ID`).
    pub(crate) fn handle_hook(
        &mut self,
        tab: &str,
        agent: AgentId,
        event: &str,
        message: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        tracing::debug!(%tab, ?agent, %event, ?message, "agent hook");
        let Some(ix) = tab.parse().ok().and_then(|id| self.find_session(id)) else { return };
        let Some(signal) = integrations::signal(agent, event) else { return };
        let terminal = self.sessions[ix].terminal().clone();
        let before = terminal.read(cx).status();
        let status = terminal.update(cx, |t, _| t.apply_signal(agent, signal, message.clone()));
        let looking = self.on_screen(ix) && window.is_window_active();
        if looking && matches!(status, Status::Done | Status::Failed) {
            terminal.update(cx, |t, _| t.acknowledge());
        }
        if status != before {
            // At once rather than on the next tick; after this update, so this window can be read.
            cx.defer(notifications::sweep);
        }
        cx.notify();
    }

    // ---------------------------------------------------------------- persistence

    fn window_record(&self, cx: &App) -> WindowRecord {
        // The groups keep only the terminals that are still here.
        let mut view = self.sidebar.view.clone();
        view.groups.retain_terminals(|id| self.sessions.iter().any(|session| session.id == id));
        let layout = Layout::new(self.panes, self.right_tab, view);
        WindowRecord {
            id: self.window_id,
            active: self.active,
            layout: serde_json::to_string(&layout).ok(),
            bounds: self.bounds.clone(),
            sessions: self.sessions.iter().map(|s| s.record(cx)).collect(),
            splits: self.split_records(),
        }
    }

    fn save(&mut self, cx: &mut App) {
        self.dirty = false;
        if self.discarded {
            return;
        }
        let record = self.window_record(cx);
        app_mut(cx).save_window(&self.profile, &record);
    }

    fn save_scrollback(&mut self, cx: &mut App) {
        self.last_scrollback_save = Instant::now();
        if self.discarded {
            return;
        }
        let Some(store) = app(cx).store_of(&self.profile) else { return };
        for pane in self.sessions.iter().map(|session| &session.pane) {
            let text = pane.terminal.read(cx).scrollback_text(SCROLLBACK_LINES);
            if let Err(err) = store.save_scrollback(pane.id, &text) {
                tracing::warn!("scrollback save failed: {err:#}");
            }
        }
    }

    /// Quit the app (every window is saved on the way out).
    #[cfg(feature = "devctl")]
    pub(crate) fn quit(&mut self, cx: &mut Context<Self>) {
        cx.quit();
    }

    fn mark_dirty(&mut self, cx: &mut Context<Self>) {
        self.dirty = true;
        cx.notify();
    }

    #[cfg(feature = "devctl")]
    pub(crate) fn debug_state(&self, cx: &App) -> String {
        let mut out = format!(
            "window {} of {} (id {}) close_dot={} closing_terminal_prompt={} profile={}\nlayout sidebar={} notes={} width={} sidebar_shown={} notes_wanted={} notes_visible={} resizing={} status_bar={} groups={} folded={:?} listed={:?} numbered={:?}\n",
            self.window_number(cx),
            app(cx).windows.len(),
            self.window_id,
            self.close_dot,
            self.closing_session_prompt,
            self.profile,
            self.panes.sidebar_width,
            self.panes.panel_width,
            self.window_width,
            self.panes.sidebar_open,
            self.panes.panel_wanted,
            self.notes_visible(),
            self.resizing.is_some(),
            self.status_bar_drawn.is_some(),
            self.sidebar.view.groups.iter().count(),
            self.sidebar.view.folded,
            self.session_order(cx).into_iter().map(|ix| ix + 1).collect::<Vec<_>>(),
            self.session_listing(cx).numbered().into_iter().map(|ix| ix + 1).collect::<Vec<_>>(),
        );
        for (ix, session) in self.sessions.iter().enumerate() {
            let terminal = session.terminal().read(cx);
            let marker = if ix == self.active {
                "*"
            } else if self.on_screen(ix) {
                "+"
            } else {
                " "
            };
            out.push_str(&format!(
                "{marker}{} id={} {:?} agent={:?}{}{} name={:?} cwd={:?}{}{} command={:?} shell={:?} runs={:?}\n",
                ix + 1,
                session.id,
                terminal.status(),
                terminal.agent,
                // A program that reports its own status and is no agent, and a failed command's
                // exit code (the row's cue).
                terminal.program_name().map(|name| format!(" program={name:?}")).unwrap_or_default(),
                terminal.failure().map(|failure| format!(" failed=exit {}", failure.code)).unwrap_or_default(),
                session.name(cx),
                terminal.cwd,
                if session.pinned { " pinned" } else { "" },
                session.split.map(|id| format!(" split={id}")).unwrap_or_default(),
                terminal.foreground().map(|f| f.command.clone()),
                session.pane.shell,
                terminal.shell_name(),
            ));
            for task in &session.tasks {
                out.push_str(&format!("    [{}] {}\n", if task.done { "x" } else { " " }, task.text));
            }
            for note in &session.notes {
                out.push_str(&format!(
                    "    note{}: {:?}\n",
                    if note.collapsed { " (collapsed)" } else { "" },
                    note.state.read(cx).value()
                ));
            }
        }
        for split in &self.splits {
            out.push_str(&format!(
                "split {} members={:?} panes={:?} focused={} zoomed={:?} layout={:?}\n",
                split.id,
                self.members(split.id).into_iter().map(|ix| ix + 1).collect::<Vec<_>>(),
                split.layout.panes(),
                split.layout.focused(),
                split.layout.zoomed(),
                split.layout.preset(),
            ));
        }
        out
    }

    // ---------------------------------------------------------------- sessions

    pub(crate) fn new_session(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.new_session_with(None, window, cx);
    }

    /// A new terminal running the shell `shell` names (`None`: the default), where the current
    /// one is.
    pub(crate) fn new_session_with(
        &mut self,
        shell: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let cwd = self.default_cwd(cx);
        self.open_session(shell, cwd, window, cx);
    }

    /// A new terminal in `cwd` running `shell` (a profile's key; `None`: the default), selected.
    pub(in crate::workspace) fn open_session(
        &mut self,
        shell: Option<String>,
        cwd: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> usize {
        self.open_session_in(None, shell, cwd, window, cx)
    }

    /// [`Self::open_session`] in a group of its own choosing: `Some(Some(group))` joins that group,
    /// `Some(None)` stands in no group, `None` beside the selected terminal, in its group if it
    /// has one (⌘T), whereas ⇧⌘T opens one in no group.
    pub(in crate::workspace) fn open_session_in(
        &mut self,
        group: Option<Option<u32>>,
        shell: Option<String>,
        cwd: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> usize {
        let mut record = SessionRecord::new(self.alloc_id(cx), None, cwd);
        record.shell = shell;
        let group = group.unwrap_or_else(|| self.active_group());
        let id = record.id;
        let ix = self.add_session(Some(record), window, cx);
        if let Some(group) = group {
            let ids = self.session_ids();
            self.sidebar.view.place_in_group(&ids, id, group, None);
        }
        self.activate(ix, window, cx);
        self.dirty = true;
        ix
    }

    fn add_session(
        &mut self,
        record: Option<SessionRecord>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> usize {
        let number = self.next_number;
        self.next_number += 1;
        let record = record.unwrap_or_else(|| {
            let id = self.alloc_id(cx);
            SessionRecord::new(id, None, self.default_cwd(cx))
        });
        let pane = self.make_pane(&record, window, cx);
        self.sessions.push(Session {
            id: record.id,
            number,
            custom_name: record.name,
            pane,
            split: record.split_id,
            notes: Vec::new(),
            notes_list: notes::NotesList::default(),
            tasks: record.tasks.into_iter().map(|t| TaskItem { text: t.text.into(), done: t.done }).collect(),
            tasks_collapsed: record.tasks_collapsed,
            notes_collapsed: record.notes_collapsed,
            task_edit: None,
            git: changes::GitState::default(),
            compare: None,
            history: history::HistoryState::default(),
            project: None,
            notes_root: None,
            notes_base: None,
            repo_root: Default::default(),
            pinned: record.pinned,
            extras: Default::default(),
            review: Default::default(),
        });
        let ix = self.sessions.len() - 1;
        for note in record.notes {
            self.add_note(ix, Some(note.body), note.collapsed, window, cx);
        }
        ix
    }

    fn alloc_id(&self, cx: &mut App) -> i64 {
        app_mut(cx).alloc_id_in(&self.profile)
    }

    /// Where a new terminal starts: where the current one is.
    fn default_cwd(&self, cx: &App) -> Option<String> {
        self.sessions
            .get(self.active)
            .and_then(|s| s.terminal().read(cx).cwd.clone())
            .map(|p| p.to_string_lossy().into_owned())
            // Dev/test override for where the first session starts.
            .or_else(|| std::env::var("WORMHOLE_START_DIR").ok())
    }

    /// Start the shell of a session's terminal, with the id the session has: the profile it was
    /// opened with, else a trusted project's shell (its env applies too), else the default (core's
    /// `shells::for_terminal`), in the folder and with the scrollback the record has.
    fn make_pane(&mut self, record: &SessionRecord, window: &mut Window, cx: &mut Context<Self>) -> Pane {
        // A terminal moved here from another window keeps running.
        if let Some(terminal) = app_mut(cx).adopted.remove(&record.id) {
            return Pane::new(record.id, record.shell.clone(), terminal, window, cx);
        }
        let cwd = record.cwd.as_ref().map(PathBuf::from);
        let (mut env, project_shell) = Self::project_spawn_options(cwd.as_deref(), cx);
        // A worktree Wormhole made: its own ports, whatever the project says.
        if let Some(made) = cwd.as_deref().and_then(|cwd| app(cx).worktrees.find(cwd)) {
            env.extend(made.env().into_iter().map(|(name, value)| (name.to_string(), value)));
        }
        let shell = wormhole_core::shells::for_terminal(
            record.shell.as_deref(),
            project_shell.as_deref(),
            settings_in(&self.profile, cx),
        );
        let mut options = project::spawn_options(record.id, cwd, record.scrollback.clone(), env, shell);
        // After the project's environment is in: what a trusted project set is kept. The profile's
        // own environment is the person's (not repository content), so it applies in an untrusted
        // project as well.
        if let Some(profile) = app(cx).profiles.get(&self.profile) {
            options.apply_profile(profile);
        }
        // A welcome terminal: the shell starts through `Wormhole welcome`, and only this terminal's
        // environment has the token that lets that program talk to the app as the welcome.
        if let Some((_, token)) = self.pending_welcome.take_if(|(id, _)| *id == record.id) {
            // (Not in the app's own tests: their binary is not Wormhole and would take `welcome` for
            // a test name.)
            options.welcome = !cfg!(test);
            options.env.insert(wormhole_core::welcome::TOKEN_VAR.to_string(), token);
        }
        let terminal = cx.new(|cx| TerminalView::new(options, self.profile.clone(), window, cx));
        let zoom = self.font_zoom;
        terminal.update(cx, |terminal, cx| terminal.set_font_zoom(zoom, cx));
        Pane::new(record.id, record.shell.clone(), terminal, window, cx)
    }

    /// Select terminal `ix` and give it the keyboard. Selecting a row of a split shows the split
    /// with that pane focused.
    pub(crate) fn activate(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        if ix >= self.sessions.len() {
            return;
        }
        self.select(ix, cx);
        window.focus(&self.sessions[ix].terminal().focus_handle(cx), cx);
        cx.notify();
    }

    /// Select terminal `ix` without moving the keyboard (it is already there, or goes there next):
    /// what belongs to the terminal on screen starts fresh, its split shows with its pane focused,
    /// and what is on screen counts as seen.
    fn select(&mut self, ix: usize, cx: &mut Context<Self>) {
        if self.active != ix {
            self.focused_note = None;
            // The diff and branch picker belong to the terminal they were opened from.
            self.diff_view = None;
            self.picker = None;
            self.dirty = true;
        }
        self.active = ix;
        let id = self.sessions[ix].id;
        if let Some(split) = self.active_split_mut()
            && split.layout.focused() != id
        {
            split.layout.focus(id);
            self.dirty = true;
        }
        self.acknowledge_selected(cx);
    }

    /// Close terminal `ix`: its shell ends. In a split its pane goes and the others take its room;
    /// one left alone becomes a plain row. The selection stays in the split when the closed one
    /// was selected (the keyboard goes where the layout sends it: the next pane in reading order).
    fn close_session(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        let len = self.sessions.len();
        if ix >= len {
            return;
        }
        let was_selected = ix == self.active;
        // The member that takes the selection, by id: the split's next pane, else the one left
        // alone when the split dissolves.
        let split_id = self.sessions[ix].split;
        let heir = split_id.and_then(|id| {
            let split = self.splits.iter().find(|split| split.id == id)?;
            let mut layout = split.layout.clone();
            layout.close(self.sessions[ix].id);
            Some(layout.focused())
        });
        self.leave_layout(ix);
        let after = session::session_after_close(self.active, ix, len);
        let closed = self.sessions.remove(ix);
        self.sidebar.view.groups.leave(closed.id);
        notifications::dismiss(closed.pane.id, cx);
        welcome::forget(closed.id, cx);
        self.end_welcome_demo(closed.id);
        self.dirty = true;
        // Selecting a session from `usize::MAX` is what resets what belongs to the session on
        // screen (its diff, branch picker and focused note), so only a real change of session
        // goes that way: closing a background session leaves the one you are in alone.
        match after {
            None => {
                self.active = usize::MAX;
                self.new_session(window, cx);
            }
            Some((next, true)) => {
                let next = heir.filter(|_| was_selected).and_then(|id| self.find_session(id)).unwrap_or(next);
                self.active = usize::MAX;
                self.activate(next, window, cx);
            }
            Some((next, false)) => {
                self.active = next;
                cx.notify();
            }
        }
    }

    /// The selected terminal and the panes shown with it are in front of the user: a finished
    /// agent in any of them turns idle, and its notification goes.
    fn acknowledge_selected(&self, cx: &mut Context<Self>) {
        for ix in self.on_screen_sessions() {
            let pane = &self.sessions[ix].pane;
            pane.terminal.update(cx, |t, _| t.acknowledge());
            notifications::dismiss(pane.id, cx);
        }
    }

    fn active_session(&self) -> &Session {
        &self.sessions[self.active]
    }

    pub(crate) fn active_terminal(&self) -> Entity<TerminalView> {
        self.active_session().terminal().clone()
    }

    // ---------------------------------------------------------------- notes
}
