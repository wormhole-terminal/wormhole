//! The window's side of the sidebar (`sidebar::Sidebar`, an entity of its own): the facts it is told
//! every frame (the terminals as rows), what the window does when the list asks for something
//! (`SidebarEvent`), and the few calls the rest of the window makes into the list's state. The
//! sidebar never reads the `Workspace`; this is the only place that reads both.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::panes::Axis;
use wormhole_core::view::sidebar::command::{OutputFacts, ends_by_itself, has_new_output};
use wormhole_core::view::sidebar::{Listing, RowState, TerminalsIn, WindowSidebar};

use crate::app_state::app;
use crate::settings::{settings_in, ui_in};
use crate::sidebar::profile_chip::Profiles;
use crate::sidebar::terminal_drag::TerminalDrag;
use crate::sidebar::{NewTerminal, RowFailure, RowInfo, Sidebar, SidebarEvent, SidebarInput, row_times};
use crate::style::traffic_light_position;
use crate::terminal_view::Status;
use crate::window::Workspace;
use crate::{sidebar::shells, style};

/// What the window keeps for the sidebar that is neither the list's state nor the terminals': where the
/// terminals are listed, the traffic lights' row, and the repositories worktrees belong to.
pub(super) struct SidebarFeed {
    /// The terminals are in tabs (Settings → Show terminals in), in every window.
    pub(super) tabs_on: bool,
    /// The height of the row the traffic lights were last centred on.
    lights_row: Option<f32>,
    /// Which repository a worktree belongs to, by its root (read from disk once per refresh).
    worktrees: RefCell<HashMap<PathBuf, Option<PathBuf>>>,
}

impl SidebarFeed {
    pub(super) fn new() -> Self {
        Self { tabs_on: false, lights_row: None, worktrees: RefCell::default() }
    }
}

impl Workspace {
    // ---------------------------------------------------------------- the facts

    /// The repository a worktree belongs to, by its root; read from disk once per refresh.
    fn worktree_of(&self, root: &Path) -> Option<PathBuf> {
        let mut cache = self.sidebar_feed.worktrees.borrow_mut();
        // A hit allocates nothing: the key is only made for a root not seen yet.
        if let Some(known) = cache.get(root) {
            return known.clone();
        }
        let found = wormhole_core::git::worktree_of(root);
        cache.insert(root.to_path_buf(), found.clone());
        found
    }

    /// Every terminal's row facts, in the window's (manual) order. From each pane's summary, not
    /// the terminal: a terminal's output changes nothing here.
    pub(super) fn row_infos(&self, cx: &App) -> Vec<RowInfo> {
        let mut rows: Vec<RowInfo> = self
            .sessions
            .iter()
            .enumerate()
            .map(|(ix, session)| {
                let summary = &session.pane.summary;
                let status = summary.status;
                let foreground = summary.foreground.as_ref();
                let source = summary.source;
                let told = source.told();
                let command = foreground.map(|f| f.command.clone()).filter(|_| !told);
                let program_running = command.is_some();
                let git = session.git.status.as_ref().filter(|_| session.git.cwd == summary.cwd);
                let root = summary.cwd.as_deref().map(|cwd| session.repo_root.of(cwd));
                RowInfo {
                    ix,
                    id: session.id,
                    name: session.name(),
                    agent: summary.agent,
                    source,
                    state: RowState::of(told, status, command.is_some()),
                    message: summary.agent_message.clone(),
                    since_at: summary.status_since,
                    cwd: summary.cwd.clone(),
                    worktree: root.as_deref().and_then(|root| self.worktree_of(root)),
                    repo: root.clone(),
                    running_since: foreground
                        .filter(|f| !told && ends_by_itself(&f.command))
                        .map(|f| f.started),
                    command,
                    bell: !told && summary.bell_rang,
                    failure: summary.failure.as_ref().filter(|_| !told).map(|failure| RowFailure {
                        code: failure.code,
                        command: failure.command.clone(),
                        at: failure.at,
                    }),
                    unread: has_new_output(OutputFacts {
                        agent: told,
                        // Seen on screen: the selected terminal and the panes shown with it.
                        selected: self.on_screen(ix),
                        program_running,
                        unseen: matches!(status, Status::Busy | Status::Done),
                    }),
                    pinned: self.split_pinned(ix),
                    split: session.split,
                    forget_text: session.forget_text,
                    changes: git.and_then(|status| {
                        let n = status.files.len();
                        (n > 0).then_some(n)
                    }),
                    extra: None,
                }
            })
            .collect();
        self.feature_row_extras(&mut rows, cx);
        rows
    }

