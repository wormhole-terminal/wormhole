//! Which terminals a program that controls Wormhole reaches, and the terminals the user hid from it, in real
//! windows (`../repo-docs/docs/agent-permissions.md`, sections 5 and 6). A program uses the terminals it
//! opened and the one it runs in; any other is asked about, for that terminal alone; a hidden terminal is
//! not found by anybody who is not inside it. Nothing here shows a sheet: a script answers
//! (`agent_access::testing`). The harness is `test_harness`.

use gpui_kit::TestAppContext;
use wormhole_core::config::AgentAccess;
use wormhole_core::ipc::control::{Action, ErrorCode, Reason, Reply, Request};
use wormhole_core::ipc::{Answer, AskJob, Decision, Peer};
use wormhole_core::storage::store::{SessionRecord, WindowRecord};
use wormhole_text::{Language, with_language};

use crate::agent_access::{self, Button, Which, testing};
use crate::app::control;
use crate::settings::update_settings;
use crate::window::reported_tests::terminal;
use crate::window::test_harness::{drive, isolate_in, read, start, start_with};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// A caller outside every terminal: a script, or an agent started from somewhere else.
fn outside() -> Peer {
    Peer::new(900_002, vec![1])
}

fn answer(cx: &mut TestAppContext, request: Request, peer: &Peer) -> Answer {
    cx.update(|cx| control::answer(request, peer, cx))
}

