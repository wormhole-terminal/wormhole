//! A terminal's status from what happens to it, through the model as a shell drives it: events in,
//! facts and the row out. The cases are the app's `terminal_view` status tests (hooks, the bell, the
//! shell's marks, output going quiet, programs that report, the agent in front), without a window.

use wormhole_term::program_status::{Kind, State, Summary};
use wormhole_term::{Mark, PromptKind};

use crate::agents::AgentId;
use crate::view::sidebar::{RowState, Status};
use crate::workspace::{Action, Ask, Event, ForegroundLook, Front, StatusEvent, WorkspaceModel};

use super::model;

const CLAUDE: AgentId = AgentId::Claude;

fn feed(m: &mut WorkspaceModel, id: i64, event: StatusEvent, at: u64) -> Vec<Event> {
    m.feed(id, event, at)
}

fn hook(m: &mut WorkspaceModel, id: i64, event: &str, message: Option<&str>, at: u64) {
    let event = StatusEvent::Hook { agent: CLAUDE, event: event.into(), message: message.map(Into::into) };
    feed(m, id, event, at);
}

fn state(m: &WorkspaceModel, id: i64) -> (Status, RowState) {
    let facts = m.facts(id).expect("terminal");
    (facts.status, facts.row_state())
}

fn mark(m: &mut WorkspaceModel, id: i64, mark: Mark, ended: Option<&str>, at: u64) {
    feed(m, id, StatusEvent::Mark { mark, ended: ended.map(Into::into) }, at);
}

fn front(command: &str, since: u64) -> ForegroundLook {
    ForegroundLook {
        in_front: Some(true),
        command: Some(Front { command: command.into(), since }),
        agent: None,
        shell_in_front: false,
    }
}

fn shell_back(agent: Option<AgentId>) -> ForegroundLook {
    ForegroundLook { in_front: Some(false), command: None, agent, shell_in_front: true }
}

fn report(state: State, kind: Option<Kind>, msg: Option<&str>) -> Summary {
    Summary { state, kind, progress: None, app: None, title: None, msg: msg.map(str::to_string) }
}

#[test]
fn an_agents_hooks_are_its_status_and_a_question_keeps_its_own_words() {
    let mut m = model(2);
    hook(&mut m, 2, "UserPromptSubmit", None, 1_000);
    assert_eq!(state(&m, 2), (Status::Busy, RowState::Working));
    assert_eq!(m.facts(2).map(|f| f.since), Some(1_000));
    // The prompt's words, then the notification that the same prompt still waits: the first stays.
    hook(&mut m, 2, "PermissionRequest:Bash", Some("Approve cargo test?"), 2_000);
    hook(&mut m, 2, "Notification:permission_prompt", Some("Claude needs your permission"), 8_000);
    assert_eq!(state(&m, 2), (Status::Attention, RowState::NeedsYou));
    assert_eq!(m.facts(2).and_then(|f| f.message.clone()).as_deref(), Some("Approve cargo test?"));
    assert_eq!(m.facts(2).map(|f| f.since), Some(2_000), "the same status keeps its time");
    // An idle reminder is no question.
    hook(&mut m, 2, "Stop", None, 9_000);
    hook(&mut m, 2, "Notification:idle_prompt", Some("waiting"), 10_000);
    assert_eq!(state(&m, 2), (Status::Done, RowState::Finished));
    // The agent's name comes from the hook.
    assert_eq!(m.facts(2).and_then(|f| f.agent), Some(CLAUDE));
}

#[test]
fn a_result_nobody_saw_is_spent_by_looking_and_a_question_is_not() {
    let mut m = model(2);
    hook(&mut m, 2, "Stop", None, 1_000);
    assert_eq!(state(&m, 2).0, Status::Done);
    m.apply(Action::Select(2));
    assert_eq!(state(&m, 2).0, Status::Idle, "selecting spends the result");

    m.apply(Action::Select(1));
    hook(&mut m, 2, "PermissionRequest:Bash", Some("Approve ls?"), 2_000);
    m.apply(Action::Select(2));
    assert_eq!(state(&m, 2).0, Status::Attention, "a question stays until it is answered");
    // A key that answers it: no hook says so until the tool has run.
    feed(&mut m, 2, StatusEvent::Key { key: "up".into(), control: false }, 3_000);
    assert_eq!(state(&m, 2).0, Status::Attention, "moving through the choices answers nothing");
    feed(&mut m, 2, StatusEvent::Key { key: "enter".into(), control: false }, 4_000);
    assert_eq!(state(&m, 2).0, Status::Busy);
    hook(&mut m, 2, "PermissionRequest:Bash", Some("Approve rm?"), 5_000);
    feed(&mut m, 2, StatusEvent::Key { key: "c".into(), control: true }, 6_000);
    assert_eq!(state(&m, 2).0, Status::Idle, "Control-C turns it down");
}

