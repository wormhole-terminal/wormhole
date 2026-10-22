//! Terminals that run in session hosts (`repo-docs/docs/session-host.md`, sections 10 and 11), as
//! the windows see them: which start a host and which stay in the app's process, the hosts found
//! at launch (attached to, or adopted into a Recovered group), which are fed while on screen, and
//! letting go of every host as the app quits, so their programs go on running.

use std::path::PathBuf;
use std::sync::OnceLock;

use gpui_kit::*;
use wormhole_core::config::profile::ProfileList;
use wormhole_core::ipc::hosts::{self, HostMeta, LaunchHosts, Restore};
use wormhole_core::storage::paths::DataRoot;
use wormhole_core::storage::store::SessionRecord;
use wormhole_core::storage::store::Store;
use wormhole_core::terminal::SpawnOptions;

use crate::app_state::{AppState, app, app_mut};
use crate::terminal_view::Start;
use crate::terminal_view::TerminalView;
use crate::window::Workspace;

/// The host executable, looked for once: it sits beside the app's, which doesn't move while it runs.
fn host_binary() -> Option<&'static PathBuf> {
    static BINARY: OnceLock<Option<PathBuf>> = OnceLock::new();
    BINARY.get_or_init(wormhole_core::ipc::host_binary::host_binary).as_ref()
}

/// The hosts this data folder has at launch, matched against what every profile's database names:
/// one scan, before any window opens. `live` is the shown profile's open database.
pub(crate) fn scan_at_launch(root: &DataRoot, profiles: &ProfileList, live: Option<&Store>) -> LaunchHosts {
    let found = match hosts::scan(&hosts::hosts_dir(&wormhole_core::storage::paths::data_dir())) {
        Ok(scan) => scan.live,
        Err(err) => {
            tracing::warn!("couldn't look for running session hosts: {err}");
            return LaunchHosts::default();
        }
    };
    if found.is_empty() {
        return LaunchHosts::default();
    }
    let mut named: Vec<String> = Vec::new();
    let mut add = |store: &Store| match store.host_ids() {
        Ok(ids) => named.extend(ids.into_iter().map(|(_, id)| id)),
        Err(err) => tracing::warn!("reading which hosts the terminals run in: {err:#}"),
    };
    if let Some(store) = live {
        add(store);
    }
    // A host another profile's terminal runs in is that profile's, not an orphan: its database is
    // opened for the ids only, as `first_free_ids` does.
    for profile in profiles.profiles().iter().filter(|p| &p.id != profiles.active_id()) {
        let paths = root.profile(&profile.id);
        if Store::exists_for(&paths) {
            match Store::open_for(&paths) {
                Ok(store) => add(&store),
                Err(err) => tracing::warn!("reading the hosts of profile {}: {err:#}", profile.id),
            }
        }
    }
    tracing::info!(live = found.len(), named = named.len(), "session hosts at launch");
    LaunchHosts::plan(named.iter().map(String::as_str), found)
}

/// After the windows opened at launch: the live hosts no terminal named come into the front window's
/// Recovered group, attached headless. Never ended: what runs in them is the person's.
pub(crate) fn adopt_orphans(cx: &mut App) {
    let orphans = app_mut(cx).launch_hosts.take_orphans();
    if orphans.is_empty() {
        return;
    }
    let Some(front) = AppState::front_window(cx) else { return };
    AppState::with_workspace(&front, cx, |ws, window, cx| ws.adopt_orphans(orphans, window, cx));
}

/// As the app quits, before anything drops a terminal (GPUI drops the windows, and with them every
/// terminal, right after the quit observers run): let go of every host, so dropping a hosted `Pty`
/// leaves its program running instead of ending it.
pub(crate) fn detach_all(cx: &mut App) {
    // First: a start or attach that finishes after this lets go of its host too.
    crate::terminal_view::note_app_quitting();
    let mut terminals: Vec<Entity<TerminalView>> = Vec::new();
    for window in &app(cx).windows {
        if let Some(workspace) = window.workspace.upgrade() {
            terminals.extend(workspace.read(cx).sessions.iter().map(|s| s.terminal().clone()));
        }
    }
    // On their way to another window (a group moved): not in any workspace for the moment.
    terminals.extend(app(cx).adopted.values().cloned());
    let mut detached = 0;
    for terminal in terminals {
        detached += usize::from(terminal.update(cx, |terminal, _| terminal.detach_host()));
    }
    if detached > 0 {
        tracing::info!(detached, "left the session hosts running on quit");
    }
}

