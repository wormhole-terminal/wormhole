//! View tests of the failed status: a command that ends with a failing exit code (the shell's
//! OSC 133 marks), an agent that stops on an error, and a program that is no agent but reports its
//! own status (OSC 7501), which is told in the sidebar, the bar and the banners like one.

use gpui_kit::{Entity, TestAppContext};
use wormhole_core::notify::{Source, State};
use wormhole_term::{Mark, PtyEvent};

use crate::app_state::OpenWindow;
use crate::notifications;
use crate::terminal_view::{Status, TerminalView};
use crate::window::ambient::TabFacts;
use crate::window::reported_tests::{banners_in_front, event, now, report, terminal, words};
use crate::window::test_harness::{drive, hook, isolate_in, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// The shell ran a command in `t` that ended with `code`, as its marks say, and the tick settled it
/// (`looking`: the user has the terminal in front).
fn command_ends(cx: &mut TestAppContext, t: &Entity<TerminalView>, code: Option<i32>, looking: bool) {
    event(cx, t, PtyEvent::Mark(Mark::OutputStart, 0));
    event(cx, t, PtyEvent::Mark(Mark::Finished(code), 0));
    cx.update(|cx| t.update(cx, |t, _| t.tick(looking, looking)));
}

fn failure(cx: &mut TestAppContext, t: &Entity<TerminalView>) -> Option<i32> {
    t.read_with(cx, |t, _| t.failure().map(|failure| failure.code))
}

fn look(cx: &mut TestAppContext, t: &Entity<TerminalView>) -> bool {
    cx.update(|cx| {
        t.update(cx, |t, cx| {
            let seen = t.acknowledge();
            // The window's own paths tell it (`Workspace::sync_summary`).
            cx.notify();
            seen
        })
    })
}

#[gpui_kit::test]
fn a_command_that_fails_unseen_is_failed_until_the_terminal_is_opened(cx: &mut TestAppContext) {
    if isolate("a_command_that_fails_unseen_is_failed_until_the_terminal_is_opened") {
        return;
    }
    let window = start(cx, 2);
    let t = terminal(cx, &window, 1);
    event(cx, &t, PtyEvent::Mark(Mark::OutputStart, 0));
    assert_eq!(now(cx, &t).0, Status::Busy);
    event(cx, &t, PtyEvent::Mark(Mark::Finished(Some(101)), 0));
    cx.update(|cx| t.update(cx, |t, _| t.tick(false, false)));
    assert_eq!(now(cx, &t).0, Status::Failed, "nobody was looking");
    assert_eq!(failure(cx, &t), Some(101));
    assert_eq!(
        t.read_with(cx, |t, _| t.summary().failure.map(|failure| failure.code)),
        Some(101),
        "the sidebar hears of it"
    );

    // Opened: the status clears like done's, the row's cue stays until the next command.
    assert!(look(cx, &t));
    assert_eq!(now(cx, &t).0, Status::Idle);
    assert_eq!(failure(cx, &t), Some(101), "the row still says the last command failed");
    assert!(!look(cx, &t), "nothing more to see");

    // The next command starts: it is history, and the terminal is at work.
    event(cx, &t, PtyEvent::Mark(Mark::OutputStart, 0));
    assert_eq!((now(cx, &t).0, failure(cx, &t)), (Status::Busy, None));
    event(cx, &t, PtyEvent::Mark(Mark::Finished(Some(0)), 0));
    cx.update(|cx| t.update(cx, |t, _| t.tick(false, false)));
    assert_eq!((now(cx, &t).0, failure(cx, &t)), (Status::Done, None), "it went well");
}

#[gpui_kit::test]
fn a_failure_seen_at_once_is_idle_but_the_row_keeps_the_cue(cx: &mut TestAppContext) {
    if isolate("a_failure_seen_at_once_is_idle_but_the_row_keeps_the_cue") {
        return;
    }
    let window = start(cx, 1);
    let t = terminal(cx, &window, 0);
    command_ends(cx, &t, Some(2), true);
    assert_eq!(now(cx, &t).0, Status::Idle, "the user watched it fail");
    assert_eq!(failure(cx, &t), Some(2));
    // A different code is a new failure, with its own number.
    let first = t.read_with(cx, |t, _| t.failure().map(|f| f.serial));
    command_ends(cx, &t, Some(2), true);
    let second = t.read_with(cx, |t, _| t.failure().map(|f| f.serial));
    assert_ne!(first, second, "the same code again is news again");
    // A command that went well ends the story.
    command_ends(cx, &t, Some(0), true);
    assert_eq!(failure(cx, &t), None);
}

/// Ctrl-C is the user's own doing: shells report it as 130, and it is neither news nor an error to
/// look at.
#[gpui_kit::test]
fn an_interrupt_is_not_a_failure(cx: &mut TestAppContext) {
    if isolate("an_interrupt_is_not_a_failure") {
        return;
    }
    let window = start(cx, 2);
    let t = terminal(cx, &window, 1);
    command_ends(cx, &t, Some(130), false);
    assert_eq!((now(cx, &t).0, failure(cx, &t)), (Status::Done, None));
    // A shell that gave no code says nothing of failing either.
    command_ends(cx, &t, None, false);
    assert_eq!((now(cx, &t).0, failure(cx, &t)), (Status::Done, None));
    // 130 right after a real failure takes the cue away too: the last command did not fail.
    command_ends(cx, &t, Some(1), false);
    assert_eq!(failure(cx, &t), Some(1));
    command_ends(cx, &t, Some(130), false);
    assert_eq!(failure(cx, &t), None);
}

/// A command that went quiet counts as done; the exit code, when it comes, can still say it failed.
#[gpui_kit::test]
fn an_exit_code_that_comes_late_turns_done_into_failed(cx: &mut TestAppContext) {
    if isolate("an_exit_code_that_comes_late_turns_done_into_failed") {
        return;
    }
    let window = start(cx, 2);
    let t = terminal(cx, &window, 1);
    command_ends(cx, &t, Some(0), false);
    assert_eq!(now(cx, &t).0, Status::Done);
    event(cx, &t, PtyEvent::Mark(Mark::Finished(Some(3)), 0));
    cx.update(|cx| t.update(cx, |t, _| t.tick(false, false)));
    assert_eq!(now(cx, &t).0, Status::Failed);
    // And it is seen like the rest.
    assert!(look(cx, &t));
    assert_eq!(now(cx, &t).0, Status::Idle);
}

/// The facts the ambient surfaces read about each terminal of the window.
fn facts(cx: &mut TestAppContext, window: &OpenWindow) -> Vec<TabFacts> {
    cx.update(|cx| window.workspace.read_with(cx, |ws, _| ws.tab_facts(false))).expect("the window is open")
}

/// What the banners say now: every terminal's change since the last look, as the tick sweeps.
fn notices(cx: &mut TestAppContext) -> Vec<(String, String, String)> {
    cx.update(notifications::sweep);
    cx.delivered_system_notifications()
        .into_iter()
        .map(|n| (n.title.to_string(), n.body.to_string(), n.tag.to_string()))
        .collect()
}

/// An agent's StopFailure hook (an API error ended the turn) is a failure, not a finish.
#[gpui_kit::test]
fn an_agent_that_stops_on_an_error_fails(cx: &mut TestAppContext) {
    if isolate("an_agent_that_stops_on_an_error_fails") {
        return;
    }
    let window = start(cx, 2);
    let t = terminal(cx, &window, 1);
    hook(cx, &window, 2, "UserPromptSubmit", None);
    assert_eq!(now(cx, &t).0, Status::Busy);
    hook(cx, &window, 2, "StopFailure:rate_limit", None);
    assert_eq!(now(cx, &t).0, Status::Failed);
    let all = facts(cx, &window);
    assert_eq!((all[1].state, all[1].source), (State::Failed, Source::Agent));
    // Looked at, it goes like a finish does.
    assert!(look(cx, &t));
    assert_eq!(now(cx, &t).0, Status::Idle);
}

/// A program that is no known agent (`app=brew`) is a first-class reporter: its states are the
/// terminal's, under its own name, in the facts the bar, the menu and the banners read.
#[gpui_kit::test]
fn a_program_that_is_no_agent_works_asks_finishes_and_fails_like_one(cx: &mut TestAppContext) {
    if isolate("a_program_that_is_no_agent_works_asks_finishes_and_fails_like_one") {
        return;
    }
    let window = start(cx, 2);
    banners_in_front(cx);
    let t = terminal(cx, &window, 1);
    let name = drive(cx, &window, |ws, _, _| ws.sessions[1].name().to_string());
    let brew = |cx: &mut TestAppContext| {
        let all = facts(cx, &window);
        (all[1].state, all[1].source, all[1].program.clone(), all[1].agent)
    };
    assert!(notices(cx).is_empty());

    // Working, with how far along it is.
    report(cx, &t, "state=working:app=brew:progress=40:msg={msg:Installing updates}");
    assert_eq!(now(cx, &t), (Status::Busy, words("Installing updates · 40%")));
    assert_eq!(brew(cx), (State::Working, Source::Program, Some("brew".to_string()), None));
    assert_eq!(t.read_with(cx, |t, _| t.summary().source), Source::Program);
    assert!(notices(cx).is_empty(), "work is not news");

    // Blocked on you: a question, not a bell.
    report(cx, &t, "state=blocked:kind=auth:app=brew:msg={msg:Password for sudo}");
    assert_eq!(now(cx, &t), (Status::Attention, words("Password for sudo")));
    assert_eq!(brew(cx).0, State::NeedsYou);
    let told = notices(cx);
    assert_eq!(told.len(), 1, "{told:?}");
    assert_eq!(
        (told[0].0.as_str(), told[0].1.as_str()),
        (format!("{name} needs input").as_str(), "Password for sudo")
    );

    // Done, in its own words: a banner of the finished kind, though no command crossed a threshold.
    report(cx, &t, "state=working:app=brew");
    notices(cx);
    report(cx, &t, "state=done:app=brew:msg={msg:Upgraded 12 packages}");
    assert_eq!(now(cx, &t), (Status::Done, words("Upgraded 12 packages")));
    assert_eq!(brew(cx).0, State::Done);
    let told = notices(cx);
    assert_eq!(told.len(), 1, "{told:?}");
    assert_eq!(told[0].0, format!("{name} finished"));
    assert!(told[0].1.starts_with("Upgraded 12 packages"), "{}", told[0].1);

    // An error: failed, with what went wrong.
    report(cx, &t, "state=working:app=brew");
    notices(cx);
    report(cx, &t, "state=error:app=brew:msg={msg:Error: no such formula}");
    assert_eq!(now(cx, &t), (Status::Failed, words("Error: no such formula")));
    assert_eq!(brew(cx).0, State::Failed);
    let told = notices(cx);
    assert_eq!(told.len(), 1, "{told:?}");
    assert_eq!(
        (told[0].0.as_str(), told[0].1.as_str()),
        (format!("{name} failed").as_str(), "Error: no such formula")
    );

    // Seen, it is over; nothing of it is left in the facts.
    assert!(look(cx, &t));
    assert_eq!(now(cx, &t), (Status::Idle, None));
    assert_eq!(brew(cx), (State::Idle, Source::Plain, None, None));
}

/// A program that names a known agent is that agent; one that names nothing known keeps its name.
#[gpui_kit::test]
fn a_programs_own_name_is_shown_and_a_known_agents_is_not_taken_for_one(cx: &mut TestAppContext) {
    if isolate("a_programs_own_name_is_shown_and_a_known_agents_is_not_taken_for_one") {
        return;
    }
    let window = start(cx, 2);
    let (a, b) = (terminal(cx, &window, 0), terminal(cx, &window, 1));
    report(cx, &a, "state=working:app=terraform");
    report(cx, &b, "state=working:app=claude-code");
    let all = facts(cx, &window);
    assert_eq!(
        (all[0].source, all[0].program.as_deref(), all[0].who()),
        (Source::Program, Some("terraform"), Some("terraform"))
    );
    assert_eq!(
        (all[1].source, all[1].program.as_deref(), all[1].who()),
        (Source::Agent, None, Some("Claude Code"))
    );
    // The records are the program's, not a child's: a report that names no app inherits its parent's.
    report(cx, &a, "state=working:app=terraform:id=plan");
    report(cx, &a, "state=blocked:kind=question:id=plan/apply");
    assert_eq!(facts(cx, &window)[0].program.as_deref(), Some("terraform"));
}
