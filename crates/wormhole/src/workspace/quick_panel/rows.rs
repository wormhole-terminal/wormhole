//! What the quick panel lists, as data: without a query the status item's sections and
//! Do; with one, the terminals and the things to do that it finds.

use std::path::PathBuf;

use wormhole_core::notify::{self, Terminal};

use crate::menubar::ActionItem;

/// Something the panel's Do section offers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Do {
    /// A new terminal where the one in front is.
    NewTerminal {
        cwd: Option<PathBuf>,
    },
    OpenFolder,
    /// A project's action.
    Action(ActionItem),
}

impl Do {
    pub(crate) fn title(&self) -> String {
        match self {
            Do::NewTerminal { .. } => "New terminal".into(),
            Do::OpenFolder => "Open folder…".into(),
            Do::Action(action) => format!("Run {}", action.title),
        }
    }

    pub(crate) fn detail(&self) -> Option<String> {
        match self {
            Do::NewTerminal { cwd } => {
                let home = std::env::var_os("HOME").map(PathBuf::from);
                cwd.as_deref().map(|cwd| {
                    format!("in {}", wormhole_core::view::sidebar::display_path(cwd, home.as_deref()))
                })
            }
            Do::OpenFolder => None,
            Do::Action(action) => Some(format!("action · {}", action.project)),
        }
    }

    /// The app's own key for it, shown as a key cap.
    pub(crate) fn keys(&self) -> Option<&'static str> {
        match self {
            Do::NewTerminal { .. } => Some("⇧⌘N"),
            Do::OpenFolder => Some("⌘O"),
            Do::Action(_) => None,
        }
    }

    fn matches(&self, query: &str) -> bool {
        let haystack = format!("{} {}", self.title(), self.detail().unwrap_or_default()).to_lowercase();
        query.split_whitespace().all(|word| haystack.contains(&word.to_lowercase()))
    }
}

/// One line of the panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Row {
    /// A section's label ("Needs you 2").
    Label {
        title: &'static str,
        count: Option<usize>,
    },
    /// A quiet line where a section has nothing ("Nothing needs you").
    Note(&'static str),
    Terminal(Terminal),
    Do(Do),
}

impl Row {
    /// Whether ↑ ↓ stop on it.
    pub(crate) fn selectable(&self) -> bool {
        matches!(self, Row::Terminal(_) | Row::Do(_))
    }

    /// A terminal that waits for an answer: ⌘⏎ opens Reply on it.
    pub(crate) fn waiting(&self) -> Option<&Terminal> {
        match self {
            Row::Terminal(terminal) if terminal.state == notify::State::NeedsYou => Some(terminal),
            _ => None,
        }
    }
}

/// The panel's rows for `query` (empty: the sections).
pub(crate) fn rows(terminals: &[Terminal], items: &[Do], query: &str) -> Vec<Row> {
    let query = query.trim();
    let sections = notify::sections(terminals);
    let mut rows = Vec::new();
    let terminal = |ix: &usize| Row::Terminal(terminals[*ix].clone());
    if query.is_empty() {
        rows.push(Row::Label { title: "Needs you", count: Some(sections.needs_you.len()) });
        if sections.needs_you.is_empty() {
            rows.push(Row::Note("Nothing needs you"));
        }
        rows.extend(sections.needs_you.iter().map(terminal));
        for (title, list) in [("Working", &sections.working), ("Recent", &sections.finished)] {
            if !list.is_empty() {
                rows.push(Row::Label { title, count: Some(list.len()) });
                rows.extend(list.iter().map(terminal));
            }
        }
        rows.push(Row::Label { title: "Do", count: None });
        rows.extend(items.iter().cloned().map(Row::Do));
        return rows;
    }
    let found: Vec<Row> = [&sections.needs_you, &sections.working, &sections.finished, &sections.other]
        .into_iter()
        .flatten()
        .filter(|ix| notify::matches(query, &terminals[**ix]))
        .map(terminal)
        .collect();
    if !found.is_empty() {
        rows.push(Row::Label { title: "Terminals", count: Some(found.len()) });
        rows.extend(found);
    }
    let found: Vec<Row> = items.iter().filter(|item| item.matches(query)).cloned().map(Row::Do).collect();
    if !found.is_empty() {
        rows.push(Row::Label { title: "Do", count: None });
        rows.extend(found);
    }
    if rows.is_empty() {
        rows.push(Row::Note("Nothing matches"));
    }
    rows
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use wormhole_core::notify::{Source, State, Terminal};

    use super::{Do, Row, rows};
    use crate::menubar::ActionItem;

    fn terminal(tab: i64, name: &str, state: State, secs: u64) -> Terminal {
        Terminal {
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
            folder: None,
            branch: None,
        }
    }

    fn shape(rows: &[Row]) -> Vec<String> {
        rows.iter()
            .map(|row| match row {
                Row::Label { title, count } => {
                    format!("[{title}{}]", count.map(|n| format!(" {n}")).unwrap_or_default())
                }
                Row::Note(text) => format!("({text})"),
                Row::Terminal(t) => t.name.clone(),
                Row::Do(item) => item.title(),
            })
            .collect()
    }

    fn items() -> Vec<Do> {
        vec![
            Do::NewTerminal { cwd: None },
            Do::OpenFolder,
            Do::Action(ActionItem {
                project: "api-server".into(),
                root: "/src/api-server".into(),
                id: "test".into(),
                title: "test".into(),
            }),
        ]
    }

    #[test]
    fn without_a_query_it_shows_the_menus_sections_then_do() {
        let terminals = [
            terminal(1, "diff perf", State::NeedsYou, 120),
            terminal(2, "prod", State::NeedsYou, 300),
            terminal(3, "tests", State::Working, 72),
            terminal(4, "build", State::Done, 120),
            terminal(5, "idle", State::Idle, 0),
        ];
        assert_eq!(
            shape(&rows(&terminals, &items(), "")),
            [
                "[Needs you 2]",
                "prod",
                "diff perf",
                "[Working 1]",
                "tests",
                "[Recent 1]",
                "build",
                "[Do]",
                "New terminal",
                "Open folder…",
                "Run test",
            ]
        );
    }

    #[test]
    fn nothing_waiting_says_so() {
        let shown = shape(&rows(&[terminal(1, "idle", State::Idle, 0)], &items(), " "));
        assert_eq!(shown[..2], ["[Needs you 0]", "(Nothing needs you)"]);
        assert_eq!(shown[2], "[Do]");
    }

    #[test]
    fn a_query_finds_terminals_of_any_state_and_things_to_do() {
        let terminals =
            [terminal(1, "diff perf", State::NeedsYou, 0), terminal(2, "diff idle", State::Idle, 0)];
        assert_eq!(shape(&rows(&terminals, &items(), "diff")), ["[Terminals 2]", "diff perf", "diff idle"]);
        assert_eq!(shape(&rows(&terminals, &items(), "api")), ["[Do]", "Run test"]);
        assert_eq!(shape(&rows(&terminals, &items(), "zebra")), ["(Nothing matches)"]);
    }

    #[test]
    fn only_terminals_and_actions_take_the_selection_and_only_waiting_ones_take_a_reply() {
        let waiting = Row::Terminal(terminal(1, "a", State::NeedsYou, 0));
        let working = Row::Terminal(terminal(2, "b", State::Working, 0));
        assert!(waiting.selectable() && working.selectable() && Row::Do(Do::OpenFolder).selectable());
        assert!(!Row::Label { title: "Do", count: None }.selectable() && !Row::Note("x").selectable());
        assert!(waiting.waiting().is_some() && working.waiting().is_none());
    }
}
