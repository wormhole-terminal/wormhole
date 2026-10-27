//! Programs asking to control Wormhole, in real windows: the gate every control request passes, the question
//! it raises, what each answer does, and that nothing a request says can change what is allowed. Nothing
//! here shows a sheet: under a test the question is answered by a script (`agent_access::testing`). The
//! harness is `test_harness`.

use std::time::Duration;

use gpui_kit::TestAppContext;
use wormhole_core::config::AgentAccess;
use wormhole_core::ipc::bus::Next;
use wormhole_core::ipc::control::event::CloseReason;
use wormhole_core::ipc::control::{Action, ErrorCode, Reason, Reply, Request, Target};
use wormhole_core::ipc::{Answer, AskJob, Decision, Peer};

use crate::agent_access::{self, Button, Which, testing};
use crate::app::control;
use crate::app_state::{app, app_mut};
use crate::settings::{settings, update_settings};
use crate::window::test_harness::{isolate_in, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// A caller outside every terminal: what a script, or an agent started from somewhere else, is.
fn outside() -> Peer {
    Peer::new(900_002, vec![1])
}

fn typing() -> Action {
    Action::SendText { target: Target::Terminal(1), text: "ls".into(), submit: false }
}

fn answer(cx: &mut TestAppContext, action: Action, waits: bool) -> Answer {
    let request = Request::new(action, None);
    let request = if waits { request.asking() } else { request };
    cx.update(|cx| control::answer(request, &outside(), cx))
}

fn reply(answer: Answer) -> Reply {
    match answer {
        Answer::Reply(reply) => reply,
        other => panic!("one line was expected: {other:?}"),
    }
}

fn waiting(answer: Answer) -> AskJob {
    match answer {
        Answer::Ask(job) => job,
        other => panic!("a wait for the user was expected: {other:?}"),
    }
}

fn reason(reply: &Reply) -> Option<Reason> {
    reply.err().and_then(|error| error.reason)
}

fn decision(job: &AskJob) -> Decision {
    job.decision_within(Duration::from_secs(2)).expect("the question was answered")
}

fn questions(cx: &mut TestAppContext) -> Vec<agent_access::Waiting> {
    cx.update(|cx| agent_access::pending_questions(true, cx))
}

fn press(cx: &mut TestAppContext, button: Button) -> bool {
    let pressed = cx.update(|cx| agent_access::answer_pending(button, cx));
    cx.run_until_parked();
    pressed
}

fn mode(cx: &mut TestAppContext, mode: AgentAccess) {
    cx.update(|cx| update_settings(cx, |s| s.agent_access = mode));
}

/// Wormhole with one window and nothing allowed: the question stays until the test answers it.
fn start_asking(cx: &mut TestAppContext) {
    let _window = start(cx, 1);
    testing::allow(cx, false, false);
    testing::hold(cx);
}

#[gpui_kit::test]
fn a_program_nothing_allows_is_refused_at_once_when_it_cannot_wait_and_the_question_stays(
    cx: &mut TestAppContext,
) {
    if isolate("a_program_nothing_allows_is_refused_at_once_when_it_cannot_wait_and_the_question_stays") {
        return;
    }
    start_asking(cx);
    let refused = reply(answer(cx, typing(), false));
    assert_eq!(refused.err().map(|e| e.code), Some(ErrorCode::NotAllowed));
    assert_eq!(reason(&refused), Some(Reason::ApprovalNeeded));
    // The user is asked all the same: they may want to say yes for the next attempt.
    let asked = questions(cx);
    assert_eq!(asked.len(), 1, "{asked:?}");
    assert!(asked[0].shown && asked[0].label == "Scripts");
    // A script is offered no "for this session" (it has no process) and no "always".
    assert_eq!(asked[0].buttons, [Button::Deny, Button::Quit]);
    // Asking again does not ask a second question.
    let _ = answer(cx, typing(), false);
    assert_eq!(questions(cx).len(), 1);
}

#[gpui_kit::test]
fn a_caller_that_waits_is_held_until_the_answer_and_then_let_in(cx: &mut TestAppContext) {
    if isolate("a_caller_that_waits_is_held_until_the_answer_and_then_let_in") {
        return;
    }
    start_asking(cx);
    let job = waiting(answer(cx, typing(), true));
    assert!(job.decision_within(Duration::from_millis(50)).is_none(), "nobody has answered");
    assert!(press(cx, Button::Quit));
    assert!(matches!(decision(&job), Decision::Retry));
    // Running the request again, as the socket's thread does, finds the answer.
    let typed = reply(answer(cx, typing(), true));
    assert!(typed.ok, "{typed:?}");
    // And Settings lists it, to be taken back.
    let listed = cx.update(agent_access::approved);
    assert_eq!(listed.len(), 1);
    assert_eq!((listed[0].label.as_str(), listed[0].how), ("Scripts", agent_access::How::UntilQuit));
    assert!(questions(cx).is_empty());
}

#[gpui_kit::test]
fn two_requests_of_one_program_wait_behind_one_question_and_one_answer_serves_both(cx: &mut TestAppContext) {
    if isolate("two_requests_of_one_program_wait_behind_one_question_and_one_answer_serves_both") {
        return;
    }
    start_asking(cx);
    let (first, second) = (waiting(answer(cx, typing(), true)), waiting(answer(cx, typing(), true)));
    assert_eq!(questions(cx).len(), 1);
    assert!(press(cx, Button::Quit));
    assert!(matches!(decision(&first), Decision::Retry) && matches!(decision(&second), Decision::Retry));
}

#[gpui_kit::test]
fn dont_allow_refuses_the_waiting_request_and_the_program_is_not_asked_again_for_two_minutes(
    cx: &mut TestAppContext,
) {
    if isolate("dont_allow_refuses_the_waiting_request_and_the_program_is_not_asked_again_for_two_minutes") {
        return;
    }
    start_asking(cx);
    let job = waiting(answer(cx, typing(), true));
    assert!(press(cx, Button::Deny));
    let Decision::Refuse(refusal) = decision(&job) else { panic!("a refusal") };
    assert_eq!(
        (refusal.err().map(|e| e.code), reason(&refusal)),
        (Some(ErrorCode::NotAllowed), Some(Reason::Declined))
    );
    // A looping program is not turned into a storm of questions.
    let again = reply(answer(cx, typing(), true));
    assert_eq!(reason(&again), Some(Reason::DeclinedRecently));
    assert!(questions(cx).is_empty(), "no new question");
}

#[gpui_kit::test]
fn a_question_nobody_answers_counts_as_no_after_a_minute_and_is_not_remembered(cx: &mut TestAppContext) {
    if isolate("a_question_nobody_answers_counts_as_no_after_a_minute_and_is_not_remembered") {
        return;
    }
    start_asking(cx);
    let job = waiting(answer(cx, typing(), true));
    cx.executor().advance_clock(Duration::from_secs(59));
    cx.run_until_parked();
    assert_eq!(questions(cx).len(), 1, "still up at 59 s");
    cx.executor().advance_clock(Duration::from_secs(2));
    cx.run_until_parked();
    let Decision::Refuse(refusal) = decision(&job) else { panic!("a refusal") };
    assert_eq!(reason(&refusal), Some(Reason::AskTimedOut));
    assert!(questions(cx).is_empty());
    // Not remembered as a no: asked again at the next attempt.
    let next = reply(answer(cx, typing(), false));
    assert_eq!(reason(&next), Some(Reason::ApprovalNeeded));
    assert_eq!(questions(cx).len(), 1);
}

#[gpui_kit::test]
fn off_lets_nothing_in_and_open_lets_everything_in_without_a_question(cx: &mut TestAppContext) {
    if isolate("off_lets_nothing_in_and_open_lets_everything_in_without_a_question") {
        return;
    }
    start_asking(cx);
    mode(cx, AgentAccess::Off);
    let off = reply(answer(cx, typing(), true));
    assert_eq!(reason(&off), Some(Reason::AgentAccessOff));
    assert!(questions(cx).is_empty());
    // What needs nothing still works.
    assert!(reply(answer(cx, Action::List, true)).ok);
    mode(cx, AgentAccess::Open);
    assert!(reply(answer(cx, typing(), true)).ok);
    assert!(questions(cx).is_empty());
}

#[gpui_kit::test]
fn switching_access_off_refuses_the_questions_that_are_waiting(cx: &mut TestAppContext) {
    if isolate("switching_access_off_refuses_the_questions_that_are_waiting") {
        return;
    }
    start_asking(cx);
    let job = waiting(answer(cx, typing(), true));
    mode(cx, AgentAccess::Off);
    cx.update(|cx| agent_access::tick(1, cx));
    cx.update(|cx| agent_access::tick(2, cx));
    let Decision::Refuse(refusal) = decision(&job) else { panic!("a refusal") };
    assert_eq!(reason(&refusal), Some(Reason::AgentAccessOff));
    assert!(questions(cx).is_empty());
}

#[gpui_kit::test]
fn an_approval_in_the_database_lets_a_program_in_and_revoking_it_ends_what_it_started(
    cx: &mut TestAppContext,
) {
    if isolate("an_approval_in_the_database_lets_a_program_in_and_revoking_it_ends_what_it_started") {
        return;
    }
    use wormhole_core::ipc::access::{CapSet, Identity};
    use wormhole_core::ipc::bus::Bus;
    use wormhole_core::ipc::control::EventsSpec;
    use wormhole_core::storage::store::AgentGrantRow;

    let _window = start(cx, 1);
    let bus = Bus::new();
    cx.update(|cx| crate::automation::install(bus.clone(), cx));
    // The test app has no database of its own: one in a folder of its own, as the app's is.
    let dir = std::env::temp_dir().join(format!("wormhole-agent-grants-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    cx.update(|cx| {
        app_mut(cx).store = Some(wormhole_core::storage::store::Store::open_at(&dir).expect("opens"))
    });
    testing::allow(cx, false, false);
    testing::hold(cx);
    let id = cx.update(|cx| {
        let profile = app(cx).active_profile().clone();
        let row = AgentGrantRow {
            key: Identity::scripts().key(),
            label: "Scripts".into(),
            caps: CapSet::STANDARD.bits(),
            created: 1,
            ..AgentGrantRow::default()
        };
        app_mut(cx).store_of_mut(&profile).expect("a database").save_agent_grant(&row).expect("saves")
    });
    assert!(reply(answer(cx, typing(), false)).ok, "a stored approval is enough, with no question");
    assert!(questions(cx).is_empty());

    // A stream of what terminals print is under the approval it was started with.
    let spec = EventsSpec { output: true, topics: vec!["output".into()], ..EventsSpec::default() };
    let Answer::Stream { subscription, .. } = answer(cx, Action::Events(spec), false) else {
        panic!("a stream")
    };
    cx.update(|cx| agent_access::revoke(Which::Stored(id), cx));
    assert!(matches!(subscription.next(Duration::from_secs(1)), Next::Closed(CloseReason::NotAllowed)));
    // Taken back: the program is asked again.
    let after = reply(answer(cx, typing(), false));
    assert_eq!(reason(&after), Some(Reason::ApprovalNeeded));
    assert!(cx.update(agent_access::approved).is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[gpui_kit::test]
fn an_approval_for_a_session_goes_when_the_program_has_ended(cx: &mut TestAppContext) {
    if isolate("an_approval_for_a_session_goes_when_the_program_has_ended") {
        return;
    }
    let _window = start(cx, 1);
    testing::allow(cx, false, false);
    testing::grant_session_of_a_gone_process(cx);
    assert_eq!(cx.update(agent_access::approved).len(), 1);
    // The sweep runs every fifth tick.
    cx.update(|cx| agent_access::tick(5, cx));
    assert!(cx.update(agent_access::approved).is_empty());
}

#[gpui_kit::test]
fn a_test_never_puts_a_sheet_up(cx: &mut TestAppContext) {
    if isolate("a_test_never_puts_a_sheet_up") {
        return;
    }
    let _window = start(cx, 1);
    testing::allow(cx, false, false);
    // Nothing set: a question is answered by the script that says no, at once, without any window.
    let job = waiting(answer(cx, typing(), true));
    cx.run_until_parked();
    let Decision::Refuse(refusal) = decision(&job) else { panic!("a refusal") };
    assert_eq!(reason(&refusal), Some(Reason::Declined));
}

/// The sheet's words are all in the catalogs: in the pseudo-language each message is accented and in `⟦ ⟧`, so a
/// word of it still written in Rust shows (the program's name, path and signer are data and stay as they are).
#[gpui_kit::test]
fn the_sheet_is_worded_by_the_catalogs_only(cx: &mut TestAppContext) {
    if isolate("the_sheet_is_worded_by_the_catalogs_only") {
        return;
    }
    use wormhole_text::{Language, with_language};
    start_asking(cx);
    let _ = answer(cx, typing(), false);
    let asked = questions(cx);
    assert_eq!(asked.len(), 1);
    let said = |text: &str| text.starts_with('⟦') && text.ends_with('⟧');
    with_language(Language::PSEUDO, || {
        let asked = questions(cx);
        let question = &asked[0];
        assert!(said(&question.title), "{}", question.title);
        for paragraph in question.detail.split("\n\n") {
            assert!(said(paragraph), "{paragraph:?}");
        }
        // The labels of the facts block are catalog words too; the values after them are data.
        for line in question.facts.lines() {
            let label = line.split("  ").next().unwrap_or_default();
            assert!(said(label), "{line:?}");
        }
    });
    // And in English they read as sentences: the title names the program.
    with_language(Language::ENGLISH, || {
        let asked = questions(cx);
        assert_eq!(asked[0].title, "Allow scripts to control Wormhole?");
        assert!(asked[0].detail.starts_with("It wants to type into terminals."), "{}", asked[0].detail);
    });
}

/// A request for each action, enough to be carried out in a window of two terminals.
fn sample(name: &str) -> serde_json::Value {
    use serde_json::json;
    let target = json!({ "kind": "terminal", "id": 1 });
    let mut request = json!({ "version": 1, "action": name });
    let extra = match name {
        "notify" => json!({ "body": "hello" }),
        "send_text" => json!({ "target": target, "text": "echo hi" }),
        "read" | "blocks" | "close" => json!({ "target": target }),
        "send_keys" => json!({ "target": target, "keys": [{ "key": "enter" }] }),
        "mouse" => json!({ "target": target, "op": { "kind": "move", "row": 0, "col": 0 } }),
        "split" | "focus" => json!({ "target": target }),
        "rename" => json!({ "target": target, "name": "renamed" }),
        "wait" => json!({ "target": target, "status": ["idle"], "timeout_ms": 100 }),
        "run" => json!({ "target": target, "command": "true", "timeout_ms": 100 }),
        "welcome" => json!({ "token": "no-such-token", "op": { "kind": "state" } }),
        _ => json!({}),
    };
    if let (Some(base), Some(extra)) = (request.as_object_mut(), extra.as_object()) {
        base.extend(extra.clone());
    }
    request
}

/// No request, of any action, changes what is allowed, the mode or the questions: only the user does, on a
/// sheet or in Settings. (A new action fails here until it has a sample, so it is walked too.)
#[gpui_kit::test]
fn no_control_action_changes_what_is_allowed(cx: &mut TestAppContext) {
    if isolate("no_control_action_changes_what_is_allowed") {
        return;
    }
    use wormhole_core::ipc::control::Action as A;
    let _window = start(cx, 2);
    testing::allow(cx, true, true);
    testing::hold(cx);
    let before = cx.update(|cx| {
        (
            agent_access::approved(cx),
            settings(cx).agent_access,
            settings(cx).control_write,
            settings(cx).feature_flags.clone(),
            agent_access::pending_questions(false, cx),
        )
    });
    for name in A::KNOWN {
        let request: Request = serde_json::from_value(sample(name))
            .unwrap_or_else(|err| panic!("the sample of `{name}` does not parse: {err}"));
        let _ = cx.update(|cx| control::answer(request, &outside(), cx));
        cx.run_until_parked();
        let after = cx.update(|cx| {
            (
                agent_access::approved(cx),
                settings(cx).agent_access,
                settings(cx).control_write,
                settings(cx).feature_flags.clone(),
                agent_access::pending_questions(false, cx),
            )
        });
        assert_eq!(after, before, "`{name}` changed what is allowed");
    }
}
