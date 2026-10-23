//! One window's life: opening siblings, closing, saving, and being the one in front.
use std::time::Duration;

use gpui_kit::*;
use wormhole_core::storage::paths::ProfileId;
use wormhole_core::storage::store::{SessionRecord, WindowRecord};
use wormhole_core::view::layout::RememberedPanes;
use wormhole_core::view::sidebar::WindowSidebar;

use crate::app_state::AppState;
use crate::app_state::{app, app_mut};
use crate::notifications;
use crate::settings::settings;
use crate::terminal_view::Status;
use crate::window::session::{self, Session};
use crate::window::window_lifecycle::open_workspace_window_in;
use crate::window::{Layout, Workspace, dialog, window_bounds};

impl Layout {
    /// A new window's layout, made from another's: the same panels, but the sidebar the way a
    /// new window starts (no groups, nothing folded).
    fn for_new_window(json: Option<&str>) -> Option<String> {
        let mut layout: Layout = json.and_then(|json| serde_json::from_str(json).ok()).unwrap_or_default();
        layout.sidebar = WindowSidebar::default();
        serde_json::to_string(&layout).ok()
    }
}

impl Layout {
    /// The layout of a window opened with none saved: the columns as they were last left, the
    /// sidebar the way a new window starts.
    pub(super) fn remembered(last: &RememberedPanes) -> Option<String> {
        let sidebar = WindowSidebar::default();
        serde_json::to_string(&Layout::new(last.panes(), Layout::default().right_tab, sidebar)).ok()
    }
}