#[test]
fn a_result_that_lands_where_the_person_looks_is_seen_at_once() {
    let mut m = model(1);
    hook(&mut m, 1, "UserPromptSubmit", None, 0);
    hook(&mut m, 1, "Stop", None, 1_000);
    assert_eq!(state(&m, 1).0, Status::Idle, "on screen in the window in front");
    m.set_window_active(false, 2_000);
    hook(&mut m, 1, "UserPromptSubmit", None, 3_000);
    hook(&mut m, 1, "Stop", None, 4_000);
    assert_eq!(state(&m, 1).0, Status::Done, "the window is behind: nobody saw");
    m.set_window_active(true, 5_000);
    assert_eq!(state(&m, 1).0, Status::Idle, "coming to the front looks at what is on screen");
}

#[test]
fn work_in_the_background_keeps_a_turn_that_ended_working() {
    let mut m = model(2);
    for event in ["UserPromptSubmit", "PostToolUse:wakes=b7", "Stop:shells=b7"] {
        hook(&mut m, 2, event, None, 1_000);
    }
    assert_eq!(state(&m, 2).0, Status::Busy, "a subagent's shell wakes the session again");
    hook(&mut m, 2, "Stop", None, 2_000);
    assert_eq!(state(&m, 2).0, Status::Done);
    hook(&mut m, 2, "StopFailure:rate_limit", Some("rate limited"), 3_000);
    assert_eq!(
        (state(&m, 2).0, m.facts(2).and_then(|f| f.message.clone()).as_deref().map(str::to_owned)),
        (Status::Failed, Some("rate limited".into()))
    );
    hook(&mut m, 2, "SessionEnd", None, 4_000);
    assert_eq!(state(&m, 2).0, Status::Idle);
    assert!(m.facts(2).is_some_and(|f| f.message.is_none()));
}

#[test]
fn hooks_say_it_all_so_output_marks_and_the_bell_stand_aside() {
    let mut m = model(2);
    hook(&mut m, 2, "UserPromptSubmit", None, 0);
    hook(&mut m, 2, "Stop", None, 100);
    feed(&mut m, 2, StatusEvent::Output, 5_000);
    feed(&mut m, 2, StatusEvent::Bell, 5_100);
    mark(&mut m, 2, Mark::OutputStart, None, 5_200);
    assert_eq!(state(&m, 2).0, Status::Done, "told, not guessed");
    assert!(m.facts(2).is_some_and(|f| !f.bell));
    assert_eq!(m.next_due(), None, "nothing is waiting for a quiet moment");
}

#[test]
fn an_agent_without_hooks_asks_with_its_bell_and_a_plain_terminal_marks_it() {
    let mut m = model(3);
    // Terminal 2 runs an agent the system found, terminal 3 nothing.
    feed(&mut m, 2, StatusEvent::Foreground(ForegroundLook { agent: Some(CLAUDE), ..front("claude", 0) }), 0);
    feed(&mut m, 2, StatusEvent::Bell, 100_000);
    assert_eq!(state(&m, 2), (Status::Attention, RowState::NeedsYou));
    assert!(m.facts(2).is_some_and(|f| !f.bell), "an agent's bell is a question, not a mark");

    feed(&mut m, 3, StatusEvent::Bell, 100_000);
    assert_eq!(state(&m, 3), (Status::Attention, RowState::NeedsYou));
    assert!(m.facts(3).is_some_and(|f| f.bell));
    // Heard: selecting spends both.
    m.apply(Action::Select(3));
    assert_eq!(state(&m, 3), (Status::Idle, RowState::Idle));
    assert!(m.facts(3).is_some_and(|f| !f.bell));
    // Ringing in a loop asks once a minute; the mark stays.
    m.apply(Action::Select(1));
    feed(&mut m, 3, StatusEvent::Bell, 200_000);
    m.apply(Action::Select(1));
    feed(&mut m, 3, StatusEvent::Bell, 210_000);
    m.set_facts(3, crate::workspace::TerminalFacts { bell: true, ..Default::default() });
    assert_eq!(state(&m, 3).0, Status::Idle);
}

