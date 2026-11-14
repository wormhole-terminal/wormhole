//! The gate: the mode in Settings decides, and in "ask" mode the person does, through the desk.

use serde_json::json;

use wormhole_core::config::AgentAccess;

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

/// Rewrites the profile's `config.json` as an editor would: nothing in Wormhole made the change.
fn edit_mode_outside(f: &super::support::Fixture, mode: &str) {
    edit_file_outside(f, mode, true);
}

/// [`edit_mode_outside`], with the Automation flag as `automation` says.
fn edit_file_outside(f: &super::support::Fixture, mode: &str, automation: bool) {
    let file = f.desk.profile().config_file();
    let config =
        json!({ "agent_access": mode, "control_write": true, "feature_flags": { "automation": automation } });
    std::fs::write(file, config.to_string()).expect("config.json");
    f.server.settings_changed(f.desk.profile().id().as_str().to_string());
}

/// Sets the mode as the Settings page does, through the profile's settings. The server is not told: whatever looks
/// next finds the person's change.
fn set_mode_in_settings(f: &super::support::Fixture, mode: AgentAccess) {
    let mut settings = crate::settings::State::load(f.desk.profile());
    assert!(settings.update(|s| s.agent_access = mode), "the mode changed in Settings");
}

fn mode_in_file(f: &super::support::Fixture) -> String {
    let text = std::fs::read_to_string(f.desk.profile().config_file()).expect("config.json");
    serde_json::from_str::<serde_json::Value>(&text).expect("json")["agent_access"]
        .as_str()
        .unwrap_or("")
        .to_string()
}

fn type_into(f: &super::support::Fixture, id: i64) -> serde_json::Value {
    f.client().control(json!({"action": "send_text", "target": {"kind": "terminal", "id": id}, "text": "x"}))
}

#[test]
fn a_mode_widened_outside_wormhole_is_held_back_until_the_person_keeps_it() {
    let f = fixture("off");
    let id = f.terminal();
    assert_eq!(
        type_into(&f, id)["error"]["reason"],
        "agent_access_off",
        "the first look is the mode in force"
    );
    for (from, to, held) in [("off", "open", "agent_access_off"), ("off", "ask", "agent_access_off")] {
        edit_mode_outside(&f, to);
        assert_eq!(type_into(&f, id)["error"]["reason"], held, "{from} to {to} is not in force yet");
        let question = f.desk.next_mode_question().expect("the question about the change");
        assert!(f.desk.is_waiting_mode(question.token));
        assert_ne!(question.previous, question.requested);
        assert!(f.desk.next_mode_question().is_none(), "asked once");
        if to == "open" {
            // Not kept: the earlier mode is back in the file, and in force.
            f.desk.answer_mode(question.token, false);
            assert_eq!(mode_in_file(&f), "off");
            assert_eq!(type_into(&f, id)["error"]["reason"], "agent_access_off");
        } else {
            f.desk.answer_mode(question.token, true);
            assert_eq!(mode_in_file(&f), "ask");
            assert_eq!(type_into(&f, id)["error"]["reason"], "approval_needed", "kept: Ask is in force");
        }
        assert!(!f.desk.is_waiting_mode(question.token));
    }
}

#[test]
fn narrowing_and_an_edit_made_in_settings_apply_at_once_and_ask_nothing() {
    let f = fixture("ask");
    let id = f.terminal();
    assert_eq!(type_into(&f, id)["error"]["reason"], "approval_needed");
    edit_mode_outside(&f, "off");
    assert_eq!(type_into(&f, id)["error"]["reason"], "agent_access_off", "narrowing applies at once");
    assert!(f.desk.next_mode_question().is_none());
    // Settings tells the mode it sets: what it writes is the person's.
    set_mode_in_settings(&f, AgentAccess::Open);
    assert_eq!(type_into(&f, id)["ok"], true, "widened in Settings: in force at once");
    assert!(f.desk.next_mode_question().is_none());
}

/// Order: the person changes the mode in Settings (narrowing Open to Ask, or widening Off to Ask); before anything
/// has looked, a program writes Open into the file. That the person changed the mode says nothing for the program's
/// value: theirs is in force, and Open is asked about.
#[test]
fn a_program_writing_after_the_persons_own_change_does_not_get_its_mode_in_force() {
    for from in ["open", "off"] {
        let f = fixture(from);
        let id = f.terminal();
        set_mode_in_settings(&f, AgentAccess::Ask);
        edit_mode_outside(&f, "open");
        assert_eq!(
            type_into(&f, id)["error"]["reason"],
            "approval_needed",
            "the person's Ask is in force, from {from}"
        );
        let question = f.desk.next_mode_question().expect("the person is asked about Open");
        assert!(f.desk.is_waiting_mode(question.token));
    }
}

/// Order: the Automation flag is off; the person changes the mode in Settings, with no request to look at it; later
/// a program writes Open and turns the flag on in one write. The change made meanwhile is no credit for the program.
#[test]
fn a_change_made_while_automation_is_off_lets_no_later_widening_through() {
    let f = fixture("off");
    let id = f.terminal();
    edit_file_outside(&f, "off", false);
    set_mode_in_settings(&f, AgentAccess::Ask);
    f.server.settings_changed(f.desk.profile().id().as_str().to_string());
    edit_file_outside(&f, "open", true);
    assert_eq!(type_into(&f, id)["error"]["reason"], "approval_needed", "the person's Ask is in force");
    assert!(f.desk.next_mode_question().is_some(), "the person is asked about Open");
}

