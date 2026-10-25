//! The palette's rows: what each scope and query lists.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use wormhole_core::keys::keymap::ActionId;
use wormhole_core::keys::keymap::menus::MenuState;
use wormhole_core::keys::keymap::palette;
use wormhole_core::notify::{Source, State, Terminal};
use wormhole_core::view::search::Scope;

use wormhole_core::terminal::shells::ShellChoice;

use super::detail::{question, terminal_detail};
use super::items::{FolderItem, Item, Recent, Row, TerminalItem};
use super::snapshot::Snapshot;
use super::{result_count, rows};

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
    assert_ne!(settings(Scope::All), page_order[..settings(Scope::All).len()].to_vec(), "really reordered");
}

#[test]
fn the_terminals_scope_lists_every_window_by_who_needs_you() {
    let snapshot = snapshot();
    assert_eq!(
        shape(&snapshot, &rows(&snapshot, Scope::Terminals, "")),
        [
            "[Needs input 1]",
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
    let labels: Vec<String> = shape(&snapshot, &found).into_iter().filter(|r| r.starts_with('[')).collect();
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
    assert_eq!(terminal_detail(&waiting, false, now), "Needs input · Approve cargo test?");
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
