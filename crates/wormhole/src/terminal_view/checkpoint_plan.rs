//! What a hosted terminal asks its host for when "restore after restart" is on: to keep a checkpoint, sealed with a
//! key the app derived for this terminal. The window decides (it knows the terminal's record and the profile's
//! settings) and hands over a [`CheckpointPlan`]; the terminal sends it as soon as it is connected to its host,
//! whether the host was just started or attached to. A terminal without a plan asks for nothing, and its host
//! writes nothing.

use wormhole_term::checkpoint::Key;

use super::TerminalView;

/// The key the host seals with (none: the app has none, or the person does not want the terminal's text kept,
/// and then the host keeps the folder and the shell only) and the key of the shell the terminal runs.
#[derive(Clone)]
pub(crate) struct CheckpointPlan {
    pub key: Option<Key>,
    pub shell: String,
}

/// Remove the checkpoint of host `host_id` from the data folder.
pub(crate) fn forget_checkpoint_of(host_id: &str) {
    use wormhole_core::terminal::checkpoint as core;
    core::forget(&core::sessions_dir(&wormhole_core::storage::paths::data_dir()), host_id);
}

impl TerminalView {
    /// This terminal's host is to keep checkpoints as `plan` says. Sent now if the terminal is connected, else
    /// when it is (`host_ready`).
    pub(crate) fn set_checkpoint_plan(&mut self, plan: CheckpointPlan) {
        self.host.checkpoint = Some(plan);
        self.send_checkpoint_plan();
    }

    /// Send the plan to the host, if there is a plan and a connection to a host that can keep checkpoints.
    pub(super) fn send_checkpoint_plan(&self) {
        let (Some(plan), Some(pty)) = (&self.host.checkpoint, &self.pty) else { return };
        if pty.start_checkpoints(plan.key.as_ref(), &plan.shell) {
            tracing::debug!(keyed = plan.key.is_some(), "asked the host to keep checkpoints");
        }
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
    }

    /// Clear Buffer, as the key and the menu do.
    #[cfg(test)]
    pub(crate) fn clear_buffer_for_test(&mut self, cx: &mut gpui_kit::Context<Self>) {
        self.run_command(super::input::TerminalCommand::Clear, cx);
    }

    #[cfg(test)]
    pub(crate) fn set_host_id_for_test(&mut self, id: &str) {
        self.host.id = Some(id.to_owned());
        self.host.checkpoint = Some(CheckpointPlan { key: None, shell: "login".into() });
    }
}
