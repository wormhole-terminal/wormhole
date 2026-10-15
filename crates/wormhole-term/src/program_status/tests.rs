//! The spec's own examples, from the bytes of the sequence to what the terminal holds.

use super::{Kind, Parsed, ProgramStatus, QUERY_REPLY, State, parse};

/// Send one body (`OSC 7501 ; body ESC \`) to the store the way the terminal does; `false` when
/// the terminal ignored it.
fn send(store: &mut ProgramStatus, body: &str) -> bool {
    match parse(body.as_bytes(), 2) {
        Some(Parsed::Report(report)) => {
            store.apply(report);
            true
        }
        _ => false,
    }
}

fn shown(store: &ProgramStatus) -> (State, Option<String>, Option<String>) {
    let summary = store.summary().expect("something to show");
    (summary.state, summary.app.clone(), summary.message())
}

#[test]
fn one_program_working_blocked_working_and_done() {
    let mut store = ProgramStatus::default();
    let words = |text: &str| Some(text.to_string());
    let app = Some("brew".to_string());

    assert!(send(&mut store, "state=working:app=brew:msg=SW5zdGFsbGluZyB1cGRhdGVz"));
    assert_eq!(shown(&store), (State::Working, app.clone(), words("Installing updates")));

    assert!(send(
        &mut store,
        "state=blocked:kind=auth:app=brew:msg=UGFzc3dvcmQgcmVxdWlyZWQgdG8gaW5zdGFsbCB1cGRhdGVz"
    ));
    let summary = store.summary().expect("blocked");
    assert_eq!((summary.state, summary.kind), (State::Blocked, Some(Kind::Auth)));
    assert_eq!(summary.message().as_deref(), Some("Password required to install updates"));

    assert!(send(&mut store, "state=working:app=brew:msg=SW5zdGFsbGluZyB1cGRhdGVz"));
    let summary = store.summary().expect("working");
    assert_eq!((summary.state, summary.kind), (State::Working, None), "the kind went with the record");

    assert!(send(&mut store, "state=done:app=brew:msg=VXBncmFkZWQgMTIgcGFja2FnZXM="));
    assert_eq!(shown(&store), (State::Done, app.clone(), words("Upgraded 12 packages")));

    // The done record survives the shell prompt that follows (and the program's exit), until the
    // user has looked.
    store.on_process_exit();
    store.on_prompt();
    assert_eq!(shown(&store), (State::Done, app, words("Upgraded 12 packages")));
    store.acknowledge();
    assert!(store.is_empty());
}

#[test]
fn several_records_with_clears_and_the_apps_they_inherit() {
    let mut store = ProgramStatus::default();
    let sequence = [
        "state=working:app=deploy:msg=RGVwbG95aW5nIHYyLjQuMQ==",
        "state=working:id=us-east:title=VVMgRWFzdA==:progress=40:msg=UHVzaGluZyBpbWFnZQ==",
        "state=blocked:kind=permission:id=eu-west:title=RVUgV2VzdA==:msg=QXBwcm92ZSBkZXBsb3kgdG8gZXUtd2VzdCAocHJvZHVjdGlvbik/",
    ];
    for body in sequence {
        assert!(send(&mut store, body), "{body}");
    }
    assert_eq!(store.len(), 3);
    // eu-west is blocked while the root still says working; both are true, and the one that needs
    // the user is shown. It takes app=deploy from the root.
    let summary = store.summary().expect("something");
    assert_eq!((summary.state, summary.kind), (State::Blocked, Some(Kind::Permission)));
    assert_eq!(summary.app.as_deref(), Some("deploy"));
    assert_eq!(summary.label().as_deref(), Some("EU West"));
    assert_eq!(summary.message().as_deref(), Some("Approve deploy to eu-west (production)?"));

    assert!(send(&mut store, "state=done:id=us-east:title=VVMgRWFzdA==:msg=SGVhbHRoeQ=="));
    assert!(send(&mut store, "state=clear:id=us-east"));
    assert_eq!(store.len(), 2);
    assert!(send(&mut store, "state=clear:id=eu-west"));
    assert_eq!(store.len(), 1);
    assert_eq!(shown(&store).0, State::Working, "only the root is left");

    assert!(send(&mut store, "state=done:app=deploy:msg=RGVwbG95ZWQgdG8gMyByZWdpb25z"));
    assert_eq!(shown(&store), (State::Done, Some("deploy".into()), Some("Deployed to 3 regions".into())));
}

