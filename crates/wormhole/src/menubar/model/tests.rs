use std::path::PathBuf;
use std::time::{Duration, Instant};

use wormhole_core::config::MenuBarIcon;
use wormhole_core::keys::keyspec::KeySpec;
use wormhole_core::notify::{ItemState, Source, State, Terminal};

use super::{
    DockEntry, Entry, Glyph, ItemModel, KeyEquivalent, MAX_SESSIONS, Mark, build, dock_menu, dump,
    status_icon,
};
use crate::menubar::{ActionItem, MenuCommand, MenuSnapshot};

fn terminal(tab: i64, state: State, waited: u64, now: Instant) -> Terminal {
    Terminal {
        session: tab,
        tab,
        name: format!("t{tab}"),
        state,
        since: now - Duration::from_secs(waited),
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

fn agent(mut t: Terminal, message: Option<&str>) -> Terminal {
    t.source = Source::Agent;
    t.who = Some("Claude Code".into());
    t.message = message.map(str::to_string);
    t
}

fn items(entries: &[Entry]) -> Vec<&ItemModel> {
    entries
        .iter()
        .filter_map(|e| match e {
            Entry::Item(i) => Some(i),
            _ => None,
        })
        .collect()
}

fn submenu<'a>(model: &'a [Entry], title: &str) -> Option<&'a [Entry]> {
    model.iter().find_map(|e| match e {
        Entry::Submenu { title: t, entries, .. } if t == title => Some(entries.as_slice()),
        _ => None,
    })
}

fn headers(entries: &[Entry]) -> Vec<&str> {
    entries
        .iter()
        .filter_map(|e| match e {
            Entry::Header(h) => Some(h.as_str()),
            _ => None,
        })
        .collect()
}

fn key(spec: &str) -> Option<KeyEquivalent> {
    KeyEquivalent::from_spec(&spec.parse::<KeySpec>().expect("valid spec"))
}

/// The status item's menu: two waiting, two working, one finished, one failed and four others.
fn f40(now: Instant) -> MenuSnapshot {
    let mut diff = agent(terminal(1, State::NeedsYou, 130, now), Some("Approve cargo test?"));
    diff.name = "diff perf".into();
    let mut prod = terminal(2, State::NeedsYou, 300, now);
    prod.name = "prod".into();
    prod.command = Some("ssh deploy@prod-1".into());
    let mut polish = agent(terminal(3, State::Working, 252, now), Some("Editing sidebar.rs…"));
    polish.name = "sidebar polish".into();
    let mut tests = terminal(4, State::Working, 72, now);
    tests.name = "tests".into();
    tests.command = Some("cargo test -p wormhole-core".into());
    let mut build = terminal(5, State::Failed, 120, now);
    build.name = "build".into();
    build.command = Some("cargo build".into());
    build.took = Some(Duration::from_secs(38));
    build.exit = Some(101);
    let mut auth = agent(terminal(6, State::Done, 720, now), None);
    auth.name = "auth tokens".into();
    auth.changed = Some(7);
    let mut terminals = vec![polish, auth, diff, tests, prod, build];
    terminals.extend((7..11).map(|tab| terminal(tab, State::Idle, 0, now)));
    MenuSnapshot {
        terminals,
        recent_projects: vec![("terminal".into(), PathBuf::from("/src/terminal"))],
        actions: vec![ActionItem {
            project: "api-server".into(),
            root: "/src/api-server".into(),
            id: "test".into(),
            title: "test".into(),
        }],
        hotkey: key("shift-cmd-space"),
        jump_key: key("alt-cmd-j"),
        ..MenuSnapshot::default()
    }
}

#[test]
fn the_menu_lists_terminals_by_status() {
    let now = Instant::now();
    let model = build(&f40(now), now);
    assert_eq!(headers(&model.entries), ["Needs input", "Working", "Finished"]);
    let rows: Vec<_> = items(&model.entries)
        .into_iter()
        .take(6)
        .map(|i| (i.title.as_str(), i.trailing.as_deref(), i.subtitle.as_deref(), i.glyph))
        .collect();
    assert_eq!(
        rows,
        [
            ("prod", Some("5m"), Some("Bell · ssh deploy@prod-1"), Some(Glyph::Bell)),
            ("diff perf", Some("2m"), Some("Claude Code · Approve cargo test?"), Some(Glyph::NeedsYou)),
            ("sidebar polish", Some("4:12"), Some("Claude Code · Editing sidebar.rs…"), Some(Glyph::Working)),
            ("tests", Some("1:12"), Some("cargo test -p wormhole-core"), Some(Glyph::Running)),
            ("build", Some("2m ago"), Some("cargo build · exit 101 · after 38s"), Some(Glyph::Failed)),
            ("auth tokens", Some("12m ago"), Some("Claude Code · 7 files changed"), Some(Glyph::Done)),
        ],
        "longest waiting, longest running, newest finished first; a failed one among the finished"
    );
    // Choosing a row goes to its tab.
    assert_eq!(items(&model.entries)[0].command, Some(MenuCommand::OpenTab(2)));
    let others = model.entries.iter().find_map(|e| match e {
        Entry::Submenu { title, trailing, entries } if title == "Other Terminals" => {
            Some((trailing, entries))
        }
        _ => None,
    });
    let (count, others) = others.expect("idle terminals fold into Other Terminals");
    assert_eq!((count.as_deref(), others.len()), (Some("4"), 4));
}