impl Workspace {
    /// ⌘N: another window, laid out like this one, with a session starting here.
    pub(crate) fn new_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let cwd = self.default_cwd(cx);
        self.new_window_in(cwd, window, cx);
    }

    /// Another window, laid out like this one, with a session starting in `cwd` (the command
    /// palette's ⌘↩).
    pub(crate) fn new_window_in(&mut self, cwd: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let profile = self.profile.clone();
        self.new_window_in_profile(cwd, profile, window, cx);
    }

    /// [`new_window_in`](Self::new_window_in) in `profile`, which may not be the one this window
    /// (or Wormhole) is in: then the new window is the odd one.
    pub(crate) fn new_window_in_profile(
        &mut self,
        cwd: Option<String>,
        profile: ProfileId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let layout = Layout::for_new_window(self.window_record(cx).layout.as_deref());
        let source = window.bounds();
        // Deferred: the new window renders right away, and nothing may read
        // this workspace while it is still being updated.
        cx.defer(move |cx| {
            let id = app_mut(cx).alloc_window_id();
            let session_id = AppState::alloc_id_in(&profile, cx);
            let record = WindowRecord {
                id,
                layout,
                sessions: vec![SessionRecord::new(session_id, None, cwd)],
                ..Default::default()
            };
            open_workspace_window_in(&profile, Some(record), Some(source), cx);
        });
    }

    /// ⇧⌘W and the close button.
    pub(crate) fn close_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.should_close(window, cx) {
            window.remove_window();
        }
    }

    /// The close button's dot, as Terminal shows it: on while closing the window would ask first
    /// (an agent at work or waiting, or a command running in any of its terminals). Through GPUI,
    /// never AppKit's `setDocumentEdited` directly: changing the mark puts the traffic lights back
    /// where AppKit draws them by default, and GPUI moves them to our position again after it.
    pub(super) fn sync_close_dot(&mut self, window: &mut Window, cx: &App) {
        let dot = self.sessions.iter().any(|s| s.needs_close_confirmation(cx));
        if dot != self.close_dot {
            self.close_dot = dot;
            window.set_window_edited(dot);
        }
    }

    /// Whether the window may close right now. A "no" can mean the app is
    /// quitting instead, or that a question is on screen (which closes the
    /// window itself if it is confirmed).
    pub(crate) fn should_close(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let state = app(cx);
        if state.quitting {
            return true;
        }
        if self.is_last_window(cx) {
            // Closing the last window ends the app, saved like any other quit (so the sessions
            // come back next time), and asks first as quitting does. Deferred: the question reads
            // every window's workspace, this one included.
            cx.defer(request_quit);
            return false;
        }
        // The rule for closing one terminal, for each of them: only while an agent
        // works or waits, or a command runs. Notes and tasks alone never ask.
        let running: Vec<(Status, bool)> = self
            .sessions
            .iter()
            .filter(|s| s.needs_close_confirmation(cx))
            .map(|s| (s.status(cx), s.command_running(cx)))
            .collect();
        if running.is_empty() {
            self.discard(cx);
            return true;
        }
        if self.closing_prompt {
            return false;
        }
        self.closing_prompt = true;
        cx.notify();
        let stakes = self.close_stakes(&self.session_ids(), cx);
        let programs: Vec<String> = self
            .sessions
            .iter()
            .filter(|s| s.needs_close_confirmation(cx))
            .flat_map(|s| s.running_programs(false, cx))
            .collect();
        let question = dialog::CloseQuestion {
            title: "Close this window?".into(),
            detail: session::close_window_detail(self.sessions.len(), &running, &programs, &stakes).into(),
            confirm: "Close Window",
        };
        let (confirm, dismiss) = (cx.weak_entity(), cx.weak_entity());
        dialog::ask_before_closing(
            window,
            cx,
            question,
            move |window, cx| {
                let _ = confirm.update(cx, |this, cx| {
                    // The other windows may have gone while the question was up:
                    // the last one is saved, not discarded.
                    if this.is_last_window(cx) {
                        cx.quit();
                    } else {
                        this.discard(cx);
                        window.remove_window();
                    }
                });
            },
            move |cx| {
                let _ = dismiss.update(cx, |this, cx| {
                    this.closing_prompt = false;
                    cx.notify();
                });
            },
        );
        false
    }

    /// The × on a terminal's row, or ⌘W. Closing a terminal ends its shell, so it asks first
    /// while an agent or a command is running in it; notes and tasks alone don't ask.
    pub(crate) fn request_close_session(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(session) = self.sessions.get(ix) else { return };
        // A terminal in a worktree Wormhole made asks whether the worktree goes with it.
        if let Some(worktree) = self.made_worktree_of(ix, cx) {
            self.request_close_worktree_session(ix, worktree, window, cx);
            return;
        }
        if !session.needs_close_confirmation(cx) {
            self.close_session(ix, window, cx);
            return;
        }
        if self.closing_session_prompt {
            return;
        }
        self.closing_session_prompt = true;
        // The sidebar drops the ×'s tooltip while the question is up.
        cx.notify();
        // By id: sessions may come and go while the question is up.
        let id = session.id;
        let agent = session.told_name(cx);
        let (status, command) = (session.status(cx), session.command_running(cx));
        // The agent the question names is not listed again among the programs.
        let programs = session.running_programs(matches!(status, Status::Busy | Status::Attention), cx);
        let question = dialog::CloseQuestion {
            title: session::close_session_title(&session.name(cx)).into(),
            detail: session::close_session_detail(
                agent.as_deref(),
                &self.close_stakes(&[id], cx),
                status,
                command,
                &programs,
            )
            .into(),
            confirm: "Close",
        };
        let (confirm, dismiss) = (cx.weak_entity(), cx.weak_entity());
        dialog::ask_before_closing(
            window,
            cx,
            question,
            move |window, cx| {
                let _ = confirm.update(cx, |this, cx| {
                    if let Some(ix) = this.sessions.iter().position(|session| session.id == id) {
                        this.close_session(ix, window, cx);
                    }
                });
            },
            move |cx| {
                let _ = dismiss.update(cx, |this, cx| {
                    this.closing_session_prompt = false;
                    cx.notify();
                });
            },
        );
    }

    fn is_last_window(&self, cx: &App) -> bool {
        app(cx).windows.iter().all(|w| w.id == self.window_id || w.hidden)
    }

    /// The window is going away for good: forget its sessions, and stop saving them.
    fn discard(&mut self, cx: &mut Context<Self>) {
        self.discarded = true;
        for session in &self.sessions {
            notifications::dismiss(session.id, cx);
        }
        app_mut(cx).forget_window(&self.profile, self.window_id);
    }

    /// Save the window, its sessions and their scrollback (on the way out).
    pub(crate) fn persist(&mut self, window: &Window, cx: &mut Context<Self>) {
        self.bounds = window_bounds::encode(window.window_bounds());
        self.save(cx);
        self.save_scrollback(cx);
    }

    /// This window just became the one in front: its features look again (the project of its terminal
    /// applies its settings and theme).
    pub(super) fn became_front(&mut self, cx: &mut Context<Self>) {
        app_mut(cx).front = self.window_id;
        self.refresh_features(cx);
        // Coming back after installing git is a retry; polling a missing git is not (on a Mac
        // without the Command Line Tools each run asks to install them). Only the terminal in
        // front: the others ask when they are in front and the window comes back.
        let ix = self.active;
        if self.sessions[ix].git.waits_for_retry() {
            self.refresh_git(ix, cx);
        }
        if self.sessions[ix].history.waits_for_retry() {
            self.refresh_history(ix, cx);
        }
    }

    /// Per tick: remember where the window is, and name it after the session.
    pub(super) fn track_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(moved) = self.bounds_watch.moved(window.window_bounds(), self.bounds.as_deref()) {
            self.bounds = Some(moved);
            self.dirty = true;
        }
        let title = self.active_session().name(cx);
        if title != self.title {
            window.set_window_title(&title);
            self.title = title;
        }
    }

    /// The saved window this is (`OpenWindow` in the shared state has the same id).
    #[cfg(feature = "devctl")]
    pub(crate) fn window_id(&self) -> i64 {
        self.window_id
    }

    /// Whether the terminal a hook names (`WORMHOLE_TAB_ID`, its id) is in this window.
    pub(crate) fn owns_terminal(&self, id: &str) -> bool {
        id.parse().ok().is_some_and(|id| self.find_session(id).is_some())
    }

    pub(super) fn is_front(&self, cx: &App) -> bool {
        app(cx).front == self.window_id
    }

    /// This window's position among the open ones (devctl addresses windows by it).
    #[cfg(feature = "devctl")]
    pub(crate) fn window_number(&self, cx: &App) -> usize {
        app(cx).windows.iter().position(|w| w.id == self.window_id).map_or(1, |ix| ix + 1)
    }
}

