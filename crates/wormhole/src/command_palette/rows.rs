//! What the command palette lists, as data: what it knows when it opens (`Snapshot`), and
//! the rows that make of a scope and a query. Core's `search` finds and ranks; the settings come
//! from core's `settings_index` search, in its order.

use std::ops::Range;
use std::path::PathBuf;
use std::time::Instant;

use wormhole_core::config::settings_index::{self, Query, SectionId};
use wormhole_core::git::Commit;
use wormhole_core::keys::keymap::ActionId;
use wormhole_core::keys::keymap::palette::{self as actions, Action, Section as ActionSection};
use wormhole_core::notify::{self, State, Terminal};
use wormhole_core::terminal::shells::ShellChoice;
use wormhole_core::view::search::{self, Candidate, Group, Ranked, Scope};

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

fn label(title: impl Into<String>, count: Option<usize>) -> Row {
    Row::Label { title: title.into(), count, trusted: false }
}

fn result(item: Item) -> Row {
    Row::Result { item, marks: Vec::new() }
}

/// The section a terminal is listed under in the Terminals scope: who needs you first.
fn state_title(state: State) -> &'static str {
    match state {
        State::NeedsYou => "Needs you",
        State::Working => "Working",
        State::Done | State::Failed => "Finished",
        State::Idle | State::Exited => "Idle",
    }
}

impl Snapshot {
    /// The terminals in the order that matters: needing you (longest waiting first), working,
    /// finished (newest first), then the rest as the windows list them.
    fn terminals_by_urgency(&self) -> Vec<usize> {
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
    fn shell_items(&self) -> impl Iterator<Item = Item> + '_ {
        self.shells.iter().map(|choice| Item::NewTerminalWith(choice.key.clone()))
    }

    pub(crate) fn profile(&self, id: &str) -> Option<&ProfileChoice> {
        self.profiles.iter().find(|choice| choice.id == id)
    }

