//! View tests of a terminal whose programs report their own status (OSC 7501): what the status and
//! its words become, what the output heuristics leave alone, how hooks and reports take turns, and
//! what reaches a notification. The sequences are fed to the terminal's records as its PTY thread
//! would (`/bin/cat` prints none).

use gpui_kit::{Entity, TestAppContext};
use wormhole_core::agents::AgentId;
use wormhole_core::config::InFront;
use wormhole_term::{Mark, PtyEvent};

use crate::app_state::OpenWindow;
use crate::notifications;
use crate::settings::SettingsStore;
use crate::terminal_view::{Status, TerminalView};
use crate::window::Workspace;
use crate::window::test_harness::{drive, hook, isolate_in, read, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

pub(super) fn terminal(cx: &mut TestAppContext, window: &OpenWindow, ix: usize) -> Entity<TerminalView> {
    read(cx, window, |ws: &Workspace| ws.sessions[ix].terminal().clone())
}

/// Standard base64 with padding.
fn base64(text: &str) -> String {
    const DIGITS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in text.as_bytes().chunks(3) {
        let n = chunk.iter().enumerate().fold(0u32, |n, (i, byte)| n | u32::from(*byte) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(DIGITS[(n >> (18 - 6 * i) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// `OSC 7501 ; body ST` for the terminal, with a message given as plain text where the body says
/// `{msg:text}`.
pub(super) fn report(cx: &mut TestAppContext, terminal: &Entity<TerminalView>, body: &str) {
    let body = match body.split_once("{msg:") {
        Some((before, rest)) => {
            let (text, after) = rest.split_once('}').expect("a closing brace");
            format!("{before}{}{after}", base64(text))
        }
        None => body.to_string(),
    };
    cx.update(|cx| terminal.update(cx, |t, cx| t.report_for_test(&body, cx)));
}

pub(super) fn now(cx: &mut TestAppContext, terminal: &Entity<TerminalView>) -> (Status, Option<String>) {
    terminal.read_with(cx, |t, _| {
        let summary = t.summary();
        (summary.status, summary.agent_message)
    })
}

pub(super) fn words(text: &str) -> Option<String> {
    Some(text.to_string())
}

pub(super) fn event(cx: &mut TestAppContext, terminal: &Entity<TerminalView>, event: PtyEvent) {
    cx.update(|cx| terminal.update(cx, |t, cx| t.event_for_test(event, cx)));
}

#[gpui_kit::test]
fn a_programs_report_is_the_terminals_status_and_words(cx: &mut TestAppContext) {
    if isolate("a_programs_report_is_the_terminals_status_and_words") {
        return;
    }
    let window = start(cx, 1);
    let t = terminal(cx, &window, 0);

    report(cx, &t, "state=blocked:kind=permission");
    assert_eq!(now(cx, &t), (Status::Attention, words("Waiting for approval")), "a default per kind");
    report(cx, &t, "state=blocked:kind=question:app=terraform:msg={msg:Apply 3 to add?}");
    assert_eq!(now(cx, &t), (Status::Attention, words("Apply 3 to add?")), "its own words");
    report(cx, &t, "state=working:app=brew:msg={msg:Installing updates}");
    assert_eq!(now(cx, &t), (Status::Busy, words("Installing updates")));
    report(cx, &t, "state=done:app=brew:msg={msg:Upgraded 12 packages}");
    assert_eq!(now(cx, &t), (Status::Done, words("Upgraded 12 packages")));
    // An error is a failure of its own, with what went wrong.
    report(cx, &t, "state=error:msg={msg:rsync failed}");
    assert_eq!(now(cx, &t), (Status::Failed, words("rsync failed")));
    report(cx, &t, "state=idle");
    assert_eq!(now(cx, &t), (Status::Idle, None));
    // Everything it said is gone: nothing left of the status either.
    report(cx, &t, "state=working");
    report(cx, &t, "state=clear");
    assert_eq!(now(cx, &t), (Status::Idle, None));
    assert!(!t.read_with(cx, |t, _| t.reported_by_program()));

    // Text that does not decode, or an id that is not one, is not heard: the working report stands.
    report(cx, &t, "state=working");
    report(cx, &t, "state=done:msg=a");
    report(cx, &t, "state=clear:id=//");
    assert_eq!(now(cx, &t).0, Status::Busy);
    assert!(t.read_with(cx, |t, _| t.reported_by_program()));
}

#[gpui_kit::test]
fn the_words_a_program_sends_are_cleaned_before_they_show(cx: &mut TestAppContext) {
    if isolate("the_words_a_program_sends_are_cleaned_before_they_show") {
        return;
    }
    let window = start(cx, 1);
    let t = terminal(cx, &window, 0);
    // A direction override and a zero-width space could disguise what it says; a long line is cut.
    report(cx, &t, "state=blocked:msg={msg:Delete\u{202e} all   the\u{200b} files?}");
    assert_eq!(now(cx, &t), (Status::Attention, words("Delete all the files?")));
    let long = "w".repeat(1000);
    report(cx, &t, &format!("state=blocked:msg={{msg:{long}}}"));
    let (_, message) = now(cx, &t);
    assert_eq!(message.map(|m| m.chars().count()), Some(201));
}

#[gpui_kit::test]
fn a_programs_name_gives_the_agent_only_when_none_was_found(cx: &mut TestAppContext) {
    if isolate("a_programs_name_gives_the_agent_only_when_none_was_found") {
        return;
    }
    let window = start(cx, 2);
    let (first, second) = (terminal(cx, &window, 0), terminal(cx, &window, 1));
    let agent = |cx: &mut TestAppContext, t: &Entity<TerminalView>| t.read_with(cx, |t, _| t.agent);
    report(cx, &first, "state=working:app=codex");
    assert_eq!(agent(cx, &first), Some(AgentId::Codex));
    report(cx, &first, "state=working:app=terraform");
    assert_eq!(agent(cx, &first), Some(AgentId::Codex), "a report without a known app leaves it");
    // Detected already (here by an agent's hook): the report doesn't take it over.
    hook(cx, &window, 2, "UserPromptSubmit", None);
    assert_eq!(agent(cx, &second), Some(AgentId::Claude));
    report(cx, &second, "state=working:app=codex");
    assert_eq!(agent(cx, &second), Some(AgentId::Claude));
}

#[gpui_kit::test]
fn a_programs_name_is_compared_with_the_summary_without_making_it(cx: &mut TestAppContext) {
    if isolate("a_programs_name_is_compared_with_the_summary_without_making_it") {
        return;
    }
    let window = start(cx, 1);
    let t = terminal(cx, &window, 0);
    let agrees = |cx: &mut TestAppContext| {
        t.read_with(cx, |t, _| {
            let name = t.program_name();
            t.program_name_is(name.as_deref())
                && !t.program_name_is(Some("not-its-name"))
                && (name.is_some() || t.program_name_is(None))
                && t.summary().matches(t)
        })
    };
    assert!(agrees(cx), "no program reports");
    report(cx, &t, "state=working:app=terraform");
    assert_eq!(t.read_with(cx, |t, _| t.program_name()), Some("terraform".to_string()));
    assert!(agrees(cx), "a program that names itself");
    let before = t.read_with(cx, |t, _| t.summary());
    report(cx, &t, "state=working:app=pulumi");
    assert!(t.read_with(cx, |t, _| !before.matches(t)), "another name is another summary");
}

#[gpui_kit::test]
fn the_output_heuristics_leave_a_reported_status_alone(cx: &mut TestAppContext) {
    if isolate("the_output_heuristics_leave_a_reported_status_alone") {
        return;
    }
    let window = start(cx, 1);
    let t = terminal(cx, &window, 0);
    report(cx, &t, "state=idle");
    // A command's start (OSC 133 C), its end and the bell, and a notification: all things the
    // heuristics would take for work, for needing you, or for the end of work.
    for mark in [Mark::OutputStart, Mark::Finished(Some(0))] {
        event(cx, &t, PtyEvent::Mark(mark, 0));
    }
    event(cx, &t, PtyEvent::Bell);
    assert_eq!(now(cx, &t).0, Status::Idle, "idle is what it said");
    report(cx, &t, "state=working");
    event(cx, &t, PtyEvent::Mark(Mark::Finished(Some(0)), 0));
    let settled = cx.update(|cx| t.update(cx, |t, _| t.tick(false, false)));
    assert!(!settled, "the tick does not settle work the program says goes on");
    assert_eq!(now(cx, &t).0, Status::Busy);
    event(cx, &t, PtyEvent::Bell);
    assert_eq!(now(cx, &t).0, Status::Busy, "the bell does not ask for you: the program says when");
    assert!(!t.read_with(cx, |t, _| t.bell_rang()));

    // Reporting over, the heuristics are back: a command starting is work again.
    report(cx, &t, "state=clear");
    event(cx, &t, PtyEvent::Mark(Mark::OutputStart, 0));
    assert_eq!(now(cx, &t).0, Status::Busy);
    event(cx, &t, PtyEvent::Mark(Mark::Finished(Some(0)), 0));
    cx.update(|cx| t.update(cx, |t, _| t.tick(false, false)));
    assert_eq!(now(cx, &t).0, Status::Done, "settled by the tick as before");
}

#[gpui_kit::test]
fn a_reported_question_stays_and_a_finished_result_goes_when_looked_at(cx: &mut TestAppContext) {
    if isolate("a_reported_question_stays_and_a_finished_result_goes_when_looked_at") {
        return;
    }
    let window = start(cx, 1);
    let t = terminal(cx, &window, 0);
    let look = |cx: &mut TestAppContext| cx.update(|cx| t.update(cx, |t, _| t.acknowledge()));

    report(cx, &t, "state=blocked:kind=auth");
    assert!(!look(cx));
    assert_eq!(now(cx, &t).0, Status::Attention, "it waits for the program to move on, like a hook's");
    report(cx, &t, "state=working");
    assert_eq!(now(cx, &t).0, Status::Busy);
    assert!(!look(cx), "nothing to see in work going on");

    // Done is seen when the user looks, and does not come back with the next read of the records.
    report(cx, &t, "state=done:msg={msg:Built}");
    assert_eq!(now(cx, &t).0, Status::Done);
    assert!(look(cx));
    assert_eq!(now(cx, &t), (Status::Idle, None));
    assert_eq!(t.read_with(cx, |t, _| t.records_for_test()), 0, "the record is dropped with it");
    event(cx, &t, PtyEvent::ProgramStatus);
    assert_eq!(now(cx, &t).0, Status::Idle);
    // The same for a failure.
    report(cx, &t, "state=error");
    assert_eq!(now(cx, &t).0, Status::Failed);
    assert!(look(cx));
    assert_eq!(now(cx, &t), (Status::Idle, None));
    assert_eq!(t.read_with(cx, |t, _| t.records_for_test()), 0);

    // A worker still going keeps the status busy when the root's result is seen.
    report(cx, &t, "state=working:id=worker");
    report(cx, &t, "state=done:msg={msg:Planned}");
    assert_eq!(now(cx, &t).0, Status::Done);
    assert!(look(cx));
    assert_eq!(now(cx, &t).0, Status::Busy);
}

#[gpui_kit::test]
fn a_prompt_or_the_programs_exit_ends_work_but_not_what_finished(cx: &mut TestAppContext) {
    if isolate("a_prompt_or_the_programs_exit_ends_work_but_not_what_finished") {
        return;
    }
    let window = start(cx, 1);
    let t = terminal(cx, &window, 0);
    let prompt = |cx: &mut TestAppContext| cx.update(|cx| t.update(cx, |t, cx| t.prompt_for_test(cx)));
    let left = |cx: &mut TestAppContext| cx.update(|cx| t.update(cx, |t, _| t.left_for_test()));

    report(cx, &t, "state=working");
    prompt(cx);
    assert_eq!(now(cx, &t), (Status::Idle, None));
    report(cx, &t, "state=blocked:kind=question");
    prompt(cx);
    assert_eq!(now(cx, &t).0, Status::Idle);
    report(cx, &t, "state=done:msg={msg:Upgraded 12 packages}");
    prompt(cx);
    assert_eq!(now(cx, &t), (Status::Done, words("Upgraded 12 packages")), "done survives the prompt");

    // The program that was in front is gone, and said nothing: no stale work is left.
    report(cx, &t, "state=clear");
    assert!(!left(cx), "nothing reported, nothing to end");
    report(cx, &t, "state=blocked:kind=permission");
    assert!(left(cx));
    assert_eq!(now(cx, &t), (Status::Idle, None));
    report(cx, &t, "state=error:msg={msg:rsync failed}");
    assert!(!left(cx));
    assert_eq!(now(cx, &t), (Status::Failed, words("rsync failed")), "an error outlives its process");
}

#[gpui_kit::test]
fn hooks_and_reports_take_turns_and_the_last_one_wins(cx: &mut TestAppContext) {
    if isolate("hooks_and_reports_take_turns_and_the_last_one_wins") {
        return;
    }
    let window = start(cx, 1);
    let t = terminal(cx, &window, 0);
    hook(cx, &window, 1, "UserPromptSubmit", None);
    assert_eq!(now(cx, &t).0, Status::Busy);
    // The program says it needs input: that is the latest word.
    report(cx, &t, "state=blocked:kind=permission:msg={msg:Run it?}");
    assert_eq!(now(cx, &t), (Status::Attention, words("Run it?")));
    // The hook says it works again; the program's records don't reset it by being cleared.
    hook(cx, &window, 1, "UserPromptSubmit", None);
    assert_eq!(now(cx, &t), (Status::Busy, None));
    report(cx, &t, "state=clear");
    assert_eq!(now(cx, &t), (Status::Busy, None), "a clear never reset what a hook set");
    // An empty set of records that never spoke changes nothing either.
    report(cx, &t, "state=clear");
    assert_eq!(now(cx, &t).0, Status::Busy);
}

/// Banners while Wormhole is in front (the test window always is): the default there is nothing.
pub(super) fn banners_in_front(cx: &mut TestAppContext) {
    cx.update(|cx| SettingsStore::update(cx, |s| s.notify_in_front = InFront::Banner));
}

#[gpui_kit::test]
fn a_reported_question_is_told_but_a_program_cannot_flood_the_banners(cx: &mut TestAppContext) {
    if isolate("a_reported_question_is_told_but_a_program_cannot_flood_the_banners") {
        return;
    }
    let window = start(cx, 2);
    banners_in_front(cx);
    // The second terminal is in the background; the tick would sweep, the test does.
    let t = terminal(cx, &window, 1);
    let notices = |cx: &mut TestAppContext| -> Vec<(String, String)> {
        cx.update(notifications::sweep);
        cx.delivered_system_notifications()
            .into_iter()
            .map(|n| (n.title.to_string(), n.body.to_string()))
            .collect()
    };
    // An unnamed terminal is named after what runs in it, so the first report, which names the
    // agent, renames it: read when a banner is checked.
    let name = |cx: &mut TestAppContext| drive(cx, &window, |ws, _, _| ws.sessions[1].name().to_string());

    report(cx, &t, "state=blocked:kind=permission:app=claude-code:msg={msg:Allow Bash?}");
    let needs_you = format!("{} needs input", name(cx));
    assert_eq!(notices(cx), [(needs_you, "Allow Bash?".to_string())]);
    report(cx, &t, "state=working:app=claude-code");
    assert!(notices(cx).is_empty(), "it moved on: the banner goes");
    // Asking again straight away, and again: a program can do that as fast as it writes. The
    // sidebar shows it; no new banner, sound or bounce does.
    for _ in 0..3 {
        report(cx, &t, "state=blocked:kind=permission:app=claude-code:msg={msg:Allow Bash again?}");
        assert!(notices(cx).is_empty(), "told at most once in a while");
        assert_eq!(now(cx, &t).0, Status::Attention);
        report(cx, &t, "state=working:app=claude-code");
        notices(cx);
    }
    // Finishing is a kind of its own, with its own clock.
    report(cx, &t, "state=done:app=claude-code:msg={msg:All done}");
    let told = notices(cx);
    assert_eq!(told.len(), 1, "{told:?}");
    assert_eq!(told[0].0, format!("{} finished", name(cx)));
}