fn reply(answer: Answer) -> Reply {
    match answer {
        Answer::Reply(reply) => reply,
        Answer::Read(job) => job.run(&mut wormhole_term::ReadScratch::default()),
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

fn request(json: serde_json::Value) -> Request {
    serde_json::from_value(json).expect("a request")
}

fn target(id: i64) -> serde_json::Value {
    serde_json::json!({ "kind": "terminal", "id": id })
}

fn typing(id: i64) -> Request {
    request(serde_json::json!({ "version": 1, "action": "send_text", "target": target(id), "text": "ls" }))
}

fn reading(id: i64) -> Request {
    request(serde_json::json!({ "version": 1, "action": "read", "target": target(id) }))
}

fn questions(cx: &mut TestAppContext) -> Vec<agent_access::Waiting> {
    cx.update(|cx| agent_access::pending_questions(true, cx))
}

fn press(cx: &mut TestAppContext, button: Button) -> bool {
    let pressed = cx.update(|cx| agent_access::answer_pending(button, cx));
    cx.run_until_parked();
    pressed
}

fn decision(job: &AskJob) -> Decision {
    job.decision_within(std::time::Duration::from_secs(2)).expect("the question was answered")
}

/// Wormhole with `n` terminals and Scripts let in with the scopes in force: it reaches what it opens and nothing
/// else without a question, which stays until the test answers it.
fn start_scoped(cx: &mut TestAppContext, n: i64, read: bool, write: bool) -> crate::app_state::OpenWindow {
    let window = start(cx, n);
    testing::allow_scoped(cx, read, write);
    testing::hold(cx);
    cx.update(|cx| agent_access::tick(0, cx));
    window
}

/// Open a terminal as `peer` and say which one it is.
fn open_terminal(cx: &mut TestAppContext, peer: &Peer) -> i64 {
    let opened =
        reply(answer(cx, request(serde_json::json!({ "version": 1, "action": "new_terminal" })), peer));
    assert!(opened.ok, "{opened:?}");
    opened.result.expect("a result")["terminal"].as_i64().expect("the terminal's id")
}

#[gpui_kit::test]
fn a_program_reaches_the_terminals_it_opened_and_is_asked_about_any_other(cx: &mut TestAppContext) {
    if isolate("a_program_reaches_the_terminals_it_opened_and_is_asked_about_any_other") {
        return;
    }
    let _window = start_scoped(cx, 2, true, true);
    let peer = outside();
    // What it opened is its own: typing and reading need no question.
    let mine = open_terminal(cx, &peer);
    assert!(reply(answer(cx, typing(mine), &peer)).ok);
    assert!(reply(answer(cx, reading(mine), &peer)).ok);
    assert!(questions(cx).is_empty());
    // The user's own terminal 1 is another's: it asks, for terminal 1, and the question says so.
    let job = waiting(answer(cx, typing(1).asking(), &peer));
    let asked = questions(cx);
    assert_eq!(asked.len(), 1, "{asked:?}");
    assert_eq!(asked[0].about, Some(1));
    // A terminal is never approved for good, and a script has no process to allow it for.
    assert_eq!(asked[0].buttons, [Button::Deny, Button::Quit]);
    assert!(press(cx, Button::Quit));
    assert!(matches!(decision(&job), Decision::Retry));
    assert!(reply(answer(cx, typing(1), &peer)).ok, "it is in scope now");
    // Only that terminal: terminal 2 is asked about on its own...
    let refused = reply(answer(cx, typing(2), &peer));
    assert_eq!(reason(&refused), Some(Reason::ApprovalNeeded));
    let asked = questions(cx);
    assert_eq!(asked.iter().map(|q| q.about).collect::<Vec<_>>(), [Some(2)]);
    assert!(press(cx, Button::Deny));
    // ...and only for what was asked: typing into 1 was allowed, reading it was not.
    let refused = reply(answer(cx, reading(1), &peer));
    assert_eq!(reason(&refused), Some(Reason::ApprovalNeeded));
    assert_eq!(questions(cx).iter().map(|q| q.about).collect::<Vec<_>>(), [Some(1)]);
}

#[gpui_kit::test]
fn no_to_a_terminal_cools_down_for_that_terminal_only(cx: &mut TestAppContext) {
    if isolate("no_to_a_terminal_cools_down_for_that_terminal_only") {
        return;
    }
    let _window = start_scoped(cx, 2, true, true);
    let peer = outside();
    let job = waiting(answer(cx, typing(1).asking(), &peer));
    assert!(press(cx, Button::Deny));
    let Decision::Refuse(refusal) = decision(&job) else { panic!("refused") };
    assert_eq!(refusal.err().and_then(|e| e.reason), Some(Reason::Declined));
    // Not asked again about terminal 1 for two minutes; terminal 2 is another matter.
    assert_eq!(reason(&reply(answer(cx, typing(1), &peer))), Some(Reason::DeclinedRecently));
    assert!(questions(cx).is_empty());
    assert_eq!(reason(&reply(answer(cx, typing(2), &peer))), Some(Reason::ApprovalNeeded));
    assert_eq!(questions(cx).iter().map(|q| q.about).collect::<Vec<_>>(), [Some(2)]);
}

#[gpui_kit::test]
fn a_split_of_an_owned_and_a_foreign_terminal_is_in_scope_where_it_is_owned(cx: &mut TestAppContext) {
    if isolate("a_split_of_an_owned_and_a_foreign_terminal_is_in_scope_where_it_is_owned") {
        return;
    }
    let _window = start_scoped(cx, 1, true, true);
    let peer = outside();
    // Splitting the user's terminal is opening, which is not scoped: what it makes is the program's own.
    let made = reply(answer(
        cx,
        request(serde_json::json!({ "version": 1, "action": "split", "target": target(1) })),
        &peer,
    ));
    assert!(made.ok, "{made:?}");
    let new = made.result.expect("a result")["terminal"].as_i64().expect("the new terminal");
    assert!(reply(answer(cx, reading(new), &peer)).ok, "the half it made is its own");
    assert!(questions(cx).is_empty());
    // The other half is the user's.
    assert_eq!(reason(&reply(answer(cx, reading(1), &peer))), Some(Reason::ApprovalNeeded));
    assert_eq!(questions(cx).iter().map(|q| q.about).collect::<Vec<_>>(), [Some(1)]);
}

#[gpui_kit::test]
fn the_terminal_one_runs_in_is_in_scope_without_a_question(cx: &mut TestAppContext) {
    if isolate("the_terminal_one_runs_in_is_in_scope_without_a_question") {
        return;
    }
    let window = start_scoped(cx, 2, true, true);
    let shell = terminal(cx, &window, 0).read_with(cx, |t, _| t.shell_pid()).expect("a shell runs");
    let inside = Peer::new(900_001, vec![shell]);
    assert!(reply(answer(cx, reading(1), &inside)).ok);
    assert!(questions(cx).is_empty());
    // And another one is not.
    assert_eq!(reason(&reply(answer(cx, reading(2), &inside))), Some(Reason::ApprovalNeeded));
}

#[gpui_kit::test]
fn what_a_program_opened_is_forgotten_when_the_terminal_closes_and_is_not_kept_across_a_restart(
    cx: &mut TestAppContext,
) {
    if isolate("what_a_program_opened_is_forgotten_when_the_terminal_closes_and_is_not_kept_across_a_restart")
    {
        return;
    }
    let window = start_scoped(cx, 1, true, true);
    let peer = outside();
    let mine = open_terminal(cx, &peer);
    assert_eq!(testing::owned_by_scripts(cx), [mine]);
    let ix = read(cx, &window, |ws| ws.sessions.iter().position(|s| s.id == mine)).expect("it is open");
    drive(cx, &window, |ws, window, cx| ws.close_session(ix, window, cx));
    cx.update(|cx| agent_access::tick(5, cx));
    assert!(testing::owned_by_scripts(cx).is_empty(), "a closed terminal drops out");
    // Memory only: a new run of Wormhole starts with nothing owned.
    let again = open_terminal(cx, &peer);
    assert_eq!(testing::owned_by_scripts(cx), [again]);
    cx.update(testing::restart);
    assert!(testing::owned_by_scripts(cx).is_empty(), "what a program opened before is somebody else's now");
}

#[gpui_kit::test]
fn switching_a_capability_off_asks_again_for_it_and_back_on_lets_it_through(cx: &mut TestAppContext) {
    if isolate("switching_a_capability_off_asks_again_for_it_and_back_on_lets_it_through") {
        return;
    }
    use wormhole_core::ipc::access::Capability;
    let _window = start_scoped(cx, 1, true, true);
    let peer = outside();
    let mine = open_terminal(cx, &peer);
    assert!(reply(answer(cx, typing(mine), &peer)).ok);
    let listed = cx.update(agent_access::approved);
    assert_eq!(listed.len(), 1);
    let Which::Memory(_) = listed[0].which else { panic!("held in memory") };
    for capability in [Capability::Read, Capability::Type, Capability::Run, Capability::Open] {
        assert!(listed[0].caps.contains(capability), "{capability:?}");
    }
    // Type off: typing is asked about again; reading is not.
    cx.update(|cx| agent_access::set_capability(listed[0].which, Capability::Type, false, cx));
    assert_eq!(reason(&reply(answer(cx, typing(mine), &peer))), Some(Reason::ApprovalNeeded));
    assert!(reply(answer(cx, reading(mine), &peer)).ok);
    assert!(!cx.update(agent_access::approved)[0].caps.contains(Capability::Type));
    cx.update(|cx| agent_access::set_capability(listed[0].which, Capability::Type, true, cx));
    assert!(reply(answer(cx, typing(mine), &peer)).ok);
}

#[gpui_kit::test]
fn the_question_about_a_terminal_is_worded_by_the_catalogs(cx: &mut TestAppContext) {
    if isolate("the_question_about_a_terminal_is_worded_by_the_catalogs") {
        return;
    }
    let _window = start_scoped(cx, 1, true, true);
    let _ = answer(cx, typing(1), &outside());
    let said = |text: &str| text.starts_with('⟦') && text.ends_with('⟧');
    with_language(Language::PSEUDO, || {
        let asked = questions(cx);
        let question = &asked[0];
        assert!(said(&question.title), "{}", question.title);
        for paragraph in question.detail.split("\n\n") {
            assert!(said(paragraph), "{paragraph:?}");
        }
        for line in question.facts.lines() {
            assert!(said(line.split("  ").next().unwrap_or_default()), "{line:?}");
        }
    });
    with_language(Language::ENGLISH, || {
        let asked = questions(cx);
        assert!(asked[0].title.starts_with("Allow scripts to type into “"), "{}", asked[0].title);
        assert!(asked[0].detail.starts_with("Scripts did not open this terminal."), "{}", asked[0].detail);
        assert!(asked[0].facts.starts_with("Terminal"), "{}", asked[0].facts);
    });
}

/// A request for each action that names a terminal, `id` its target.
fn about(name: &str, id: i64) -> Option<Request> {
    use serde_json::json;
    let target = target(id);
    let extra = match name {
        "status" => json!({ "target": target }),
        "notify" => json!({ "target": target, "body": "hello" }),
        "send_text" => json!({ "target": target, "text": "echo hi" }),
        "read" | "blocks" | "close" | "split" | "focus" => json!({ "target": target }),
        "send_keys" => json!({ "target": target, "keys": [{ "key": "enter" }] }),
        "mouse" => json!({ "target": target, "op": { "kind": "move", "row": 0, "col": 0 } }),
        "rename" => json!({ "target": target, "name": "renamed" }),
        "wait" => json!({ "target": target, "status": ["idle"], "timeout_ms": 100 }),
        "run" => json!({ "target": target, "command": "true", "timeout_ms": 100 }),
        _ => return None,
    };
    let mut request = json!({ "version": 1, "action": name });
    request.as_object_mut()?.extend(extra.as_object()?.clone());
    serde_json::from_value(request).ok()
}

/// A hidden terminal and a terminal that does not exist are the same to every action that names one: the same
/// code, the same words (but the number), no reason. Walks every action, as a new one fails the guard in
/// `Need::of` until it is classified, and here until it is compared (or listed as naming no terminal).
fn assert_hidden_is_missing(cx: &mut TestAppContext, peer: &Peer, hidden: i64) {
    const MISSING: i64 = 987_654;
    let naming_none = ["list", "doctor", "new_terminal", "new_window", "events", "welcome"];
    for name in Action::KNOWN {
        let (Some(of_hidden), Some(of_missing)) = (about(name, hidden), about(name, MISSING)) else {
            assert!(
                naming_none.contains(&name),
                "`{name}` is neither compared nor listed as naming no terminal"
            );
            continue;
        };
        let (a, b) = (reply(answer(cx, of_hidden, peer)), reply(answer(cx, of_missing, peer)));
        let (a, b) =
            (a.err().unwrap_or_else(|| panic!("`{name}` found it: {a:?}")), b.err().expect("an error"));
        assert_eq!(a.code, ErrorCode::NotFound, "`{name}`");
        assert_eq!(a.code, b.code, "`{name}`");
        assert_eq!(a.reason, b.reason, "`{name}`");
        assert_eq!(
            a.message.replace(&hidden.to_string(), "N"),
            b.message.replace(&MISSING.to_string(), "N"),
            "`{name}` words a hidden terminal differently from a missing one"
        );
    }
}

#[gpui_kit::test]
fn a_hidden_terminal_is_not_found_exactly_as_a_missing_one_on_every_action_in_every_mode(
    cx: &mut TestAppContext,
) {
    if isolate("a_hidden_terminal_is_not_found_exactly_as_a_missing_one_on_every_action_in_every_mode") {
        return;
    }
    let window = start(cx, 2);
    testing::allow(cx, true, true);
    drive(cx, &window, |ws, _, cx| ws.set_agent_hidden(1, true, cx));
    // Ask mode with everything allowed, and Allowed while open: neither lets anybody see it.
    assert_hidden_is_missing(cx, &outside(), 2);
    cx.update(|cx| update_settings(cx, |s| s.agent_access = AgentAccess::Open));
    cx.update(|cx| agent_access::tick(0, cx));
    assert_hidden_is_missing(cx, &outside(), 2);
    // Shown again, it is found.
    drive(cx, &window, |ws, _, cx| ws.set_agent_hidden(1, false, cx));
    let found = reply(answer(cx, about("status", 2).expect("a status"), &outside()));
    assert!(found.ok, "{found:?}");
}

#[gpui_kit::test]
fn list_and_status_leave_hidden_terminals_out_and_a_program_inside_one_still_reads_itself(
    cx: &mut TestAppContext,
) {
    if isolate("list_and_status_leave_hidden_terminals_out_and_a_program_inside_one_still_reads_itself") {
        return;
    }
    let window = start(cx, 3);
    // Read is held: without it the gate asks before it looks a terminal up, and a missing id and a hidden
    // one are refused alike (not allowed), which leaks nothing but is not what this is about.
    testing::allow(cx, true, false);
    drive(cx, &window, |ws, _, cx| ws.set_agent_hidden(1, true, cx));
    let ids = |reply: &Reply| -> Vec<i64> {
        reply.result.as_ref().expect("a result")["windows"][0]["terminals"]
            .as_array()
            .expect("terminals")
            .iter()
            .filter_map(|terminal| terminal["id"].as_i64())
            .collect()
    };
    let listed =
        reply(answer(cx, request(serde_json::json!({ "version": 1, "action": "list" })), &outside()));
    assert_eq!(ids(&listed), [1, 3], "terminal 2 is hidden");
    let summary =
        reply(answer(cx, request(serde_json::json!({ "version": 1, "action": "status" })), &outside()));
    assert!(summary.ok, "{summary:?}");
    assert_eq!(summary.result.as_ref().expect("a summary")["terminals"], 2, "{summary:?}");
    // A program that runs inside the hidden terminal reads itself (a read of its own terminal needs no
    // approval), and outside it the same read finds nothing.
    let shell = terminal(cx, &window, 1).read_with(cx, |t, _| t.shell_pid()).expect("a shell runs");
    let inside = Peer::new(900_001, vec![shell]);
    let own = reply(answer(cx, reading(2), &inside));
    assert!(own.ok, "{own:?}");
    assert_eq!(reply(answer(cx, reading(2), &outside())).err().map(|e| e.code), Some(ErrorCode::NotFound));
}

#[gpui_kit::test]
fn a_hidden_terminal_comes_back_hidden_and_is_part_of_what_is_saved(cx: &mut TestAppContext) {
    if isolate("a_hidden_terminal_comes_back_hidden_and_is_part_of_what_is_saved") {
        return;
    }
    let mut hidden = SessionRecord::new(2, None, None);
    hidden.agent_hidden = true;
    let record = WindowRecord {
        id: 1,
        sessions: vec![SessionRecord::new(1, None, None), hidden],
        ..Default::default()
    };
    let window = start_with(cx, record);
    testing::allow(cx, true, true);
    // It was restored hidden: nobody outside finds it...
    assert_eq!(reply(answer(cx, reading(2), &outside())).err().map(|e| e.code), Some(ErrorCode::NotFound));
    // ...the window says so in the facts it draws from, and saves it again.
    read(cx, &window, |ws| {
        assert_eq!(
            ws.sessions.iter().map(|s| s.pane.summary.agent_hidden).collect::<Vec<_>>(),
            [false, true]
        );
    });
    let saved =
        cx.update(|cx| window.workspace.read_with(cx, |ws, cx| ws.sessions[1].record(cx)).expect("open"));
    assert!(saved.agent_hidden);
    let shown =
        cx.update(|cx| window.workspace.read_with(cx, |ws, cx| ws.sessions[0].record(cx)).expect("open"));
    assert!(!shown.agent_hidden);
}

#[gpui_kit::test]
fn a_new_terminal_does_not_start_in_the_folder_of_a_hidden_one_in_front(cx: &mut TestAppContext) {
    if isolate("a_new_terminal_does_not_start_in_the_folder_of_a_hidden_one_in_front") {
        return;
    }
    let window = start(cx, 2);
    testing::allow(cx, true, true);
    let secret = std::env::temp_dir().join("hidden-terminal-folder");
    drive(cx, &window, |ws, _, cx| {
        let active = ws.active;
        ws.sessions[active].terminal().update(cx, |terminal, _| terminal.cwd = Some(secret.clone()));
        ws.set_agent_hidden(active, true, cx);
    });
    let opened =
        reply(answer(cx, request(serde_json::json!({ "version": 1, "action": "new_terminal" })), &outside()));
    assert!(opened.ok, "{opened:?}");
    let folder = opened.result.as_ref().and_then(|result| result["folder"].as_str());
    assert!(
        folder.is_none_or(|folder| !folder.contains("hidden-terminal-folder")),
        "the hidden terminal's folder was told: {folder:?}"
    );
}