#[test]
fn output_makes_a_plain_terminal_busy_and_quiet_settles_it() {
    let mut m = model(2);
    feed(&mut m, 2, StatusEvent::Output, 10_000);
    assert_eq!(state(&m, 2).0, Status::Busy);
    assert_eq!(m.next_due(), Some((2, 11_800)), "one timer, at the end of the quiet");
    // More output only moves the deadline.
    feed(&mut m, 2, StatusEvent::Output, 11_000);
    assert_eq!(m.next_due(), Some((2, 12_800)));
    // Early tick: still busy; on time: done, because nobody looks at it.
    feed(&mut m, 2, StatusEvent::Tick, 11_900);
    assert_eq!(state(&m, 2).0, Status::Busy);
    feed(&mut m, 2, StatusEvent::Tick, 12_800);
    assert_eq!(state(&m, 2).0, Status::Done);
    assert_eq!(m.next_due(), None);
    // The terminal in front goes idle, not done.
    feed(&mut m, 1, StatusEvent::Output, 20_000);
    feed(&mut m, 1, StatusEvent::Tick, 21_800);
    assert_eq!(state(&m, 1).0, Status::Idle);
    // Unseen output of a finished command is what the row calls new.
    assert!(m.facts(2).is_some_and(|f| f.unread));
    assert!(m.facts(1).is_some_and(|f| !f.unread));
}

#[test]
fn what_answers_the_person_is_not_work() {
    let mut m = model(2);
    feed(&mut m, 2, StatusEvent::Typed, 10_000);
    feed(&mut m, 2, StatusEvent::Output, 10_100);
    assert_eq!(state(&m, 2).0, Status::Idle, "the echo of typing");
    feed(&mut m, 2, StatusEvent::Resized, 20_000);
    feed(&mut m, 2, StatusEvent::Output, 20_500);
    assert_eq!(state(&m, 2).0, Status::Idle, "the redraw after a resize");
    feed(&mut m, 2, StatusEvent::FocusReported, 30_000);
    feed(&mut m, 2, StatusEvent::Output, 30_600);
    assert_eq!(state(&m, 2).0, Status::Idle, "an agent redraws when told of a focus change");
    feed(&mut m, 2, StatusEvent::Output, 31_000);
    assert_eq!(state(&m, 2).0, Status::Busy, "the program's own output is");
    // A shell that is at its prompt prints prompts, not work.
    m.apply(Action::NewTerminal);
    let id = *m.ids().last().expect("a new terminal");
    mark(&mut m, id, Mark::PromptStart(PromptKind::Initial), None, 40_000);
    feed(&mut m, id, StatusEvent::Output, 41_000);
    assert_eq!(state(&m, id).0, Status::Idle);
}

