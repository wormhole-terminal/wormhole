//! One window. A [`Workspace`] is the view of one window's terminals with their splits and panes, the
//! sidebar, the side panels (notes, changes, history, project), the title rows and the status bar; it
//! saves and restores itself. What must exist once, whatever the number of windows, is `AppState`
//! (`app_state`), which the windows read and update: the two are one tier of the order, and the
//! app's only cycle. Windows are opened and closed in `window_lifecycle`. The type keeps the name
//! `Workspace`, and so do the action namespace `workspace` and the key context `"Workspace"`: they
//! are in users' keymaps.
//!
//! Inside, by folder: `panes/` the split on screen, its model (`splits`) and the pane drop; `title/`
//! the title rows (breadcrumb, chips, extras, the path card and menu); `focus/` keyboard focus: F6's
//! areas, the stops, the Esc ladder and the ring; `repo/` what git says of each terminal's folder.
//!
//! and by file: what a feature is to the window (`feature`, `feature_host`, its facts in `facts`), the
//! session (`session`), the window's terminals (`terminals`) and its copy of what each shows of itself
//! (`summaries`: everything the window draws of a terminal comes from it, never from the terminal), its
//! tick (`tick`), its saves (`save`, `saved_layout`, `window_bounds`), its life (`windows`,
//! `window_lifecycle`, `hosting` for the persistent terminals), what it answers from outside
//! (`control`, `restart_facts`, `ambient`, `menu_state`, and `devctl` in a devctl build), its
//! actions and keys (`window_actions`, `shortcuts`), its look (`layout`, `chrome`, `resize`, `slide`,
//! `glass`, `status_bar`; `regions`: the chrome GPUI keeps between frames) and what opens over it
//! (`overlay`).
//!
//! The window sits between the modules it hosts and the ones that use it. Below it, and used by it:
//! `sidebar` (the terminal list with its groups, rows, menus and drag) and `onboarding` (the intro window
//! and the welcome), which keep their state in entities of their own that the window tells how it stands
//! (`sidebar_host`, `onboarding_host`, and for the terminal welcome `welcome_host`) and hears through
//! their events. Above it, and using it: the features (`features`, which the app installs as a table the
//! window holds and never names), the secondary windows' options (`aux_window`), and everything else the
//! app builds on top (`notifications`, `update`, `about`, `studio`, `settings_window`, the status item,
//! the command palette, the shortcut sheet, `app`). The window uses nothing above it: what it would tell
//! the surfaces above goes through `app_state::attention` (a terminal is in front or gone, a status
//! changed), which the notifications listen to; the welcome's agents step opens Settings by dispatching
//! `actions::OpenSettingsAt`, which the app root answers; and the control socket hands the welcome the
//! budgets, the notifications' permission and the windows to show its example band in
//! (`onboarding::welcome::Services`). `impl Workspace` and `impl Session` are allowed only here (the
//! `impl-outside` rule).

use std::time::{Duration, Instant};

use gpui_kit::prelude::*;
use gpui_kit::*;

pub(crate) use self::windows::{request_quit, request_quit_ending_hosts};
use crate::app_state::attention::{self, Attention};
use crate::platform::vibrancy;
use crate::settings::settings_in;
use wormhole_core::agents::AgentId;
use wormhole_core::storage::paths::ProfileId;