/// ⌘Q, Quit in the menus, and closing the last window: quit, asking first while an agent or a
/// command runs in any window (by the rule for closing one). The terminals come back
/// next time; what runs in them doesn't. The Dock's Quit and logging out go to AppKit directly and
/// don't ask.
pub(crate) fn request_quit(cx: &mut App) {
    quit_asking(cx, false);
}

/// Quit and End All Terminals: the same question and the same quit, and the terminals the session
/// hosts keep running end first (they would otherwise outlive the app).
pub(crate) fn request_quit_ending_hosts(cx: &mut App) {
    quit_asking(cx, true);
}

/// Whether the persistent-terminals feature is on (its flag, see core `features`): the menu item
/// that ends the hosts' terminals shows only then.
pub(crate) fn persistent_terminals_on(cx: &App) -> bool {
    settings(cx).feature_enabled(wormhole_core::config::features::FeatureFlag::PersistentTerminals)
}

/// How long one session host gets to end when the app quits and takes them along: its shell's
/// hangup grace (3 s) and a margin.
const END_HOSTS_WAIT: Duration = Duration::from_secs(5);

/// Quit now. With `end_hosts`, every live session host of this data folder is ended first, on a
/// background thread (the sockets and the hang-ups are not for the UI thread), and the app quits
/// when they have answered, each within [`END_HOSTS_WAIT`]. The windows are saved before the
/// terminals go, so what they showed is what comes back as text next time.
fn finish_quit(cx: &mut App, end_hosts: bool) {
    if !end_hosts {
        cx.quit();
        return;
    }
    // Before the first save: from here on no save records a host for any terminal, the ones on the
    // way out included, and nothing starts a new one (`quitting`).
    app_mut(cx).ending_hosts = true;
    crate::window::window_lifecycle::save_all(cx);
    let hosts_dir = wormhole_core::ipc::hosts::hosts_dir(&wormhole_core::storage::paths::data_dir());
    let ending = cx
        .background_executor()
        .spawn(async move { wormhole_core::ipc::hosts::end_all_live(&hosts_dir, END_HOSTS_WAIT) });
    cx.spawn(async move |cx| {
        for (id, result) in ending.await {
            match result {
                Ok(how) => tracing::info!(id, ?how, "ended a session host on quit"),
                Err(why) => tracing::warn!(id, why, "couldn't end a session host on quit"),
            }
        }
        cx.update(|cx| cx.quit());
    })
    .detach();
}

