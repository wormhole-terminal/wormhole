//! A window's terminals: opening one (its shell, folder and saved text), selecting and closing it,
//! and finding them by folder, project or ⌘-number for the palette and the menu bar.
//!
//! Part of the window: it must not use the surfaces above it (the command palette, the shortcut
//! sheet, the Settings and Studio windows, the status item, the app root); they call these.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use gpui_kit::*;
use wormhole_core::storage::paths::ProfileId;
use wormhole_core::storage::store::SessionRecord;
use wormhole_core::terminal::SpawnOptions;
use wormhole_core::terminal::shells::ShellCommand;

use crate::app_state::attention::{self, Attention};
use crate::app_state::{AppState, app, app_mut};
use crate::settings::settings_in;
use crate::terminal_view::TerminalView;
use crate::window::Workspace;
use crate::window::feature::SpawnAdd;
use crate::window::repo::GitState;
use crate::window::session::{Pane, Session};

impl Workspace {
    pub(crate) fn new_session(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.new_session_with(None, window, cx);
    }

    /// A new terminal running the shell `shell` names (`None`: the default), where the current
    /// one is, or in its group's folder if the group has one (it joins that group).
    pub(crate) fn new_session_with(
        &mut self,
        shell: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let cwd = self.start_cwd(self.active_group(cx), cx);
        self.open_session(shell, cwd, window, cx);
    }

