//! Terminals that come back after the Mac restarts (`FeatureFlag::RestoreAfterRestart`; the format and the host's
//! writing are `wormhole_term::checkpoint` and `wormhole-host`, the app's reading and care are core's
//! `terminal::checkpoint`).
//!
//! With the flag on, each hosted terminal is asked to keep a checkpoint, sealed with a key the app derives for it
//! from the profile's saved-text key and its host id (the host keeps the key in memory only). After a restart the
//! hosts are gone but their checkpoints are not: a terminal whose host is gone is started in a new host with the
//! checkpoint's screen drawn above its shell, a divider saying when, instead of its saved text, which is what is
//! used when anything about the checkpoint does not read.
//!
//! A host never deletes a checkpoint. The app does: when a terminal is closed, when its buffer is cleared, when
//! its session is forgotten (`Wormhole sessions forget`), when it was restored (the new host has its own), and at
//! launch, for those older than a week or over the budget.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::SystemTime;

use gpui_kit::*;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::storage::paths::ProfileId;
use wormhole_core::storage::store::SessionRecord;
use wormhole_core::terminal::SpawnOptions;
use wormhole_core::terminal::checkpoint as core;
use wormhole_term::checkpoint::Key;

use crate::app_state::app;
use crate::settings::settings_in;
use crate::terminal_view::{CheckpointPlan, Start, forget_checkpoint_of};
use crate::window::Workspace;
use crate::window::session::Session;

/// Whether terminals of `profile` keep checkpoints: the flag, persistent terminals (checkpoints are the host's),
/// and the profile's "restore terminal text after restart" (the screen is the same sort of thing as that text).
pub(crate) fn on(profile: &ProfileId, cx: &App) -> bool {
    let settings = settings_in(profile, cx);
    settings.feature_enabled(FeatureFlag::RestoreAfterRestart)
        && settings.feature_enabled(FeatureFlag::PersistentTerminals)
        && settings.restore_terminal_text
}

/// Where the checkpoints are.
pub(crate) fn sessions_dir() -> PathBuf {
    core::sessions_dir(&wormhole_core::storage::paths::data_dir())
}

/// At launch: make the folder (`0700`, out of backups) and take what is old or too much, off the main thread.
/// Nothing when no profile has the feature on, so a Mac that never used it gets no folder.
pub(crate) fn launch(running: HashSet<String>, cx: &mut App) {
    let any_on = app(cx).profiles.profiles().iter().any(|p| on(&p.id, cx));
    if !any_on {
        return;
    }
    cx.background_spawn(async move {
        let dir = sessions_dir();
        if let Err(err) = core::prepare_dir(&dir, wormhole_core::platform::backup::exclude) {
            tracing::warn!("no folder for checkpoints: {err}");
            return;
        }
        let removed = core::sweep(&dir, &running, SystemTime::now());
        if !removed.is_empty() {
            tracing::info!(count = removed.len(), "removed old checkpoints");
        }
    })
    .detach();
}

impl Workspace {
    /// The terminal of `record` is to start in a new host because the one it ran in is gone: if it kept a
    /// checkpoint, make `options` restore its screen in place of the saved text. The checkpoint is removed once it
    /// is in memory: the new host writes its own, and a crash before it has is a terminal restored from its saved
    /// text, as without the feature.
    pub(super) fn restore_from_checkpoint(
        &self,
        record: &SessionRecord,
        start: &Start,
        options: &mut SpawnOptions,
        cx: &App,
    ) -> bool {
        let (Start::Host { .. }, Some(old)) = (start, record.host_id.as_deref()) else { return false };
        if record.forget_text || !on(&self.profile, cx) {
            return false;
        }
        let key = app(cx).store_of(&self.profile).and_then(|store| store.checkpoint_key(old));
        let restored = core::seed_options(&sessions_dir(), old, key.as_ref(), options);
        if restored {
            forget_checkpoint_of(old);
        }
        restored
    }

    /// What the host of the terminal of `record` is to be asked: to keep checkpoints, with the key for `host_id`
    /// (none when the terminal's text is not to be kept, or the profile has no key).
    pub(super) fn checkpoint_plan(
        &self,
        record: &SessionRecord,
        host_id: &str,
        cx: &App,
    ) -> Option<CheckpointPlan> {
        if !on(&self.profile, cx) {
            return None;
        }
        let key = (!record.forget_text)
            .then(|| app(cx).store_of(&self.profile).and_then(|store| store.checkpoint_key(host_id)))
            .flatten()
            .map(Key::new);
        Some(CheckpointPlan { key, shell: record.shell.clone().unwrap_or_else(|| "login".into()) })
    }

    /// A terminal is closed for good: the checkpoint its host kept goes with it (whether or not the feature is on
    /// now: it may have been when the checkpoint was written).
    pub(super) fn forget_checkpoint_of(&self, session: &Session, cx: &App) {
        session.terminal().read(cx).forget_checkpoint();
    }
}
