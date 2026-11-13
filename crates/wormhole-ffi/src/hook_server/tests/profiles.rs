//! A profile is a trust domain: a request is judged by the mode and the approvals of the profile of the window it
//! reaches, never by the one the server started with or the one that is shown.

use serde_json::json;
use wormhole_core::config::profile::{NewProfile, ProfileList, StartFrom};
use wormhole_core::storage::paths::DataRoot;
use wormhole_core::workspace::Action;

use super::support::{Fixture, fixture_of_windows, opened};
use crate::{AccessAnswer, AccessDesk, Terminal, Workspace, WorkspaceConfig};

fn write_mode(root: &DataRoot, profile: &str, mode: &str) {
    let id = wormhole_core::storage::paths::ProfileId::parse(profile).expect("an id");
    let file = root.profile(&id).config_file();
    let config =
        json!({ "agent_access": mode, "control_write": true, "feature_flags": { "automation": true } });
    crate::settings::tests::write_file(&file, &config.to_string());
}

/// Profile Work, opened beside Default's windows, with its own desk and one terminal in a window of its own.
fn work_window(f: &Fixture, mode: &str) -> (std::sync::Arc<AccessDesk>, i64) {
    let root = DataRoot::new(&f.dir);
    let mut list = ProfileList::default();
    list.create(
        &root,
        NewProfile { name: "Work".into(), color: None, avatar: None, from: StartFrom::Defaults },
    )
    .expect("makes Work");
    list.save(&root).expect("saves the list");
    write_mode(&root, "work", mode);
    let windows = f.windows.clone().expect("windows").beside_in(&root, "work").expect("Work beside Default");
    let desk = AccessDesk::new(Some("work".into()), Some(f.dir.display().to_string()), f.wakes.clone())
        .expect("a desk");
    f.server.add_desk(desk.clone());
    let config = WorkspaceConfig {
        home: None,
        numbers_follow_list: true,
        start_folder: None,
        pane_header_height: 0.0,
    };
    let workspace = Workspace::in_window(windows.clone(), windows.alloc_window_id(), config, f.wakes.clone());
    let id = opened(&workspace.apply(Action::NewTerminal));
    let terminal =
        Terminal::start_for_tab(id, "/bin/cat".into(), Vec::new(), None, 80, 24, 8.0, 16.0, f.wakes.clone())
            .expect("a terminal");
    workspace.register_terminal(id, terminal);
    // The window must outlive the test body: the server holds windows weakly.
    std::mem::forget(workspace);
    (desk, id)
}

fn type_into(f: &Fixture, id: i64) -> serde_json::Value {
    f.client().control(json!({"action": "send_text", "target": {"kind": "terminal", "id": id}, "text": "x"}))
}

#[test]
fn a_request_is_judged_by_the_mode_of_the_profile_of_the_window_it_reaches() {
    let f = fixture_of_windows("ask");
    let default_terminal = f.terminal();
    let (work_desk, work_terminal) = work_window(&f, "off");

    // Work is off: refused outright, and nobody is asked anywhere.
    assert_eq!(type_into(&f, work_terminal)["error"]["reason"], "agent_access_off");
    assert_eq!((f.desk.waiting(), work_desk.waiting()), (0, 0));

    // Default asks, and the question is on Default's desk, not Work's.
    assert_eq!(type_into(&f, default_terminal)["error"]["reason"], "approval_needed");
    assert_eq!((f.desk.waiting(), work_desk.waiting()), (1, 0));
}

#[test]
fn approvals_are_kept_by_the_desk_of_the_profile_and_revoked_there() {
    let f = fixture_of_windows("open");
    let (work_desk, work_terminal) = work_window(&f, "ask");
    let root = DataRoot::new(&f.dir);
    assert_eq!(type_into(&f, work_terminal)["error"]["reason"], "approval_needed");
    let question = work_desk.next_question().expect("Work's desk was asked");
    assert_eq!(f.desk.waiting(), 0, "Default's desk was not");
    work_desk.answer(question.id, AccessAnswer::UntilQuit);
    // A program Work let in is still asked about the terminal; Default is open and never asks.
    assert_eq!(type_into(&f, work_terminal)["error"]["reason"], "approval_needed");
    assert_eq!(work_desk.held().len(), 1);
    let held = work_desk.held()[0].id;
    assert!(!f.server.revoke_approval("default".into(), held), "Default's desk holds no such approval");
    assert!(f.server.revoke_approval("work".into(), held));
    assert!(work_desk.held().is_empty());
    // Work's mode follows its own file: changed to open in Settings (an edit from outside would be held back and asked about,
    // see `a_mode_widened_outside_wormhole_is_held_back_until_the_person_keeps_it`), its next request is let in.
    let work = wormhole_core::storage::paths::ProfileId::parse("work").expect("an id");
    crate::settings::edits::note(root.profile(&work).dir());
    write_mode(&root, "work", "open");
    f.server.settings_changed("work".into());
    assert_eq!(type_into(&f, work_terminal)["ok"], true);
}

#[test]
fn a_mode_change_in_one_profile_settles_its_own_waiting_questions_and_leaves_the_other_profiles_alone() {
    let f = fixture_of_windows("ask");
    let default_terminal = f.terminal();
    let (work_desk, work_terminal) = work_window(&f, "ask");
    let root = DataRoot::new(&f.dir);
    assert_eq!(type_into(&f, default_terminal)["error"]["reason"], "approval_needed");
    assert_eq!(type_into(&f, work_terminal)["error"]["reason"], "approval_needed");
    assert_eq!((f.desk.waiting(), work_desk.waiting()), (1, 1));
    // Work is turned off in Settings: its question is refused when the server is told, Default's still waits.
    let work = wormhole_core::storage::paths::ProfileId::parse("work").expect("an id");
    crate::settings::edits::note(root.profile(&work).dir());
    write_mode(&root, "work", "off");
    f.server.settings_changed("work".into());
    assert_eq!((f.desk.waiting(), work_desk.waiting()), (1, 0));
    assert_eq!(type_into(&f, work_terminal)["error"]["reason"], "agent_access_off");
    assert_eq!(type_into(&f, default_terminal)["error"]["reason"], "approval_needed");
}
