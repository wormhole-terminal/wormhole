//! A row's words: the second line, how long until it changes, and what VoiceOver reads. The cases are
//! the app's `sidebar/row/tests.rs`, asked of the model's snapshot.

use std::path::PathBuf;

use crate::agents::AgentId;
use crate::view::sidebar::Status;
use crate::workspace::{Action, Failure, ModelConfig, SecondLine, Tone, WorkspaceModel};

use super::model;

const MINUTE: u64 = 60_000;

fn one(change: impl FnOnce(&mut crate::workspace::TerminalFacts)) -> WorkspaceModel {
    let mut m = model(1);
    m.update_facts(1, change);
    m
}

fn line(m: &WorkspaceModel, now: u64) -> SecondLine {
    m.sidebar_snapshot(now).sections[0].rows[0].second_line.clone()
}

#[test]
fn an_agent_working_says_how_long_and_one_that_asks_says_what() {
    let m = one(|f| {
        f.agent = Some(AgentId::Claude);
        f.status = Status::Busy;
        f.since = 0;
    });
    assert_eq!(line(&m, 30_000).text.as_deref(), Some("Working · <1m"));
    assert_eq!(line(&m, 3 * MINUTE + 5_000).text.as_deref(), Some("Working · 3m"));
    let m = one(|f| {
        f.agent = Some(AgentId::Claude);
        f.status = Status::Attention;
        f.message = Some("Approve cargo test?".into());
    });
    let asking = line(&m, 0);
    assert_eq!(asking.text.as_deref(), Some("Needs input · Approve cargo test?"));
    assert_eq!(asking.tone, Tone::Attention);
}

#[test]
fn an_idle_agent_and_a_bare_shell_say_where_they_are_or_what_they_are() {
    let mut m = model(1);
    m.update_facts(1, |f| f.agent = Some(AgentId::Codex));
    assert_eq!(line(&m, 0).text.as_deref(), Some("Idle"), "no folder is known yet");
    let mut at_home =
        WorkspaceModel::new(ModelConfig { home: Some(PathBuf::from("/Users/me")), ..Default::default() });
    at_home.apply(Action::NewTerminal);
    at_home.update_facts(1, |f| f.cwd = Some(PathBuf::from("/Users/me/Projects/app")));
    assert_eq!(line(&at_home, 0).text.as_deref(), Some("~/Projects/app"));
    let bare = model(1);
    assert_eq!(line(&bare, 0).text.as_deref(), Some("Shell"));
}

#[test]
fn a_command_runs_with_its_clock_and_a_failed_one_says_how_it_ended() {
    let m = one(|f| {
        f.command = Some("cargo test".into());
        f.running_since = Some(0);
    });
    let running = line(&m, 72_000);
    assert_eq!(
        (running.text.as_deref(), running.trail.as_deref(), running.mono),
        (Some("cargo test"), Some("1:12"), true)
    );
    let m = one(|f| {
        f.status = Status::Failed;
        f.failure = Some(Failure { code: 101, command: Some("cargo build".into()), at: 0 });
    });
    let failed = line(&m, 2 * MINUTE);
    assert_eq!(failed.text.as_deref(), Some("cargo build · exit 101"));
    assert_eq!((failed.tone, failed.trail.as_deref()), (Tone::Failed, Some("2m ago")));
    let m = one(|f| f.failure = Some(Failure { code: 2, command: None, at: 0 }));
    assert_eq!(line(&m, 0).text.as_deref(), Some("exit 2"));
    let m = one(|f| f.unread = true);
    assert_eq!(line(&m, 0).trail.as_deref(), None, "new output is for a command that has ended");
}

#[test]
fn the_snapshot_says_when_its_words_next_move_and_an_idle_window_asks_for_nothing() {
    let m = one(|f| {
        f.agent = Some(AgentId::Claude);
        f.status = Status::Busy;
    });
    assert_eq!(m.sidebar_snapshot(45_000).refresh_in_ms, Some(15_000));
    let running = one(|f| {
        f.command = Some("make".into());
        f.running_since = Some(0);
    });
    assert_eq!(running.sidebar_snapshot(500).refresh_in_ms, Some(500), "a command's clock counts seconds");
    assert_eq!(model(2).sidebar_snapshot(0).refresh_in_ms, None);
    let said = one(|f| {
        f.agent = Some(AgentId::Claude);
        f.status = Status::Busy;
        f.message = Some("Editing sidebar.rs…".into());
    });
    assert_eq!(said.sidebar_snapshot(0).refresh_in_ms, None, "its own words do not move");
}

#[test]
fn voiceover_reads_name_state_selection_place_and_shortcut() {
    let mut m = model(3);
    m.update_facts(2, |f| {
        f.agent = Some(AgentId::Claude);
        f.status = Status::Attention;
        f.title = Some("Fix the login".into());
        f.message = Some("Approve cargo test?".into());
    });
    m.apply(Action::Select(2));
    let spoken = m.sidebar_snapshot(0).sections[0].rows[1].spoken.clone();
    assert_eq!(spoken, "Fix the login, needs input, Approve cargo test?, selected, 2 of 3, Command 2");
    let plain = m.sidebar_snapshot(0).sections[0].rows[0].spoken.clone();
    assert_eq!(plain, "Terminal 1, shell, nothing running, 1 of 3, Command 1");
}

#[test]
fn a_group_head_is_spoken_with_its_count_and_a_folded_one_with_what_needs_attention() {
    let mut m = model(3);
    m.update_facts(1, |f| {
        f.agent = Some(AgentId::Claude);
        f.status = Status::Failed;
    });
    m.apply(Action::NewGroup { members: vec![1, 2] });
    m.apply(Action::RenameGroup { group: 1, name: "Release".into() });
    m.apply(Action::ToggleGroup(1));
    let head = m.sidebar_snapshot(0).sections[0].head.clone().expect("the group");
    assert!(head.spoken.starts_with("Release, "), "{}", head.spoken);
    assert!(
        head.spoken.contains("2 terminals") && head.spoken.contains("1 need attention"),
        "{}",
        head.spoken
    );
    assert!(head.spoken.ends_with("collapsed, 1 of 2"), "{}", head.spoken);
    assert_eq!(m.sidebar_snapshot(0).sections[0].alert_count, 1);
}

#[test]
fn a_split_member_says_so() {
    let mut m = model(1);
    m.apply(Action::SplitBeside { anchor: 1, axis: crate::view::panes::Axis::Row });
    let rows = m.sidebar_snapshot(0).sections[0].rows.clone();
    assert!(rows[0].spoken.contains("in a split with 1 other, on screen"), "{}", rows[0].spoken);
}
