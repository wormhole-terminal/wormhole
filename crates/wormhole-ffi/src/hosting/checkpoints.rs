//! Terminals that come back after the Mac restarts (`FeatureFlag::RestoreAfterRestart`), as `window/restart_restore.rs`
//! does for the GPUI app: a hosted terminal's host is asked to keep a checkpoint of the screen, sealed with a key
//! derived from the profile's saved-text key and the host's id. After a restart the hosts are gone and the checkpoints
//! are not: a terminal whose host is gone starts in a new host with the checkpoint's screen drawn above its shell.
//!
//! The host never deletes a checkpoint; everything that removes one is here: a terminal closed (and once more a
//! moment later), the new host's first checkpoint on disk (it replaces the one restored from), and the launch sweep.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime};

use wormhole_core::config::Config;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::storage::paths::ProfilePaths;
use wormhole_core::terminal::SpawnOptions;
use wormhole_core::terminal::checkpoint as core;
use wormhole_term::checkpoint::Key;

use crate::terminal::Terminal;

/// How long after a close the checkpoint is looked for once more: the host drops its writer before it hangs up the
/// shell, so nothing should be left; this is for a write already under way.
const FORGET_AGAIN_AFTER: Duration = Duration::from_secs(5);

/// How long the new host has to say its first checkpoint is on disk (`checkpoint_plan.rs` `CONFIRM_WITHIN`); after
/// that the old checkpoint stays, for the retention sweep, and a late word from the host no longer removes it.
const CONFIRM_WITHIN: Duration = Duration::from_secs(10);

/// Whether terminals of `profile` keep checkpoints: the flag, persistent terminals (checkpoints are the host's) and the
/// profile's "restore terminal text after restart" (the screen is the same sort of thing).
pub(crate) fn on(profile: &ProfilePaths) -> bool {
    let settings = Config::load_for(profile).settings;
    settings.feature_enabled(FeatureFlag::RestoreAfterRestart)
        && settings.feature_enabled(FeatureFlag::PersistentTerminals)
        && settings.restore_terminal_text
}

pub(crate) struct Checkpoints {
    dir: PathBuf,
    /// The hosts whose checkpoint a restore under way still needs: the launch sweep leaves them alone however old they
    /// are, until the new host has written its own.
    pending: Arc<Mutex<HashSet<String>>>,
}

impl Checkpoints {
    pub(crate) fn new(data_dir: &Path) -> Self {
        Self { dir: core::sessions_dir(data_dir), pending: Arc::default() }
    }

    /// Everything a restore may read: the hosts of the terminals that start again, until each is confirmed.
    pub(crate) fn expect_restores(&self, hosts: impl IntoIterator<Item = String>) {
        self.pending.lock().unwrap_or_else(PoisonError::into_inner).extend(hosts);
    }

    /// Make `options` draw the screen of `old`'s checkpoint above the new shell, in place of its saved text. Whether it
    /// did. File I/O and decryption: not for the main thread.
    pub(crate) fn seed(&self, old: &str, key: &Key, options: &mut SpawnOptions) -> bool {
        core::seed_options(
            &self.dir,
            old,
            Some(key),
            options,
            &wormhole_core::terminal::restored::restored_words,
        )
    }

    /// Ask `terminal`'s host (`id`) to keep checkpoints with `key`. A terminal restored from `replaces`' checkpoint has
    /// its new host write its own at once; when that is on disk and reads, the old one is spent.
    pub(crate) fn start(&self, terminal: &Terminal, id: &str, key: Key, replaces: Option<String>) {
        let _ = core::prepare_dir(&self.dir, wormhole_core::platform::backup::exclude);
        if !terminal.with_pty(|pty| pty.start_checkpoints(&key)).unwrap_or(false) {
            return;
        }
        let Some(old) = replaces else { return };
        let (dir, new, pending) = (self.dir.clone(), id.to_owned(), Arc::clone(&self.pending));
        // Whichever comes first, the host's word or the timeout, settles it; the other then does nothing.
        let settled = Arc::new(AtomicBool::new(false));
        let (late, late_pending, late_old) = (Arc::clone(&settled), Arc::clone(&self.pending), old.clone());
        let timeout =
            std::thread::Builder::new().name("wormhole-ffi-checkpoint-confirm".into()).spawn(move || {
                std::thread::sleep(CONFIRM_WITHIN);
                if !late.swap(true, Ordering::AcqRel) {
                    tracing::warn!(
                        old = late_old,
                        "the new host said nothing of its first checkpoint; the old one stays"
                    );
                    late_pending.lock().unwrap_or_else(PoisonError::into_inner).remove(&late_old);
                }
            });
        drop(timeout);
        terminal.events.on_checkpoint_written(Box::new(move |_generation| {
            if settled.swap(true, Ordering::AcqRel) {
                return;
            }
            if core::forget_when_confirmed(&dir, &old, &new, &key) {
                pending.lock().unwrap_or_else(PoisonError::into_inner).remove(&old);
            } else {
                tracing::warn!(old, "the new host's checkpoint does not read; the old one stays");
            }
        }));
        terminal.with_pty(|pty| pty.checkpoint_now(true));
    }

    /// A terminal is closed for good: its checkpoint goes now and again a moment later.
    pub(crate) fn forget_later(&self, hosts: Vec<String>) {
        let dir = self.dir.clone();
        for id in &hosts {
            core::forget(&dir, id);
        }
        let again =
            std::thread::Builder::new().name("wormhole-ffi-checkpoint-forget".into()).spawn(move || {
                std::thread::sleep(FORGET_AGAIN_AFTER);
                for id in hosts {
                    core::forget(&dir, &id);
                }
            });
        drop(again);
    }

    /// Off the main thread, at launch: with no profile having the feature on, remove what it left (when there is a
    /// folder at all); else take what is old or too much. Hosts that run and restores under way are spared.
    pub(crate) fn sweep(&self, running: HashSet<String>, any_on: bool) {
        let protected: HashSet<String> = running
            .into_iter()
            .chain(self.pending.lock().unwrap_or_else(PoisonError::into_inner).clone())
            .collect();
        if !any_on {
            if self.dir.is_dir() {
                let removed = core::remove_unused(&self.dir, &protected);
                tracing::info!(count = removed.len(), "removed the checkpoints of a feature that is off");
            }
        } else if core::prepare_dir(&self.dir, wormhole_core::platform::backup::exclude).is_ok() {
            let removed = core::sweep(&self.dir, &protected, SystemTime::now());
            tracing::info!(count = removed.len(), "removed old checkpoints");
        }
    }
}
