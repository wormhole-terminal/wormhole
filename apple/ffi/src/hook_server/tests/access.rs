//! The gate: the mode in Settings decides, and in "ask" mode the person does, through the desk.

use serde_json::json;

use super::support::{eventually, fixture};
use crate::AccessAnswer;

#[test]
fn off_refuses_what_needs_approval_but_still_lists() {
    let f = fixture("off");
    let id = f.terminal();
    let mut client = f.client();
    let typed =
        client.control(json!({"action": "send_text", "target": {"kind": "terminal", "id": id}, "text": "x"}));
    assert_eq!(typed["error"]["code"], "not_allowed");
    assert_eq!(typed["error"]["reason"], "agent_access_off");
    assert_eq!(client.control(json!({"action": "list"}))["ok"], true, "titles and states need no approval");
    assert_eq!(f.desk.waiting(), 0, "nobody was asked");
}

#[test]
fn in_ask_mode_a_request_that_cannot_wait_is_refused_and_the_question_stays() {
    let f = fixture("ask");
    let id = f.terminal();
    let mut client = f.client();
    let typed =
        client.control(json!({"action": "send_text", "target": {"kind": "terminal", "id": id}, "text": "x"}));
    assert_eq!(typed["error"]["code"], "not_allowed");
    assert_eq!(typed["error"]["reason"], "approval_needed");
    let question = f.desk.next_question().expect("the question is waiting for the person");
    assert_eq!(question.about_terminal, None, "it is about the program, not a terminal");
    // The person says no: the next try is cooled down, not asked again.
    f.desk.answer(question.id, AccessAnswer::Deny);
    let again =
        client.control(json!({"action": "send_text", "target": {"kind": "terminal", "id": id}, "text": "x"}));
    assert_eq!(again["error"]["reason"], "declined_recently");
}

#[test]
fn a_request_that_waits_is_let_in_when_the_person_allows_it() {
    let f = fixture("ask");
    let id = f.terminal();
    let mut client = f.client();
    client.send(&json!({
        "kind": "control", "version": 1, "ask": true,
        "action": "send_text", "target": {"kind": "terminal", "id": id}, "text": "allowed words", "submit": true
    }));
    // The request holds its connection while the question is on the person's screen: first whether the
    // program may control Wormhole at all, then, because it opened nothing here, whether it may use this terminal.
    eventually("the question about the program", || f.desk.waiting() == 1);
    let program = f.desk.next_question().expect("a question");
    assert_eq!(program.about_terminal, None);
    f.desk.answer(program.id, AccessAnswer::UntilQuit);
    eventually("the question about the terminal", || f.desk.waiting() == 1);
    let terminal_question = f.desk.next_question().expect("a question");
    assert_eq!(terminal_question.about_terminal, Some(id));
    f.desk.answer(terminal_question.id, AccessAnswer::UntilQuit);
    let reply = client.line().expect("the answer");
    assert_eq!(reply["ok"], true, "{reply}");
    let terminal = f.workspace.terminal(id).expect("registered");
    eventually("the typed text", || terminal.screen_text().contains("allowed words"));
    // What was allowed holds for the next request without asking, and a terminal the program opened is its own.
    // (The connection that waited is the answer's; the next request is a new one.)
    let mut client = f.client();
    let next = client.control(
        json!({"action": "send_keys", "target": {"kind": "terminal", "id": id}, "keys": [{"key": "enter"}]}),
    );
    assert_eq!(next["ok"], true, "{next}");
    assert_eq!(f.desk.waiting(), 0);
    let opened = client.control(json!({"action": "new_terminal"}));
    assert_eq!(opened["ok"], true, "{opened}");
    let own = opened["result"]["terminal"].as_i64().expect("an id");
    f.start(own);
    let typed = client
        .control(json!({"action": "send_text", "target": {"kind": "terminal", "id": own}, "text": "mine"}));
    assert_eq!(typed["ok"], true, "what a program opened is its own: {typed}");
    assert_eq!(f.desk.waiting(), 0, "nothing more was asked");
}