#[test]
fn the_tail_is_f40s() {
    let now = Instant::now();
    let model = build(&f40(now), now);
    let tail: Vec<_> = items(&model.entries)
        .into_iter()
        .skip(6)
        .map(|i| (i.title.as_str(), i.key.as_ref().map(super::key_text)))
        .collect();
    let s = |t: &'static str| Some(t.to_string());
    assert_eq!(
        tail,
        [
            ("New Terminal", s("⌘T")),
            ("Open Folder…", s("⌘O")),
            ("Show Wormhole", s("⇧⌘Space")),
            ("Jump to Next Waiting", s("⌥⌘J")),
            ("Settings…", s("⌘,")),
            ("Check for Updates…", None),
            ("Quit Wormhole", s("⌘Q")),
        ]
    );
    let projects = submenu(&model.entries, "Projects").expect("recent projects and actions");
    assert_eq!(headers(projects), ["Recent", "Actions · api-server"]);
    assert!(matches!(projects.last(), Some(Entry::Item(i)) if i.title == "Open Folder…"));
}

#[test]
fn no_terminals_says_so_above_new_terminal() {
    let model = build(&MenuSnapshot::default(), Instant::now());
    let titles: Vec<_> = items(&model.entries).iter().map(|i| i.title.as_str()).collect();
    assert_eq!(titles[0], "No terminals open");
    assert!(items(&model.entries)[0].command.is_none(), "a disabled line");
    assert_eq!(titles[1], "New Terminal");
    assert!(submenu(&model.entries, "Projects").is_none(), "nothing to list there");
    let jump = items(&model.entries).into_iter().find(|i| i.title == "Jump to Next Waiting").cloned();
    assert_eq!(jump.and_then(|j| j.command), None, "nothing waiting: disabled");
}

#[test]
fn past_twenty_rows_the_rest_is_n_more() {
    let now = Instant::now();
    let terminals = (0..25).map(|i| terminal(i, State::Working, i as u64, now)).collect();
    let model = build(&MenuSnapshot { terminals, ..MenuSnapshot::default() }, now);
    let rows = items(&model.entries);
    assert!(rows.iter().take(MAX_SESSIONS).all(|r| r.glyph.is_some()));
    assert_eq!(rows[MAX_SESSIONS].title, "5 more…");
    assert_eq!(rows[MAX_SESSIONS].command, Some(MenuCommand::ShowApp));
}

#[test]
fn the_status_item_shows_the_most_urgent_state_its_way() {
    let now = Instant::now();
    let all = [
        terminal(1, State::NeedsYou, 0, now),
        terminal(2, State::NeedsYou, 0, now),
        terminal(3, State::Working, 0, now),
    ];
    let icon = status_icon(&all, MenuBarIcon::StatusCount, true);
    assert_eq!(
        (icon.state, icon.mark, icon.count.as_deref()),
        (ItemState::NeedsYou, Mark::NeedsYou, Some("2"))
    );
    assert!(!icon.animate, "only the working state animates");
    assert_eq!(icon.tooltip, "Wormhole — 2 need input · 1 working");
    let dot = status_icon(&all, MenuBarIcon::Status, false);
    assert_eq!((dot.mark, dot.count), (Mark::NeedsYou, None), "the dot without the count");
    let plain = status_icon(&all, MenuBarIcon::Plain, false);
    assert_eq!((plain.mark, plain.count), (Mark::None, None), "never coloured");

    let finished = status_icon(&[terminal(1, State::Done, 0, now)], MenuBarIcon::StatusCount, false);
    assert_eq!((finished.state, finished.mark, finished.count), (ItemState::Finished, Mark::Finished, None));
    // Failed outranks finished and working, and needing you outranks failed.
    let mixed = [
        terminal(1, State::Done, 0, now),
        terminal(2, State::Failed, 0, now),
        terminal(3, State::Working, 0, now),
    ];
    let failed = status_icon(&mixed, MenuBarIcon::StatusCount, false);
    assert_eq!((failed.state, failed.mark, failed.count), (ItemState::Failed, Mark::Failed, None));
    assert_eq!(failed.tooltip, "Wormhole — 1 failed · 1 working · 1 finished");
    let asks = [terminal(4, State::NeedsYou, 0, now), mixed[1].clone()];
    assert_eq!(status_icon(&asks, MenuBarIcon::StatusCount, false).mark, Mark::NeedsYou);
    let plain_failed = status_icon(&mixed, MenuBarIcon::Plain, false);
    assert_eq!(plain_failed.mark, Mark::None, "the plain icon is never coloured");
    let working = status_icon(&[terminal(1, State::Working, 0, now)], MenuBarIcon::StatusCount, true);
    assert_eq!((working.state, working.mark), (ItemState::Working, Mark::None));
    assert!(working.animate);
    let many: Vec<_> = (0..12).map(|i| terminal(i, State::NeedsYou, 0, now)).collect();
    assert_eq!(status_icon(&many, MenuBarIcon::StatusCount, false).count.as_deref(), Some("9+"));
}

