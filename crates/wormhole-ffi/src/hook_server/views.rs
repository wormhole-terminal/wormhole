//! The workspace as the control protocol's `list` tells it: one record per terminal, built from the model's
//! own snapshot and facts (the words and states are core's), and the splits.

use wormhole_core::ipc::control::{PaneInfo, SplitInfo, TabInfo, TerminalInfo, TerminalState};
use wormhole_core::view::sidebar::RowState;
use wormhole_core::workspace::WorkspaceModel;

/// One terminal as seen at a moment.
#[derive(Clone, Debug)]
pub(crate) struct Seen {
    pub info: TerminalInfo,
    /// The title its program set, for the title events.
    pub title: Option<String>,
    /// Hidden from agents ("Hide from Agents"): `list` leaves it out.
    pub hidden: bool,
}

impl Seen {
    /// The agent's last words, when it has some.
    pub(crate) fn message(&self) -> Option<String> {
        self.info.tabs.first().and_then(|tab| tab.message.clone())
    }
}

/// Every terminal in the list's order, and every split.
#[derive(Clone, Debug, Default)]
pub(crate) struct Views {
    pub terminals: Vec<Seen>,
    pub splits: Vec<SplitInfo>,
}

impl Views {
    /// The terminals `list` shows.
    pub(crate) fn listed(&self) -> Vec<TerminalInfo> {
        self.terminals.iter().filter(|seen| !seen.hidden).map(|seen| seen.info.clone()).collect()
    }

    /// The splits `list` shows: a hidden terminal is not in one's list, a split of hidden terminals only is
    /// not listed, and the keyboard in a hidden pane is told as the first listed.
    pub(crate) fn listed_splits(&self) -> Vec<SplitInfo> {
        let hidden = |id: &i64| self.terminals.iter().any(|seen| seen.info.id == *id && seen.hidden);
        self.splits
            .iter()
            .filter_map(|split| {
                let terminals: Vec<i64> = split.terminals.iter().copied().filter(|id| !hidden(id)).collect();
                let first = *terminals.first()?;
                let focused = Some(split.focused).filter(|id| terminals.contains(id)).unwrap_or(first);
                Some(SplitInfo { id: split.id, terminals, focused })
            })
            .collect()
    }
}

/// A row's state as the protocol speaks of it.
pub(crate) fn state_of(state: RowState) -> TerminalState {
    match state {
        RowState::NeedsYou => TerminalState::NeedsYou,
        RowState::Working => TerminalState::Working,
        RowState::Finished => TerminalState::Finished,
        RowState::Failed => TerminalState::Failed,
        RowState::Idle => TerminalState::Idle,
        RowState::Exited => TerminalState::Exited,
    }
}

/// An agent's id as the hooks spell it (`claude`, `codex`).
fn agent_word(agent: wormhole_core::agents::AgentId) -> Option<String> {
    serde_json::to_value(agent).ok().and_then(|value| value.as_str().map(str::to_string))
}

/// What the model says now. `now_ms` is the workspace's clock, for how long a terminal has been in its state.
pub(crate) fn read(model: &WorkspaceModel, now_ms: u64) -> Views {
    let terminals = model
        .terminals_snapshot()
        .terminals
        .into_iter()
        .map(|summary| {
            let facts = model.facts(summary.id);
            let status = state_of(summary.state);
            let agent = facts.and_then(|facts| facts.agent).and_then(agent_word);
            let folder = summary.cwd.clone();
            let tab = TabInfo {
                id: summary.id,
                name: summary.name.clone(),
                status,
                agent: agent.clone(),
                message: summary.message.clone(),
                folder: folder.clone(),
                shell: facts.map(|facts| facts.shell.clone()).unwrap_or_default(),
                command: summary.command.clone(),
                since_seconds: facts.map_or(0, |facts| now_ms.saturating_sub(facts.since) / 1000),
                selected: true,
                panes: vec![PaneInfo { id: summary.id }],
            };
            let info = TerminalInfo {
                id: summary.id,
                name: summary.name,
                status,
                agent,
                folder,
                branch: summary.branch,
                changed: facts.and_then(|facts| facts.changes),
                pinned: summary.pinned,
                profile: None,
                selected: summary.selected,
                split: summary.split,
                tabs: vec![tab],
            };
            Seen { info, title: facts.and_then(|facts| facts.title.clone()), hidden: summary.agent_hidden }
        })
        .collect();
    let splits = model
        .splits_overview()
        .into_iter()
        .map(|split| SplitInfo { id: split.id, terminals: split.terminals, focused: split.focused })
        .collect();
    Views { terminals, splits }
}
