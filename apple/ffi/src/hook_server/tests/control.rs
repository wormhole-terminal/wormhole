//! The control verbs over a real socket, the workspace and `/bin/cat` terminals, in the mode "Allowed while
//! open": the replies are core's own, so these check what the Swift app's side decides and carries out.

use serde_json::{Value, json};
use wormhole_core::ipc::control::SEND_TEXT_MAX_BYTES;
use wormhole_core::workspace::Action;

use super::support::{eventually, fixture};
use crate::WorkspaceEvent;
use crate::hook_server::HookEvent;

fn ok(reply: &Value) -> &Value {
    assert_eq!(reply["ok"], true, "{reply}");
    &reply["result"]
}

fn code(reply: &Value) -> &str {
    assert_eq!(reply["ok"], false, "{reply}");
    reply["error"]["code"].as_str().expect("a code")
}

#[test]
fn list_and_status_tell_the_terminals_as_the_sidebar_speaks_of_them() {
    let f = fixture("open");
    let (first, second) = (f.terminal(), f.terminal());
    let mut client = f.client();
    let listing = client.control(json!({"action": "list"}));
    let window = &ok(&listing)["windows"][0];
    assert_eq!(window["front"], true);
    let ids: Vec<i64> = window["terminals"]
        .as_array()
        .expect("terminals")
        .iter()
        .map(|t| t["id"].as_i64().expect("id"))
        .collect();
    assert_eq!(ids, [first, second]);
    assert_eq!(window["terminals"][1]["selected"], true, "the last one made is selected");
    assert_eq!(window["terminals"][0]["status"], "idle");

    let status = client.control(json!({"action": "status", "target": {"kind": "terminal", "id": first}}));
    assert_eq!(ok(&status)["terminal"]["id"], first);
    assert_eq!(ok(&status)["tab"], first);
    let summary = client.control(json!({"action": "status"}));
    assert_eq!((ok(&summary)["windows"].as_i64(), ok(&summary)["terminals"].as_i64()), (Some(1), Some(2)));
    assert_eq!(
        code(&client.control(json!({"action": "status", "target": {"kind": "terminal", "id": 99}}))),
        "not_found"
    );
}

#[test]
fn send_text_types_into_the_terminal_and_send_keys_presses_keys() {
    let f = fixture("open");
    let id = f.terminal();
    let mut client = f.client();
    let target = json!({"kind": "terminal", "id": id});
    let reply = client
        .control(json!({"action": "send_text", "target": target, "text": "hello wormhole", "submit": true}));
    assert_eq!(ok(&reply)["tab"], id);
    let terminal = f.workspace.terminal(id).expect("registered");
    eventually("the echo", || terminal.screen_text().matches("hello wormhole").count() >= 2);
    let keys = client.control(
        json!({"action": "send_keys", "target": target, "keys": [{"text": "abc"}, {"key": "enter"}]}),
    );
    assert_eq!(ok(&keys)["keys"], 2);
    eventually("the keys' echo", || terminal.screen_text().contains("abc"));
    // Too much text is refused by core's cap, a missing terminal is not found.
    let huge = "x".repeat(SEND_TEXT_MAX_BYTES + 1);
    assert_eq!(
        code(&client.control(json!({"action": "send_text", "target": target, "text": huge}))),
        "too_large"
    );
    assert_eq!(
        code(
            &client.control(json!({"action": "send_text", "target": {"kind": "tab", "id": 99}, "text": "x"}))
        ),
        "not_found"
    );
}

#[test]
fn read_gives_the_screen_text() {
    let f = fixture("open");
    let id = f.terminal();
    let mut client = f.client();
    let target = json!({"kind": "terminal", "id": id});
    client.control(json!({"action": "send_text", "target": target, "text": "readable line", "submit": true}));
    let terminal = f.workspace.terminal(id).expect("registered");
    eventually("the echo", || terminal.screen_text().contains("readable line"));
    let read = client.control(json!({"action": "read", "target": target}));
    assert!(ok(&read).to_string().contains("readable line"), "{read}");
}