fn quit_asking(cx: &mut App, end_hosts: bool) {
    let state = app(cx);
    if state.quitting {
        return;
    }
    // A second ⌘Q while the question is up leaves it up; a question whose window went is gone.
    if state.quit_prompt.is_some_and(|id| state.windows.iter().any(|w| w.id == id)) {
        return;
    }
    let windows = state.windows.clone();
    let (mut running, mut programs) = (Vec::new(), Vec::new());
    // Which of them are hosted (they go on after a plain quit), and whether any terminal is.
    let (mut hosted_running, mut local_running, mut any_hosted) = (0, 0, false);
    // Terminals of a profile that is not shown run on out of sight: the question names them.
    let mut elsewhere: Vec<(String, usize)> = Vec::new();
    for window in &windows {
        let Some(workspace) = window.workspace.upgrade() else { continue };
        let busy = |s: &&Session| s.needs_close_confirmation(cx);
        any_hosted |= workspace.read(cx).sessions.iter().any(|s| s.is_hosted(cx));
        for session in workspace.read(cx).sessions.iter().filter(busy) {
            // A hosted terminal goes on running after a plain quit, which the question's box can
            // turn into ending it too.
            if session.is_hosted(cx) {
                hosted_running += 1;
            } else {
                local_running += 1;
            }
            running.push((session.status(cx), session.command_running(cx)));
            programs.extend(session.running_programs(false, cx));
            if window.hidden {
                let name = app(cx)
                    .profiles
                    .get(&window.profile)
                    .map_or(window.profile.to_string(), |p| p.name.clone());
                match elsewhere.iter_mut().find(|(n, _)| *n == name) {
                    Some((_, count)) => *count += 1,
                    None => elsewhere.push((name, 1)),
                }
            }
        }
    }
    let front = AppState::front_window(cx);
    let ask = session::quit_ask(end_hosts, hosted_running, local_running, any_hosted);
    let (Some(front), Some(ask)) = (front, ask) else {
        finish_quit(cx, end_hosts);
        return;
    };
    let confirm = if end_hosts { "Quit and End All" } else { "Quit" };
    let title = if end_hosts { "Quit and End All Terminals?" } else { "Quit Wormhole?" };
    let detail = match ask {
        session::QuitAsk::Plain => session::quit_detail(&running, &programs, &elsewhere),
        session::QuitAsk::KeepBox { local_running } => {
            session::quit_detail_keeping(&running, &programs, &elsewhere, hosted_running > 0, local_running)
        }
    };
    let question = dialog::CloseQuestion { title: title.into(), detail: detail.into(), confirm };
    let asked = AppState::with_workspace(&front, cx, |_, window, cx| {
        let on_dismiss = |cx: &mut App| app_mut(cx).quit_prompt = None;
        match ask {
            session::QuitAsk::Plain => dialog::ask_before_closing(
                window,
                cx,
                question,
                move |_, cx| finish_quit(cx, end_hosts),
                on_dismiss,
            ),
            session::QuitAsk::KeepBox { .. } => dialog::ask_keeping_terminals(
                window,
                cx,
                question,
                move |keep_running, _, cx| finish_quit(cx, session::quit_ends_hosts(keep_running)),
                on_dismiss,
            ),
        }
    });
    match asked {
        Some(()) => app_mut(cx).quit_prompt = Some(front.id),
        // The window couldn't take the question: quit as before rather than not at all.
        None => finish_quit(cx, end_hosts),
    }
}