pub(crate) mod ambient;
#[cfg(test)]
mod automation_tests;
mod cached;
#[cfg(test)]
mod cached_tests;
pub(crate) mod chrome;
#[cfg(test)]
mod clear_tests;
#[cfg(test)]
mod close_tests;
#[cfg(feature = "devctl")]
pub(crate) mod devctl;
#[cfg(test)]
mod drag_motion_tests;
#[cfg(test)]
mod drag_tests;
pub(crate) mod facts;
#[cfg(test)]
mod failed_tests;
pub(crate) mod feature;
mod feature_host;
pub(crate) use self::feature_host::{redraw_windows_of, refresh_all_features};
#[cfg(all(test, feature = "devctl"))]
mod devctl_tests;
mod feature_terminals;
#[cfg(test)]
mod feature_tests;
pub(crate) mod focus;
mod glass;
mod group_ops;
#[cfg(test)]
mod group_tests;
pub(crate) mod grow;
pub(crate) mod hosting;
#[cfg(test)]
mod key_tests;
mod layout;
#[cfg(test)]
mod layout_tests;
#[cfg(test)]
mod list_tests;
pub(crate) mod menu_state;
#[cfg(test)]
mod menu_tests;
#[cfg(test)]
mod motion_tests;
#[cfg(test)]
mod notes_tab_tests;
#[cfg(test)]
mod notice_tests;
mod notices;
#[cfg(test)]
mod notification_tests;
mod onboarding_host;
#[cfg(test)]
mod onboarding_tests;
pub(crate) mod overlay;
#[cfg(test)]
mod overlay_tests;
#[cfg(test)]
mod pane_memory_tests;
#[cfg(test)]
mod pane_tests;
mod panel;
mod panel_switch;
#[cfg(test)]
mod panel_switch_tests;
#[cfg(test)]
mod panel_tab_tests;
mod panes;
mod plays_motion;
#[cfg(test)]
mod profile_look_tests;
mod published;
mod regions;
#[cfg(test)]
mod regions_tests;
pub(crate) mod repo;
#[cfg(test)]
mod reported_tests;
mod resize;
pub(crate) mod restart_facts;
mod save;
mod saved_layout;
pub(crate) mod scaling;
#[cfg(test)]
mod scaling_tests;
mod session;
#[cfg(test)]
mod session_tests;
mod shortcuts;
#[cfg(feature = "devctl")]
mod sidebar_devctl;
mod sidebar_drops;
mod sidebar_host;
#[cfg(test)]
mod sidebar_tests;
mod slide;
#[cfg(test)]
mod slide_tests;
#[cfg(test)]
mod split_opening_tests;
#[cfg(test)]
mod stable_list_tests;
mod status_bar;
#[cfg(test)]
mod status_motion_tests;
mod status_rolls;
mod summaries;
#[cfg(test)]
mod summary_tests;
pub(crate) mod tear_off;
#[cfg(test)]
mod tear_off_tests;
mod terminal_move;
#[cfg(test)]
mod terminal_tests;
mod terminals;
#[cfg(test)]
mod test_access;
#[cfg(test)]
pub(crate) mod test_harness;
mod tick;
pub(crate) mod title;
mod welcome_host;
#[cfg(test)]
mod welcome_tests;
pub(crate) use self::welcome_host::remember_first_run;
mod control;
mod window_actions;
mod window_bounds;
pub(crate) mod window_lifecycle;
pub(crate) mod windows;

use self::saved_layout::Layout;
use self::session::Session;
use wormhole_core::config::TabId;

use crate::terminal_view::Status;
use wormhole_core::storage::store::WindowRecord;

use wormhole_core::view::layout::{Edge, Panes};