/// A program that reports its own status is listed like an agent, under its own name.
#[test]
fn a_reporting_program_reads_like_an_agent_in_the_menu() {
    let now = Instant::now();
    let mut brew = terminal(1, State::NeedsYou, 130, now);
    brew.source = Source::Program;
    brew.who = Some("brew".into());
    brew.message = Some("Password:".into());
    let mut deploy = terminal(2, State::Failed, 60, now);
    deploy.source = Source::Program;
    deploy.who = Some("deploy.sh".into());
    deploy.message = Some("rsync failed".into());
    let model = build(&MenuSnapshot { terminals: vec![brew, deploy], ..MenuSnapshot::default() }, now);
    let rows: Vec<_> =
        items(&model.entries).into_iter().take(2).map(|i| (i.subtitle.as_deref(), i.glyph)).collect();
    assert_eq!(
        rows,
        [
            (Some("brew · Password:"), Some(Glyph::NeedsYou)),
            (Some("deploy.sh · Failed · rsync failed"), Some(Glyph::Failed)),
        ],
        "a question, not a bell"
    );
}

#[test]
fn key_equivalents() {
    let space = key("alt-space").expect("valid");
    assert_eq!(space, KeyEquivalent { key: " ".into(), option: true, ..KeyEquivalent::default() });
    let t = key("ctrl-alt-T").expect("valid");
    assert!(t.control && t.option && !t.command && t.key == "t");
    assert_eq!(key("cmd-backtick").map(|k| k.key), Some("`".into()));
    assert_eq!(key("cmd--").map(|k| k.key), Some("-".into()));
    assert_eq!(key("cmd-f12"), None);
    assert_eq!(key("cmd-enter"), None);
}

#[test]
fn identical_snapshots_give_equal_models() {
    let now = Instant::now();
    let snapshot = f40(now);
    assert_eq!(build(&snapshot, now), build(&snapshot.clone(), now));
}

/// Two open projects named `app` (two clones, or worktrees) whose actions share
/// ids: their menu commands must not be the same, or a click runs the other project's action.
#[test]
fn same_named_projects_stay_apart() {
    let action = |root: &str| ActionItem {
        project: "app".into(),
        root: root.into(),
        id: "deploy".into(),
        title: "Deploy".into(),
    };
    let snapshot =
        MenuSnapshot { actions: vec![action("/a/app"), action("/b/app")], ..MenuSnapshot::default() };
    let model = build(&snapshot, Instant::now());
    let projects = submenu(&model.entries, "Projects").expect("actions");
    let commands: Vec<_> = items(projects).iter().filter_map(|i| i.command.clone()).collect();
    assert_eq!(
        commands[..2],
        [
            MenuCommand::RunAction { root: "/a/app".into(), id: "deploy".into() },
            MenuCommand::RunAction { root: "/b/app".into(), id: "deploy".into() },
        ]
    );
    assert_eq!(headers(projects), ["Actions · app — /a/app", "Actions · app — /b/app"]);
}

#[test]
fn the_dock_menu_lists_who_waits_and_works() {
    let now = Instant::now();
    let snapshot = f40(now);
    let menu = dock_menu(&snapshot.terminals, now);
    assert_eq!(
        menu,
        [
            DockEntry::Header("Needs input"),
            DockEntry::Terminal { label: "prod   5m".into(), tab: 2 },
            DockEntry::Terminal { label: "diff perf   2m".into(), tab: 1 },
            DockEntry::Header("Working"),
            DockEntry::Terminal { label: "sidebar polish   4m".into(), tab: 3 },
            DockEntry::Terminal { label: "tests   1m".into(), tab: 4 },
            DockEntry::Separator,
            DockEntry::NewTerminal,
            DockEntry::NewWindow,
        ]
    );
    assert_eq!(dock_menu(&[], now), [DockEntry::NewTerminal, DockEntry::NewWindow]);
}

#[test]
fn the_dump_reads_like_the_menu() {
    let now = Instant::now();
    let text = dump(&build(&f40(now), now));
    assert!(text.starts_with("icon state=NeedsYou mark=NeedsYou count=Some(\"2\")"), "{text}");
    assert!(text.contains("== Needs input\n[Bell] prod  ⇥ 5m\n    Bell · ssh deploy@prod-1\n"), "{text}");
    assert!(text.contains("Other Terminals  ⇥ 4  ▸\n"), "{text}");
    assert!(text.contains("Jump to Next Waiting  ⌨ ⌥⌘J\n"), "{text}");
}