    /// What the sidebar is told of the window, `width` points wide.
    pub(super) fn sidebar_input(&self, width: f32, cx: &App) -> SidebarInput {
        SidebarInput {
            rows: self.row_infos(cx),
            active: self.active,
            profile: self.profile.clone(),
            width,
            window_width: self.window_width,
            tabs_on: self.sidebar_feed.tabs_on,
            glass: self.glass_on(cx).then(|| self.glass.clone()),
            asking: self.closing_session_prompt || self.closing_prompt,
            terminal_drag: self.terminal_drag.as_ref().map(|drag| (drag.id, drag.origin)),
            group_drag: self.group_drag,
            welcomes: self
                .sessions
                .iter()
                .map(|session| session.id)
                .filter(|&id| crate::onboarding::welcome::running(id, cx))
                .collect(),
        }
    }

    /// Tell the sidebar how the window stands now (the frame does, and every call below that needs
    /// the terminals as they are).
    pub(super) fn push_sidebar(&self, width: f32, cx: &mut Context<Self>) {
        let input = self.sidebar_input(width, cx);
        self.sidebar.update(cx, |sidebar, _| sidebar.set_input(input));
    }

    /// Run `f` on the sidebar after telling it how the window stands now.
    pub(super) fn with_sidebar<R>(
        &self,
        cx: &mut Context<Self>,
        f: impl FnOnce(&mut Sidebar, &mut Context<Sidebar>) -> R,
    ) -> R {
        let input = self.sidebar_input(self.sidebar.read(cx).width(), cx);
        self.sidebar.update(cx, |sidebar, cx| {
            sidebar.set_input(input);
            f(sidebar, cx)
        })
    }

    /// The sidebar's saved view: the groups, their order, which are folded.
    pub(super) fn sidebar_view<'a>(&'a self, cx: &'a App) -> &'a WindowSidebar {
        self.sidebar.read(cx).view()
    }

    /// Change the sidebar's saved view without telling it to draw or the window to save: for the
    /// window's own changes to the terminals (one made, one closed, a pane moved), which save and draw
    /// themselves.
    pub(super) fn edit_sidebar_view<R>(&self, cx: &mut App, f: impl FnOnce(&mut WindowSidebar) -> R) -> R {
        self.sidebar.update(cx, |sidebar, _| f(sidebar.view_mut()))
    }

    // ---------------------------------------------------------------- the list, as the window asks

    /// The list without drawing it (shortcuts, devctl).
    pub(super) fn session_listing(&self, cx: &App) -> Listing {
        let input = self.sidebar_input(0.0, cx);
        self.sidebar.read(cx).listing_of(&input, cx)
    }

    /// Every terminal top to bottom as the sidebar lists it (folded ones included).
    pub(super) fn session_order(&self, cx: &App) -> Vec<usize> {
        self.session_listing(cx).order().to_vec()
    }

    /// The window's terminals' ids, in the order they were made.
    pub(super) fn session_ids(&self) -> Vec<i64> {
        self.sessions.iter().map(|session| session.id).collect()
    }

    /// The group the selected terminal is in: where a new terminal goes.
    pub(super) fn active_group(&self, cx: &App) -> Option<u32> {
        let id = self.sessions.get(self.active)?.id;
        self.sidebar_view(cx).groups.group_of(id)
    }

    /// Where a new terminal in `group` (`None`: in no group) starts: the group's folder if it has
    /// one that is still a directory, else where the selected terminal is. A split's new pane and
    /// a terminal in no group never ask, so they keep starting beside the selected one.
    pub(super) fn start_cwd(&self, group: Option<u32>, cx: &App) -> Option<String> {
        group.and_then(|group| self.sidebar.read(cx).start_folder(group)).or_else(|| self.default_cwd(cx))
    }

    // ---------------------------------------------------------------- where the terminals are listed