pub struct Workspace {
    /// The window's terminals, one session each, in the order the sidebar's manual order keeps.
    pub(in crate::window) sessions: Vec<Session>,
    /// The splits some of them are shown in (a split's members are adjacent in `sessions`).
    splits: Vec<crate::window::panes::splits::Split>,
    /// Which saved window this is.
    pub(in crate::window) window_id: i64,
    /// The profile this window belongs to: its sessions are saved in that profile's database and its
    /// terminals start with that profile's environment.
    pub(in crate::window) profile: ProfileId,
    /// The selected terminal. When it is in a split, the split is on screen with its pane focused.
    pub(in crate::window) active: usize,
    next_number: usize,
    /// The side panels as the user left them (persisted). Whether the right panel is drawn
    /// also depends on the room left, see `notes_visible`.
    pub(in crate::window) panes: Panes,
    /// The window's width as of the last render, for layout decisions made
    /// outside `render` (a shortcut that reveals the panel). Unbounded until the
    /// first frame so nothing collapses on a guess.
    window_width: f32,
    /// How much of the window the last frame gave to something beside the workspace: the debug
    /// Inspector's panel, which GPUI lays out in the same window (`layout::column_probe`).
    beside: f32,
    /// The terminal list: an entity of its own, told how the window stands (`sidebar_host`) and heard
    /// through its events.
    sidebar: Entity<crate::sidebar::Sidebar>,
    /// What the window keeps for the list besides: where the terminals are listed, the traffic lights.
    sidebar_feed: self::sidebar_host::SidebarFeed,
    /// The chrome around the terminal, kept by GPUI between the frames that change none of it.
    regions: self::regions::Regions,
    pub(in crate::window) right_tab: TabId,
    /// The features the app has (`feature`), copied from the installed table when the window was
    /// made so the frame never reads the global. A test installs its own before it opens one.
    pub(in crate::window) features: &'static [self::feature::FeatureDef],
    /// The entity of each feature that has state, in the table's order (`None` where the entry has
    /// none), made with the window whether the feature is on or off.
    feature_states: Vec<Option<self::feature_host::FeatureState>>,
    /// A feature's text field has the keyboard (`FeatureEvent::Editing`): the panel's content is the
    /// focused area, as for a note being written.
    feature_editing: bool,
    /// The tab that was open when a tab that returns to it (Usage) was opened: where the panel goes back to
    /// when the condition no longer holds (no agent is left to show usage for).
    tab_before_previous: Option<TabId>,
    /// Scrolls the panel's short tabs (Project, Usage; the lists keep their own), from the top
    /// again whenever another tab is shown.
    panel_scroll: ScrollHandle,
    /// A drag on a panel's edge, while the button is down.
    resizing: Option<self::resize::ResizeDrag>,
    /// A drag on a divider between two panes, while the button is down.
    pane_drag: Option<crate::window::panes::PaneDrag>,
    /// A terminal being dragged by its row or its pane's header, and what the pointer is over
    /// (`terminal_drag`); gone once the drag is.
    terminal_drag: Option<crate::sidebar::terminal_drag::TerminalDrag>,
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
    /// The "+n" menu of the title row's ports is open.
    ports_menu: bool,
    /// The path's hover card, a popover under the title row's folder.
    path_card: crate::window::title::path_card::PathCard,
    /// Where the title row's folder is drawn: the card and the path menu open under it. In a row
    /// that doesn't show the folder, its keyboard stop, which ⌃↩'s menu of the row opens under.
    path_anchor: crate::platform::Anchor,
    /// The other windows' terminals as the bar counts them, by window id, read on the tick.
    status_others: Vec<(i64, Vec<self::status_bar::Counted>)>,
    blur: vibrancy::Blur,
    /// Liquid Glass behind the sidebar or the tab bar (macOS 26).
    glass: std::rc::Rc<vibrancy::Glass>,
    /// Where the window was last seen, as saved.
    bounds: Option<String>,
    bounds_watch: self::window_bounds::BoundsWatch,
    /// The window title last set (the session's name).
    title: SharedString,
    /// What F6 moves the keyboard to besides the terminal.
    pub(in crate::window) areas: self::focus::FocusAreas,
    /// Points over the terminal font size in Settings, for this window (⌘+ / ⌘-; not saved).
    font_zoom: f32,
    /// A "close this window?" question is on screen.
    closing_prompt: bool,
    /// Whether the close button shows its dot: closing would ask first (`should_close`).
    close_dot: bool,
    /// A "close this terminal?" question is on screen.
    pub(in crate::window) closing_session_prompt: bool,
    /// The window was closed on purpose: nothing of it is saved any more.
    discarded: bool,
    dirty: bool,
    /// The first run: the intro window while it is open, and the note it leaves in the sidebar. An
    /// entity of its own, told how the window stands (`onboarding_host`) and heard through its events.
    first_run: Entity<crate::onboarding::FirstRun>,
    /// The command palette or the shortcut sheet, while one is open over the window, and a palette that
    /// fades out as it closes.
    overlay: self::overlay::Overlays,
    /// The sidebar or the right panel sliding in or out.
    slide: Option<self::slide::Slide>,
    last_scrollback_save: Instant,
    /// While the window opens: the saved terminals on screen, which attach to their hosts with a
    /// repaint at once (the rest attach headless).
    shown_at_launch: Vec<i64>,
    pub(in crate::window) ticks: u64,
    _tick: Task<()>,
}