#[test]
fn a_command_runs_finishes_and_a_failure_is_kept_until_the_next_one() {
    let mut m = model(2);
    mark(&mut m, 2, Mark::OutputStart, None, 1_000);
    feed(&mut m, 2, StatusEvent::Foreground(front("cargo build", 1_000)), 1_100);
    assert_eq!(state(&m, 2), (Status::Busy, RowState::Working), "busy before it prints");
    assert_eq!(m.facts(2).and_then(|f| f.command.clone()).as_deref(), Some("cargo build"));
    assert_eq!(m.facts(2).and_then(|f| f.running_since), Some(1_000));
    mark(&mut m, 2, Mark::Finished(Some(101)), Some("cargo build"), 5_000);
    feed(&mut m, 2, StatusEvent::Foreground(shell_back(None)), 5_100);
    let facts = m.facts(2).expect("facts");
    assert_eq!(facts.status, Status::Failed, "settled at once, not after the quiet");
    let failure = facts.failure.as_ref().expect("failure");
    assert_eq!((failure.code, failure.command.as_deref(), failure.at), (101, Some("cargo build"), 5_000));
    assert_eq!(
        facts.finished_command.as_deref(),
        Some("cargo build"),
        "its result is unseen: it still names the terminal"
    );
    assert_eq!((facts.command.as_deref(), facts.running_since), (None, None));
    // The next command ends the story of the last one.
    mark(&mut m, 2, Mark::OutputStart, None, 9_000);
    assert!(m.facts(2).is_some_and(|f| f.failure.is_none() && f.status == Status::Busy));
    // A command that succeeds, or was interrupted, is no failure.
    for code in [Some(0), Some(130), None] {
        mark(&mut m, 2, Mark::OutputStart, None, 10_000);
        mark(&mut m, 2, Mark::Finished(code), Some("make"), 11_000);
        assert!(m.facts(2).is_some_and(|f| f.failure.is_none()), "{code:?}");
        assert_eq!(state(&m, 2).0, Status::Done, "{code:?}");
    }
    // In front: nothing to settle into, but the failure's cue stays on the row.
    mark(&mut m, 1, Mark::OutputStart, None, 20_000);
    mark(&mut m, 1, Mark::Finished(Some(2)), Some("false"), 21_000);
    assert_eq!(state(&m, 1).0, Status::Idle);
    assert!(m.facts(1).is_some_and(|f| f.failure.as_ref().is_some_and(|f| f.code == 2)));
}

#[test]
fn a_program_that_reports_its_status_has_the_last_word() {
    let mut m = model(2);
    let blocked = report(State::Blocked, Some(Kind::Permission), None);
    feed(&mut m, 2, StatusEvent::Reported { summary: Some(blocked), by_report: true }, 1_000);
    let facts = m.facts(2).expect("facts");
    assert_eq!(
        (facts.status, facts.reports, facts.message.as_deref()),
        (Status::Attention, true, Some("Waiting for approval"))
    );
    assert_eq!(facts.row_state(), RowState::NeedsYou);
    // While it holds a record the heuristics stand aside, and there is nothing to settle.
    feed(&mut m, 2, StatusEvent::Output, 2_000);
    assert_eq!(m.next_due(), None);
    // The person looks: a question stays, and the shell is not asked to drop it.
    m.apply(Action::Select(2));
    assert_eq!(state(&m, 2).0, Status::Attention);
    assert!(m.take_asks().is_empty());
    // It works, with words and progress.
    let working = Summary { progress: Some(40), ..report(State::Working, None, Some("Installing")) };
    feed(&mut m, 2, StatusEvent::Reported { summary: Some(working), by_report: true }, 3_000);
    assert_eq!(m.facts(2).and_then(|f| f.message.clone()).as_deref(), Some("Installing · 40%"));
    assert_eq!(state(&m, 2), (Status::Busy, RowState::Working));
    // It finishes while nobody looks; looking at it asks the shell to drop the record.
    m.apply(Action::Select(1));
    let done = report(State::Done, None, Some("Upgraded 12"));
    feed(&mut m, 2, StatusEvent::Reported { summary: Some(done), by_report: true }, 4_000);
    assert_eq!(state(&m, 2).0, Status::Done);
    m.apply(Action::Select(2));
    assert_eq!(m.take_asks(), [(2, Ask::AcknowledgeReports)]);
    feed(&mut m, 2, StatusEvent::Reported { summary: None, by_report: false }, 5_000);
    assert_eq!(state(&m, 2).0, Status::Idle);
    assert!(m.facts(2).is_some_and(|f| !f.reports && f.message.is_none()));
}

#[test]
fn a_program_that_left_takes_what_only_it_could_say_with_it() {
    let mut m = model(2);
    feed(&mut m, 2, StatusEvent::Foreground(front("deploy", 0)), 0);
    let working = report(State::Working, None, None);
    feed(&mut m, 2, StatusEvent::Reported { summary: Some(working), by_report: true }, 1_000);
    assert_eq!(state(&m, 2).0, Status::Busy);
    // The shell has the terminal back: the program's records are dropped, a crash included.
    feed(&mut m, 2, StatusEvent::Foreground(shell_back(None)), 2_000);
    assert_eq!(m.take_asks(), [(2, Ask::ProgramLeft)]);
    feed(&mut m, 2, StatusEvent::Reported { summary: None, by_report: false }, 2_100);
    assert_eq!(state(&m, 2).0, Status::Idle);
    // A look that finds the shell in front when no program ever held it asks nothing.
    feed(&mut m, 2, StatusEvent::Foreground(shell_back(None)), 3_000);
    assert!(m.take_asks().is_empty());
}

