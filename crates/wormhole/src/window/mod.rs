//! One window. A [`Workspace`] is the view of one window's terminals with their splits and panes, the
//! sidebar, the side panels (notes, changes, history, project), the title rows and the status bar; it
//! saves and restores itself. What must exist once, whatever the number of windows, is `AppState`
//! (`app_state`), which the windows read and update: the two are one tier of the order until phase 5
//! separates them. Windows are opened and closed in `window_lifecycle`. The type keeps the name
//! `Workspace`, and so do the action namespace `workspace` and the key context `"Workspace"`: they
//! are in users' keymaps.
//!
//! Inside, by folder:
//! - `panes/` the split on screen, its model (`splits`) and the pane drop; `title/` the title rows
//!   (breadcrumb, chips, extras, the path card and menu); `focus/` keyboard focus: F6's areas, the
//!   stops, the Esc ladder and the ring.
//! - `features/` the side panels' contents: a folder each for `notes`, `project` and `git`, and the
//!   file `usage`.
//! - `sidebar/` the terminal list with its groups, rows, menus and drag; `onboarding/` the intro
//!   window and the welcome.
//!
//! and by file: what a feature is to the window (`feature`, its facts in `facts`), the session
//! (`session`), the window's terminals (`terminals`) and their drag (`terminal_drag`), its tick
//! (`tick`), its saves (`save`, `saved_layout`, `window_bounds`), its life (`windows`,
//! `window_lifecycle`, `hosting` for the persistent terminals), what it answers from outside
//! (`window_control`, `restart_facts`, `ambient`, `menu_state`, and `devctl` in a devctl build), its
//! actions and keys (`window_actions`, `shortcuts`), its look (`layout`, `chrome`, `resize`, `slide`,
//! `glass`, `status_bar`, `panel_list`, `folder_color`), the windows it opens beside itself
//! (`aux_window`), the apps it opens a folder in (`open_in`) and what opens over it (`overlay`,
//! `dialog`).
//!
//! Everything that sits above the window has left this folder (`app`, `command_palette`,
//! `shortcut_sheet`, `status_item`, `settings_window`, `studio`, `about`, `notifications`,
//! `updates`), and so have the shared state and the actions it builds on (`app_state`, `actions`).
//! The window uses nothing above it but two known exceptions, kept until phase 5:
//! - it calls `notifications`' `dismiss` and `sweep` (the window will emit seen and closed events
//!   instead) from `tick`, `terminals`, `panes` (and `panes::splits`), `windows` and this file's
//!   window-activation observer, `finished_commands` from `ambient`, and `permission_asks` from
//!   the welcome;
//! - the welcome's agents step opens Settings (`onboarding`: `settings_window::open_settings`,
//!   which an action carrying the section will replace).
//!
//! The nested areas (`sidebar/`, `onboarding/`, `features/`) still keep their state in `Workspace`:
//! their `impl Workspace` blocks are the `impl-outside` rule's `pending` list in
//! `scripts/structure.json`, phase 5's shrink-only counter. Outside `window/` no `impl Workspace`
//! is allowed.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use gpui_kit::component::color_picker::{ColorPickerEvent, ColorPickerState};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::prelude::*;
use gpui_kit::*;

#[cfg(feature = "devctl")]
pub(crate) use self::focus::set_front;
pub(crate) use self::windows::{request_quit, request_quit_ending_hosts};
use crate::app_state::app_mut;
use crate::platform::vibrancy;
use crate::settings::settings_in;
use wormhole_core::agents::AgentId;
use wormhole_core::agents::integrations;
use wormhole_core::storage::paths::ProfileId;