    /// The profile rows: with another profile to go to, a row to switch to each profile that
    /// isn't shown, to open a window in each, and to move this window to each it isn't in.
    fn profile_items(&self) -> Vec<Item> {
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
    fn shell_words(&self, item: &Item) -> Option<(String, &str)> {
        let key = match item {
            Item::NewTerminalWith(key) => key,
            _ => return None,
        };
        let choice = self.shell(key)?;
        Some((item.shell_label(choice)?, choice.command_line.as_str()))
    }
}

/// The palette's rows for `scope` and `query` (the words after any prefix).
pub(crate) fn rows(snapshot: &Snapshot, scope: Scope, query: &str) -> Vec<Row> {
    let query = query.trim();
    let rows = match (scope, query.is_empty()) {
        (Scope::All, true) => suggestions(snapshot),
        (Scope::Terminals, true) => terminals_by_state(snapshot),
        (Scope::Actions, true) => actions_by_section(snapshot),
        (Scope::Settings, true) => recent_settings(snapshot),
        _ => found(snapshot, scope, query),
    };
    if rows.is_empty() {
        return vec![Row::Note(if query.is_empty() { "Nothing here yet" } else { "Nothing matches" })];
    }
    rows
}

impl Snapshot {
    /// What was chosen in the palette before and is still there, most recent first.
    fn recent_items(&self) -> impl Iterator<Item = Item> + '_ {
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

/// The empty query: Recent (what was opened from the palette last, terminals, folders,
/// settings and commits in one list), then the actions that make sense here, each with its key.
fn suggestions(snapshot: &Snapshot) -> Vec<Row> {
    let recent: Vec<Row> = snapshot.recent_items().take(RECENT_MAX).map(result).collect();
    let mut rows = Vec::new();
    if !recent.is_empty() {
        rows.push(label("Recent", None));
        rows.extend(recent);
    }
    let suggested = actions::suggested(&snapshot.actions);
    if !suggested.is_empty() {
        rows.push(label("Actions", None));
        rows.extend(suggested.into_iter().map(|a| result(Item::Action(a.id))));
    }
    rows
}

/// The Terminals scope with nothing typed: every window's, under who needs you, working,
/// finished and idle.
fn terminals_by_state(snapshot: &Snapshot) -> Vec<Row> {
    let mut rows = Vec::new();
    let order = snapshot.terminals_by_urgency();
    for state in [State::NeedsYou, State::Working, State::Done, State::Idle] {
        let title = state_title(state);
        let tabs: Vec<i64> = order
            .iter()
            .map(|&ix| &snapshot.terminals[ix].terminal)
            .filter(|t| state_title(t.state) == title)
            .map(|t| t.tab)
            .collect();
        if tabs.is_empty() {
            continue;
        }
        rows.push(label(title, Some(tabs.len())));
        rows.extend(tabs.into_iter().map(|tab| result(Item::Terminal(tab))));
    }
    rows
}

/// The Actions scope with nothing typed: the table's actions in their groups (the shells to open
/// a terminal with at the end of Terminal's), the project's own (once trusted) between
/// View and App.
fn actions_by_section(snapshot: &Snapshot) -> Vec<Row> {
    let mut rows = action_section(snapshot, ActionSection::Terminal);
    if !rows.is_empty() {
        rows.extend(snapshot.shell_items().map(result));
    }
    rows.extend(action_section(snapshot, ActionSection::View));
    if let Some(project) = snapshot.project.as_ref().filter(|p| !p.actions.is_empty()) {
        rows.push(Row::Label { title: format!("Project · {}", project.name), count: None, trusted: true });
        rows.extend(project.actions.iter().map(|a| result(Item::ProjectAction(a.id.clone()))));
    }
    rows.extend(action_section(snapshot, ActionSection::App));
    let profiles = snapshot.profile_items();
    if !profiles.is_empty() {
        rows.push(label("Profiles", None));
        rows.extend(profiles.into_iter().map(result));
    }
    rows
}

fn action_section(snapshot: &Snapshot, section: ActionSection) -> Vec<Row> {
    let actions: Vec<Row> = snapshot
        .actions
        .iter()
        .filter(|a| a.section == section)
        .map(|a| result(Item::Action(a.id)))
        .collect();
    if actions.is_empty() {
        return actions;
    }
    std::iter::once(label(section.title(), None)).chain(actions).collect()
}

/// The Settings scope with nothing typed: the settings opened from the palette lately, as
/// Recent lists them; typing finds any setting.
fn recent_settings(snapshot: &Snapshot) -> Vec<Row> {
    let recent: Vec<Row> =
        snapshot.recent_items().filter(|item| matches!(item, Item::Setting(_))).map(result).collect();
    if recent.is_empty() {
        return vec![Row::Note("Type to find a setting")];
    }
    std::iter::once(label("Recent", None)).chain(recent).collect()
}

/// A query: every group that finds something (or the scope's), in the palette's group order.
fn found(snapshot: &Snapshot, scope: Scope, query: &str) -> Vec<Row> {
    let wanted = |group: Group| scope.group().is_none_or(|g| g == group);
    let mut groups: Vec<(Group, Vec<Ranked>)> = Vec::new();
    let mut items: Vec<(Group, Vec<Item>)> = Vec::new();
    // The shell entries' labels are made here (a candidate borrows its words).
    let shell_words: Vec<(Item, String, &str)> = snapshot
        .shell_items()
        .filter_map(|item| snapshot.shell_words(&item).map(|(label, line)| (item, label, line)))
        .collect();
    let profile_words: Vec<(Item, String)> = snapshot
        .profile_items()
        .into_iter()
        .filter_map(|item| {
            let Item::Profile(_, id) = &item else { return None };
            let label = item.profile_label(snapshot.profile(id)?)?;
            Some((item, label))
        })
        .collect();
    if wanted(Group::Settings) {
        // The settings index scores its own rows (label first, then config key, description…)
        // and lists them page by page, as its scope does; everything searched shows only a few
        // of them, so there the best match comes first.
        // Drop hits in a section this build hides (a Preview section whose flag is off), so a
        // release build never lists Project folders or Feature flags rows here.
        let hits: Vec<_> = settings_index::search(&Query::new(query))
            .into_iter()
            .filter(|hit| !snapshot.hidden_sections.contains(&hit.section))
            .collect();
        let mut ranked: Vec<Ranked> = hits
            .iter()
            .enumerate()
            .map(|(index, hit)| Ranked {
                index,
                found: search::Match {
                    score: hit.score as i32,
                    ranges: hit.label_range.iter().cloned().collect(),
                },
            })
            .collect();
        if scope == Scope::All {
            ranked.sort_by(|a, b| b.found.score.cmp(&a.found.score).then(a.index.cmp(&b.index)));
        }
        items.push((Group::Settings, hits.iter().map(|hit| Item::Setting(hit.entry.id)).collect()));
        groups.push((Group::Settings, ranked));
    }
    if wanted(Group::Actions) {
        let mut list: Vec<Item> = snapshot.actions.iter().map(|a| Item::Action(a.id)).collect();
        let mut candidates: Vec<Candidate> =
            snapshot.actions.iter().map(|a| Candidate::new(&a.label)).collect();
        if let Some(project) = &snapshot.project {
            list.extend(project.actions.iter().map(|a| Item::ProjectAction(a.id.clone())));
            candidates.extend(project.actions.iter().map(|a| match &a.command {
                Some(command) => Candidate::new(&a.title).with(command),
                None => Candidate::new(&a.title),
            }));
        }
        for (item, label, command_line) in &shell_words {
            list.push(item.clone());
            candidates.push(Candidate::new(label).with(command_line));
        }
        for item in &profile_words {
            list.push(item.0.clone());
            candidates.push(Candidate::new(&item.1));
        }
        groups.push((Group::Actions, search::rank(query, candidates)));
        items.push((Group::Actions, list));
    }
    if wanted(Group::Terminals) {
        let candidates = snapshot.terminals.iter().map(|t| {
            let terminal = &t.terminal;
            let mut candidate = Candidate::new(&terminal.name);
            for text in
                [&terminal.folder, &terminal.branch, &terminal.command, &terminal.who, &terminal.message]
                    .into_iter()
                    .flatten()
            {
                candidate = candidate.with(text);
            }
            candidate
        });
        groups.push((Group::Terminals, search::rank(query, candidates)));
        items.push((
            Group::Terminals,
            snapshot.terminals.iter().map(|t| Item::Terminal(t.terminal.tab)).collect(),
        ));
    }
    if wanted(Group::Folders) {
        let candidates = snapshot.folders.iter().map(|f| Candidate::new(&f.name).with(&f.parent));
        groups.push((Group::Folders, search::rank(query, candidates)));
        items.push((Group::Folders, snapshot.folders.iter().map(|f| Item::Folder(f.path.clone())).collect()));
    }
    if wanted(Group::Commits) {
        let candidates =
            snapshot.commits.iter().map(|c| Candidate::new(&c.subject).with(&c.short).with(&c.author));
        groups.push((Group::Commits, search::rank(query, candidates)));
        items.push((Group::Commits, snapshot.commits.iter().map(|c| Item::Commit(c.sha.clone())).collect()));
    }
    let mut rows = Vec::new();
    for section in search::sections(scope, groups) {
        let Some((_, list)) = items.iter().find(|(group, _)| *group == section.group) else { continue };
        rows.push(label(section.group.title(), Some(section.total)));
        rows.extend(section.shown.into_iter().filter_map(|ranked| {
            let item = list.get(ranked.index)?.clone();
            Some(Row::Result { item, marks: ranked.found.ranges })
        }));
    }
    rows
}

/// How many results a query found (the footer's count): every group's whole count.
pub(crate) fn result_count(rows: &[Row]) -> usize {
    rows.iter()
        .filter_map(|row| match row {
            Row::Label { count: Some(n), .. } => Some(*n),
            _ => None,
        })
        .sum()
}

/// A terminal's detail in the palette: its window and folder in a scope ("Window 1 · terminal ·
/// 4:12"), else what it is doing ("Needs you · Approve cargo test?", "terminal · Editing …").
pub(crate) fn terminal_detail(item: &TerminalItem, scoped: bool, now: Instant) -> String {
    let terminal = &item.terminal;
    let elapsed = notify::elapsed(terminal.state, now.saturating_duration_since(terminal.since));
    if scoped {
        let window = format!("Window {}", item.window_number);
        return [Some(window), terminal.folder.clone(), elapsed]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" · ");
    }
    let doing = match terminal.state {
        State::NeedsYou => Some("Needs you".to_string()),
        State::Done => Some("Finished".to_string()),
        State::Failed => Some("Failed".to_string()),
        State::Working | State::Idle | State::Exited => terminal.folder.clone(),
    };
    let exit = terminal.exit.map(|code| format!("exit {code}"));
    let words = match terminal.state {
        State::NeedsYou | State::Working => terminal.message.clone().or_else(|| terminal.command.clone()),
        // What went wrong: a program's own words, else the command and how it exited.
        State::Failed => terminal.message.clone().or_else(|| {
            let parts: Vec<String> = [terminal.command.clone(), exit].into_iter().flatten().collect();
            (!parts.is_empty()).then(|| parts.join(" · "))
        }),
        _ => None,
    };
    [doing, words].into_iter().flatten().collect::<Vec<_>>().join(" · ")
}

/// The line under a terminal that waits for an answer, in the Terminals scope.
pub(crate) fn question(item: &TerminalItem) -> Option<String> {
    let terminal = &item.terminal;
    if terminal.state != State::NeedsYou {
        return None;
    }
    let message = terminal.message.as_deref()?;
    Some(match &terminal.who {
        Some(who) => format!("{who} is asking: {message}"),
        None => message.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    use wormhole_core::keys::keymap::ActionId;
    use wormhole_core::keys::keymap::menus::MenuState;
    use wormhole_core::keys::keymap::palette;
    use wormhole_core::notify::{Source, State, Terminal};
    use wormhole_core::view::search::Scope;

    use wormhole_core::terminal::shells::ShellChoice;

    use super::{
        FolderItem, Item, Recent, Row, Snapshot, TerminalItem, question, result_count, rows, terminal_detail,
    };

    fn terminal(tab: i64, name: &str, state: State, secs: u64) -> TerminalItem {
        TerminalItem {
            terminal: Terminal {
                session: tab,
                tab,
                name: name.into(),
                state,
                since: Instant::now() - Duration::from_secs(secs),
                source: Source::Plain,
                who: None,
                message: None,
                command: None,
                took: None,
                exit: None,
                changed: None,
                folder: Some("terminal".into()),
                branch: None,
            },
            window_number: 1,
            number: Some(tab as u8),
        }
    }

    fn snapshot() -> Snapshot {
        Snapshot {
            terminals: vec![
                terminal(1, "sidebar polish", State::Working, 240),
                terminal(2, "diff perf", State::NeedsYou, 60),
                terminal(3, "zsh", State::Idle, 0),
                terminal(4, "auth tokens", State::Done, 720),
                terminal(5, "theme shape metrics", State::Working, 700),
            ],
            folders: vec![FolderItem {
                path: PathBuf::from("/p/api-server"),
                name: "api-server".into(),
                parent: "~/Projects".into(),
                terminals: 2,
            }],
            actions: palette::actions(&MenuState::default()),
            recent: vec![
                Recent::Terminal(2),
                Recent::Folder(PathBuf::from("/p/api-server")),
                Recent::Setting("font-terminal"),
                Recent::Folder(PathBuf::from("/gone")),
                Recent::Terminal(99),
                Recent::Terminal(1),
            ],
            ..Snapshot::default()
        }
    }

    /// The rows as text: "[Label n]", results by name.
    fn shape(snapshot: &Snapshot, rows: &[Row]) -> Vec<String> {
        rows.iter()
            .map(|row| match row {
                Row::Label { title, count, .. } => {
                    format!("[{title}{}]", count.map(|n| format!(" {n}")).unwrap_or_default())
                }
                Row::Note(text) => format!("({text})"),
                Row::Result { item, .. } => match item {
                    Item::Terminal(tab) => {
                        snapshot.terminal(*tab).map(|t| t.terminal.name.clone()).unwrap_or_default()
                    }
                    Item::Folder(path) => format!("folder {}", path.display()),
                    Item::Commit(sha) => format!("commit {sha}"),
                    Item::Setting(id) => format!("setting {id}"),
                    Item::Action(id) => snapshot.action(*id).map(|a| a.label.clone()).unwrap_or_default(),
                    Item::ProjectAction(id) => format!("project {id}"),
                    Item::NewTerminalWith(key) => {
                        snapshot.shell(key).and_then(|c| item.shell_label(c)).unwrap_or_default()
                    }
                    Item::Profile(_, id) => {
                        snapshot.profile(id).and_then(|c| item.profile_label(c)).unwrap_or_default()
                    }
                },
            })
            .collect()
    }

    fn shell(key: &str, name: &str, command_line: &str) -> ShellChoice {
        ShellChoice {
            key: key.into(),
            name: name.into(),
            command_line: command_line.into(),
            kind: wormhole_core::terminal::shells::ShellKind::Profile,
        }
    }

    /// The shells besides the default come after the Terminal actions, a terminal with each; a
    /// query finds them by name and by what they run.
    #[test]
    fn the_shells_follow_the_terminal_actions_and_are_found_by_what_they_run() {
        let plain = snapshot();
        let with_shells = Snapshot {
            shells: vec![
                shell("fish", "fish", "/opt/homebrew/bin/fish -l"),
                shell("Claude", "Claude", "/bin/zsh -l -c 'exec claude'"),
            ],
            ..plain.clone()
        };
        let shown = shape(&with_shells, &rows(&with_shells, Scope::Actions, ""));
        let view = shown.iter().position(|r| r == "[View]").expect("a View group");
        assert_eq!(&shown[view - 2..view], ["New terminal: fish", "New terminal: Claude"]);
        assert_eq!(
            shape(&with_shells, &rows(&with_shells, Scope::Actions, "exec claude")),
            ["[Actions 1]", "New terminal: Claude"]
        );
        let found = shape(&with_shells, &rows(&with_shells, Scope::All, "terminal fish"));
        assert!(found.contains(&"New terminal: fish".to_string()), "{found:?}");
        // Nothing but the default: no entries at all.
        let shown = shape(&plain, &rows(&plain, Scope::Actions, ""));
        assert!(!shown.iter().any(|r| r.starts_with("New terminal:")));
    }

    /// Recent is one list of what was opened from the palette, in the order it
    /// was used, whatever its kind; what is gone since is left out.
    #[test]
    fn the_empty_query_lists_recent_then_suggested_actions() {
        let snapshot = snapshot();
        assert_eq!(
            shape(&snapshot, &rows(&snapshot, Scope::All, "")),
            [
                "[Recent]",
                "diff perf",
                "folder /p/api-server",
                "setting font-terminal",
                "sidebar polish",
                "[Actions]",
                "New terminal",
                "Open folder…",
                "Show changes",
                "Hide right panel",
                "Stash prompt",
                "Keyboard shortcuts",
            ]
        );
        let fresh = Snapshot { recent: Vec::new(), ..snapshot };
        assert_eq!(shape(&fresh, &rows(&fresh, Scope::All, ""))[0], "[Actions]", "nothing used yet");
    }

    /// The Settings scope with nothing typed lists the settings used lately (the
    /// Recent), and says what to do when there are none.
    #[test]
    fn the_settings_scope_lists_recent_settings() {
        let snapshot = snapshot();
        assert_eq!(
            shape(&snapshot, &rows(&snapshot, Scope::Settings, "")),
            ["[Recent]", "setting font-terminal"]
        );
        let fresh = Snapshot { recent: Vec::new(), ..snapshot };
        assert_eq!(shape(&fresh, &rows(&fresh, Scope::Settings, "")), ["(Type to find a setting)"]);
    }

    /// Everything searched shows the best-matching settings first; the Settings scope
    /// keeps the pages' order, as the settings window's own search does.
    #[test]
    fn the_unscoped_settings_group_puts_the_best_match_first() {
        use wormhole_core::config::settings_index::{Query, search};
        let snapshot = snapshot();
        let settings = |scope: Scope| -> Vec<&'static str> {
            rows(&snapshot, scope, "font")
                .iter()
                .filter_map(|row| match row {
                    Row::Result { item: Item::Setting(id), .. } => Some(*id),
                    _ => None,
                })
                .collect()
        };
        let hits = search(&Query::new("font"));
        let score = |id: &str| hits.iter().find(|hit| hit.entry.id == id).map(|hit| hit.score);
        let scores: Vec<Option<u32>> = settings(Scope::All).iter().map(|id| score(id)).collect();
        assert!(scores.windows(2).all(|pair| pair[0] >= pair[1]), "{scores:?}");
        assert_eq!(scores[0], hits.iter().map(|hit| hit.score).max());
        let page_order: Vec<&str> = hits.iter().map(|hit| hit.entry.id).collect();
        assert_eq!(settings(Scope::Settings), page_order);
        assert_ne!(
            settings(Scope::All),
            page_order[..settings(Scope::All).len()].to_vec(),
            "really reordered"
        );
    }

    #[test]
    fn the_terminals_scope_lists_every_window_by_who_needs_you() {
        let snapshot = snapshot();
        assert_eq!(
            shape(&snapshot, &rows(&snapshot, Scope::Terminals, "")),
            [
                "[Needs you 1]",
                "diff perf",
                "[Working 2]",
                "theme shape metrics",
                "sidebar polish",
                "[Finished 1]",
                "auth tokens",
                "[Idle 1]",
                "zsh",
            ]
        );
    }

    #[test]
    fn a_query_groups_settings_and_actions_before_content() {
        let snapshot = snapshot();
        let found = rows(&snapshot, Scope::All, "theme");
        let labels: Vec<String> =
            shape(&snapshot, &found).into_iter().filter(|r| r.starts_with('[')).collect();
        assert_eq!(labels.first().map(|l| l.starts_with("[Settings")), Some(true), "{labels:?}");
        let actions = labels.iter().position(|l| l.starts_with("[Actions"));
        let terminals = labels.iter().position(|l| l.starts_with("[Terminals"));
        assert!(actions < terminals && terminals.is_some(), "{labels:?}");
        // The terminal's name is marked where the query was found.
        let marks = found.iter().find_map(|row| match row {
            Row::Result { item: Item::Terminal(5), marks } => Some(marks.clone()),
            _ => None,
        });
        assert_eq!(marks.as_deref(), Some(std::slice::from_ref(&(0..5))));
        assert!(result_count(&found) >= 3);
        // A scope shows only its own group.
        let scoped = shape(&snapshot, &rows(&snapshot, Scope::Terminals, "theme"));
        assert_eq!(scoped, ["[Terminals 1]", "theme shape metrics"]);
        assert_eq!(shape(&snapshot, &rows(&snapshot, Scope::Folders, "zebra")), ["(Nothing matches)"]);
    }

    #[test]
    fn the_actions_scope_follows_the_table_and_adds_the_projects_own() {
        let mut snapshot = snapshot();
        snapshot.project = Some(super::ProjectItem {
            name: "terminal/.wormhole".into(),
            actions: vec![super::ProjectAction {
                id: "test".into(),
                title: "Run tests".into(),
                command: Some("cargo test --workspace".into()),
                key: Some("ctrl-cmd-t".into()),
            }],
        });
        let shown = shape(&snapshot, &rows(&snapshot, Scope::Actions, ""));
        let labels: Vec<&String> = shown.iter().filter(|r| r.starts_with('[')).collect();
        assert_eq!(labels, ["[Terminal]", "[View]", "[Project · terminal/.wormhole]", "[App]"]);
        assert!(shown.contains(&"project test".to_string()));
        // The command finds it too.
        let found = shape(&snapshot, &rows(&snapshot, Scope::Actions, "cargo"));
        assert_eq!(found, ["[Actions 1]", "project test"]);
        assert!(snapshot.action(ActionId::KeyboardShortcuts).is_some());
    }

    #[test]
    fn a_terminal_says_where_it_is_in_a_scope_and_what_it_does_elsewhere() {
        let now = Instant::now();
        let mut waiting = terminal(2, "diff perf", State::NeedsYou, 0);
        waiting.terminal.source = Source::Agent;
        waiting.terminal.who = Some("Claude Code".into());
        waiting.terminal.message = Some("Approve cargo test?".into());
        assert_eq!(terminal_detail(&waiting, false, now), "Needs you · Approve cargo test?");
        assert!(terminal_detail(&waiting, true, now).starts_with("Window 1 · terminal"));
        assert_eq!(question(&waiting).as_deref(), Some("Claude Code is asking: Approve cargo test?"));
        let working = terminal(1, "sidebar polish", State::Working, 0);
        assert_eq!(terminal_detail(&working, false, now), "terminal");
        assert_eq!(question(&working), None);
    }

    #[test]
    fn a_failed_terminal_says_what_went_wrong() {
        let now = Instant::now();
        let mut build = terminal(1, "build", State::Failed, 0);
        build.terminal.command = Some("cargo build".into());
        build.terminal.exit = Some(101);
        assert_eq!(terminal_detail(&build, false, now), "Failed · cargo build · exit 101");
        // A program that reports says it in its own words, and is asking nothing.
        let mut deploy = terminal(2, "deploy", State::Failed, 0);
        deploy.terminal.source = Source::Program;
        deploy.terminal.who = Some("deploy.sh".into());
        deploy.terminal.message = Some("rsync failed".into());
        assert_eq!(terminal_detail(&deploy, false, now), "Failed · rsync failed");
        assert_eq!(question(&deploy), None);
        // A program's question reads like an agent's.
        let mut brew = terminal(3, "upgrade", State::NeedsYou, 0);
        brew.terminal.source = Source::Program;
        brew.terminal.who = Some("brew".into());
        brew.terminal.message = Some("Password:".into());
        assert_eq!(question(&brew).as_deref(), Some("brew is asking: Password:"));
    }
}
