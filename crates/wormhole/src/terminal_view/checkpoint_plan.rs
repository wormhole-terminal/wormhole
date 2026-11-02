//! What a hosted terminal asks its host for when "restore after restart" is on: to keep a checkpoint, sealed with a
//! key the app derived for this terminal. The window decides (it knows the terminal's record and the profile's
//! settings) and hands over a [`CheckpointPlan`]; the terminal sends it as soon as it is connected to its host,
//! whether the host was just started or attached to. A terminal without a plan asks for nothing, and its host
//! writes nothing.

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

/// How long the app waits for a new host's first checkpoint to land, in steps that double: the host writes at
/// once on a flush, and the disk is quick, so the first looks find it; a host that never does leaves the old
/// checkpoint where it is. (The host sends no word when it has written, so the app looks at the files.)
const CONFIRM_STEPS: [u64; 8] = [50, 100, 200, 400, 800, 1_600, 3_200, 6_400];

/// The key the host seals with (none: the app has none, or the person does not want the terminal's text kept,
/// and then the host keeps the folder and the shell only) and the key of the shell the terminal runs.
#[derive(Clone)]
pub(crate) struct CheckpointPlan {
    pub key: Option<Key>,
    pub shell: String,
    /// The terminal was restored from this host's checkpoint: the new host is asked to write its own at once, and
    /// the old one is removed when that has landed.
    pub replaces: Option<Arc<PendingRestore>>,
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
        if !pty.start_checkpoints(plan.key.as_ref(), &plan.shell) {
            return;
        }
        tracing::debug!(keyed = plan.key.is_some(), "asked the host to keep checkpoints");
        // A restored terminal: its new host writes its first checkpoint now, not after three quiet seconds,
        // and the old checkpoint goes when that is on disk (and not before: a crash or another restart in
        // between must still find the old one).
        if let (Some(replaces), Some(new)) = (plan.replaces.clone(), self.host.id.clone()) {
            pty.checkpoint_now(true);
            let key = plan.key.clone();
            self.confirm_restore(replaces, new, key, cx);
        }
    }

    /// Wait for host `new` to have written its first checkpoint, then remove the one `replaces` names.
    fn confirm_restore(
        &mut self,
        replaces: Arc<PendingRestore>,
        new: String,
        key: Option<Key>,
        cx: &mut Context<Self>,
    ) {
        use wormhole_core::terminal::checkpoint as core;
        self._tasks.push(cx.spawn(async move |_this, cx| {
            let bg = cx.background_executor().clone();
            for step in CONFIRM_STEPS {
                bg.timer(Duration::from_millis(step)).await;
                let (old, new, key) =
                    (replaces.old.clone(), new.clone(), key.as_ref().map(|k| *k.as_bytes()));
                let landed = bg
                    .spawn(async move {
                        let dir = core::sessions_dir(&wormhole_core::storage::paths::data_dir());
                        core::forget_when_confirmed(&dir, &old, &new, key.as_ref())
                    })
                    .await;
                if landed {
                    return;
                }
            }
            tracing::warn!(
                old = replaces.old,
                "the new host's first checkpoint did not land; the old one stays"
            );
        }));
    }

    /// Ask the host for a checkpoint now (after Clear Buffer, whose result should not be the old screen).
    pub(super) fn checkpoint_now(&self) {
        if self.host.checkpoint.is_some()
            && let Some(pty) = &self.pty
        {
            pty.checkpoint_now(false);
        }
    }

    /// Remove the checkpoint of this terminal's host, if it runs in one: its buffer was cleared, or it is closed.
    pub(crate) fn forget_checkpoint(&self) {
        if let Some(id) = self.host.id.as_deref() {
            forget_checkpoint_of(id);
        }
        // Closed before its new host had written: the old one it was restored from is no one's now.
        if let Some(replaces) = self.host.checkpoint.as_ref().and_then(|p| p.replaces.as_ref()) {
            forget_checkpoint_of(&replaces.old);
        }
    }

    /// Clear Buffer, as the key and the menu do.
    #[cfg(test)]
    pub(crate) fn clear_buffer_for_test(&mut self, cx: &mut gpui_kit::Context<Self>) {
        self.run_command(super::input::TerminalCommand::Clear, cx);
    }

    #[cfg(test)]
    pub(crate) fn set_host_id_for_test(&mut self, id: &str) {
        self.host.id = Some(id.to_owned());
        self.host.checkpoint = Some(CheckpointPlan { key: None, shell: "login".into(), replaces: None });
    }
}
