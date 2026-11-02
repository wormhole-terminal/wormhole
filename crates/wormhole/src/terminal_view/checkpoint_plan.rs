//! What a hosted terminal asks its host for when "restore after restart" is on: to keep a checkpoint, sealed with a
//! key the app derived for this terminal. The window decides (it knows the terminal's record and the profile's
//! settings) and hands over a [`CheckpointPlan`]; the terminal sends it as soon as it is connected to its host,
//! whether the host was just started or attached to. A terminal without a plan asks for nothing, and its host
//! writes nothing: no key, no checkpoint.

use std::collections::HashSet;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use gpui_kit::*;
use wormhole_term::checkpoint::Key;

use super::TerminalView;

/// The hosts whose checkpoints a restore still needs: the terminal was restored from one, and its new host has not
/// written its own yet. The launch sweep leaves them alone, however old they are.
static PENDING: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// A restore from the checkpoint of host `old` that is not confirmed yet. Registered while it lives.
pub(crate) struct PendingRestore {
    pub old: String,
}

impl PendingRestore {
    pub(crate) fn new(old: &str) -> Arc<Self> {
        PENDING.lock().unwrap_or_else(PoisonError::into_inner).push(old.to_owned());
        Arc::new(Self { old: old.to_owned() })
    }
}

impl Drop for PendingRestore {
    fn drop(&mut self) {
        let mut pending = PENDING.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(at) = pending.iter().position(|id| *id == self.old) {
            pending.remove(at);
        }
    }
}

/// The hosts whose checkpoints a restore under way still needs.
pub(crate) fn pending_restores() -> HashSet<String> {
    PENDING.lock().unwrap_or_else(PoisonError::into_inner).iter().cloned().collect()
}

/// How long the app waits for the new host to say its first checkpoint is on disk. A host that never does leaves
/// the old checkpoint where it is (the retention sweep takes it in time): the one timeout there is, not a poll.
const CONFIRM_WITHIN: Duration = Duration::from_secs(10);

/// The key the host seals with. A terminal whose text is not to be kept, or a profile without a key, has no plan.
#[derive(Clone)]
pub(crate) struct CheckpointPlan {
    pub key: Key,
    /// The terminal was restored from this host's checkpoint: the new host is asked to write its own at once and to
    /// say when it has; the old one is removed after that, once the new one has been read back.
    pub replaces: Option<Arc<PendingRestore>>,
}

/// What a terminal restored from a checkpoint reads it with, in the background.
#[derive(Clone)]
pub(crate) struct RestoreFrom {
    pub old: String,
    pub key: Key,
}

impl std::fmt::Debug for RestoreFrom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RestoreFrom").field("old", &self.old).finish_non_exhaustive()
    }
}

/// Remove the checkpoint of host `host_id` from the data folder.
fn forget_checkpoint_of(host_id: &str) {
    use wormhole_core::terminal::checkpoint as core;
    core::forget(&core::sessions_dir(&wormhole_core::storage::paths::data_dir()), host_id);
}

impl TerminalView {
    /// This terminal's host is to keep checkpoints as `plan` says. Sent now if the terminal is connected, else
    /// when it is (`host_ready`).
    pub(crate) fn set_checkpoint_plan(&mut self, plan: CheckpointPlan, cx: &mut Context<Self>) {
        self.host.checkpoint = Some(plan);
        self.send_checkpoint_plan(cx);
    }

    /// Send the plan to the host, if there is a plan and a connection to a host that can keep checkpoints.
    pub(super) fn send_checkpoint_plan(&mut self, cx: &mut Context<Self>) {
        let (Some(plan), Some(pty)) = (&self.host.checkpoint, &self.pty) else { return };
        if !pty.start_checkpoints(&plan.key) {
            return;
        }
        tracing::debug!("asked the host to keep checkpoints");
        // A restored terminal: its new host writes its first checkpoint now, not after three quiet seconds, and
        // says when it is on disk (`checkpoint_written`). The old checkpoint goes after that and not before: a
        // crash or another restart in between must still find the old one.
        if plan.replaces.is_some() {
            pty.checkpoint_now(true);
            self.await_confirmation(cx);
        }
    }