    /// Whether the sidebar is on screen: wanted, and the terminals aren't in tabs.
    pub(super) fn sidebar_shown(&self) -> bool {
        self.panes.sidebar_open && !self.sidebar_feed.tabs_on
    }

    /// Whether the terminals are in tabs across the top.
    pub(super) fn tabs_on_top(&self) -> bool {
        self.sidebar_feed.tabs_on
    }

    /// Read where the terminals are listed from the settings (once a frame, before anything is laid
    /// out), and centre the traffic lights on the row they sit in.
    pub(super) fn sync_tab_layout(&mut self, window: &mut Window, cx: &App) {
        self.sidebar_feed.tabs_on = settings_in(&self.profile, cx).terminals_in == TerminalsIn::Tabs;
        let row = style::TITLE_ROW_H;
        if self.sidebar_feed.lights_row != Some(row) {
            self.sidebar_feed.lights_row = Some(row);
            window.set_traffic_light_position(traffic_light_position(row));
        }
    }

    // ---------------------------------------------------------------- drawing

    /// The sidebar, `width` points wide (its column's, or its width at rest while it slides): the
    /// window's header over the list, which is the sidebar's, and the first-run note under it.
    pub(super) fn render_sidebar(
        &mut self,
        width: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = ui_in(&self.profile, cx);
        let glass = self.glass_on(cx);
        self.push_sidebar(width, cx);
        div()
            .id("sidebar")
            .relative()
            .flex()
            .flex_col()
            .w(px(width))
            // Long, untruncated text must never widen the panel.
            .min_w(px(width))
            .max_w(px(width))
            .overflow_hidden()
            .h_full()
            .flex_none()
            .bg(if glass { gpui_kit::transparent_black() } else { ui.panel })
            .child(self.render_sidebar_header(window, cx))
            // The list is a view of its own, drawn after this render returns, from what was pushed above.
            .child(self.sidebar.clone())
            .children(self.render_no_agents_note(cx))
            .into_any_element()
    }

    /// The tab bar, when the terminals are in tabs.
    pub(super) fn render_tab_bar(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        self.push_sidebar(0.0, cx);
        self.sidebar.update(cx, |sidebar, cx| sidebar.render_tab_bar(window, cx))
    }

    /// The app's profiles for the chip, while there are two or more to choose between.
    pub(super) fn profiles_for_chip(cx: &App) -> Option<Profiles> {
        let list = &app(cx).profiles;
        (list.profiles().len() >= 2)
            .then(|| Profiles { list: list.profiles().to_vec(), active: list.active_id().clone() })
    }

    /// Whether the profile chip is drawn: two or more profiles.
    #[cfg(test)]
    pub(super) fn profile_chip_shown(&self, cx: &App) -> bool {
        Self::profiles_for_chip(cx).is_some()
    }

    /// The profile chip, for the sidebar's header.
    pub(super) fn render_profile_chip(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        self.sidebar.update(cx, |sidebar, cx| sidebar.render_profile_chip(cx))
    }

    /// `button` (a +) with the shells' menu under a right-click: choosing one opens a terminal with it.
    pub(super) fn with_shell_menu(
        &self,
        id: &'static str,
        button: impl IntoElement,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let this = cx.weak_entity();
        shells::with_shell_menu(
            id,
            button,
            std::rc::Rc::new(move |key, window, cx| {
                this.update(cx, |ws, cx| ws.new_session_with(Some(key), window, cx)).ok();
            }),
        )
    }

    // ---------------------------------------------------------------- the keys and the hold

    /// The modifier keys changed (from the root's `on_modifiers_changed`).
    pub(super) fn on_modifiers(
        &mut self,
        event: &ModifiersChangedEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sidebar.update(cx, |sidebar, cx| sidebar.modifiers(event, window, cx));
    }

    /// The window lost focus: whatever is held is not for this window.
    pub(super) fn hints_blur(&mut self, cx: &mut Context<Self>) {
        self.sidebar.update(cx, |sidebar, cx| sidebar.blur(cx));
    }

    /// Coming into the list (F6, ⇥): the keys go back where they were, or onto the selected terminal.
    pub(super) fn enter_sidebar(&mut self, cx: &mut Context<Self>) {
        self.with_sidebar(cx, |sidebar, cx| sidebar.enter(cx));
    }