/// Order: the Automation flag is off; the person sets Open in Settings and then narrows the file to Ask by hand;
/// later a program writes Open back and turns the flag on. What they set in Settings before does not pass it.
#[test]
fn a_mode_narrowed_by_hand_while_automation_is_off_is_not_widened_back_unasked() {
    let f = fixture("off");
    let id = f.terminal();
    edit_file_outside(&f, "off", false);
    set_mode_in_settings(&f, AgentAccess::Open);
    f.server.settings_changed(f.desk.profile().id().as_str().to_string());
    edit_file_outside(&f, "ask", false);
    edit_file_outside(&f, "open", true);
    assert_eq!(type_into(&f, id)["error"]["reason"], "approval_needed", "Ask, narrowed by hand, is in force");
    assert!(f.desk.next_mode_question().is_some(), "the person is asked about Open");
}

/// Order: the Settings window has read the widened file; the person says Don't Keep; before that window has read
/// the file again they change another setting in it. Both stand: the earlier mode and the other setting.
#[test]
fn dont_keep_goes_through_the_settings_the_app_holds() {
    let f = fixture("off");
    let root = wormhole_core::storage::paths::DataRoot::new(&f.dir);
    let profile = wormhole_core::storage::paths::ProfileId::default_profile();
    let handle = crate::settings::SettingsHandle::open_at(&root, &profile, f.wakes.clone());
    edit_mode_outside(&f, "open");
    eventually("Settings read the widened file", || {
        let mut state = handle.shared.lock();
        state.poll();
        state.effective().agent_access == AgentAccess::Open
    });
    let change = f.desk.next_mode_question().expect("the question about the change");
    f.desk.answer_mode(change.token, false);
    handle.set("diff_wrap".into(), "true".into()).expect("sets");
    assert_eq!(mode_in_file(&f), "off", "the next write of Settings kept the earlier mode");
    assert!(handle.shared.lock().effective().diff_wrap, "and the other setting was written");
    assert_eq!(type_into(&f, f.terminal())["error"]["reason"], "agent_access_off");
    assert!(f.desk.next_mode_question().is_none());
}

/// The person changes another setting in Settings; later something outside writes Open into `config.json`. The
/// widening is not theirs: the earlier mode stays in force and the person is asked.
#[test]
fn an_edit_of_another_setting_does_not_let_a_widening_from_outside_through() {
    let f = fixture("off");
    let id = f.terminal();
    assert_eq!(type_into(&f, id)["error"]["reason"], "agent_access_off", "the first look is the mode in force");
    let mut settings = crate::settings::State::load(f.desk.profile());
    assert!(settings.update(|s| s.font_size += 1.0), "the font size changed in Settings");
    edit_mode_outside(&f, "open");
    assert_eq!(type_into(&f, id)["error"]["reason"], "agent_access_off", "Off is still in force");
    let question = f.desk.next_mode_question().expect("the person is asked about the widening");
    assert!(f.desk.is_waiting_mode(question.token));
}

/// The mode widened in Settings itself is the person's: in force at once, nothing asked.
#[test]
fn a_widening_made_in_settings_applies_at_once() {
    let f = fixture("off");
    let id = f.terminal();
    assert_eq!(type_into(&f, id)["error"]["reason"], "agent_access_off");
    let mut settings = crate::settings::State::load(f.desk.profile());
    assert!(settings.update(|s| s.agent_access = wormhole_core::config::AgentAccess::Open));
    f.server.settings_changed(f.desk.profile().id().as_str().to_string());
    assert_eq!(type_into(&f, id)["ok"], true, "Open, set in Settings, is in force");
    assert!(f.desk.next_mode_question().is_none());
}

#[test]
fn a_question_nobody_answers_is_a_no_and_the_sheet_goes() {
    let f = fixture("ask");
    let id = f.terminal();
    f.desk.wait_only(std::time::Duration::from_millis(150));
    let mut client = f.client();
    client.send(&json!({
        "kind": "control", "version": 1, "ask": true,
        "action": "send_text", "target": {"kind": "terminal", "id": id}, "text": "x"
    }));
    eventually("the question", || f.desk.waiting() == 1);
    let question = f.desk.next_question().expect("a question");
    assert!(f.desk.is_waiting(question.id));
    eventually("the question timed out", || !f.desk.is_waiting(question.id));
    let reply = client.line().expect("the answer");
    assert_eq!(reply["error"]["reason"], "ask_timed_out", "{reply}");
    assert_eq!(f.desk.waiting(), 0, "not remembered, and nothing waits");

    // The same for the mode: not answered is not kept.
    edit_mode_outside(&f, "off");
    edit_mode_outside(&f, "open");
    let change = f.desk.next_mode_question().expect("the question about the change");
    eventually("the change timed out", || !f.desk.is_waiting_mode(change.token));
    assert_eq!(mode_in_file(&f), "off");
}