    /// Give the new host [`CONFIRM_WITHIN`] to say its checkpoint is written; after that the old one stays.
    fn await_confirmation(&mut self, cx: &mut Context<Self>) {
        self._tasks.push(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(CONFIRM_WITHIN).await;
            let _ = this.update(cx, |this, _| {
                if let Some(plan) = this.host.checkpoint.as_mut()
                    && let Some(replaces) = plan.replaces.take()
                {
                    tracing::warn!(
                        old = replaces.old,
                        "the new host said nothing of its first checkpoint; the old one stays"
                    );
                }
            });
        }));
    }

    /// The host says generation `generation` of its checkpoint is on disk (it was asked to flush). If this
    /// terminal was restored from an older host's checkpoint, read the new one back once; if it opens, the old
    /// one is spent.
    pub(super) fn checkpoint_written(&mut self, generation: u64, cx: &mut Context<Self>) {
        let (Some(plan), Some(new)) = (self.host.checkpoint.as_mut(), self.host.id.clone()) else { return };
        let Some(replaces) = plan.replaces.take() else { return };
        let key = plan.key.clone();
        tracing::debug!(generation, "the host's first checkpoint is on disk");
        self._tasks.push(cx.background_spawn(async move {
            use wormhole_core::terminal::checkpoint as core;
            let dir = core::sessions_dir(&wormhole_core::storage::paths::data_dir());
            if !core::forget_when_confirmed(&dir, &replaces.old, &new, &key) {
                tracing::warn!(
                    old = replaces.old,
                    "the new host's checkpoint does not read; the old one stays"
                );
            }
            // (`replaces` drops here: the sweep may have what is left.)
        }));
    }

    /// Stop the host's checkpoints and remove what it wrote: the setting went off.
    pub(crate) fn stop_checkpoints(&mut self) {
        if self.host.checkpoint.is_none() {
            return;
        }
        self.host.checkpoint = None;
        if let Some(pty) = &self.pty {
            pty.stop_checkpoints();
        }
        self.forget_checkpoint();
    }

    /// Remove the checkpoint of this terminal's host, if it runs in one: it is closed (or the setting went off).
    /// The host drops its own writer first when told to close, so nothing writes it again; the app looks once more
    /// when the host is gone, as a backstop (`forget_checkpoint_later`).
    pub(crate) fn forget_checkpoint(&self) {
        if let Some(id) = self.host.id.as_deref() {
            forget_checkpoint_of(id);
        }
        // Closed before its new host had written: the old one it was restored from is no one's now.
        if let Some(replaces) = self.host.checkpoint.as_ref().and_then(|p| p.replaces.as_ref()) {
            forget_checkpoint_of(&replaces.old);
        }
    }

    /// The host ids whose checkpoints [`forget_checkpoint`](Self::forget_checkpoint) removes.
    pub(crate) fn checkpoint_hosts(&self) -> Vec<String> {
        let own = self.host.id.clone();
        let old = self.host.checkpoint.as_ref().and_then(|p| p.replaces.as_ref()).map(|r| r.old.clone());
        own.into_iter().chain(old).collect()
    }

    #[cfg(test)]
    pub(crate) fn set_host_id_for_test(&mut self, id: &str) {
        self.host.id = Some(id.to_owned());
        self.host.checkpoint = Some(CheckpointPlan { key: Key::new([9; 32]), replaces: None });
    }

    #[cfg(test)]
    pub(crate) fn checkpoint_written_for_test(&mut self, generation: u64, cx: &mut Context<Self>) {
        self.checkpoint_written(generation, cx);
    }

    #[cfg(test)]
    pub(crate) fn has_checkpoint_plan_for_test(&self) -> bool {
        self.host.checkpoint.is_some()
    }
}