    // ---------------------------------------------------------------- the tick

    /// The sidebar's part of the window's 400 ms tick: what moves on its own (a row's times), and
    /// which repository a worktree belongs to, read from disk now and then.
    pub(super) fn sidebar_tick(&mut self, window_active: bool, cx: &mut Context<Self>) {
        if self.ticks % 5 == 1 {
            self.sidebar_feed.worktrees.borrow_mut().clear();
        }
        // Only what is on screen: the times on the rows (the tab bar draws them too).
        let shown = self.panes.sidebar_open || self.sidebar_feed.tabs_on;
        // The times on the rows ("Working · 3m", a command's clock) move on their own; redraw
        // when one would read differently, but not for a window nobody is looking at.
        if window_active && shown {
            let now = row_times(&self.row_infos(cx), &self.profile, cx);
            if self.sidebar.update(cx, |sidebar, _| sidebar.times_changed(now)) {
                cx.notify();
            }
        }
    }

    // ---------------------------------------------------------------- what the list asks

    /// The list asks for something (`SidebarEvent`). Handled when the update that raised it ends, so
    /// nothing here runs while the sidebar is being updated.
    pub(super) fn on_sidebar_event(
        &mut self,
        _: &Entity<Sidebar>,
        event: &SidebarEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            SidebarEvent::Changed => self.mark_dirty(cx),
            &SidebarEvent::Activate { id, then_focus_list } => {
                if let Some(ix) = self.find_session(id) {
                    self.activate(ix, window, cx);
                    if then_focus_list {
                        // Space shows it while the keys keep browsing.
                        let list = self.sidebar.read(cx).focus_handle().clone();
                        window.focus(&list, cx);
                    }
                }
            }
            &SidebarEvent::Close(id) => {
                if let Some(ix) = self.find_session(id) {
                    self.request_close_session(ix, window, cx);
                }
            }
            SidebarEvent::NewTerminal(kind) => match kind {
                NewTerminal::Plain => self.new_session(window, cx),
                NewTerminal::Shell(key) => self.new_session_with(Some(key.clone()), window, cx),
                &NewTerminal::InGroup(group) => self.new_session_in_group(group, window, cx),
                NewTerminal::InNewGroup => {
                    self.new_session(window, cx);
                    if let Some(id) = self.sessions.get(self.active).map(|session| session.id) {
                        self.group_terminals(&[id], cx);
                    }
                }
            },
            &SidebarEvent::SplitRight(id) => {
                if let Some(ix) = self.find_session(id) {
                    self.activate(ix, window, cx);
                    self.split_pane(Axis::Row, window, cx);
                }
            }
            &SidebarEvent::LeaveSplit(id) => {
                if let Some(ix) = self.find_session(id) {
                    self.leave_split(ix, cx);
                }
            }
            &SidebarEvent::TogglePin(id) => {
                if let Some(ix) = self.find_session(id) {
                    self.toggle_pin(ix, cx);
                }
            }
            &SidebarEvent::ToggleForgetText(id) => {
                if let Some(ix) = self.find_session(id) {
                    self.toggle_forget_text(ix, cx);
                }
            }
            &SidebarEvent::ShowBeside { other, id } => self.show_beside(other, id, window, cx),
            &SidebarEvent::RowMenu { id, at } => {
                let beside = self.beside_choices(id, cx);
                self.sidebar.update(cx, |sidebar, cx| sidebar.show_row_menu(id, at, beside, window, cx));
            }
            &SidebarEvent::Renamed { id, ref typed } => {
                if let (Some(typed), Some(ix)) = (typed, self.find_session(id)) {
                    // Emptied, it is called what runs in it again; left as it was, it keeps no name of its own.
                    if typed.is_empty() {
                        self.sessions[ix].custom_name = None;
                    } else if typed != self.sessions[ix].name().as_ref() {
                        self.sessions[ix].custom_name = Some(typed.clone());
                    }
                    self.mark_dirty(cx);
                }
                self.focus_terminal(window, cx);
                cx.notify();
            }
            SidebarEvent::FocusTerminal => self.focus_terminal(window, cx),
            &SidebarEvent::CloseGroup(group) => self.close_group(group, window, cx),
            &SidebarEvent::MoveGroupToWindow(group) => self.move_group_to_window(group, window, cx),
            &SidebarEvent::TerminalDragStarted { id, origin } => {
                self.terminal_drag = Some(TerminalDrag::new(id, origin));
                cx.notify();
            }
            &SidebarEvent::GroupDragStarted(group) => {
                self.group_drag = Some(group);
                cx.notify();
            }
            SidebarEvent::GroupDragEnded => {
                self.group_drag = None;
                cx.notify();
            }
            SidebarEvent::TerminalDropped { dragged, origin, over } => {
                self.finish_terminal_drop(*dragged, *origin, *over, window, cx)
            }
            &SidebarEvent::StepStop(delta) => self.step_stop(delta, window, cx),
        }
    }

    // ---------------------------------------------------------------- terminals and groups

    /// A new terminal in group `group` (its + or its menu), in the group's folder if it has one,
    /// else where the selected terminal is.
    pub(super) fn new_session_in_group(&mut self, group: u32, window: &mut Window, cx: &mut Context<Self>) {
        let cwd = self.start_cwd(Some(group), cx);
        self.open_session_in(Some(Some(group)), None, cwd, window, cx);
        self.mark_dirty(cx);
    }

    /// A new group holding `terminals` (and the rest of their splits). Its id.
    pub(crate) fn group_terminals(&mut self, terminals: &[i64], cx: &mut Context<Self>) -> u32 {
        self.with_sidebar(cx, |sidebar, cx| sidebar.group_terminals(terminals, cx))
    }

    /// Fold the group, or open it.
    #[cfg(any(test, feature = "devctl"))]
    pub(super) fn toggle_group_band(&mut self, group: u32, cx: &mut Context<Self>) {
        self.sidebar.update(cx, |sidebar, cx| sidebar.toggle_group_band(group, cx));
    }

    /// Pin the group, or let it go.
    #[cfg(any(test, feature = "devctl"))]
    pub(super) fn toggle_group_pin(&mut self, group: u32, cx: &mut Context<Self>) {
        self.sidebar.update(cx, |sidebar, cx| sidebar.toggle_group_pin(group, cx));
    }

    /// Terminal `id` is renamed in its row (a field with the name selected).
    #[cfg(any(test, feature = "devctl"))]
    pub(super) fn start_terminal_rename(&mut self, id: i64, window: &mut Window, cx: &mut Context<Self>) {
        self.with_sidebar(cx, |sidebar, cx| sidebar.start_terminal_rename(id, window, cx));
    }

    /// The welcome in `tab` says its sidebar step is up (`show`) or was left.
    pub(crate) fn set_welcome_demo(&mut self, tab: i64, show: bool, cx: &mut Context<Self>) {
        let in_window = self.sessions.iter().any(|session| session.id == tab);
        self.with_sidebar(cx, |sidebar, cx| sidebar.set_welcome_demo(tab, show, in_window, cx));
    }

    /// Take the example band away if it is `tab`'s.
    pub(super) fn end_welcome_demo(&mut self, tab: i64, cx: &mut Context<Self>) {
        self.sidebar.update(cx, |sidebar, _| sidebar.end_welcome_demo(tab));
    }

    /// The example rows as drawn now (devctl and the tests).
    #[cfg(any(test, feature = "devctl"))]
    pub(super) fn welcome_demo_rows(
        &self,
        cx: &App,
    ) -> Option<[wormhole_core::onboarding::welcome::demo::Row; 3]> {
        self.sidebar.read(cx).welcome_demo_rows(&self.sidebar_input(0.0, cx), cx)
    }

    /// ⌃⌥] and ⌃⌥[: the next or previous group, in the switcher's order. Both layouts.
    pub(super) fn step_group(&mut self, forward: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.with_sidebar(cx, |sidebar, cx| sidebar.step_group(forward, cx));
    }

    /// ⌃⌥1–9: the group with that number in the switcher's list.
    pub(super) fn go_to_group(&mut self, number: u8, _: &mut Window, cx: &mut Context<Self>) {
        self.with_sidebar(cx, |sidebar, cx| sidebar.go_to_group(number, cx));
    }
}