#[test]
fn the_agent_in_front_is_found_and_forgotten_with_its_hooks() {
    let mut m = model(2);
    let found = ForegroundLook { agent: Some(CLAUDE), ..front("claude", 0) };
    feed(&mut m, 2, StatusEvent::Foreground(found.clone()), 0);
    assert_eq!(m.facts(2).and_then(|f| f.agent), Some(CLAUDE));
    hook(&mut m, 2, "UserPromptSubmit", None, 1_000);
    // Still running: a look that finds it again changes nothing.
    feed(&mut m, 2, StatusEvent::Foreground(found), 2_000);
    assert_eq!(state(&m, 2).0, Status::Busy);
    // It quit or crashed: hooks can't say so, and the status goes back to the output heuristic.
    feed(&mut m, 2, StatusEvent::Foreground(shell_back(None)), 3_000);
    let facts = m.facts(2).expect("facts");
    assert_eq!((facts.agent, facts.status, facts.message.as_deref()), (None, Status::Idle, None));
    assert_eq!(m.next_due(), None);
    // An agent a program says it is (it may run over ssh) is kept while the shell is not back.
    let ssh = ForegroundLook { in_front: Some(true), command: None, agent: None, shell_in_front: false };
    let codex = Summary { app: Some("codex".into()), ..report(State::Working, None, None) };
    feed(&mut m, 2, StatusEvent::Reported { summary: Some(codex), by_report: true }, 4_000);
    feed(&mut m, 2, StatusEvent::Foreground(ssh), 5_000);
    assert_eq!(m.facts(2).and_then(|f| f.agent), Some(AgentId::Codex));
}

#[test]
fn the_shell_ending_is_exited_and_nothing_brings_it_back() {
    let mut m = model(2);
    feed(&mut m, 2, StatusEvent::Foreground(front("sleep 5", 0)), 0);
    feed(&mut m, 2, StatusEvent::Exited, 1_000);
    assert_eq!(state(&m, 2), (Status::Exited, RowState::Exited));
    assert!(m.facts(2).is_some_and(|f| f.command.is_none()));
    hook(&mut m, 2, "UserPromptSubmit", None, 2_000);
    feed(&mut m, 2, StatusEvent::Output, 2_100);
    feed(&mut m, 2, StatusEvent::Bell, 2_200);
    assert_eq!(state(&m, 2).0, Status::Exited);
}

#[test]
fn a_title_that_keeps_changing_renames_at_a_steady_pace_with_one_timer() {
    let mut m = model(2);
    feed(&mut m, 2, StatusEvent::Title("⠋ 1%".into()), 1_000);
    assert_eq!(m.facts(2).and_then(|f| f.title.clone()).as_deref(), Some("1%"), "cleaned, and shown at once");
    let events = feed(&mut m, 2, StatusEvent::Title("⠙ 2%".into()), 1_050);
    assert!(events.is_empty(), "it waits its turn: no redraw for it");
    assert_eq!(m.next_due(), Some((2, 1_250)));
    feed(&mut m, 2, StatusEvent::Title("⠹ 3%".into()), 1_100);
    feed(&mut m, 2, StatusEvent::Tick, 1_250);
    assert_eq!(m.facts(2).and_then(|f| f.title.clone()).as_deref(), Some("3%"), "the latest wins");
    assert_eq!(m.next_due(), None);
    feed(&mut m, 2, StatusEvent::ResetTitle, 5_000);
    assert!(m.facts(2).is_some_and(|f| f.title.is_none()));
}

#[test]
fn a_change_that_shows_is_one_listing_and_output_that_changes_nothing_is_none() {
    let mut m = model(2);
    assert_eq!(feed(&mut m, 2, StatusEvent::Output, 10_000), [Event::Listing]);
    assert!(feed(&mut m, 2, StatusEvent::Output, 10_500).is_empty(), "already busy, already unread");
    assert!(feed(&mut m, 99, StatusEvent::Output, 10_600).is_empty(), "a terminal that closed is ignored");
    m.apply(Action::Close(2));
    assert!(m.next_due().is_none());
}
