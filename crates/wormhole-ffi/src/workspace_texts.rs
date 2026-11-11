//! Restore terminal text after restart, for the window's terminals (`window/save.rs` in the GPUI app): what each one
//! starts with (the shell it was opened with, the profile, the text it showed when the app last quit), the text saved
//! each minute and at quit, and erased at once when the person clears the terminal (⌘K) or asks for it not to be
//! kept. The store seals it with the profile's key (core `Store::save_scrollbacks`) and keeps none of a terminal whose
//! row says not to.

use std::collections::HashMap;
use std::sync::Mutex;

use wormhole_core::config::Config;
use wormhole_core::storage::store::WindowRecord;

use crate::windows::Windows;
use crate::workspace::{Shared, Workspace};

/// How many lines of each terminal's history are saved and restored, whatever `scrollback_lines` says
/// (`SCROLLBACK_LINES` in `window/save.rs`).
pub(crate) const SAVED_LINES: usize = 2000;

/// What a terminal starts with besides its folder.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct LaunchFacts {
    /// The shell it was opened with ("New Terminal with ▸"), as `terminal::shells` keys it; `None` for the default.
    pub shell: Option<String>,
    /// The text it showed when the app last quit, for a terminal restored from the window's record (once).
    pub saved_text: Option<String>,
    /// The profile the window is in (`None` outside the app's windows: the one Wormhole is in).
    pub profile: Option<String>,
}

/// The texts of the terminals a window was restored with, until each starts (`Shared::restored_texts`).
#[derive(Default)]
pub(crate) struct RestoredTexts(Mutex<HashMap<i64, String>>);

impl RestoredTexts {
    /// Keep the texts `record` loaded with, when its profile keeps text at all.
    pub(crate) fn keep(&self, record: &WindowRecord, keeps_text: bool) {
        let texts = record
            .sessions
            .iter()
            .filter(|session| keeps_text && !session.forget_text)
            .filter_map(|session| Some((session.id, session.scrollback.clone()?)));
        self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner).extend(texts);
    }

    fn take(&self, id: i64) -> Option<String> {
        self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner).remove(&id)
    }
}

#[uniffi::export]
impl Workspace {
    /// What terminal `id` starts with (ask as it starts): see [`LaunchFacts`].
    pub fn launch_facts(&self, id: i64) -> LaunchFacts {
        let shared = self.shared();
        LaunchFacts {
            shell: shared.peek_model(|model| model.shell_key(id).map(str::to_owned)),
            saved_text: shared.restored_texts.take(id),
            profile: shared.windows().map(|(windows, _)| windows.profile().id().to_string()),
        }
    }

    /// Save the text of every terminal of the window that runs in the app's own process (a session host's are its
    /// desk's, `HostedTerminals::save_texts`), unless the profile keeps none ("Restore terminal text after restart")
    /// or the terminal's row says not to. Reads the engines and seals: off the main thread. How many were written.
    pub fn save_texts(&self) -> u32 {
        let shared = self.shared();
        let Some((windows, _)) = shared.windows() else { return 0 };
        if !Config::load_for(windows.profile()).settings.restore_terminal_text {
            return 0;
        }
        let forget: Vec<i64> = shared
            .peek_model(|model| model.terminals_snapshot())
            .terminals
            .iter()
            .filter(|terminal| terminal.forget_text)
            .map(|terminal| terminal.id)
            .collect();
        let texts: Vec<(i64, String)> = shared
            .registered()
            .into_iter()
            .filter(|(id, terminal)| terminal.host.is_none() && !forget.contains(id))
            .filter_map(|(id, terminal)| {
                Some((id, terminal.with_pty(|pty| pty.scrollback().text(SAVED_LINES))??))
            })
            .collect();
        match windows.store_locked().save_scrollbacks(&texts) {
            Ok(written) => u32::try_from(written).unwrap_or(u32::MAX),
            Err(err) => {
                tracing::warn!("saving the terminals' text failed: {err:#}");
                0
            }
        }
    }

    /// [`Windows::erase_saved_text_if_off`] for the profile this window is in (ask as it opens and when the settings
    /// change). How many terminals' text went.
    pub fn erase_saved_text_if_off(&self) -> u32 {
        self.shared().windows().map_or(0, |(windows, _)| windows.erase_saved_text_if_off())
    }

    /// Terminal `id`'s program is the one its record named (its session host was attached again at launch): a result
    /// nobody had seen comes back with it (`WorkspaceModel::attached_again`).
    pub fn attached_again(&self, id: i64) {
        let shared = self.shared();
        let at = shared.now();
        let events = shared.with_model(|model| model.attached_again(id, at));
        shared.post(events);
    }
}

#[uniffi::export]
impl Windows {
    /// "Restore terminal text after restart" is off for this profile: what its store holds of its terminals' text goes
    /// (`app/saved_text.rs` in the GPUI app). Ask at launch (it may have been turned off in `config.json` meanwhile)
    /// and whenever the settings change; nothing happens while it is on. How many terminals' text went.
    pub fn erase_saved_text_if_off(&self) -> u32 {
        if Config::load_for(self.profile()).settings.restore_terminal_text {
            return 0;
        }
        match self.store_locked().erase_saved_text() {
            Ok(count) => u32::try_from(count).unwrap_or(u32::MAX),
            Err(err) => {
                tracing::warn!("erasing the saved terminal text failed: {err:#}");
                0
            }
        }
    }
}

impl Shared {
    /// What was saved of terminal `id`'s text goes at once: it was cleared (⌘K), or the person asked for it not to be
    /// kept. A restart before the next save must not bring it back.
    pub(crate) fn erase_saved_text(&self, id: i64) {
        let Some((windows, _)) = self.windows() else { return };
        if let Err(err) = windows.store_locked().clear_scrollback(id) {
            tracing::warn!("erasing terminal {id}'s saved text failed: {err:#}");
        }
    }
}
