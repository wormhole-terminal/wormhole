//! A workspace's part in several windows: made in a window (ids from the registry's counter, found by the
//! control line), filled from the window's saved record, and terminals taken out of it and into another
//! with their programs running. The bookkeeping is core's (`WorkspaceModel::restore`, `take_out`, `receive`);
//! what this adds is what only the shell owns: the running `Terminal`s and their feeds.

use std::sync::{Arc, Mutex, PoisonError};

use wormhole_core::workspace::Departing;

use super::registry::Windows;
use crate::terminal::Terminal;
use crate::workspace::{Workspace, WorkspaceConfig, WorkspaceEvent, WorkspaceListener};
use crate::workspace_feed;

/// What leaves: the records and the running terminals under their ids.
type Cargo = (Departing, Vec<(i64, Arc<Terminal>)>);

/// Terminals taken out of one window, running, until another takes them in (or they are dropped, which ends them).
#[derive(uniffi::Object)]
pub struct Departure {
    cargo: Mutex<Option<Cargo>>,
    ids: Vec<i64>,
}

#[uniffi::export]
impl Departure {
    /// The terminals on the way, in the window's order.
    pub fn ids(&self) -> Vec<i64> {
        self.ids.clone()
    }
}

/// What `Workspace::take_out` did: what left (none when this window had none of them) and the events of the
/// close that followed, which the window handles as any other (`closed`, a new terminal's `opened`).
#[derive(uniffi::Record)]
pub struct TakenOut {
    pub departure: Option<Arc<Departure>>,
    pub events: Vec<WorkspaceEvent>,
}

#[uniffi::export]
impl Workspace {
    /// A workspace that is window `window` of `windows`: its terminal and split ids are the app's, never the
    /// same as another window's, and the control line finds its terminals.
    #[uniffi::constructor]
    pub fn in_window(
        windows: Arc<Windows>,
        window: i64,
        config: WorkspaceConfig,
        listener: Arc<dyn WorkspaceListener>,
    ) -> Arc<Self> {
        let workspace = Self::build(config, listener, Some(windows.ids()));
        // Set once, here: the workspace was made a moment ago. Before it is registered: whoever hears of the new window
        // (the control socket's publishers) reads its id.
        workspace.shared().set_windows(windows.clone(), window);
        windows.register(window, workspace.shared());
        workspace
    }

    /// Fill this window from what it was saved with (or what it was planned with): a terminal for each of its
    /// sessions, `opened` for every one, its groups, split panes, columns and selection. Nothing when the
    /// window has no record, or already has terminals.
    pub fn restore(&self) -> Vec<WorkspaceEvent> {
        let Some((windows, window)) = self.shared().windows() else { return Vec::new() };
        let Some(record) = windows.record_of(window) else { return Vec::new() };
        let keeps_text =
            wormhole_core::config::Config::load_for(windows.profile()).settings.restore_terminal_text;
        self.shared().restored_texts.keep(&record, keeps_text);
        let mut events = self.shared().with_model(|model| model.restore(&record));
        // The files each terminal had changed come back with it, so its row counts them again.
        let ids: Vec<i64> = record.sessions.iter().map(|session| session.id).collect();
        let claims = windows.claims_of(window, &ids);
        if !claims.is_empty() {
            events.extend(self.shared().with_model(|model| model.git_restore_claims(claims)));
        }
        self.shared().pump.kick();
        self.shared().close_what_closed(events)
    }

    /// Take terminals `ids` out of this window for another: their programs keep running, here they are as good as
    /// closed (a window left with none starts a new one).
    pub fn take_out(&self, ids: Vec<i64>) -> TakenOut {
        let taken = self.shared().with_model(|model| model.take_out(&ids));
        let Some((departing, events)) = taken else {
            return TakenOut { departure: None, events: Vec::new() };
        };
        // Unregistered before the close is told: `Closed` ends a registered terminal's program.
        let terminals: Vec<(i64, Arc<Terminal>)> = departing
            .ids()
            .into_iter()
            .filter_map(|id| self.shared().take_terminal(id).map(|terminal| (id, terminal)))
            .collect();
        let ids = departing.ids();
        let departure = Arc::new(Departure { cargo: Mutex::new(Some((departing, terminals))), ids });
        self.shared().pump.kick();
        TakenOut { departure: Some(departure), events: self.shared().close_what_closed(events) }
    }

    /// Take in what another window gave up: the terminals join the end of the list, running (`adopted`), the one
    /// that was selected there is selected here. A departure is taken in once.
    pub fn receive(&self, departure: Arc<Departure>) -> Vec<WorkspaceEvent> {
        let Some((departing, terminals)) =
            departure.cargo.lock().unwrap_or_else(PoisonError::into_inner).take()
        else {
            return Vec::new();
        };
        for (id, terminal) in &terminals {
            self.shared().put_terminal(*id, terminal.clone());
        }
        let events = self.shared().with_model(|model| model.receive(&departing));
        // Their events feed this window's model from now on, not the one they left.
        for (id, terminal) in &terminals {
            workspace_feed::attach(self.shared(), *id, terminal);
        }
        self.shared().pump.kick();
        self.shared().close_what_closed(events)
    }
}