#[test]
fn new_terminal_opens_a_row_for_swift_to_start_and_a_split_joins_it() {
    let f = fixture("open");
    let anchor = f.terminal();
    let mut client = f.client();
    let folder = f.dir.display().to_string();
    let made = client.control(json!({"action": "new_terminal", "folder": folder}));
    let terminal = ok(&made)["terminal"].as_i64().expect("an id");
    assert_eq!(
        (ok(&made)["window"].as_i64(), ok(&made)["folder"].as_str()),
        (Some(1), Some(folder.as_str()))
    );
    // Swift hears of it as of any terminal the person opens, with the folder asked for.
    eventually("the opened terminal", || {
        f.workspace.drain().contains(&WorkspaceEvent::Opened { id: terminal, cwd: Some(folder.clone()) })
    });
    f.start(terminal);

    let split = client.control(
        json!({"action": "split", "target": {"kind": "terminal", "id": anchor}, "direction": "down"}),
    );
    let beside = ok(&split)["terminal"].as_i64().expect("an id");
    assert!(ok(&split)["split"].as_i64().is_some());
    let listing = client.control(json!({"action": "list"}));
    let splits = &ok(&listing)["windows"][0]["splits"][0];
    assert_eq!(splits["terminals"], json!([anchor, beside]));
    assert_eq!(f.workspace.selected(), Some(terminal), "a split leaves the selection where it was");
    // To the left is not made by this app, and says so.
    let left = client.control(
        json!({"action": "split", "target": {"kind": "terminal", "id": anchor}, "direction": "left"}),
    );
    assert_eq!(code(&left), "unsupported");
}

#[test]
fn focus_rename_close_notify_and_new_window() {
    let f = fixture("open");
    let (first, second) = (f.terminal(), f.terminal());
    let mut client = f.client();
    f.events();
    let first_target = json!({"kind": "terminal", "id": first});
    ok(&client.control(json!({"action": "focus", "target": first_target, "activate": true})));
    assert_eq!(f.workspace.selected(), Some(first));
    assert!(f.events().contains(&HookEvent::Activate));

    let renamed = client.control(json!({"action": "rename", "target": first_target, "name": "builder"}));
    assert_eq!(
        (ok(&renamed)["name"].as_str(), ok(&renamed)["custom"].as_bool()),
        (Some("builder"), Some(true))
    );
    let back = client.control(json!({"action": "rename", "target": first_target}));
    assert_eq!(ok(&back)["custom"], false);

    // The close question goes to the person, one at a time, and nothing closes by itself.
    let asked = client.control(json!({"action": "close", "target": {"kind": "terminal", "id": second}}));
    assert_eq!(ok(&asked)["asked"], true);
    assert_eq!(f.events(), [HookEvent::CloseAsked { terminal: second }]);
    let again = client.control(json!({"action": "close", "target": first_target}));
    assert_eq!(code(&again), "refused");
    assert_eq!(again["error"]["reason"], "question_open");
    assert!(f.workspace.terminals().terminals.iter().any(|t| t.id == second));
    f.server.close_question_over();
    assert_eq!(ok(&client.control(json!({"action": "close", "target": first_target})))["asked"], true);

    let told = client
        .control(json!({"action": "notify", "target": first_target, "title": "Build", "body": "done\nnow"}));
    assert_eq!(ok(&told)["tab"], first);
    assert!(f.events().iter().any(
        |e| matches!(e, HookEvent::Notice { terminal, title, .. } if *terminal == first && title == "Build")
    ));
    assert_eq!(code(&client.control(json!({"action": "new_window"}))), "unsupported");
    // The person closing it by hand is the model's own business and the registry follows.
    f.workspace.apply(Action::Close(second));
    assert!(f.workspace.terminal(second).is_none());
}

#[test]
fn doctor_says_where_the_caller_stands() {
    let f = fixture("open");
    let id = f.terminal();
    let mut client = f.client();
    let doctor = client.control(json!({"action": "doctor", "from": id.to_string()}));
    let facts = ok(&doctor);
    assert_eq!((facts["automation"].as_bool(), facts["protocol"].as_i64()), (Some(true), Some(1)));
    assert_eq!(facts["access"]["mode"], "open");
    assert_eq!(facts["access"]["standing"], "open");
    assert_eq!(facts["caller"]["terminal"], id);
    assert_eq!(facts["caller"]["verified"], false, "said, not found by the process tree");
}
