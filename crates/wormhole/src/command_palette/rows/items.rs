//! The things the palette lists and the rows it draws them in: a terminal, a folder, a project's
//! action, a profile, what was chosen before, what a result is and what a line of the palette is.

use std::ops::Range;
use std::path::PathBuf;

use wormhole_core::keys::keymap::ActionId;
use wormhole_core::notify::{State, Terminal};
use wormhole_core::terminal::shells::ShellChoice;

/// A terminal of any window, as the palette lists it.
#[derive(Clone, Debug)]
pub(crate) struct TerminalItem {
    pub terminal: Terminal,
    /// Its window as `state` and the title bar number them ("Window 2").
    pub window_number: usize,
    /// Its ⌘ key in the palette's own window, as the sidebar numbers them.
    pub number: Option<u8>,
}

/// A folder with terminals open in it, or one opened lately.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FolderItem {
    pub path: PathBuf,
    pub name: String,
    /// Where it is ("~/Projects").
    pub parent: String,
    /// Its terminals, in every window.
    pub terminals: usize,
}

/// The front terminal's project: its name for the group's label, its actions.
#[derive(Clone, Debug)]
pub(crate) struct ProjectItem {
    /// "terminal/.wormhole".
    pub name: String,
    pub actions: Vec<ProjectAction>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProjectAction {
    pub id: String,
    pub title: String,
    /// The command it runs, if it runs one.
    pub command: Option<String>,
    pub key: Option<String>,
}

/// A profile the palette can switch to, open a window in or move the window to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProfileChoice {
    pub id: String,
    pub name: String,
}

/// What a profile row of the palette does with its profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProfileCommand {
    /// Wormhole goes to it: every window switches.
    Switch,
    OpenWindowIn,
    MoveWindowTo,
}

/// Something chosen in the palette before, which its empty query lists under Recent, in the
/// order they were used (one mixed list of terminals, folders, settings and commits).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Recent {
    /// A terminal, by its id (what its hooks report).
    Terminal(i64),
    Folder(PathBuf),
    Setting(&'static str),
    Commit(String),
}

/// How many of the things chosen before Recent lists.
pub(crate) const RECENT_MAX: usize = 6;

/// What a result is, to draw it and to carry it out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Item {
    /// A terminal, by the tab that speaks for it.
    Terminal(i64),
    Folder(PathBuf),
    /// A commit, by its sha.
    Commit(String),
    Setting(&'static str),
    Action(ActionId),
    ProjectAction(String),
    /// A new terminal running the shell with this key (core `shells`).
    NewTerminalWith(String),
    /// A profile's id, and what to do with it.
    Profile(ProfileCommand, String),
}

impl Item {
    /// What a shell entry is called: "New terminal: fish".
    pub(crate) fn shell_label(&self, choice: &ShellChoice) -> Option<String> {
        match self {
            Item::NewTerminalWith(_) => Some(format!("New terminal: {}", choice.name)),
            _ => None,
        }
    }

    /// What a profile row is called: "Switch to Work", "Open window in Work", "Move this window to Work".
    pub(crate) fn profile_label(&self, choice: &ProfileChoice) -> Option<String> {
        match self {
            Item::Profile(ProfileCommand::Switch, _) => Some(format!("Switch to {}", choice.name)),
            Item::Profile(ProfileCommand::OpenWindowIn, _) => Some(format!("Open window in {}", choice.name)),
            Item::Profile(ProfileCommand::MoveWindowTo, _) => {
                Some(format!("Move this window to {}", choice.name))
            }
            _ => None,
        }
    }
}

/// One line of the palette.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Row {
    /// A group's label ("Settings 3"); `trusted` marks the project's group.
    Label { title: String, count: Option<usize>, trusted: bool },
    /// A result, with the characters of its title the search found.
    Result { item: Item, marks: Vec<Range<usize>> },
    /// Where a list has nothing.
    Note(&'static str),
}

impl Row {
    pub(crate) fn selectable(&self) -> bool {
        matches!(self, Row::Result { .. })
    }

    pub(crate) fn item(&self) -> Option<&Item> {
        match self {
            Row::Result { item, .. } => Some(item),
            _ => None,
        }
    }
}

pub(super) fn label(title: impl Into<String>, count: Option<usize>) -> Row {
    Row::Label { title: title.into(), count, trusted: false }
}

pub(super) fn result(item: Item) -> Row {
    Row::Result { item, marks: Vec::new() }
}

/// The section a terminal is listed under in the Terminals scope: who needs you first.
pub(super) fn state_title(state: State) -> &'static str {
    match state {
        State::NeedsYou => "Needs you",
        State::Working => "Working",
        State::Done | State::Failed => "Finished",
        State::Idle | State::Exited => "Idle",
    }
}