pub(crate) mod ambient;
pub(crate) mod aux_window;
pub(crate) mod chrome;
#[cfg(test)]
mod clear_tests;
#[cfg(test)]
mod close_tests;
#[cfg(feature = "devctl")]
pub(crate) mod devctl;
pub(crate) mod dialog;
#[cfg(test)]
mod drag_tests;
mod facts;
#[cfg(test)]
mod failed_tests;
mod feature;
mod feature_host;
#[cfg(test)]
mod feature_tests;
pub(crate) mod features;
pub(crate) mod focus;
mod folder_color;
#[cfg(test)]
mod folder_color_tests;
mod glass;
#[cfg(test)]
mod group_tests;
pub(crate) mod hosting;
#[cfg(test)]
mod key_tests;
mod layout;
pub(crate) mod menu_state;
#[cfg(test)]
mod menu_tests;
pub(crate) mod onboarding;
pub(crate) mod open_in;
pub(crate) mod overlay;
#[cfg(test)]
mod pane_memory_tests;
#[cfg(test)]
mod pane_tests;
pub(crate) mod panel_list;
#[cfg(test)]
mod panel_tab_tests;
mod panes;
#[cfg(test)]
mod profile_look_tests;
#[cfg(test)]
mod reported_tests;
mod resize;
pub(crate) mod restart_facts;
mod save;
mod saved_layout;
mod session;
mod shortcuts;
mod sidebar;
mod slide;
#[cfg(test)]
mod stable_list_tests;
mod status_bar;
mod terminal_drag;
mod terminals;
mod tick;
mod title;
#[cfg(test)]
pub(crate) mod view_tests;
mod window_actions;
mod window_bounds;
mod window_control;
pub(crate) mod window_lifecycle;
pub(crate) mod windows;

use self::saved_layout::Layout;
use self::session::Session;
use crate::window::features::git::{diff_view, picker};
use wormhole_core::config::TabId;

use crate::terminal_view::Status;
use wormhole_core::storage::store::WindowRecord;

use wormhole_core::view::layout::{Edge, Panes};

