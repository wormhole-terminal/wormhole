//! What the pieces of a row say, from the facts of a terminal: its glyph, its second line and its time, and
//! what VoiceOver is told of it.

use std::time::{Duration, SystemTime};

use wormhole_core::agents::AgentId;
use wormhole_core::notify::Source;
use wormhole_core::view::sidebar::RowState;

use super::Place;
use super::label::row_label;
use super::second_line::{SecondLine, second_line, time_text};
use super::tile::glyph_for;
use crate::sidebar::facts::{RowFailure, RowInfo, ago};
use crate::style::{RowTone, StatusGlyph};

fn info(agent: bool, state: RowState) -> RowInfo {
    RowInfo {
        agent: agent.then_some(AgentId::Claude),
        source: if agent { Source::Agent } else { Source::Plain },
        state,
        ..RowInfo::test_row(1)
    }
}

/// A program that reports its own status, no agent: the row of an agent without its logo.
fn program(state: RowState) -> RowInfo {
    RowInfo { source: Source::Program, ..info(false, state) }
}

fn failed(code: i32, command: Option<&str>, secs: u64) -> RowFailure {
    RowFailure { code, command: command.map(str::to_string), at: ago(Duration::from_secs(secs)) }
}

#[test]
fn agents_show_their_state_and_plain_terminals_the_terminal() {
    assert_eq!(glyph_for(&info(true, RowState::Working)), StatusGlyph::Working);
    assert_eq!(glyph_for(&info(true, RowState::NeedsYou)), StatusGlyph::NeedsYou);
    assert_eq!(glyph_for(&info(true, RowState::Finished)), StatusGlyph::Done);
    assert_eq!(glyph_for(&info(true, RowState::Idle)), StatusGlyph::Idle);
    let bare = info(false, RowState::Idle);
    assert_eq!(glyph_for(&bare), StatusGlyph::Shell, "a bare shell is faint");
    let running = RowInfo { command: Some("cargo test".into()), ..info(false, RowState::Working) };
    assert_eq!(glyph_for(&running), StatusGlyph::Terminal { unread: false });
    let unread = RowInfo { unread: true, ..info(false, RowState::Idle) };
    assert_eq!(glyph_for(&unread), StatusGlyph::Terminal { unread: true });
}

#[test]
fn a_failure_is_a_danger_glyph_for_everyone_until_the_next_command() {
    assert_eq!(glyph_for(&info(true, RowState::Failed)), StatusGlyph::Failed);
    assert_eq!(glyph_for(&program(RowState::Failed)), StatusGlyph::Failed);
    assert_eq!(glyph_for(&info(false, RowState::Failed)), StatusGlyph::Failed);
    // Seen, the status is idle but the row still says the last command failed.
    let seen =
        RowInfo { failure: Some(failed(101, Some("cargo build"), 120)), ..info(false, RowState::Idle) };
    assert_eq!(glyph_for(&seen), StatusGlyph::Failed);
    // A new command is running: the terminal glyph; the failure is history.
    let again = RowInfo { command: Some("cargo build".into()), ..info(false, RowState::Working) };
    assert_eq!(glyph_for(&again), StatusGlyph::Terminal { unread: false });
}

#[test]
fn a_program_that_reports_gets_an_agents_glyphs_and_words() {
    assert_eq!(glyph_for(&program(RowState::Working)), StatusGlyph::Working);
    assert_eq!(glyph_for(&program(RowState::NeedsYou)), StatusGlyph::NeedsYou);
    assert_eq!(glyph_for(&program(RowState::Finished)), StatusGlyph::Done);
    let asks = RowInfo { message: Some("Password:".into()), ..program(RowState::NeedsYou) };
    assert_eq!(
        second_line(&asks, false, || None),
        SecondLine {
            text: Some("Needs input · Password:".into()),
            tone: RowTone::Attention,
            mono: false,
            trail: None
        }
    );
    let done = RowInfo { message: Some("Upgraded 12 packages".into()), ..program(RowState::Finished) };
    assert_eq!(second_line(&done, false, || None).text.as_deref(), Some("Finished · Upgraded 12 packages"));
    assert_eq!(time_text(&done, false), None, "its words don't move");
    let broke = RowInfo { message: Some("rsync failed".into()), ..program(RowState::Failed) };
    let line = second_line(&broke, false, || None);
    assert_eq!((line.text.as_deref(), line.tone), (Some("Failed · rsync failed"), RowTone::Failed));
    let mute = program(RowState::Failed);
    assert_eq!(second_line(&mute, false, || None).text.as_deref(), Some("Failed · 3m ago"));
    assert_eq!(time_text(&mute, false).as_deref(), Some("3m ago"));
    let busy = RowInfo { message: Some("Installing · 40%".into()), ..program(RowState::Working) };
    assert_eq!(second_line(&busy, false, || None).text.as_deref(), Some("Installing · 40%"));
}

