//! What the palette knows when it opens, and the questions it asks of that.

use std::path::PathBuf;

use wormhole_core::config::settings_index::{self, SectionId};
use wormhole_core::git::Commit;
use wormhole_core::keys::keymap::ActionId;
use wormhole_core::keys::keymap::palette::Action;
use wormhole_core::notify::{self, Terminal};
use wormhole_core::terminal::shells::ShellChoice;

use super::items::{
    FolderItem, Item, ProfileChoice, ProfileCommand, ProjectAction, ProjectItem, Recent, TerminalItem,
};

/// What the palette knows, read when it opens and again while it is open (never while a window
/// draws: see `CommandPalette::refresh`).
#[derive(Clone, Debug, Default)]
pub(crate) struct Snapshot {
    pub terminals: Vec<TerminalItem>,
    pub folders: Vec<FolderItem>,
    /// The front terminal's repository and the commits on its branch, once git has answered.
    pub commits: Vec<Commit>,
    pub repo: Option<PathBuf>,
    /// The actions the window in front can do now.
    pub actions: Vec<Action>,
    /// The shells a new terminal can run besides the default (which the plain New terminal
    /// opens), in core's order.
    pub shells: Vec<ShellChoice>,
    /// The profiles, with the ids of the one Wormhole is in and of the window's own.
    pub profiles: Vec<ProfileChoice>,
    pub shown_profile: String,
    pub window_profile: String,
    pub project: Option<ProjectItem>,
    pub recent: Vec<Recent>,
    /// For the suggestions' details: where a new terminal opens, how many files changed.
    pub cwd: Option<String>,
    pub changes: Option<usize>,
    /// Settings sections not shown in this build (a feature-flagged Preview section that is off),
    /// so the palette doesn't list or reveal their rows. Empty means nothing is hidden.
    pub hidden_sections: Vec<SectionId>,
}

impl Snapshot {
    /// The terminals in the order that matters: needing you (longest waiting first), working,
    /// finished (newest first), then the rest as the windows list them.
    pub(super) fn terminals_by_urgency(&self) -> Vec<usize> {
        let terminals: Vec<Terminal> = self.terminals.iter().map(|t| t.terminal.clone()).collect();
        let sections = notify::sections(&terminals);
        [sections.needs_you, sections.working, sections.finished, sections.other].concat()
    }

    pub(crate) fn terminal(&self, tab: i64) -> Option<&TerminalItem> {
        self.terminals.iter().find(|t| t.terminal.tab == tab)
    }

    pub(crate) fn folder(&self, path: &PathBuf) -> Option<&FolderItem> {
        self.folders.iter().find(|f| &f.path == path)
    }

    pub(crate) fn commit(&self, sha: &str) -> Option<&Commit> {
        self.commits.iter().find(|c| c.sha == sha)
    }

    pub(crate) fn action(&self, id: ActionId) -> Option<&Action> {
        self.actions.iter().find(|a| a.id == id)
    }

    pub(crate) fn project_action(&self, id: &str) -> Option<&ProjectAction> {
        self.project.as_ref()?.actions.iter().find(|a| a.id == id)
    }

    pub(crate) fn shell(&self, key: &str) -> Option<&ShellChoice> {
        self.shells.iter().find(|choice| choice.key == key)
    }

    /// The shell entries, as the Actions scope lists them after the table's Terminal actions: a
    /// terminal with each shell.
    pub(super) fn shell_items(&self) -> impl Iterator<Item = Item> + '_ {
        self.shells.iter().map(|choice| Item::NewTerminalWith(choice.key.clone()))
    }

    pub(crate) fn profile(&self, id: &str) -> Option<&ProfileChoice> {
        self.profiles.iter().find(|choice| choice.id == id)
    }

    /// The profile rows: with another profile to go to, a row to switch to each profile that
    /// isn't shown, to open a window in each, and to move this window to each it isn't in.
    pub(super) fn profile_items(&self) -> Vec<Item> {
        if self.profiles.len() < 2 {
            return Vec::new();
        }
        let mut items = Vec::new();
        for (command, skip) in [
            (ProfileCommand::Switch, Some(&self.shown_profile)),
            (ProfileCommand::OpenWindowIn, None),
            (ProfileCommand::MoveWindowTo, Some(&self.window_profile)),
        ] {
            items.extend(
                self.profiles
                    .iter()
                    .filter(|choice| Some(&choice.id) != skip)
                    .map(|choice| Item::Profile(command, choice.id.clone())),
            );
        }
        items
    }

    /// What a shell entry is searched by: its label, and what it runs.
    pub(super) fn shell_words(&self, item: &Item) -> Option<(String, &str)> {
        let key = match item {
            Item::NewTerminalWith(key) => key,
            _ => return None,
        };
        let choice = self.shell(key)?;
        Some((item.shell_label(choice)?, choice.command_line.as_str()))
    }
}

impl Snapshot {
    /// What was chosen in the palette before and is still there, most recent first.
    pub(super) fn recent_items(&self) -> impl Iterator<Item = Item> + '_ {
        self.recent.iter().filter_map(|item| match item {
            Recent::Terminal(tab) if self.terminal(*tab).is_some() => Some(Item::Terminal(*tab)),
            Recent::Folder(path) if self.folder(path).is_some() => Some(Item::Folder(path.clone())),
            Recent::Setting(id)
                if settings_index::entry(id)
                    .is_some_and(|(section, _, _)| !self.hidden_sections.contains(&section)) =>
            {
                Some(Item::Setting(id))
            }
            Recent::Commit(sha) if self.commit(sha).is_some() => Some(Item::Commit(sha.clone())),
            _ => None,
        })
    }
}
