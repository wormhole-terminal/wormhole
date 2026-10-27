//! What `list` and `status` tell, and the other results of the control protocol: a terminal's
//! state as the sidebar speaks of it, the listing's tree (windows, terminals, tabs, panes), the
//! summary and where a new terminal opened.

use serde::{Deserialize, Serialize};

use crate::notify::State;

/// A terminal's state in the reply, spelled as the sidebar speaks of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalState {
    Idle,
    Working,
    NeedsYou,
    Finished,
    /// A command ended with a non-zero exit code, or an agent or program stopped on an error, and
    /// nobody has looked yet. A binary built before this state existed can't read a reply that
    /// has it; the protocol version stays, since moving it would turn away every script that
    /// still runs the older one.
    Failed,
    Exited,
    /// A state a newer Wormhole gave, which this build has no word for.
    #[serde(other)]
    Other,
}

impl From<State> for TerminalState {
    fn from(state: State) -> Self {
        match state {
            State::Idle => TerminalState::Idle,
            State::Working => TerminalState::Working,
            State::NeedsYou => TerminalState::NeedsYou,
            State::Done => TerminalState::Finished,
            State::Failed => TerminalState::Failed,
            State::Exited => TerminalState::Exited,
        }
    }
}

impl TerminalState {
    pub fn word(self) -> &'static str {
        match self {
            TerminalState::Idle => "idle",
            TerminalState::Working => "working",
            TerminalState::NeedsYou => "needs you",
            TerminalState::Finished => "finished",
            TerminalState::Failed => "failed",
            TerminalState::Exited => "exited",
            TerminalState::Other => "unknown",
        }
    }
}

/// `list`'s result.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Listing {
    pub windows: Vec<WindowInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowInfo {
    pub id: i64,
    /// The window in front (the one `new-terminal` opens in without `--window`).
    pub front: bool,
    pub terminals: Vec<TerminalInfo>,
    /// The splits of the window: which terminals are shown together. Absent from an older app's reply.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub splits: Vec<SplitInfo>,
}

/// Terminals shown side by side as one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SplitInfo {
    pub id: i64,
    /// Its terminals, in reading order (left to right, then top to bottom).
    pub terminals: Vec<i64>,
    /// The one with the keyboard when the split is shown.
    pub focused: i64,
}

/// A sidebar terminal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerminalInfo {
    pub id: i64,
    pub name: String,
    pub status: TerminalState,
    /// The coding agent in its lead tab, by id (`claude`, `codex`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// Its selected tab's folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// Changed files in its folder's repository, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changed: Option<usize>,
    pub pinned: bool,
    /// The profile it is in, named only while there is more than one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// On screen in its window.
    pub selected: bool,
    /// The split it is in, when it is in one (see the window's `splits`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub split: Option<i64>,
    pub tabs: Vec<TabInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabInfo {
    pub id: i64,
    pub name: String,
    pub status: TerminalState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// The agent's last words, when it needs you.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    /// The program the tab runs ("zsh", "claude").
    pub shell: String,
    /// The program in its foreground, when one is ("npm run dev").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Seconds in its status.
    pub since_seconds: u64,
    pub selected: bool,
    pub panes: Vec<PaneInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaneInfo {
    pub id: i64,
}

/// `status`'s result for one terminal or tab.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusInfo {
    pub window: i64,
    pub terminal: TerminalInfo,
    /// The tab the status is about: the one asked for, else the terminal's selected tab.
    pub tab: i64,
}

/// `status`'s result without a target, outside any tab: the app as a whole.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    pub windows: usize,
    pub terminals: usize,
    pub needs_you: usize,
    pub working: usize,
    pub finished: usize,
    /// Added with the failed state: absent in an older app's reply.
    #[serde(default)]
    pub failed: usize,
    pub idle: usize,
    pub exited: usize,
}

impl Summary {
    /// Count `terminals` (each by its lead tab's state) into a summary over `windows` windows.
    pub fn of(windows: usize, terminals: impl IntoIterator<Item = TerminalState>) -> Self {
        let mut summary = Summary { windows, ..Summary::default() };
        for state in terminals {
            summary.terminals += 1;
            match state {
                TerminalState::NeedsYou => summary.needs_you += 1,
                TerminalState::Working => summary.working += 1,
                TerminalState::Finished => summary.finished += 1,
                TerminalState::Failed => summary.failed += 1,
                TerminalState::Idle => summary.idle += 1,
                TerminalState::Exited => summary.exited += 1,
                // Counted as a terminal, in no state this build can name.
                TerminalState::Other => {}
            }
        }
        summary
    }
}

/// `new-terminal`'s result: where the terminal is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Opened {
    pub window: i64,
    pub terminal: i64,
    pub tab: i64,
    pub pane: i64,
    pub folder: Option<String>,
    /// The worktree's branch and folder, when one was made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree: Option<String>,
}

/// `new-terminal --worktree`'s result: the worktree is never made without the user's say, so the
/// dialog that asks is up in `window`; the terminal opens once they confirm.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Asked {
    pub asked: bool,
    pub window: i64,
    pub branch: String,
    pub folder: String,
}

/// `notify`'s and `send-text`'s result: which tab it reached.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reached {
    pub tab: i64,
}

/// `send-keys`' result: which tab got how many items.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeysSent {
    pub tab: i64,
    pub keys: usize,
}

#[cfg(test)]
mod tests {
    use super::{Summary, TerminalState};

    #[test]
    fn states_map_to_words() {
        assert_eq!(TerminalState::from(crate::notify::State::Done), TerminalState::Finished);
        assert_eq!(TerminalState::from(crate::notify::State::Failed), TerminalState::Failed);
        assert_eq!(TerminalState::NeedsYou.word(), "needs you");
        assert_eq!(TerminalState::Failed.word(), "failed");
        assert_eq!(serde_json::to_string(&TerminalState::Failed).expect("serializes"), r#""failed""#);
        assert_eq!(serde_json::to_string(&TerminalState::NeedsYou).expect("serializes"), r#""needs_you""#);
    }

    /// An app that has no failed count yet answers without it; the command line reads that as none.
    #[test]
    fn a_summary_from_an_older_app_has_no_failed_count() {
        let older =
            r#"{"windows":1,"terminals":2,"needs_you":0,"working":1,"finished":1,"idle":0,"exited":0}"#;
        let summary: Summary = serde_json::from_str(older).expect("reads");
        assert_eq!(summary.failed, 0);
        let counted = Summary::of(1, [TerminalState::Failed, TerminalState::Failed, TerminalState::Idle]);
        assert_eq!((counted.failed, counted.idle, counted.terminals), (2, 1, 3));
    }
}
