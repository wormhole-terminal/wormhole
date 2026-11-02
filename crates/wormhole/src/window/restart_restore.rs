//! Terminals that come back after the Mac restarts (`FeatureFlag::RestoreAfterRestart`; the format and the host's
//! writing are `wormhole_term::checkpoint` and `wormhole-host`, the app's reading and care are core's
//! `terminal::checkpoint`).
//!
//! With the flag on, each hosted terminal is asked to keep a checkpoint, sealed with a key the app derives for it
//! from the profile's saved-text key and its host id (the host keeps the key in memory only). A terminal without
//! a key (the profile has none, or its text is not to be kept) is asked for nothing: nothing of it is written.
//! After a restart the hosts are gone but their checkpoints are not: a terminal whose host is gone is started in a
//! new host with the checkpoint's screen drawn above its shell, a divider saying when, instead of its saved text,
//! which is what is used when anything about the checkpoint does not read.
//!
//! A host drops its checkpoint writer when its terminal is closed on purpose and never deletes a checkpoint on a
//! restart's signal; the app deletes: when a terminal is closed (and once more shortly after, when the host is
//! surely gone), when its session is forgotten (`Wormhole sessions forget`), when it was restored (the new host
//! has its own), when the setting goes off, and at launch, for those older than a week or over the budget.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

use gpui_kit::*;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::storage::paths::ProfileId;
use wormhole_core::storage::store::SessionRecord;
use wormhole_core::terminal::checkpoint as core;

use crate::app_state::app;
use crate::settings::settings_in;
use crate::terminal_view::{CheckpointPlan, PendingRestore, RestoreFrom, Start, pending_restores};
use crate::window::Workspace;
use crate::window::session::Session;

/// How long after a terminal is closed the app looks once more for a checkpoint of it. The host drops its writer
/// before it hangs up the shell, so nothing should be left; this is for a write already under way.
const FORGET_AGAIN_AFTER: Duration = Duration::from_secs(5);

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

/// The folder is made (`0700`, out of backups) before the first plan goes to a host, so a host never makes it
/// first, with the process's mask and no backup exclusion. Once per run: a few system calls.
fn ensure_dir() {
    static READY: AtomicBool = AtomicBool::new(false);
    if READY.load(Ordering::Acquire) {
        return;
    }
    match core::prepare_dir(&sessions_dir(), wormhole_core::platform::backup::exclude) {
        Ok(()) => READY.store(true, Ordering::Release),
        Err(err) => tracing::warn!("no folder for checkpoints: {err}"),
    }
}

/// The hosts that run now: live ones, and those whose pid runs but whose socket is being re-bound.
fn running_hosts() -> HashSet<String> {
    use wormhole_core::ipc::hosts::{StaleReason, hosts_dir, scan};
    let Ok(found) = scan(&hosts_dir(&wormhole_core::storage::paths::data_dir())) else {
        return HashSet::new();
    };
    let live = found.live.iter().map(|meta| meta.id.clone());
    let rebinding =
        found.stale.iter().filter(|s| s.reason == StaleReason::SocketMissing).map(|s| s.id.clone());
    live.chain(rebinding).collect()
}

/// At launch, off the main thread, once the windows are open: with no profile having the feature on, remove what
/// it left (when there is a folder at all: a Mac that never used it gets none); else make the folder and take what
/// is old or too much. Hosts that run, and checkpoints a restore under way still needs, are looked up when the
/// sweep runs, not when the app started: windows open and terminals restore in between.
pub(crate) fn launch(cx: &mut App) {
    let any_on = app(cx).profiles.profiles().iter().any(|p| on(&p.id, cx));
    cx.background_spawn(async move {
        let dir = sessions_dir();
        let protected = protected(running_hosts(), pending_restores());
        if !any_on {
            if dir.is_dir() {
                let removed = core::remove_unused(&dir, &protected);
                if !removed.is_empty() {
                    tracing::info!(count = removed.len(), "removed the checkpoints of a feature that is off");
                }
            }
            return;
        }
        if let Err(err) = core::prepare_dir(&dir, wormhole_core::platform::backup::exclude) {
            tracing::warn!("no folder for checkpoints: {err}");
            return;
        }
        let removed = core::sweep(&dir, &protected, SystemTime::now());
        if !removed.is_empty() {
            tracing::info!(count = removed.len(), "removed old checkpoints");
        }
    })
    .detach();
}

/// What the sweep must not take: the hosts that run, and the checkpoints terminals were restored from whose new
/// hosts have not yet written their own.
pub(crate) fn protected(running: HashSet<String>, pending: HashSet<String>) -> HashSet<String> {
    running.into_iter().chain(pending).collect()
}

impl Workspace {
    /// The terminal of `record` is to start in a new host because the one it ran in is gone: if it kept a
    /// checkpoint, say what to read it with (`Start::Host`'s `restore`, read off the main thread when the host
    /// starts) and which checkpoint that is. It is **not** removed: the new host writes its own only after it has
    /// its key, and a crash or another restart before that must still find this one. It goes when the new host
    /// says its first checkpoint is on disk (`TerminalView::checkpoint_written`), and the launch sweep spares it
    /// until then.
    pub(super) fn restore_from_checkpoint(
        &self,
        record: &SessionRecord,
        start: &mut Start,
        cx: &App,
    ) -> Option<Arc<PendingRestore>> {
        let (Start::Host { restore, .. }, Some(old)) = (start, record.host_id.as_deref()) else {
            return None;
        };
        if record.forget_text || !on(&self.profile, cx) {
            return None;
        }
        let key = app(cx).store_of(&self.profile).and_then(|store| store.checkpoint_key(old))?;
        *restore = Some(RestoreFrom { old: old.to_owned(), key });
        Some(PendingRestore::new(old))
    }

    /// What the host of the terminal of `record` is to be asked: to keep checkpoints, with the key for `host_id`.
    /// Nothing when the feature is off, the terminal's text is not to be kept, or the profile has no key.
    pub(super) fn checkpoint_plan(
        &self,
        record: &SessionRecord,
        host_id: &str,
        cx: &App,
    ) -> Option<CheckpointPlan> {
        if record.forget_text || !on(&self.profile, cx) {
            return None;
        }
        let key = app(cx).store_of(&self.profile).and_then(|store| store.checkpoint_key(host_id))?;
        ensure_dir();
        Some(CheckpointPlan { key, replaces: None })
    }

    /// A terminal is closed for good: the checkpoint its host kept goes with it (whether or not the feature is on
    /// now: it may have been when the checkpoint was written), and goes again a moment later in case a write was
    /// under way as the host heard of the close.
    pub(super) fn forget_checkpoint_of(&self, session: &Session, cx: &mut Context<Self>) {
        let terminal = session.terminal().read(cx);
        terminal.forget_checkpoint();
        let hosts = terminal.checkpoint_hosts();
        if hosts.is_empty() {
            return;
        }
        cx.spawn(async move |_, cx| {
            cx.background_executor().timer(FORGET_AGAIN_AFTER).await;
            cx.background_executor()
                .spawn(async move {
                    let dir = sessions_dir();
                    for id in hosts {
                        core::forget(&dir, &id);
                    }
                })
                .await;
        })
        .detach();
    }

    /// The setting went off for this window's profile: every terminal's host is told to stop and what it wrote goes.
    pub(crate) fn stop_checkpoints(&mut self, cx: &mut Context<Self>) {
        for session in &self.sessions {
            session.terminal().update(cx, |terminal, _| terminal.stop_checkpoints());
        }
    }
}