impl Workspace {
    pub fn new(
        record: WindowRecord,
        profile: ProfileId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // A window that grows from a chip starts with no blur: the whole frame would show before it has grown.
        let blur_wanted =
            crate::style::wants_blur(settings_in(&profile, cx), true) && !grow::active(record.id, cx);
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
                // The system may take the mouse away (⌘⇥, Mission Control) and leave a drag set for good.
                crate::sidebar::tear_off::cancel(window, cx);
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
        let areas = self::focus::FocusAreas::new(cx);
        let list_focus = areas.list_focus();
        let view = std::mem::take(&mut layout.sidebar);
        let sidebar =
            cx.new(|cx| crate::sidebar::Sidebar::new(view, list_focus, Self::profiles_for_chip, window, cx));
        cx.subscribe_in(&sidebar, window, Self::on_sidebar_event).detach();
        let first_run = cx.new(|_| crate::onboarding::FirstRun::default());
        cx.subscribe_in(&first_run, window, Self::on_first_run_event).detach();
        // The intro draws itself (a page, a hover): so does the window it covers.
        cx.observe(&first_run, |_, _, cx| cx.notify()).detach();

        let features = self::feature::FeatureTable::installed(cx);
        let feature_states = Self::make_feature_states(features, window, cx);

        let regions = self::regions::Regions::new(cx.weak_entity(), sidebar.clone(), cx);
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
            sidebar_feed: self::sidebar_host::SidebarFeed::new(),
            regions,
            right_tab: TabId::default(),
            features,
            feature_states,
            feature_editing: false,
            tab_before_previous: None,
            panel_scroll: ScrollHandle::new(),
            resizing: None,
            pane_drag: None,
            terminal_drag: None,
            group_drag: None,
            divider_hot: None,
            divider_hover: None,
            pane_area: Bounds::default(),
            resize_hot: None,
            resize_hover: None,
            ports_menu: false,
            path_card: Default::default(),
            path_anchor: Default::default(),
            status_others: Vec::new(),
            blur: vibrancy::Blur::install(window, blur_wanted),
            glass: std::rc::Rc::new(vibrancy::Glass::install(window)),
            bounds: record.bounds,
            bounds_watch: Default::default(),
            title: SharedString::default(),
            areas,
            font_zoom: 0.0,
            closing_prompt: false,
            close_dot: false,
            closing_session_prompt: false,
            discarded: false,
            dirty: false,
            first_run,
            overlay: Default::default(),
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
            match crate::onboarding::welcome::take_first_launch(cx) {
                Some(plan) => this.open_first_welcome(plan, window, cx),
                None => this.new_session(window, cx),
            }
        } else {
            this.activate(record.active.min(this.sessions.len() - 1), window, cx);
        }
        // Saved on the first tick, so a window opened just now survives a quit.
        this.dirty = true;
        // The sidebar is told when the window's facts change, not while it draws: now, and once at the
        // end of every update that notified the window or changed the settings.
        this.sync_sidebar(cx);
        let me = cx.entity();
        // The chrome is drawn again whenever the window is notified (or a global it draws from changes).
        cx.observe(&me, |this, _, cx| {
            this.regions.notify(cx);
            this.sync_sidebar_soon(cx);
        })
        .detach();
        cx.observe_global::<crate::settings::SettingsStore>(|this, cx| this.sync_sidebar_soon(cx)).detach();
        self::regions::Regions::watch_globals(&mut this, cx);
        // The list draws itself (a hover, a rename, a menu); the window around it hears of what it draws.
        self::regions::Regions::watch_sidebar(&mut this, cx);
        this.prepare_grow(window, cx);
        this.tell_notices(window, cx);
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
        let terminal = self.sessions[ix].terminal().clone();
        let Some(signal) = terminal.update(cx, |t, _| t.hook_signal(agent, event)) else { return };
        let before = terminal.read(cx).status();
        let status = terminal.update(cx, |t, _| t.apply_signal(agent, signal, message.clone()));
        let looking = self.on_screen(ix) && window.is_window_active();
        if looking && matches!(status, Status::Done | Status::Failed) {
            terminal.update(cx, |t, _| t.acknowledge());
        }
        if status != before {
            // At once rather than on the next tick; after this update, so this window can be read.
            attention::tell(Attention::Changed, cx);
        }
        self.sync_summary_at(ix, cx);
        cx.notify();
    }

    fn mark_dirty(&mut self, cx: &mut Context<Self>) {
        #[cfg(any(test, debug_assertions))]
        self.regions.assert_not_drawing("mark_dirty");
        self.dirty = true;
        cx.notify();
    }
}