#[test]
fn a_blocked_terraform_apply() {
    // The base64 of this message holds a `/`, which the value set allows.
    let mut store = ProgramStatus::default();
    assert!(send(
        &mut store,
        "state=blocked:kind=permission:app=terraform:msg=QXBwbHkgMyB0byBhZGQsIDEgdG8gY2hhbmdlLCAwIHRvIGRlc3Ryb3k/"
    ));
    let summary = store.summary().expect("blocked");
    assert_eq!(summary.message().as_deref(), Some("Apply 3 to add, 1 to change, 0 to destroy?"));
    assert_eq!((summary.app.as_deref(), summary.kind), (Some("terraform"), Some(Kind::Permission)));
}

#[test]
fn a_shell_function_reports_and_the_next_prompt_ends_its_working() {
    // status() { printf '\e]7501;state=%s:msg=%s\e\\' "$1" "$(printf '%s' "$2" | base64 | tr -d '\n')"; }
    let mut store = ProgramStatus::default();
    assert!(send(&mut store, "state=working:msg=U3luY2luZyBwaG90b3M="));
    assert_eq!(shown(&store), (State::Working, None, Some("Syncing photos".into())));
    // `rsync … && status done "Photos synced" || status error "rsync failed"`
    assert!(send(&mut store, "state=error:msg=cnN5bmMgZmFpbGVk"));
    assert_eq!(shown(&store), (State::Error, None, Some("rsync failed".into())));
    store.on_prompt();
    assert_eq!(shown(&store).0, State::Error, "an error survives the prompt");
}

#[test]
fn a_discarded_report_changes_nothing() {
    let mut store = ProgramStatus::default();
    assert!(send(&mut store, "state=blocked:kind=question:msg=SGk="));
    let before = store.summary();
    for bad in [
        "state=idle:msg=a",
        "state=done:id=a b",
        "state=clear:id=//",
        "state=paused",
        "state=clear:msg=aGkK",
        "app=cargo",
    ] {
        assert!(!send(&mut store, bad), "{bad}");
        assert_eq!(store.summary(), before, "{bad}");
    }
    assert_eq!(store.len(), 1);
}

#[test]
fn feature_detection_is_answered_with_a_plain_question_mark() {
    assert_eq!(parse(b"?", 2), Some(Parsed::Query));
    assert_eq!(QUERY_REPLY, b"\x1b]7501;?\x1b\\");
    // The reply holds nothing from any report: it is the same bytes whatever came before.
    assert!(!QUERY_REPLY.windows(5).any(|w| w == b"state"));
}

/// What Claude Code 2.1.299 sent, byte for byte, once the query was answered: idle at its prompt,
/// working on a turn (with words once it knows them), blocked on a permission prompt, and idle
/// again when the prompt was turned down with Esc. A finished turn is `done`. It says nothing
/// when it quits, so `done` is left for the user to find.
#[test]
fn claude_code_as_it_reports_itself() {
    let mut store = ProgramStatus::default();
    let agent = |store: &ProgramStatus| store.summary().and_then(|summary| summary.agent());
    let words = |text: &str| Some(text.to_string());
    let app = Some("claude-code".to_string());

    assert!(send(&mut store, "state=idle:app=claude-code"));
    assert_eq!(shown(&store), (State::Idle, app.clone(), None));
    assert_eq!(agent(&store), Some(crate::agents::AgentId::Claude));

    assert!(send(&mut store, "state=working:app=claude-code"));
    assert!(send(
        &mut store,
        "state=working:app=claude-code:msg=RmV0Y2hpbmcgSFRUUCBoZWFkZXJzIGZyb20gZXhhbXBsZS5vcmc="
    ));
    assert_eq!(shown(&store), (State::Working, app.clone(), words("Fetching HTTP headers from example.org")));

    assert!(send(
        &mut store,
        "state=blocked:app=claude-code:kind=permission:msg=YXBwcm92ZSBCYXNoOiBjdXJsIC1zSSBodHRwczovL2V4YW1wbGUub3Jn"
    ));
    let summary = store.summary().expect("blocked");
    assert_eq!((summary.state, summary.kind), (State::Blocked, Some(Kind::Permission)));
    assert_eq!(summary.message().as_deref(), Some("approve Bash: curl -sI https://example.org"));

    assert!(send(&mut store, "state=idle:app=claude-code"));
    assert_eq!(shown(&store), (State::Idle, app.clone(), None));

    assert!(send(&mut store, "state=working:app=claude-code"));
    assert!(send(&mut store, "state=done:app=claude-code"));
    store.on_process_exit();
    assert_eq!(shown(&store), (State::Done, app, None));
}
