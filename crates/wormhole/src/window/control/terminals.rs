//! What a window does when asked from outside it, about its terminals: the control protocol's list,
//! status, notify, send-text and new-terminal, and a notification's Reply typed into a terminal.

use gpui_kit::*;
use wormhole_core::agents::AgentId;
use wormhole_core::ipc::Peer;
use wormhole_core::ipc::control::{
    Opened, PaneInfo, SplitInfo, TabInfo, Target, TerminalInfo, TerminalState,
};
use wormhole_term::Notification;

use crate::app_state::app;
use crate::terminal_view::TerminalView;
use crate::window::Workspace;
use crate::window::ambient::state_of;

/// An agent's id as the hooks spell it (`claude`, `codex`).
pub(in crate::window) fn agent_id(agent: AgentId) -> Option<String> {
    serde_json::to_value(agent).ok().and_then(|v| v.as_str().map(str::to_string))
}

impl Workspace {
    /// The terminal and tab `target` names in this window: one id, whichever way it is named
    /// (what its hooks report).
    pub(crate) fn control_find(&self, target: Target) -> Option<(i64, i64)> {
        let (Target::Terminal(id) | Target::Tab(id) | Target::Pane(id)) = target;
        self.find_session(id).map(|ix| (self.sessions[ix].id, id))
    }

    /// Every terminal of this window, in the sidebar's order.
    pub(crate) fn control_terminals(&self, cx: &App) -> Vec<TerminalInfo> {
        self.session_order(cx).into_iter().map(|ix| self.terminal_info(ix, cx)).collect()
    }

    /// The window's splits, with their terminals in reading order.
    pub(crate) fn control_splits(&self) -> Vec<SplitInfo> {
        self.splits
            .iter()
            .map(|split| SplitInfo {
                id: split.id,
                terminals: split.layout.panes(),
                focused: split.layout.focused(),
            })
            .collect()
    }

    pub(crate) fn control_terminal(&self, session: i64, cx: &App) -> Option<TerminalInfo> {
        let ix = self.sessions.iter().position(|s| s.id == session)?;
        Some(self.terminal_info(ix, cx))
    }

    /// A tab's state and the pid of its shell (none once it has ended): what the typing rules need.
    pub(crate) fn control_tab_facts(&self, tab: i64, cx: &App) -> Option<(TerminalState, Option<u32>)> {
        let terminal = self.pane_terminal(tab)?.read(cx);
        Some((tab_state(terminal), terminal.shell_pid()))
    }

    /// The terminals of this window whose shell the caller runs under: the ones it is inside of, by
    /// its place in the process tree, whatever it says about itself.
    pub(crate) fn control_terminals_under(&self, peer: &Peer, cx: &App) -> Vec<i64> {
        self.sessions
            .iter()
            .filter(|session| session.terminal().read(cx).shell_pid().is_some_and(|pid| peer.runs_in(pid)))
            .map(|session| session.id)
            .collect()
    }

    fn terminal_info(&self, ix: usize, cx: &App) -> TerminalInfo {
        let session = &self.sessions[ix];
        let terminal = session.terminal().read(cx);
        // The status is the one read for the folder the terminal is in.
        let repo = session.git.status.as_ref().filter(|_| session.git.cwd == terminal.cwd);
        let folder = terminal.cwd.as_deref().map(|p| p.to_string_lossy().into_owned());
        TerminalInfo {
            id: session.id,
            name: session.name().to_string(),
            status: tab_state(terminal),
            agent: terminal.agent.and_then(agent_id),
            folder: folder.clone(),
            branch: repo.and_then(|g| g.branch.clone()),
            changed: repo.map(|g| g.files.len()),
            pinned: session.pinned,
            profile: {
                let profiles = &app(cx).profiles;
                (profiles.profiles().len() > 1)
                    .then(|| profiles.get(&self.profile).map(|p| p.name.clone()))
                    .flatten()
            },
            selected: ix == self.active,
            split: session.split,
            // The protocol still lists tabs: a terminal has the one, itself, with its one pane.
            tabs: vec![TabInfo {
                id: session.id,
                name: session.pane.label().to_string(),
                status: tab_state(terminal),
                agent: terminal.agent.and_then(agent_id),
                message: terminal.agent_message.clone(),
                folder,
                shell: terminal.shell_name().to_string(),
                command: terminal.foreground().map(|f| f.command.clone()),
                since_seconds: terminal.status_since().elapsed().as_secs(),
                selected: true,
                panes: vec![PaneInfo { id: session.id }],
            }],
        }
    }

    /// `notify`'s words into `tab`'s terminal, for the notification sweep to tell.
    pub(crate) fn control_notice(&mut self, tab: i64, words: Notification, cx: &mut Context<Self>) -> bool {
        let Some(terminal) = self.pane_terminal(tab) else { return false };
        terminal.update(cx, |t, _| t.notice_from_control(words));
        self.sync_summary(tab, cx);
        cx.notify();
        true
    }

    /// Paste `text` into `tab` and, with `submit`, press Return a moment later (the program reads
    /// the paste first, as `type_reply` does for a notification's Reply).
    pub(crate) fn control_send_text(
        &mut self,
        tab: i64,
        text: &str,
        submit: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(terminal) = self.pane_terminal(tab) else { return false };
        crate::window::feature_terminals::paste_text(&terminal, text, submit, cx);
        true
    }

    /// A new terminal in `folder` (where the current one is without), selected, with `command`
    /// typed into its shell and run.
    pub(crate) fn control_new_terminal(
        &mut self,
        folder: Option<String>,
        command: Option<&str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Opened {
        let cwd = folder.or_else(|| self.default_cwd(cx));
        let ix = self.open_session(None, cwd.clone(), window, cx);
        let terminal = self.sessions[ix].id;
        let tab = terminal;
        if let Some(command) = command.map(str::trim).filter(|c| !c.is_empty()) {
            self.control_send_text(tab, command, true, cx);
        }
        Opened { window: self.window_id, terminal, tab, pane: tab, folder: cwd, worktree: None }
    }

    /// Paste `text` into a tab and press Return a moment later (the program reads the paste
    /// first). Returns whether there was anything to send.
    pub(crate) fn type_reply(&mut self, tab: i64, text: &str, cx: &mut Context<Self>) -> bool {
        let text = text.trim();
        let Some(terminal) = self.pane_terminal(tab) else { return false };
        if text.is_empty() {
            return false;
        }
        crate::window::feature_terminals::paste_text(&terminal, text, true, cx);
        true
    }
}

/// A terminal's state as the sidebar counts it.
fn tab_state(terminal: &TerminalView) -> TerminalState {
    state_of(terminal.status(), terminal.source().told(), terminal.foreground()).into()
}

#[cfg(test)]
mod tests {
    use wormhole_core::agents::AgentId;

    use super::agent_id;

    #[test]
    fn agents_are_named_as_their_hooks_name_them() {
        assert_eq!(agent_id(AgentId::Claude).as_deref(), Some("claude"));
        assert_eq!(agent_id(AgentId::OpenCode).as_deref(), Some("opencode"));
    }
}