impl Workspace {
    /// How the terminal of `record` starts: attached to the host its row names when that runs, in
    /// a new host when persistent terminals are on and one can start, else in the app's process.
    /// Restored text (`record.scrollback`) is replayed only in the app's process, so a terminal
    /// restored without a live host stays there, as it would without hosts.
    pub(super) fn start_for(
        &mut self,
        record: &SessionRecord,
        options: &SpawnOptions,
        cx: &mut App,
    ) -> Start {
        if let Some(id) = record.host_id.as_deref()
            && let Restore::Attach(socket) = app_mut(cx).launch_hosts.restore(Some(id))
        {
            let visible = self.shown_at_launch.contains(&record.id);
            return Start::Attach { socket, id: id.to_string(), visible };
        }
        if record.scrollback.is_some() || !self.may_host(options, cx) {
            return Start::Local;
        }
        let Some(binary) = host_binary() else { return Start::Local };
        match hosts::new_host_id() {
            Ok(id) => Start::Host {
                binary: binary.clone(),
                data_dir: wormhole_core::storage::paths::data_dir(),
                id,
            },
            Err(err) => {
                tracing::warn!("no id for a session host, the terminal runs in the app: {err}");
                Start::Local
            }
        }
    }

    /// Whether a new terminal may start in a session host. Welcome terminals and unit tests stay in
    /// the app's process, and so does anything started once the app is quitting. A devctl
    /// instance's `WORMHOLE_SHELL` stand-in (`/bin/cat`) is hosted like any shell, so checks see
    /// hosting; `devctl-instance.sh stop` ends the instance's hosts.
    fn may_host(&self, options: &SpawnOptions, cx: &App) -> bool {
        !cfg!(test)
            && crate::window::windows::persistent_terminals_on(cx)
            && !options.welcome
            && !app(cx).quitting
    }

    /// A terminal just started a session host: its row, naming the host, goes to the database now,
    /// not on the next save, so a crash right after leaves a row the next launch attaches by.
    pub(super) fn save_host_row(&mut self, ix: usize, host: &str, cx: &mut App) {
        if self.discarded {
            return;
        }
        let record = self.sessions[ix].record(cx);
        app_mut(cx).save_session_host(&self.profile, self.window_id, &record, host);
    }

    /// The terminals on screen are fed; the rest are headless until they are shown again.
    pub(super) fn sync_host_visibility(&mut self, cx: &mut App) {
        for ix in 0..self.sessions.len() {
            let on_screen = self.on_screen(ix);
            self.sessions[ix].terminal().update(cx, |terminal, _| terminal.set_on_screen(on_screen));
        }
    }

    /// Live hosts no saved terminal named: each becomes a terminal of this window, attached
    /// headless, in the Recovered group.
    pub(super) fn adopt_orphans(
        &mut self,
        orphans: Vec<HostMeta>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut ids = Vec::with_capacity(orphans.len());
        for meta in orphans {
            tracing::info!(id = meta.id, pid = meta.pid, "adopting a session host no terminal named");
            let mut record = SessionRecord::new(self.alloc_id(cx), None, None);
            record.host_id = Some(meta.id);
            ids.push(record.id);
            self.add_session(Some(record), window, cx);
        }
        self.sidebar.view.groups.recover(&ids);
        self.dirty = true;
        cx.notify();
    }

    /// devctl `detach`: let go of every hosted terminal of this window, leaving the views. How many.
    #[cfg(feature = "devctl")]
    pub(crate) fn detach_hosts(&mut self, cx: &mut Context<Self>) -> usize {
        let terminals: Vec<_> = self.sessions.iter().map(|s| s.terminal().clone()).collect();
        terminals.into_iter().filter(|t| t.update(cx, |t, _| t.detach_host())).count()
    }

    /// devctl `reattach`: attach again to the hosts let go of with `detach`, as at launch. Their ids.
    #[cfg(feature = "devctl")]
    pub(crate) fn reattach_hosts(&mut self, cx: &mut Context<Self>) -> Vec<String> {
        let terminals: Vec<_> = self.sessions.iter().map(|s| s.terminal().clone()).collect();
        terminals.into_iter().filter_map(|t| t.update(cx, |t, cx| t.reattach(cx))).collect()
    }
}