pub struct Workspace {
    /// The window's terminals, one session each, in the order the sidebar's manual order keeps.
    sessions: Vec<Session>,
    /// The splits some of them are shown in (a split's members are adjacent in `sessions`).
    splits: Vec<crate::window::panes::splits::Split>,
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
    sidebar: self::sidebar::SidebarState,
    right_tab: TabId,
    /// The features the app has (`feature`), copied from the installed table when the window was
    /// made so the frame never reads the global. A test installs its own before it opens one.
    features: &'static [self::feature::FeatureDef],
    /// The entity of each feature that has state, in the table's order (`None` where the entry has
    /// none), made with the window whether the feature is on or off.
    feature_states: Vec<Option<self::feature::FeatureState>>,
    /// A feature's text field has the keyboard (`FeatureEvent::Editing`): the panel's content is the
    /// focused area, as for a note being written.
    feature_editing: bool,
    /// The tab that was open when Usage was opened: where the panel goes back to when no agent
    /// is left to show usage for.
    tab_before_usage: Option<TabId>,
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
    resizing: Option<self::resize::ResizeDrag>,
    /// A drag on a divider between two panes, while the button is down.
    pane_drag: Option<crate::window::panes::PaneDrag>,
    /// A terminal being dragged by its row or its pane's header, and what the pointer is over
    /// (`terminal_drag`); gone once the drag is.
    terminal_drag: Option<self::terminal_drag::TerminalDrag>,
    /// The group being dragged by its head, while it is.
    group_drag: Option<u32>,
    /// The divider the pointer has rested on (its line shows), and the timer that sets it.
    divider_hot: Option<wormhole_core::view::panes::DividerId>,
    divider_hover: Option<Task<()>>,
    /// The area the selected split's panes share, as of the last frame (`panes::pane_probe`).
    pane_area: Bounds<Pixels>,
    /// The resize handle the pointer has rested on (its line shows), and the timer that sets it.
    resize_hot: Option<Edge>,
    resize_hover: Option<Task<()>>,
    /// The status bar as last drawn (`None` while it is off), to notice when it is stale.
    status_bar_drawn: Option<self::status_bar::StatusBar>,
    /// The breadcrumb's status words as last drawn ("working · 4m"), to notice the minute.
    crumb_drawn: std::cell::RefCell<Option<String>>,
    /// The "+n" menu of the title row's ports is open.
    ports_menu: bool,
    /// The path's hover card, a popover under the title row's folder.
    path_card: crate::window::title::path_card::PathCard,
    /// Where the title row's folder is drawn: the card and the path menu open under it.
    path_anchor: crate::platform::Anchor,
    /// The other windows' terminals as the bar counts them, by window id, read on the tick.
    status_others: Vec<(i64, Vec<self::status_bar::Counted>)>,
    /// Index into the active session's notes that ⌘↵ / Stash act on.
    focused_note: Option<usize>,
    /// The note whose text has the keyboard right now (drawn as the active item).
    editing_note: Option<EntityId>,
    blur: vibrancy::Blur,
    /// Liquid Glass behind the sidebar or the tab bar (macOS 26).
    glass: std::rc::Rc<vibrancy::Glass>,
    /// Where the window was last seen, as saved.
    bounds: Option<String>,
    bounds_watch: self::window_bounds::BoundsWatch,
    /// The window title last set (the session's name).
    title: SharedString,
    /// What F6 moves the keyboard to besides the terminal.
    areas: self::focus::FocusAreas,
    /// Points over the terminal font size in Settings, for this window (⌘+ / ⌘-; not saved).
    font_zoom: f32,
    /// A "close this window?" question is on screen.
    closing_prompt: bool,
    /// Whether the close button shows its dot: closing would ask first (`should_close`).
    close_dot: bool,
    /// A "close this terminal?" question is on screen.
    closing_session_prompt: bool,
    /// The New Terminal in New Worktree dialog, while it is up.
    worktree_ask: Option<crate::window::features::git::worktree::Asking>,
    /// The comment dialog of a diff line, while it is up.
    commenting: Option<crate::window::features::git::diff::Commenting>,
    /// The window was closed on purpose: nothing of it is saved any more.
    discarded: bool,
    dirty: bool,
    /// The first-run sheet while it is open, and what it leaves behind.
    first_run: self::onboarding::FirstRun,
    /// The terminal about to be made for a welcome (its session id and token): its shell is started
    /// through `Wormhole welcome` with the token in its environment (`make_pane`).
    pending_welcome: Option<(i64, String)>,
    /// The command palette or the shortcut sheet, while one is open over the window.
    overlay: Option<self::overlay::Overlay>,
    /// The sidebar or the right panel sliding in or out.
    slide: Option<self::slide::Slide>,
    last_scrollback_save: Instant,
    /// While the window opens: the saved terminals on screen, which attach to their hosts with a
    /// repaint at once (the rest attach headless).
    shown_at_launch: Vec<i64>,
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
        let blur_wanted = self::aux_window::wants_blur(settings_in(&profile, cx), true);
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
                cx.defer(crate::app_state::look::sync_system_look);
            } else {
                this.hints_blur(cx);
                // A popover that is up has the key itself, which is why this window lost it.
                if this.path_card.is_open_popover() {
                    return;
                }
                this.path_card.reset(cx);
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
        let sidebar = self::sidebar::SidebarState::new(std::mem::take(&mut layout.sidebar), window, cx);

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

        let features = self::feature::FeatureTable::installed(cx);
        let feature_states = Self::make_feature_states(features, window, cx);

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
            right_tab: TabId::default(),
            features,
            feature_states,
            feature_editing: false,
            tab_before_usage: None,
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
            path_anchor: Default::default(),
            status_others: Vec::new(),
            focused_note: None,
            editing_note: None,
            blur: vibrancy::Blur::install(window, blur_wanted),
            glass: std::rc::Rc::new(vibrancy::Glass::install(window)),
            bounds: record.bounds,
            bounds_watch: Default::default(),
            title: SharedString::default(),
            areas: self::focus::FocusAreas::new(cx),
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
            shown_at_launch: Vec::new(),
            ticks: 0,
            _tick: tick,
        };
        this.right_tab = layout.right_tab;
        let saved: Vec<(i64, Option<i64>)> = record.sessions.iter().map(|s| (s.id, s.split_id)).collect();
        this.shown_at_launch = wormhole_core::ipc::hosts::shown_at_launch(record.active, &saved);
        for session in record.sessions {
            this.add_session(Some(session), window, cx);
        }
        this.shown_at_launch.clear();
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
            cx.defer(crate::notifications::sweep);
        }
        cx.notify();
    }

    fn mark_dirty(&mut self, cx: &mut Context<Self>) {
        self.dirty = true;
        cx.notify();
    }
}