/// F-08: danger glyph, "command · exit N" in danger, and when it ended.
#[test]
fn a_failed_command_says_what_ended_how_and_when() {
    let build =
        RowInfo { failure: Some(failed(101, Some("cargo build"), 120)), ..info(false, RowState::Failed) };
    assert_eq!(
        second_line(&build, false, || Some("~/scratch".into())),
        SecondLine {
            text: Some("cargo build · exit 101".into()),
            tone: RowTone::Failed,
            mono: true,
            trail: Some("2m ago".into()),
        }
    );
    assert_eq!(time_text(&build, false).as_deref(), Some("2m ago"));
    // Seen: the cue stays, though the state is idle now.
    let seen = RowInfo { state: RowState::Idle, ..build };
    assert_eq!(second_line(&seen, false, || None).tone, RowTone::Failed);
    // The shell gave no command: the exit code alone.
    let bare = RowInfo { failure: Some(failed(2, None, 5)), ..info(false, RowState::Failed) };
    assert_eq!(second_line(&bare, false, || None).text.as_deref(), Some("exit 2"));
    // The next command starts: the cue is gone, and the row says what runs.
    let next = RowInfo { command: Some("cargo test".into()), ..info(false, RowState::Working) };
    let line = second_line(&next, false, || None);
    assert_eq!((line.text.as_deref(), line.tone), (Some("cargo test"), RowTone::Plain));
    // Compact rows have no second line.
    assert_eq!(second_line(&bare, true, || None).text, None);
}

#[test]
fn only_moving_times_count_for_redraws() {
    assert_eq!(time_text(&info(true, RowState::Working), false).as_deref(), Some("3m"));
    let told = RowInfo { message: Some("Editing".into()), ..info(true, RowState::Working) };
    assert_eq!(time_text(&told, false), None, "the agent's own words don't move");
    assert_eq!(time_text(&info(true, RowState::Finished), false).as_deref(), Some("3m ago"));
    let running = RowInfo {
        running_since: Some(SystemTime::now() - Duration::from_secs(72)),
        ..info(false, RowState::Working)
    };
    assert_eq!(time_text(&running, false).as_deref(), Some("1:12"));
    assert_eq!(time_text(&running, true), None, "compact rows have no second line");
}

fn place(position: usize, of: usize, number: Option<usize>) -> Place {
    Place {
        selected: false,
        solo: false,
        position,
        of,
        number,
        grouped: false,
        marks: wormhole_core::view::sidebar::SplitMarks::default(),
        changes_above: false,
        bracket_draws: None,
        status: None,
    }
}

#[test]
fn a_rows_label_says_its_name_what_it_does_and_where_it_is() {
    let working = RowInfo { name: "claude".into(), ..info(true, RowState::Working) };
    assert_eq!(
        row_label(&working, true, &place(1, 3, Some(1))),
        "claude, working, selected, 1 of 3, Command 1"
    );
    let asking =
        RowInfo { name: "api".into(), message: Some("Allow edit?".into()), ..info(true, RowState::NeedsYou) };
    assert_eq!(row_label(&asking, false, &place(2, 3, None)), "api, needs input, Allow edit?, 2 of 3");
    let build = RowInfo {
        name: "web".into(),
        command: Some("cargo build --release".into()),
        running_since: Some(SystemTime::now() - Duration::from_secs(72)),
        ..info(false, RowState::Working)
    };
    assert!(row_label(&build, false, &place(3, 3, None)).starts_with("web, running cargo"));
    let failed =
        RowInfo { failure: Some(failed(101, Some("cargo test"), 5)), ..info(false, RowState::Failed) };
    assert!(row_label(&failed, false, &place(1, 1, None)).contains("last command failed, exit 101"));
    let idle = info(false, RowState::Idle);
    assert!(row_label(&idle, false, &place(1, 1, None)).contains("shell, nothing running"));
}
