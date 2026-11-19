//! Which terminal runs in which session host, kept where the GPUI app keeps it: the profile's database
//! (`tabs.host_id`, through core's `Store`). The Session Hosts window and the launch plan both read the
//! same column, so a host a row names is never mistaken for an orphan.
//!
//! Only the rows of hosted terminals are written, one session at a time (`Store::save_session_host`); a
//! window saver that comes later writes the same `SessionRecord`s with `host_id` filled from
//! `Terminal.hostId`, which is why the key here is the window's own id and the terminal's id.

use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

use wormhole_core::storage::store::{SessionRecord, Store, WindowRecord};

use super::{HostingError, SavedTerminal};

pub(crate) struct Registry {
    store: Mutex<Store>,
    window: i64,
}

impl Registry {
    /// The profile database in `dir`, with a row for `window` so that what is saved
    /// under it loads again.
    pub(crate) fn open(dir: &Path, window: i64) -> Result<Self, HostingError> {
        Ok(Self { store: Mutex::new(Self::store_in(dir, window)?), window })
    }

    fn store_in(dir: &Path, window: i64) -> Result<Store, HostingError> {
        let store = Store::open_at(dir).map_err(HostingError::store)?;
        let known = store.load().map_err(HostingError::store)?.iter().any(|w| w.id == window);
        if !known {
            store
                .save_window_meta(&WindowRecord { id: window, ..WindowRecord::default() })
                .map_err(HostingError::store)?;
        }
        Ok(store)
    }

    /// The window moved to the profile in `dir`: its rows are written and read there from now on (the terminals it
    /// runs already hold this registry, so it is the store that changes).
    pub(crate) fn follow(&self, dir: &Path) -> Result<(), HostingError> {
        *self.store() = Self::store_in(dir, self.window)?;
        Ok(())
    }

    fn store(&self) -> MutexGuard<'_, Store> {
        self.store.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The window's sessions that name a host, in the sidebar's order (`running` is for the caller to fill).
    pub(crate) fn saved(&self) -> Result<Vec<SavedTerminal>, HostingError> {
        Ok(self
            .sessions()?
            .into_iter()
            .filter_map(|s| {
                s.host_id.map(|host_id| SavedTerminal {
                    terminal: s.id,
                    host_id,
                    running: false,
                    cwd: s.cwd,
                    shell: s.shell,
                    forget_text: s.forget_text,
                    text: if s.forget_text { None } else { s.scrollback },
                })
            })
            .collect())
    }

    /// The key host `host_id` seals its checkpoint with; `None` when the profile has no key.
    pub(crate) fn checkpoint_key(&self, host_id: &str) -> Option<wormhole_term::checkpoint::Key> {
        self.store().checkpoint_key(host_id)
    }

    /// Save the screens of terminals (the store skips those with no row, no key, or text not to be kept).
    pub(crate) fn save_texts(&self, texts: &[(i64, String)]) -> Result<u32, HostingError> {
        let written = self.store().save_scrollbacks(texts).map_err(HostingError::store)?;
        Ok(u32::try_from(written).unwrap_or(u32::MAX))
    }

    /// The hosts that the sessions of the profile's other windows name: a host of theirs is not an orphan here.
    pub(crate) fn hosts_of_other_windows(&self) -> Result<Vec<String>, HostingError> {
        let windows = self.store().load().map_err(HostingError::store)?;
        Ok(windows
            .into_iter()
            .filter(|w| w.id != self.window)
            .flat_map(|w| w.sessions)
            .filter_map(|s| s.host_id)
            .collect())
    }

    /// Terminal `tab` runs in host `host_id` (saved at once: a crash right after must still find it). A terminal the
    /// window saved already keeps the rest of its row (name, notes, tasks, pin, split): the store writes the record
    /// whole, so a bare one would wipe them (the GPUI app saves the session's own record, `save_host_row`).
    pub(crate) fn remember(
        &self,
        tab: i64,
        host_id: &str,
        cwd: Option<String>,
        shell: Option<String>,
    ) -> Result<(), HostingError> {
        let session = match self.sessions()?.into_iter().find(|s| s.id == tab) {
            Some(saved) => SessionRecord { cwd: cwd.or(saved.cwd), shell: shell.or(saved.shell), ..saved },
            None => SessionRecord { shell, ..SessionRecord::new(tab, None, cwd) },
        };
        self.store().save_session_host(self.window, &session, host_id).map_err(HostingError::store)
    }

    /// The terminals the window lists besides the ones in `keep` are forgotten (closed, or restored under
    /// new ids).
    pub(crate) fn keep_only(&self, keep: &[i64]) -> Result<(), HostingError> {
        self.store().delete_sessions_except(self.window, keep).map_err(HostingError::store)
    }

    /// Terminal `tab` ended (closed on purpose): forget its row.
    pub(crate) fn forget(&self, tab: i64) -> Result<(), HostingError> {
        let keep: Vec<i64> = self.sessions()?.iter().map(|s| s.id).filter(|id| *id != tab).collect();
        self.keep_only(&keep)
    }

    /// No row of the window names a host any more, `only` one when given (Quit and End All Terminals: the hosts end,
    /// the terminals do not). Every row stays whole, its notes, tasks and saved text with it, and the next launch
    /// starts each terminal again above its text, as the GPUI app's saves with `ending_hosts` leave them.
    pub(crate) fn unname_hosts(&self, only: Option<i64>) -> Result<(), HostingError> {
        let named =
            self.sessions()?.into_iter().filter(|s| s.host_id.is_some() && only.is_none_or(|id| id == s.id));
        let mut store = self.store();
        for session in named {
            let session = SessionRecord { host_id: None, ..session };
            store.save_session(self.window, &session).map_err(HostingError::store)?;
        }
        Ok(())
    }

    /// The ids of the window's sessions that name no host: not this registry's to forget.
    pub(crate) fn unhosted_ids(&self) -> Result<Vec<i64>, HostingError> {
        Ok(self.sessions()?.iter().filter(|s| s.host_id.is_none()).map(|s| s.id).collect())
    }

    fn sessions(&self) -> Result<Vec<SessionRecord>, HostingError> {
        let windows = self.store().load().map_err(HostingError::store)?;
        Ok(windows.into_iter().find(|w| w.id == self.window).map(|w| w.sessions).unwrap_or_default())
    }
}
