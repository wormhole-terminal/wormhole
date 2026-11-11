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
    // As the window tells the model its area (the room a split must leave).
    f.workspace.apply(Action::SetPaneArea { width: 610.0, height: 1100.0 });
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
    // To the left and above join the target's pane on that side, in the target's folder, as the GPUI app does.
    let left = client.control(
        json!({"action": "split", "target": {"kind": "terminal", "id": anchor}, "direction": "left"}),
    );
    let before = ok(&left)["terminal"].as_i64().expect("an id");
    let up = client
        .control(json!({"action": "split", "target": {"kind": "terminal", "id": anchor}, "direction": "up"}));
    let above = ok(&up)["terminal"].as_i64().expect("an id");
    assert_eq!(f.workspace.selected(), Some(terminal), "no split takes the selection unless it is asked to");
    let listing = client.control(json!({"action": "list"}));
    let all = ok(&listing)["windows"][0]["splits"][0]["terminals"].as_array().expect("terminals").len();
    assert_eq!(all, 4, "the anchor, the one below, the one on its left ({before}) and above that ({above})");
    // Where they are: the left one is left of its target, the one above is over the one it was split from.
    f.workspace.apply(Action::Select(anchor));
    let panes = f.workspace.panes(610.0, 1000.0).panes;
    let at = |id: i64| panes.iter().find(|p| p.id == id).map(|p| (p.x, p.y)).expect("a pane");
    assert!(at(before).0 < at(anchor).0, "left of the anchor: {panes:?}");
    assert!(
        at(above).1 < at(anchor).1 && (at(above).0 - at(anchor).0).abs() < 1.0,
        "above its target: {panes:?}"
    );
}

#[test]
fn a_split_of_the_only_terminal_to_its_left_and_above_puts_the_new_ones_there() {
    let f = fixture("open");
    let anchor = f.terminal();
    f.workspace.apply(Action::SetPaneArea { width: 610.0, height: 1100.0 });
    let mut client = f.client();
    let target = json!({"kind": "terminal", "id": anchor});
    let left =
        ok(&client.control(json!({"action": "split", "target": target, "direction": "left"})))["terminal"]
            .as_i64()
            .expect("an id");
    let up = ok(&client.control(json!({"action": "split", "target": target, "direction": "up"})))["terminal"]
        .as_i64()
        .expect("an id");
    f.workspace.apply(Action::Select(anchor));
    let panes = f.workspace.panes(610.0, 1100.0).panes;
    let at = |id: i64| panes.iter().find(|p| p.id == id).map(|p| (p.x, p.y)).expect("a pane");
    assert!(at(left).0 < at(anchor).0, "left of the anchor: {panes:?}");
    assert!(at(up).1 < at(anchor).1, "above the anchor: {panes:?}");
}

#[test]
fn a_worktree_is_asked_for_in_the_window_and_never_made_by_the_request() {
    let f = fixture("open");
    f.terminal();
    let mut client = f.client();
    f.events();
    // A throwaway repository: `git init` under the test's own folder, with the config scrubbed.
    let repo = f.dir.join("repo");
    std::fs::create_dir_all(&repo).expect("a folder");
    let init = std::process::Command::new("git")
        .args(["init", "--quiet"])
        .arg(&repo)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .status()
        .expect("git runs");
    assert!(init.success());
    let asked = client.control(
        json!({"action": "new_terminal", "folder": repo.display().to_string(), "worktree": "fix/login"}),
    );
    assert_eq!(ok(&asked)["asked"], true, "{asked}");
    assert_eq!(ok(&asked)["branch"], "fix/login");
    assert!(
        f.events()
            .iter()
            .any(|e| matches!(e, HookEvent::WorktreeAsked { branch, .. } if branch == "fix/login")),
        "Swift is asked to put the question up"
    );
    assert!(!f.dir.join("repo-fix-login").exists(), "nothing was made");
    // A folder in no repository, and a branch git refuses, are answered as the GPUI app answers them.
    let outside = client.control(
        json!({"action": "new_terminal", "folder": f.dir.display().to_string(), "worktree": "fix/login"}),
    );
    assert_eq!(code(&outside), "not_found");
    let bad = client.control(
        json!({"action": "new_terminal", "folder": repo.display().to_string(), "worktree": "bad name"}),
    );
    assert_eq!(code(&bad), "bad_request");
}

#[test]
fn a_status_lines_gauge_reaches_swift_once_per_change() {
    let f = fixture("open");
    let id = f.terminal();
    let mut client = f.client();
    f.events();
    let gauge = |context: f64| {
        json!({"kind": "gauge", "tab": id.to_string(),
               "gauge": {"agent": "claude", "model": "Opus 4.5", "context": context}})
    };
    client.send(&gauge(41.0));
    client.send(&gauge(41.0));
    client.send(&gauge(42.0));
    // A status line repeats itself: the repeat is not told, and what was not drained yet is replaced.
    let mut told = Vec::new();
    eventually("the last gauge", || {
        told.extend(f.events().into_iter().filter_map(|event| match event {
            HookEvent::Gauge { terminal, agent, model, context } if terminal == id => {
                Some((agent, model, context))
            }
            _ => None,
        }));
        told.last().is_some_and(|(_, _, context)| *context == Some(42.0))
    });
    assert!(told.iter().filter(|(_, _, context)| *context == Some(41.0)).count() <= 1, "{told:?}");
    assert_eq!(
        told.last().map(|(agent, model, _)| (agent.as_str(), model.as_deref())),
        Some(("claude", Some("Opus 4.5")))
    );
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
    // Kept as the terminal's latest notification, for the notification rules to tell; no banner straight from here.
    let notice = f.workspace.terminals().terminals.iter().find(|t| t.id == first).map(|t| t.notice);
    assert_eq!(notice, Some(1));
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

#[test]
fn what_the_system_opens_gets_a_terminal_a_script_running_in_its_folder() {
    let f = fixture("open");
    let _first = f.terminal();
    let scripts = f.dir.join("My Scripts");
    std::fs::create_dir_all(&scripts).expect("a folder");
    let script = scripts.join("it's here.command");
    std::fs::write(&script, "echo hi\n").expect("a script");
    let url = |path: &std::path::Path| format!("file://{}", path.display().to_string().replace(' ', "%20"));
    let urls = vec![
        url(&scripts),
        url(&script),
        "https://example.com/x".to_string(),
        url(&f.dir.join("gone.command")),
    ];
    // The folder and the script open a terminal each; a web address and a file that is gone are refused.
    assert_eq!(f.server.open_from_system(urls, Some(1)), 2);
    let events = f.workspace.drain();
    let folders: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            WorkspaceEvent::Opened { cwd, .. } => cwd.clone(),
            _ => None,
        })
        .collect();
    assert_eq!(folders, [scripts.display().to_string(), scripts.display().to_string()]);
    // A control line's budget is not spent: a person's double-click is not a script's request.
    let many = vec![url(&scripts); 8];
    assert_eq!(f.server.open_from_system(many, None), 8);
}