    /// A new terminal in `cwd` running `shell` (a profile's key; `None`: the default), selected.
    pub(in crate::window) fn open_session(
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
    pub(in crate::window) fn open_session_in(
        &mut self,
        group: Option<Option<u32>>,
        shell: Option<String>,
        cwd: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> usize {
        let mut record = SessionRecord::new(self.alloc_id(cx), None, cwd);
        record.shell = shell;
        let group = group.unwrap_or_else(|| self.active_group(cx));
        let id = record.id;
        let ix = self.add_session(Some(record), window, cx);
        if let Some(group) = group {
            let ids = self.session_ids();
            self.edit_sidebar_view(cx, |view| view.place_in_group(&ids, id, group, None));
        }
        self.activate(ix, window, cx);
        self.dirty = true;
        ix
    }

    pub(crate) fn add_session(
        &mut self,
        record: Option<SessionRecord>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> usize {
        let number = self.next_number;
        self.next_number += 1;
        let mut record = record.unwrap_or_else(|| {
            let id = self.alloc_id(cx);
            SessionRecord::new(id, None, self.default_cwd(cx))
        });
        // Text the person asked not to keep is never shown again, whatever the file still holds.
        if record.forget_text || !settings_in(&self.profile, cx).restore_terminal_text {
            record.scrollback = None;
        }
        // A terminal that moved here from another window keeps running and is not a new one.
        let adopted = app(cx).adopted.contains_key(&record.id);
        let (pane, new_host) = self.make_pane(&record, window, cx);
        self.sessions.push(Session {
            id: record.id,
            number,
            custom_name: record.name.clone(),
            pane,
            split: record.split_id,
            git: GitState::default(),
            repo_root: Default::default(),
            pinned: record.pinned,
            forget_text: record.forget_text,
            extras: Default::default(),
        });
        let ix = self.sessions.len() - 1;
        self.features_opened(&record, window, cx);
        if !adopted {
            self.publish_terminal_opened(record.id, record.cwd.as_deref(), cx);
        }
        if let Some(host) = new_host {
            self.save_host_row(ix, &host, cx);
        }
        ix
    }

    /// Name terminal `ix`, or, with none, call it what runs in it again: the sidebar's rename and
    /// the devctl `rename` both end here, and so does telling those who watch.
    pub(crate) fn set_custom_name(&mut self, ix: usize, name: Option<String>, cx: &mut Context<Self>) {
        let Some(session) = self.sessions.get_mut(ix) else { return };
        session.custom_name = name;
        self.mark_dirty(cx);
        let session = &self.sessions[ix];
        self.publish_renamed(session.id, session.custom_name.as_deref(), cx);
    }

    pub(super) fn alloc_id(&self, cx: &mut App) -> i64 {
        AppState::alloc_id_in(&self.profile, cx)
    }

    /// Where a new terminal starts: where the current one is.
    pub(crate) fn default_cwd(&self, cx: &App) -> Option<String> {
        self.sessions
            .get(self.active)
            .and_then(|s| s.terminal().read(cx).cwd.clone())
            .map(|p| p.to_string_lossy().into_owned())
            // Dev/test override for where the first session starts.
            .or_else(|| std::env::var("WORMHOLE_START_DIR").ok())
    }

    /// Start the shell of a session's terminal, with the id the session has: the profile it was
    /// opened with, else a trusted project's shell (its env applies too), else the default (core's
    /// `shells::for_terminal`), in the folder and with the scrollback the record has. In a session
    /// host when it should run in one (`start_for`): then the new host's id comes back too, for the
    /// row that names it.
    pub(super) fn make_pane(
        &mut self,
        record: &SessionRecord,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (Pane, Option<String>) {
        // A terminal moved here from another window keeps running, and listens to this window now.
        if let Some(terminal) = app_mut(cx).adopted.remove(&record.id) {
            let zoom = self.font_zoom;
            terminal.update(cx, |terminal, cx| {
                terminal.moved_to(window, cx);
                terminal.adopt_font_zoom(zoom, cx);
            });
            return (Pane::new(record.id, record.shell.clone(), terminal, window, cx), None);
        }
        let cwd = record.cwd.as_ref().map(PathBuf::from);
        // What the features add to a terminal started in the folder (a trusted project's environment and
        // shell, a worktree's ports).
        let SpawnAdd { env, shell: project_shell } =
            Self::spawn_adds(&self.profile, cwd.as_deref(), self.features, cx);
        let shell = wormhole_core::terminal::shells::for_terminal(
            record.shell.as_deref(),
            project_shell.as_deref(),
            settings_in(&self.profile, cx),
        );
        let mut options = spawn_options(record.id, cwd, record.scrollback.clone(), env, shell);
        // After the project's environment is in: what a trusted project set is kept. The profile's
        // own environment is the person's (not repository content), so it applies in an untrusted
        // project as well.
        if let Some(profile) = app(cx).profiles.get(&self.profile) {
            options.apply_profile(profile);
        }
        // A welcome terminal: the shell starts through `Wormhole welcome`, and only this terminal's
        // environment has the token that lets that program talk to the app as the welcome.
        if let Some(token) = crate::onboarding::welcome::take_token(record.id, cx) {
            // (Not in the app's own tests: their binary is not Wormhole and would take `welcome` for
            // a test name.)
            options.welcome = !cfg!(test);
            options.env.insert(wormhole_core::onboarding::welcome::TOKEN_VAR.to_string(), token);
        }
        let start = self.start_for(record, &options, cx);
        let new_host = match &start {
            crate::terminal_view::Start::Host { id, .. } => Some(id.clone()),
            _ => None,
        };
        let terminal = cx.new(|cx| TerminalView::new(options, start, self.profile.clone(), window, cx));
        let zoom = self.font_zoom;
        terminal.update(cx, |terminal, cx| terminal.adopt_font_zoom(zoom, cx));
        (Pane::new(record.id, record.shell.clone(), terminal, window, cx), new_host)
    }

    /// Select terminal `ix` and give it the keyboard. Selecting a row of a split shows the split
    /// with that pane focused.
    pub(crate) fn activate(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        if ix >= self.sessions.len() {
            return;
        }
        self.select(ix, cx);
        // The cell of the terminal now shown is what the window steps by.
        self.sync_resize_increments(window, cx);
        window.focus(&self.sessions[ix].terminal().focus_handle(cx), cx);
        cx.notify();
    }

    /// Select terminal `ix` without moving the keyboard (it is already there, or goes there next):
    /// what belongs to the terminal on screen starts fresh, its split shows with its pane focused,
    /// and what is on screen counts as seen.
    pub(super) fn select(&mut self, ix: usize, cx: &mut Context<Self>) {
        if self.active != ix {
            // A feature's field belonged to the terminal it was opened for.
            self.feature_editing = false;
            self.features_selected(self.sessions[ix].id, cx);
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
        // At once, not on the next tick: a terminal switched to shows its last screen and is fed
        // from here on.
        self.sync_host_visibility(cx);
    }

    /// Close terminal `ix`: its shell ends. In a split its pane goes and the others take its room;
    /// one left alone becomes a plain row. The selection stays in the split when the closed one
    /// was selected (the keyboard goes where the layout sends it: the next pane in reading order).
    pub(crate) fn close_session(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
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
        self.leave_layout(ix, cx);
        let after = crate::window::session::session_after_close(self.active, ix, len);
        let closed = self.sessions.remove(ix);
        self.edit_sidebar_view(cx, |view| view.groups.leave(closed.id));
        self.features_closed(closed.id, cx);
        self.forget_changes(closed.id, cx);
        self.publish_terminal_closed(closed.id, cx);
        attention::tell(Attention::Seen(closed.pane.id), cx);
        if crate::onboarding::welcome::forget(closed.id, cx) {
            crate::window::remember_first_run(cx);
        }
        self.end_welcome_demo(closed.id, cx);
        self.dirty = true;
        // Selecting a session from `usize::MAX` is what resets what belongs to the session on
        // screen (a feature's diff or picker), so only a real change of session
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
    fn acknowledge_selected(&mut self, cx: &mut Context<Self>) {
        for ix in self.on_screen_sessions() {
            let pane = &self.sessions[ix].pane;
            pane.terminal.update(cx, |t, _| t.acknowledge());
            let id = pane.id;
            self.sync_summary_at(ix, cx);
            attention::tell(Attention::Seen(id), cx);
        }
    }

    pub(crate) fn active_session(&self) -> &Session {
        &self.sessions[self.active]
    }

    /// The selected terminal's name, as the sidebar shows it.
    pub(crate) fn active_session_name(&self) -> SharedString {
        self.active_session().name()
    }

    /// The profile this window belongs to.
    pub(crate) fn profile(&self) -> &ProfileId {
        &self.profile
    }

    /// The folder the active terminal belongs to, as the sidebar's bands name it: its repository, else
    /// the folder itself. `None` until its shell has said where it is.
    #[cfg(any(test, feature = "devctl"))]
    pub(crate) fn active_folder(&self, cx: &App) -> Option<PathBuf> {
        let cwd = self.active_terminal().read(cx).cwd.clone()?;
        Some(self.active_session().repo_root.of(&cwd))
    }

    pub(crate) fn active_terminal(&self) -> Entity<TerminalView> {
        self.active_session().terminal().clone()
    }

    /// The sidebar's ⌘1–9, by session id.
    pub(crate) fn palette_numbers(&self, cx: &App) -> Vec<(i64, u8)> {
        let numbered = self.session_listing(cx).numbered();
        numbered.iter().enumerate().map(|(n, &ix)| (self.sessions[ix].id, n as u8 + 1)).collect()
    }

    /// The folder each terminal is in, as the sidebar groups them: its repository, else the
    /// folder itself.
    pub(crate) fn session_folders(&self, cx: &App) -> Vec<PathBuf> {
        self.sessions
            .iter()
            .filter_map(|session| {
                let cwd = session.terminal().read(cx).cwd.clone()?;
                Some(session.repo_root.of(&cwd))
            })
            .collect()
    }

    /// The first terminal in `folder` (as [`Self::session_folders`] says), if any.
    pub(crate) fn session_in_folder(&self, folder: &Path, cx: &App) -> Option<usize> {
        self.sessions.iter().position(|session| {
            session.terminal().read(cx).cwd.as_deref().is_some_and(|cwd| session.repo_root.of(cwd) == folder)
        })
    }

    /// The folder terminal `tab` is in, if this window has it and it has reported one.
    pub(crate) fn terminal_cwd(&self, tab: i64, cx: &App) -> Option<PathBuf> {
        let ix = self.find_session(tab)?;
        self.sessions[ix].terminal().read(cx).cwd.clone()
    }

    /// How many terminals the window has.
    #[cfg(test)]
    pub(crate) fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// How many files changed in the folder the current terminal is in, once the git panel has
    /// looked there (a count for another folder, or none, is `None`).
    pub(crate) fn active_changes(&self, cx: &App) -> Option<usize> {
        let session = self.active_session();
        let cwd = &session.terminal().read(cx).cwd;
        session
            .git
            .status
            .as_ref()
            .filter(|_| &session.git.cwd == cwd)
            .map(|status| status.files.len())
            .filter(|&n| n > 0)
    }
}

/// What a terminal starts with: its folder, the scrollback to restore, the environment and the shell.
fn spawn_options(
    tab_id: i64,
    cwd: Option<PathBuf>,
    restore: Option<String>,
    env: BTreeMap<String, String>,
    shell: ShellCommand,
) -> SpawnOptions {
    SpawnOptions {
        cwd,
        tab_id: tab_id.to_string(),
        restore,
        scrollback_lines: 0,
        allow_screen_checksum: false,
        allow_window_ops: false,
        allow_title_report: false,
        env,
        shell: Some(shell),
        welcome: false,
        colors: None,
        color_scheme: None,
    }
}
