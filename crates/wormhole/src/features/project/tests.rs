//! What the project folder feature does through the window, pinned from outside: which terminal is in
//! which project, what runs and what is refused until the project is trusted (and its parts are
//! switched on), the project's shell and environment, the actions' menu lines and keys, a prompt as a
//! note, the notes.md sync, trust and the preferences, and the hub of each profile.
//!
//! The helpers at the top are the only place a test names where the state is kept, so the tests say
//! what happens and not how it is held. There is no Touch ID here: the unit-test build answers an
//! approval dialog with "cancelled", and approval itself is pinned by `trust::tests`.

use std::path::{Path, PathBuf};

use gpui_kit::{Keystroke, TestAppContext};
use wormhole_core::config::TabId;
use wormhole_core::config::profile::{NewProfile, StartFrom};
use wormhole_core::keys::keymap::ActionId;
use wormhole_core::project::{Feature, ProjectFolder, TrustStatus};
use wormhole_core::storage::paths::ProfileId;

use super::notes_sync::NotesLink;
use super::{
    Project, ProjectHub, approve_project, hub, hub_mut, revoke_project, set_feature_default, set_feature_for,
    shown_profile,
};
use crate::app_state::{AppState, OpenWindow};
use crate::features::notes::testing as notes;
use crate::window::Workspace;
use wormhole_core::storage::store::WindowRecord;

use crate::window::test_harness::{
    draw, draw_fresh, drive, isolate_in, read, read_cx, start, start_with_table,
};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

// ---------------------------------------------------------------- where the state is kept

/// The window's project entity.
fn feature(ws: &Workspace) -> gpui_kit::Entity<Project> {
    ws.feature::<Project>().expect("the window has the project feature")
}

/// The project terminal `ix` is in, as of the window's last look.
fn project_of(window: &OpenWindow, ix: usize, cx: &mut TestAppContext) -> Option<PathBuf> {
    read_cx(cx, window, |ws, cx| feature(ws).read(cx).of[&ws.sessions()[ix].id].root.clone())
}

/// The window looks for each terminal's project again.
fn refresh(ws: &mut Workspace, cx: &mut gpui_kit::Context<Workspace>) {
    // What a test changed in a terminal inside this update is heard by the window after it.
    ws.sync_summaries(cx);
    ws.refresh_features(cx);
}

/// The terminal in front is in `dir`, and the window has looked for its project.
fn go(ws: &mut Workspace, dir: &Path, cx: &mut gpui_kit::Context<Workspace>) {
    let dir = dir.to_path_buf();
    ws.active_terminal().update(cx, |terminal, cx| {
        terminal.cwd = Some(dir);
        cx.notify();
    });
    refresh(ws, cx);
}

/// A click on one of the Project tab's actions (or its key, or a menu line).
fn run_action(ws: &mut Workspace, id: &str, cx: &mut gpui_kit::Context<Workspace>) {
    super::api::run_action(ws, id, cx);
}

/// The id of the action the key `keys` runs in the project in front, if any.
fn action_for_key(window: &OpenWindow, keys: &str, cx: &mut TestAppContext) -> Option<String> {
    let keystroke = Keystroke::parse(keys).expect("a keystroke");
    read_cx(cx, window, |ws, cx| super::api::action_for_key(ws, &keystroke, cx))
}

/// The Project tab's "copy into a new note" button.
fn prompt_to_note(
    ws: &mut Workspace,
    id: &str,
    window: &mut gpui_kit::Window,
    cx: &mut gpui_kit::Context<Workspace>,
) {
    let (session, panel) = (ws.active_session().id, ws.columns().panel);
    let notes = NotesLink::of(&ws.facts(cx)).expect("the window has notes");
    feature(ws).update(cx, |project, cx| project.prompt_to_note(session, id, panel, &notes, window, cx));
}

/// The Project tab's "Open terminals" button.
fn open_layout(ws: &mut Workspace, cx: &mut gpui_kit::Context<Workspace>) {
    let session = ws.active_session().id;
    feature(ws).update(cx, |project, cx| project.open_layout(session, &ProfileId::default_profile(), cx));
}

/// The environment and shell a terminal started in `dir` gets from its project.
fn spawn_options(
    cx: &mut TestAppContext,
    dir: &Path,
) -> (std::collections::BTreeMap<String, String>, Option<String>) {
    let add = cx.update(|cx| super::detect::spawn_options(&ProfileId::default_profile(), Some(dir), cx));
    (add.env, add.shell)
}

/// One round of the notes.md sync for the window.
fn sync_notes(ws: &mut Workspace, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Workspace>) {
    feature(ws).update(cx, |project, pcx| project.sync_notes(&ws.facts(pcx), window, pcx));
}

/// What the hub says of the project at `root`.
fn status(cx: &mut TestAppContext, root: &Path) -> Option<TrustStatus> {
    cx.update(|cx| hub(cx).projects.get(root).map(|project| project.status))
}

fn with_hub<R>(cx: &mut TestAppContext, f: impl FnOnce(&ProjectHub) -> R) -> R {
    cx.update(|cx| f(hub(cx)))
}

fn trust(cx: &mut TestAppContext, root: &Path) {
    cx.update(|cx| {
        let folder = ProjectFolder::open(root).expect("a project");
        hub_mut(cx).trust.trust(&folder);
    });
}

fn flag(cx: &mut TestAppContext, on: bool) {
    cx.update(|cx| {
        crate::settings::update_settings(cx, |settings| {
            settings.feature_flags.insert("project_folders".into(), on);
        });
    });
}

// ---------------------------------------------------------------- fixtures

/// A folder in the test's scratch dir with these files under `.wormhole/`.
fn project(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = std::env::current_dir().expect("the scratch dir").join(name);
    std::fs::create_dir_all(root.join(".wormhole")).expect("mkdir");
    let root = root.canonicalize().expect("canonical");
    for (file, text) in files {
        let path = root.join(".wormhole").join(file);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
        std::fs::write(path, text).expect("write");
    }
    root
}

const ACTIONS: &str = r#"[
    { "id": "more", "title": "More", "type": "run", "command": "true", "target": "new_session", "key": "f9" },
    { "id": "say", "title": "Say", "type": "prompt", "text": "hello", "target": "current" }
]"#;

// ---------------------------------------------------------------- tests

/// Each terminal is in the project of its folder; a folder in none, and a flag that is off, give none.
#[gpui_kit::test]
fn a_terminal_is_in_the_project_of_its_folder(cx: &mut TestAppContext) {
    if isolate("a_terminal_is_in_the_project_of_its_folder") {
        return;
    }
    let window = start(cx, 2);
    let (one, two) = (project("one", &[]), project("two", &[]));
    std::fs::create_dir_all(two.join("src")).expect("mkdir");
    let bare = std::env::current_dir().expect("scratch").join("bare");
    std::fs::create_dir_all(&bare).expect("mkdir");
    drive(cx, &window, |ws, _, cx| {
        ws.sessions()[0].terminal().update(cx, |t, cx| {
            t.cwd = Some(one.clone());
            cx.notify();
        });
        ws.sessions()[1].terminal().update(cx, |t, cx| {
            t.cwd = Some(two.join("src"));
            cx.notify();
        });
        refresh(ws, cx);
    });
    assert_eq!(project_of(&window, 0, cx).as_deref(), Some(one.as_path()));
    assert_eq!(project_of(&window, 1, cx).as_deref(), Some(two.as_path()), "from a folder inside it");

    drive(cx, &window, |ws, _, cx| {
        ws.sessions()[1].terminal().update(cx, |t, cx| {
            t.cwd = Some(bare.clone());
            cx.notify();
        });
        refresh(ws, cx);
    });
    assert_eq!(project_of(&window, 1, cx), None, "a folder in no project");

    flag(cx, false);
    drive(cx, &window, |ws, _, cx| refresh(ws, cx));
    assert_eq!(project_of(&window, 0, cx), None, "with the feature off, no terminal is in a project");
    flag(cx, true);
    drive(cx, &window, |ws, _, cx| refresh(ws, cx));
    assert_eq!(project_of(&window, 0, cx).as_deref(), Some(one.as_path()), "and back when it is on");
}

/// The Project tab is offered with a folder and the flag, and an open one is reset when the flag goes.
#[gpui_kit::test]
fn the_project_tab_follows_the_flag_and_the_folder(cx: &mut TestAppContext) {
    if isolate("the_project_tab_follows_the_flag_and_the_folder") {
        return;
    }
    let window = start(cx, 1);
    let tabs = |cx: &mut TestAppContext| read_cx(cx, &window, |ws, cx| ws.panel_tab_list(cx));
    assert!(!tabs(cx).contains(&TabId::Project), "no folder yet");
    let dir = project("tab", &[]);
    drive(cx, &window, |ws, _, cx| go(ws, &dir, cx));
    assert!(tabs(cx).contains(&TabId::Project));
    drive(cx, &window, |ws, _, cx| ws.show_tab(TabId::Project, cx));
    assert_eq!(read(cx, &window, |ws| ws.right_tab()), TabId::Project);

    flag(cx, false);
    assert!(!tabs(cx).contains(&TabId::Project), "gone at once");
    drive(cx, &window, |ws, _, cx| refresh(ws, cx));
    assert_eq!(
        read(cx, &window, |ws| ws.right_tab()),
        TabId::Changes,
        "and the window's next look leaves it"
    );
}

/// Nothing a project says runs until the folder is trusted and the part is switched on.
#[gpui_kit::test]
fn actions_run_only_in_a_trusted_project_with_actions_on(cx: &mut TestAppContext) {
    if isolate("actions_run_only_in_a_trusted_project_with_actions_on") {
        return;
    }
    let window = start(cx, 1);
    let dir = project("actions", &[("actions.json", ACTIONS)]);
    drive(cx, &window, |ws, _, cx| go(ws, &dir, cx));
    let sessions = |cx: &mut TestAppContext| read(cx, &window, |ws| ws.sessions().len());

    drive(cx, &window, |ws, _, cx| run_action(ws, "more", cx));
    assert_eq!(sessions(cx), 1, "untrusted: nothing runs");
    assert_eq!(action_for_key(&window, "f9", cx), None, "and its key is not taken");
    let menu = read_cx(cx, &window, |ws, cx| ws.menu_state(cx).project_actions);
    assert!(menu.is_empty(), "nor is it listed in the menu");

    trust(cx, &dir);
    drive(cx, &window, |ws, _, cx| refresh(ws, cx));
    assert_eq!(status(cx, &dir), Some(TrustStatus::Trusted));
    assert_eq!(action_for_key(&window, "f9", cx).as_deref(), Some("more"), "trusted: its key runs it");
    assert_eq!(action_for_key(&window, "f8", cx), None, "another key is nobody's");
    let menu = read_cx(cx, &window, |ws, cx| ws.menu_state(cx).project_actions);
    assert_eq!(menu, [("more".to_string(), "More".to_string()), ("say".to_string(), "Say".to_string())]);
    drive(cx, &window, |ws, _, cx| run_action(ws, "more", cx));
    assert_eq!(sessions(cx), 2, "a new session was opened for it");
    drive(cx, &window, |ws, _, cx| run_action(ws, "nope", cx));
    assert_eq!(sessions(cx), 2, "an id the project doesn't have does nothing");

    cx.update(|cx| set_feature_for(&shown_profile(cx).clone(), &dir, Feature::Actions, Some(false), cx));
    cx.run_until_parked();
    drive(cx, &window, |ws, _, cx| {
        refresh(ws, cx);
        run_action(ws, "more", cx);
    });
    assert_eq!(sessions(cx), 2, "switched off for the project: nothing runs");
    assert_eq!(action_for_key(&window, "f9", cx), None);
    let menu = read_cx(cx, &window, |ws, cx| ws.menu_state(cx).project_actions);
    assert!(menu.is_empty());

    flag(cx, false);
    cx.update(|cx| set_feature_for(&shown_profile(cx).clone(), &dir, Feature::Actions, None, cx));
    cx.run_until_parked();
    drive(cx, &window, |ws, _, cx| run_action(ws, "more", cx));
    assert_eq!(sessions(cx), 2, "with the whole feature off, nothing runs");
}

/// A prompt becomes a note on the terminal in front where Prompts are on for the project. Pasting one
/// runs nothing, so it needs no trust.
#[gpui_kit::test]
fn a_prompt_can_be_copied_into_a_note(cx: &mut TestAppContext) {
    if isolate("a_prompt_can_be_copied_into_a_note") {
        return;
    }
    let window = start(cx, 1);
    let dir = project("prompts", &[("prompts/review.md", "# Review\n\nLook at the diff.\n")]);
    drive(cx, &window, |ws, _, cx| go(ws, &dir, cx));
    let count = |cx: &mut TestAppContext| read_cx(cx, &window, |ws, cx| notes::note_count(ws, 0, cx));

    drive(cx, &window, |ws, window, cx| prompt_to_note(ws, "nope", window, cx));
    assert_eq!(count(cx), 0, "a prompt the project doesn't have");
    drive(cx, &window, |ws, window, cx| prompt_to_note(ws, "review", window, cx));
    assert_eq!(count(cx), 1, "no trust needed: nothing runs");
    let text = read_cx(cx, &window, |ws, cx| notes::note(ws, 0, 0, cx).0.read(cx).value().to_string());
    assert_eq!(text, "Look at the diff.");

    cx.update(|cx| set_feature_for(&shown_profile(cx).clone(), &dir, Feature::Prompts, Some(false), cx));
    cx.run_until_parked();
    drive(cx, &window, |ws, window, cx| {
        refresh(ws, cx);
        prompt_to_note(ws, "review", window, cx);
    });
    assert_eq!(count(cx), 1, "Prompts switched off: no second note");
}

/// The project's terminal layout opens its sessions once, by name, and only when trusted and on.
#[gpui_kit::test]
fn the_layout_opens_its_terminals_once(cx: &mut TestAppContext) {
    if isolate("the_layout_opens_its_terminals_once") {
        return;
    }
    let window = start(cx, 1);
    let dir = project(
        "layout",
        &[("layout.json", r#"{ "sessions": [ { "name": "web" }, { "name": "api", "focus": true } ] }"#)],
    );
    drive(cx, &window, |ws, _, cx| go(ws, &dir, cx));
    let names = |cx: &mut TestAppContext| {
        read(cx, &window, |ws| ws.sessions().iter().filter_map(|s| s.custom_name.clone()).collect::<Vec<_>>())
    };

    drive(cx, &window, |ws, _, cx| open_layout(ws, cx));
    assert!(names(cx).is_empty(), "untrusted: nothing opens");

    trust(cx, &dir);
    drive(cx, &window, |ws, _, cx| refresh(ws, cx));
    drive(cx, &window, |ws, _, cx| open_layout(ws, cx));
    assert_eq!(names(cx), ["web", "api"]);
    assert_eq!(read(cx, &window, |ws| ws.active_session().custom_name.clone()).as_deref(), Some("api"));
    drive(cx, &window, |ws, _, cx| open_layout(ws, cx));
    assert_eq!(names(cx), ["web", "api"], "the ones that are open are reused");
}

/// A trusted project's shell and environment go to a terminal started in it; nothing else's does.
#[gpui_kit::test]
fn a_trusted_projects_shell_and_environment_start_its_terminals(cx: &mut TestAppContext) {
    if isolate("a_trusted_projects_shell_and_environment_start_its_terminals") {
        return;
    }
    let _window = start(cx, 1);
    let dir = project("env", &[("config.json", r#"{ "env": { "A": "1" }, "shell": "/bin/sh" }"#)]);
    assert_eq!(spawn_options(cx, &dir), Default::default(), "untrusted: nothing");
    trust(cx, &dir);
    std::fs::create_dir_all(dir.join("src")).expect("mkdir");
    let (env, shell) = spawn_options(cx, &dir.join("src"));
    assert_eq!(env.get("A").map(String::as_str), Some("1"));
    assert_eq!(shell.as_deref(), Some("/bin/sh"));

    cx.update(|cx| set_feature_for(&shown_profile(cx).clone(), &dir, Feature::Settings, Some(false), cx));
    assert_eq!(spawn_options(cx, &dir), Default::default(), "Settings switched off for the project");
    cx.update(|cx| set_feature_for(&shown_profile(cx).clone(), &dir, Feature::Settings, None, cx));
    flag(cx, false);
    assert_eq!(spawn_options(cx, &dir), Default::default(), "the feature off");
    flag(cx, true);
    let elsewhere = std::env::current_dir().expect("scratch").join("elsewhere");
    std::fs::create_dir_all(&elsewhere).expect("mkdir");
    assert_eq!(spawn_options(cx, &elsewhere), Default::default(), "a folder in no project");
}

/// The folder's notes.md and the terminal's notes and tasks are merged both ways; switched off, or with
/// the feature off, it leaves both alone.
#[gpui_kit::test]
fn the_notes_file_and_the_terminals_notes_follow_each_other(cx: &mut TestAppContext) {
    if isolate("the_notes_file_and_the_terminals_notes_follow_each_other") {
        return;
    }
    let window = start(cx, 1);
    let dir = project(
        "sync",
        &[(
            "notes.md",
            "# Notes\n<!-- wormhole:notes -->\n\n## Next steps\n- [ ] from the file\n\n## Notes\n",
        )],
    );
    trust(cx, &dir);
    drive(cx, &window, |ws, _, cx| go(ws, &dir, cx));
    let tasks = |cx: &mut TestAppContext| read_cx(cx, &window, |ws, cx| notes::tasks(ws, 0, cx));

    drive(cx, &window, sync_notes);
    assert_eq!(tasks(cx), [("from the file".to_string(), false)], "the file's task came in");

    drive(cx, &window, |ws, window, cx| {
        notes::add_task(ws, 0, "from the terminal", cx);
        sync_notes(ws, window, cx);
    });
    let file = std::fs::read_to_string(dir.join(".wormhole/notes.md")).expect("notes.md");
    assert!(file.contains("- [ ] from the terminal") && file.contains("- [ ] from the file"), "{file}");

    // Switched off for the project: both sides are left alone, and a later switch-on merges afresh.
    cx.update(|cx| set_feature_for(&shown_profile(cx).clone(), &dir, Feature::Notes, Some(false), cx));
    cx.run_until_parked();
    drive(cx, &window, |ws, window, cx| {
        notes::add_task(ws, 0, "only here", cx);
        sync_notes(ws, window, cx);
    });
    let file = std::fs::read_to_string(dir.join(".wormhole/notes.md")).expect("notes.md");
    assert!(!file.contains("only here"), "{file}");

    cx.update(|cx| set_feature_for(&shown_profile(cx).clone(), &dir, Feature::Notes, None, cx));
    flag(cx, false);
    drive(cx, &window, sync_notes);
    let file = std::fs::read_to_string(dir.join(".wormhole/notes.md")).expect("notes.md");
    assert!(!file.contains("only here"), "the feature off: nothing is read or written: {file}");
    flag(cx, true);
    drive(cx, &window, |ws, window, cx| {
        refresh(ws, cx);
        sync_notes(ws, window, cx);
    });
    let file = std::fs::read_to_string(dir.join(".wormhole/notes.md")).expect("notes.md");
    assert!(file.contains("only here"), "and on again: merged afresh: {file}");
}

/// notes.md comes with the repository: until the project is trusted it is neither merged into the
/// terminal's notes nor written back, and the Project tab says why. Trusted, it syncs.
#[gpui_kit::test]
fn an_untrusted_projects_notes_file_is_neither_read_nor_written(cx: &mut TestAppContext) {
    if isolate("an_untrusted_projects_notes_file_is_neither_read_nor_written") {
        return;
    }
    let window = start(cx, 1);
    let theirs = "# Notes\n<!-- wormhole:notes -->\n\n## Next steps\n- [ ] from the repository\n\n## Notes\n";
    let dir = project("sync-untrusted", &[("notes.md", theirs)]);
    drive(cx, &window, |ws, _, cx| go(ws, &dir, cx));
    let tasks = |cx: &mut TestAppContext| read_cx(cx, &window, |ws, cx| notes::tasks(ws, 0, cx));
    let file = || std::fs::read_to_string(dir.join(".wormhole/notes.md")).expect("notes.md");

    drive(cx, &window, |ws, window, cx| {
        notes::add_task(ws, 0, "from the terminal", cx);
        sync_notes(ws, window, cx);
    });
    assert_eq!(tasks(cx), [("from the terminal".to_string(), false)], "the file's task did not come in");
    assert_eq!(file(), theirs, "and nothing was written back");
    let said = cx.update(|cx| hub(cx).notes_errors.values().cloned().collect::<Vec<_>>());
    assert_eq!(said.len(), 1, "the Project tab is told why: {said:?}");
    assert!(said[0].contains("trusted"), "{said:?}");

    trust(cx, &dir);
    drive(cx, &window, sync_notes);
    let mut now = tasks(cx);
    now.sort();
    assert_eq!(
        now,
        [("from the repository".to_string(), false), ("from the terminal".to_string(), false)],
        "trusted, the file's task comes in"
    );
    assert!(file().contains("- [ ] from the terminal"), "and the terminal's goes out: {}", file());
    assert!(cx.update(|cx| hub(cx).notes_errors.is_empty()), "and nothing is waiting any more");
}

/// Waiting for trust keeps the last agreed state: a task deleted from notes.md while the project waited
/// stays deleted once it is trusted again, instead of coming back from the terminal's copy.
#[gpui_kit::test]
fn a_deletion_made_while_waiting_for_trust_stays_deleted(cx: &mut TestAppContext) {
    if isolate("a_deletion_made_while_waiting_for_trust_stays_deleted") {
        return;
    }
    let window = start(cx, 1);
    let theirs =
        "# Notes\n<!-- wormhole:notes -->\n\n## Next steps\n- [ ] gone soon\n- [ ] stays\n\n## Notes\n";
    let dir = project("sync-wait", &[("notes.md", theirs)]);
    trust(cx, &dir);
    drive(cx, &window, |ws, _, cx| go(ws, &dir, cx));
    drive(cx, &window, sync_notes);
    let tasks = |cx: &mut TestAppContext| {
        let mut now = read_cx(cx, &window, |ws, cx| notes::tasks(ws, 0, cx));
        now.sort();
        now
    };
    assert_eq!(tasks(cx).len(), 2, "both tasks came in while trusted");

    cx.update(|cx| hub_mut(cx).trust.revoke(&dir));
    let without = "# Notes\n<!-- wormhole:notes -->\n\n## Next steps\n- [ ] stays\n\n## Notes\n";
    std::fs::write(dir.join(".wormhole/notes.md"), without).expect("deletes a task in notes.md");
    drive(cx, &window, sync_notes);
    assert_eq!(tasks(cx).len(), 2, "untrusted, the deletion is not read yet");

    trust(cx, &dir);
    drive(cx, &window, sync_notes);
    assert_eq!(tasks(cx), [("stays".to_string(), false)], "trusted again, the deletion comes in");
    let file = std::fs::read_to_string(dir.join(".wormhole/notes.md")).expect("notes.md");
    assert!(!file.contains("gone soon"), "and the task does not come back: {file}");
}

/// The hub is a global the window does not watch: a sync that starts failing has to wake the window itself
/// for the Project tab to say why.
#[gpui_kit::test]
fn a_notes_sync_that_fails_wakes_the_window_so_the_tab_says_why(cx: &mut TestAppContext) {
    if isolate("a_notes_sync_that_fails_wakes_the_window_so_the_tab_says_why") {
        return;
    }
    let window = start(cx, 1);
    let dir = project("sync-fails", &[("notes.md", "# Notes\n<!-- wormhole:notes -->\n\n## Notes\n")]);
    trust(cx, &dir);
    drive(cx, &window, |ws, _, cx| go(ws, &dir, cx));
    drive(cx, &window, sync_notes);
    // Bytes that are not text: the file cannot be read.
    std::fs::write(dir.join(".wormhole/notes.md"), [0xff_u8, 0xfe, 0x00, 0xff]).expect("breaks notes.md");
    let wakes = std::rc::Rc::new(std::cell::Cell::new(0));
    cx.update(|cx| {
        let counted = wakes.clone();
        let workspace = window.workspace.upgrade().expect("the window is open");
        cx.observe(&workspace, move |_, _| counted.set(counted.get() + 1)).detach();
    });
    drive(cx, &window, sync_notes);
    assert!(wakes.get() >= 1, "the window is woken when the error appears");
    let said = cx.update(|cx| hub(cx).notes_errors.len());
    assert_eq!(said, 1, "and the hub has the reason");
    let after = wakes.get();
    drive(cx, &window, sync_notes);
    assert_eq!(wakes.get(), after, "the same error again wakes nobody");
}

/// The hub is the profile's, and each window of it shows the reason in its own Project tab.
#[gpui_kit::test]
fn a_notes_sync_that_fails_in_one_window_wakes_the_profiles_other_windows_too(cx: &mut TestAppContext) {
    if isolate("a_notes_sync_that_fails_in_one_window_wakes_the_profiles_other_windows_too") {
        return;
    }
    let window = start(cx, 1);
    cx.update(|cx| {
        let record = WindowRecord { id: 2, ..Default::default() };
        crate::window::window_lifecycle::open_workspace_window(Some(record), None, cx);
    });
    cx.run_until_parked();
    let other = cx
        .update(|cx| crate::app_state::app(cx).windows.iter().find(|w| w.id == 2).cloned())
        .expect("a second window of the profile");
    let dir = project("sync-fails-two", &[("notes.md", "# Notes\n<!-- wormhole:notes -->\n\n## Notes\n")]);
    trust(cx, &dir);
    drive(cx, &window, |ws, _, cx| go(ws, &dir, cx));
    drive(cx, &window, sync_notes);
    std::fs::write(dir.join(".wormhole/notes.md"), [0xff_u8, 0xfe, 0x00, 0xff]).expect("breaks notes.md");
    let wakes = std::rc::Rc::new(std::cell::Cell::new(0));
    cx.update(|cx| {
        let counted = wakes.clone();
        let workspace = other.workspace.upgrade().expect("the second window is open");
        cx.observe(&workspace, move |_, _| counted.set(counted.get() + 1)).detach();
    });
    drive(cx, &window, sync_notes);
    assert!(wakes.get() >= 1, "the other window, which shows the same error, is woken too");
}

/// Approving asks the owner (here the answer is "cancelled": no Touch ID in a test), revoking and the
/// per-project switches take effect at once, and the defaults are the hub's.
#[gpui_kit::test]
fn trust_and_the_switches_are_the_hubs(cx: &mut TestAppContext) {
    if isolate("trust_and_the_switches_are_the_hubs") {
        return;
    }
    let window = start(cx, 1);
    let dir = project("trust", &[("actions.json", ACTIONS)]);
    drive(cx, &window, |ws, _, cx| go(ws, &dir, cx));
    assert_eq!(status(cx, &dir), Some(TrustStatus::Untrusted));

    cx.update(|cx| approve_project(&shown_profile(cx).clone(), &dir, cx));
    cx.run_until_parked();
    assert_eq!(status(cx, &dir), Some(TrustStatus::Untrusted), "the owner cancelled: still not trusted");
    assert!(!with_hub(cx, |hub| hub.approving), "and nothing is waiting");
    assert!(with_hub(cx, |hub| hub.error.is_none()), "a cancel is not an error");

    trust(cx, &dir);
    drive(cx, &window, |ws, _, cx| refresh(ws, cx));
    assert_eq!(status(cx, &dir), Some(TrustStatus::Trusted));
    cx.update(|cx| revoke_project(&shown_profile(cx).clone(), &dir, cx));
    cx.run_until_parked();
    assert_eq!(status(cx, &dir), Some(TrustStatus::Untrusted), "revoked at once");

    assert!(with_hub(cx, |hub| hub.prefs.default_enabled(Feature::Layout)));
    cx.update(|cx| set_feature_default(&shown_profile(cx).clone(), Feature::Layout, false, cx));
    assert!(!with_hub(cx, |hub| hub.prefs.default_enabled(Feature::Layout)));
    cx.update(|cx| set_feature_default(&shown_profile(cx).clone(), Feature::Layout, true, cx));
}

/// Each profile has its own hub: its preferences, its trust and its notes bases. A window of the
/// shown profile reads the shown profile's, and a switch brings the other's back as it was left.
#[gpui_kit::test]
fn each_profile_has_its_own_project_hub(cx: &mut TestAppContext) {
    if isolate("each_profile_has_its_own_project_hub") {
        return;
    }
    let _window = start(cx, 1);
    cx.update(|cx| set_feature_default(&shown_profile(cx).clone(), Feature::Prompts, false, cx));
    let dir = project("profiles", &[("actions.json", ACTIONS)]);
    trust(cx, &dir);
    let work = cx
        .update(|cx| {
            let new =
                NewProfile { name: "Work".into(), color: None, avatar: None, from: StartFrom::Defaults };
            AppState::create_profile(new, cx)
        })
        .expect("makes the profile");
    cx.update(|cx| cx.set_global(crate::fonts::LoadedFonts::default()));

    cx.update(|cx| AppState::switch_profile(&work, cx)).expect("switches");
    cx.run_until_parked();
    assert!(with_hub(cx, |hub| hub.prefs.default_enabled(Feature::Prompts)), "Work has its own preferences");
    let folder = ProjectFolder::open(&dir).expect("a project");
    assert_eq!(
        with_hub(cx, |hub| hub.trust.status(&folder)),
        TrustStatus::Untrusted,
        "and its own trust: the other profile's approval is not its"
    );
    cx.update(|cx| set_feature_default(&shown_profile(cx).clone(), Feature::Layout, false, cx));

    cx.update(|cx| AppState::switch_profile(&ProfileId::default_profile(), cx)).expect("switches back");
    cx.run_until_parked();
    assert!(!with_hub(cx, |hub| hub.prefs.default_enabled(Feature::Prompts)), "the first profile's, as left");
    assert!(with_hub(cx, |hub| hub.prefs.default_enabled(Feature::Layout)), "Work's change stayed in Work");
    assert_eq!(with_hub(cx, |hub| hub.trust.status(&folder)), TrustStatus::Trusted);
}

/// A window follows the trust of the profile it lives in, whichever profile is shown: a project the shown
/// profile approved is not approved in a window of another one, and the other way round, before and after
/// a switch.
#[gpui_kit::test]
fn a_window_follows_the_trust_of_its_own_profile_not_the_shown_ones(cx: &mut TestAppContext) {
    if isolate("a_window_follows_the_trust_of_its_own_profile_not_the_shown_ones") {
        return;
    }
    let first = start(cx, 1);
    let default = ProfileId::default_profile();
    let work = cx
        .update(|cx| {
            let new =
                NewProfile { name: "Work".into(), color: None, avatar: None, from: StartFrom::Defaults };
            let id = AppState::create_profile(new, cx)?;
            AppState::ensure_open(&id, cx);
            Ok::<ProfileId, String>(id)
        })
        .expect("makes the profile");
    cx.update(|cx| cx.set_global(crate::fonts::LoadedFonts::default()));
    cx.update(|cx| AppState::open_window_in_profile(&work, cx));
    cx.run_until_parked();
    let odd = cx
        .update(|cx| crate::app_state::app(cx).windows.iter().find(|w| w.profile == work).cloned())
        .expect("the window in Work opened");

    let dir = project(
        "two-profiles",
        &[("actions.json", ACTIONS), ("config.json", r#"{"env": {"WH_TWO_PROFILES": "1"}}"#)],
    );
    let folder = ProjectFolder::open(&dir).expect("a project");
    // What a terminal or a worktree started for `profile` gets from the project: its environment, and
    // whether a worktree of it may be made. Both ask for the profile's own approval.
    let reach = |cx: &mut TestAppContext, profile: &ProfileId| {
        cx.update(|cx| {
            let added = Workspace::spawn_adds(profile, Some(&dir), crate::features::TABLE, cx);
            (
                added.env.contains_key("WH_TWO_PROFILES"),
                super::api::worktree_allowed(profile, &dir, cx).is_ok(),
            )
        })
    };
    let (starts, refuses) = ((true, true), (false, false));
    // What each window may run: its project's actions, and the pull request lookup's approval.
    let actions = |cx: &mut TestAppContext, window: &OpenWindow| {
        drive(cx, window, |ws, _, cx| go(ws, &dir, cx));
        read_cx(cx, window, |ws, cx| {
            (
                super::api::actionable(ws, cx).is_some(),
                super::api::action_for_key(ws, &Keystroke::parse("f9").expect("a key"), cx).is_some(),
                ws.programs_approved(ws.sessions()[0].id, cx),
            )
        })
    };
    let (yes, no) = ((true, true, true), (false, false, false));

    // Only the shown profile (Default) trusts it.
    cx.update(|cx| super::hub_of_mut(&default, cx).expect("Default's hub").trust.trust(&folder));
    assert_eq!(actions(cx, &first), yes, "Default's window: trusted in Default");
    assert_eq!(reach(cx, &default), starts, "Default's terminals and worktrees: trusted in Default");
    assert_eq!(actions(cx, &odd), no, "Work's window: Default's approval is not Work's");
    assert_eq!(reach(cx, &work), refuses, "Work's terminals and worktrees: Default's approval is not Work's");

    // The switch makes Work the shown profile: the windows keep their own profile's trust.
    cx.update(|cx| AppState::switch_profile(&work, cx)).expect("switches");
    cx.run_until_parked();
    assert_eq!(actions(cx, &first), yes, "Default's window, now not shown, still follows Default");
    assert_eq!(actions(cx, &odd), no, "Work's window, now shown, follows Work");

    // Work approves it, and Default takes its approval back: each window follows only its own.
    cx.update(|cx| super::hub_of_mut(&work, cx).expect("Work's hub").trust.trust(&folder));
    cx.update(|cx| revoke_project(&default, &dir, cx));
    cx.run_until_parked();
    assert_eq!(actions(cx, &odd), yes, "Work's window: trusted in Work");
    assert_eq!(reach(cx, &work), starts, "Work's terminals and worktrees: trusted in Work");
    assert_eq!(actions(cx, &first), no, "Default's window: revoked in Default, though Work trusts it");
    assert_eq!(reach(cx, &default), refuses, "Default's terminals and worktrees: revoked in Default");
}

/// A window reads the features of the profile it lives in: with the project folders off in the shown
/// profile, a window of another profile, where they are on, still runs its project's actions and
/// starts its terminals and worktrees with the project, and the shown profile's window does not.
#[gpui_kit::test]
fn a_window_reads_the_feature_switches_of_its_own_profile(cx: &mut TestAppContext) {
    if isolate("a_window_reads_the_feature_switches_of_its_own_profile") {
        return;
    }
    let first = start(cx, 1);
    let default = ProfileId::default_profile();
    let work = cx
        .update(|cx| {
            let new =
                NewProfile { name: "Work".into(), color: None, avatar: None, from: StartFrom::Defaults };
            let id = AppState::create_profile(new, cx)?;
            AppState::ensure_open(&id, cx);
            Ok::<ProfileId, String>(id)
        })
        .expect("makes the profile");
    cx.update(|cx| cx.set_global(crate::fonts::LoadedFonts::default()));
    cx.update(|cx| AppState::open_window_in_profile(&work, cx));
    cx.run_until_parked();
    let odd = cx
        .update(|cx| crate::app_state::app(cx).windows.iter().find(|w| w.profile == work).cloned())
        .expect("the window in Work opened");

    let dir = project(
        "two-masks",
        &[("actions.json", ACTIONS), ("config.json", r#"{"env": {"WH_TWO_MASKS": "1"}}"#)],
    );
    let folder = ProjectFolder::open(&dir).expect("a project");
    cx.update(|cx| super::hub_of_mut(&default, cx).expect("Default's hub").trust.trust(&folder));
    cx.update(|cx| super::hub_of_mut(&work, cx).expect("Work's hub").trust.trust(&folder));
    let reach = |cx: &mut TestAppContext, window: &OpenWindow, profile: &ProfileId| {
        drive(cx, window, |ws, _, cx| go(ws, &dir, cx));
        let actionable = read_cx(cx, window, |ws, cx| super::api::actionable(ws, cx).is_some());
        let started = cx.update(|cx| {
            Workspace::spawn_adds(profile, Some(&dir), crate::features::TABLE, cx)
                .env
                .contains_key("WH_TWO_MASKS")
        });
        (actionable, started)
    };
    assert_eq!(reach(cx, &first, &default), (true, true), "on in both");
    assert_eq!(reach(cx, &odd, &work), (true, true), "on in both");

    // Work is shown and has the project folders off; Default is parked with them on.
    cx.update(|cx| AppState::switch_profile(&work, cx)).expect("switches");
    cx.run_until_parked();
    cx.update(|cx| crate::settings::switch_feature(cx, wormhole_core::config::FeatureId::Project, false));
    assert_eq!(reach(cx, &odd, &work), (false, false), "Work's window: its own switch is off");
    assert_eq!(
        reach(cx, &first, &default),
        (true, true),
        "Default's window, not shown: its own switch is on, not the shown profile's"
    );
}

/// The status item's list of actions is the shown profile's projects': running one finds a window of the
/// shown profile, though another profile's window, opened earlier, has a terminal in the same folder.
#[gpui_kit::test]
fn a_status_item_action_runs_in_a_window_of_the_shown_profile(cx: &mut TestAppContext) {
    if isolate("a_status_item_action_runs_in_a_window_of_the_shown_profile") {
        return;
    }
    let first = start(cx, 1);
    let default = ProfileId::default_profile();
    let work = cx
        .update(|cx| {
            let new =
                NewProfile { name: "Work".into(), color: None, avatar: None, from: StartFrom::Defaults };
            let id = AppState::create_profile(new, cx)?;
            AppState::ensure_open(&id, cx);
            Ok::<ProfileId, String>(id)
        })
        .expect("makes the profile");
    cx.update(|cx| cx.set_global(crate::fonts::LoadedFonts::default()));
    cx.update(|cx| AppState::open_window_in_profile(&work, cx));
    cx.run_until_parked();
    let odd = cx
        .update(|cx| crate::app_state::app(cx).windows.iter().find(|w| w.profile == work).cloned())
        .expect("the window in Work opened");
    cx.update(|cx| AppState::switch_profile(&work, cx)).expect("switches");
    cx.run_until_parked();

    let dir = project("status-item", &[("actions.json", ACTIONS)]);
    let folder = ProjectFolder::open(&dir).expect("a project");
    for profile in [&default, &work] {
        cx.update(|cx| super::hub_of_mut(profile, cx).expect("a hub").trust.trust(&folder));
    }
    drive(cx, &first, |ws, _, cx| go(ws, &dir, cx));
    drive(cx, &odd, |ws, _, cx| go(ws, &dir, cx));
    let root = project_of(&odd, 0, cx).expect("Work's terminal is in the project");

    cx.update(|cx| {
        crate::status_item::run_command(
            crate::menubar::MenuCommand::RunAction { root, id: "more".into() },
            cx,
        )
    });
    cx.run_until_parked();
    let sessions = |cx: &mut TestAppContext, window: &OpenWindow| read(cx, window, |ws| ws.sessions().len());
    assert_eq!(sessions(cx, &odd), 2, "Work is shown: its window ran the action");
    assert_eq!(
        sessions(cx, &first),
        1,
        "Default's window, though it is earlier and in the same folder, did not"
    );
}

/// The Project tab's menu line is there for a terminal with a folder and the feature on.
#[gpui_kit::test]
fn the_menu_line_needs_a_folder_and_the_feature(cx: &mut TestAppContext) {
    if isolate("the_menu_line_needs_a_folder_and_the_feature") {
        return;
    }
    let window = start(cx, 1);
    let disabled = |cx: &mut TestAppContext| {
        read_cx(cx, &window, |ws, cx| ws.menu_state(cx).disabled.contains(&ActionId::ShowProject))
    };
    let hidden = |cx: &mut TestAppContext| {
        read_cx(cx, &window, |ws, cx| ws.menu_state(cx).hidden.contains(&ActionId::ShowProject))
    };
    assert!(disabled(cx), "no folder");
    assert!(!hidden(cx), "the feature is on: the line is there, greyed out");
    let dir = project("menu", &[]);
    drive(cx, &window, |ws, _, cx| go(ws, &dir, cx));
    assert!(!disabled(cx));
    flag(cx, false);
    assert!(disabled(cx), "the feature off");
    assert!(hidden(cx), "and the line is left out of the menus, as an off feature's actions are");
}

/// The project entity keeps an entry for exactly the terminals of the window: made with it, added with a
/// new terminal, dropped with a closed one and with one that moved to another window.
#[gpui_kit::test]
fn the_project_entity_keeps_an_entry_for_exactly_the_terminals_of_the_window(cx: &mut TestAppContext) {
    if isolate("the_project_entity_keeps_an_entry_for_exactly_the_terminals_of_the_window") {
        return;
    }
    let window = start(cx, 2);
    let ids = |cx: &mut TestAppContext, window: &OpenWindow| {
        read_cx(cx, window, |ws, cx| {
            let mut ours: Vec<i64> = ws.sessions().iter().map(|s| s.id).collect();
            ours.sort_unstable();
            let mut kept: Vec<i64> = feature(ws).read(cx).of.keys().copied().collect();
            kept.sort_unstable();
            (kept, ours)
        })
    };
    let (kept, ours) = ids(cx, &window);
    assert_eq!(kept, ours, "made with the window's terminals");

    drive(cx, &window, |ws, window, cx| ws.new_session(window, cx));
    let (kept, ours) = ids(cx, &window);
    assert_eq!((kept.clone(), kept.len()), (ours, 3), "added with a new terminal");

    let new = read(cx, &window, |ws| ws.sessions()[2].id);
    drive(cx, &window, |ws, window, cx| {
        let ix = ws.find_session(new).expect("a row");
        ws.close_session(ix, window, cx);
    });
    let (kept, ours) = ids(cx, &window);
    assert_eq!((kept.clone(), kept.len()), (ours, 2), "dropped with a closed terminal");

    let group = drive(cx, &window, |ws, _, cx| ws.group_terminals(&[2], cx));
    drive(cx, &window, |ws, window, cx| ws.move_group_to_window(group, window, cx));
    cx.run_until_parked();
    let (kept, ours) = ids(cx, &window);
    assert_eq!((kept.clone(), kept.len()), (ours, 1), "and with a terminal that moved away");
}

/// The table's entry: the tab is offered with a folder (or while it shows) and says its name; the project
/// is trusted for a terminal only in a trusted project, which is what the pull request lookup asks before
/// it starts `gh`, and a window whose table has nobody to ask approves nothing.
#[gpui_kit::test]
fn the_entry_offers_the_tab_and_says_who_is_trusted(cx: &mut TestAppContext) {
    if isolate("the_entry_offers_the_tab_and_says_who_is_trusted") {
        return;
    }
    let window = start(cx, 1);
    let dir = project("entry", &[("actions.json", ACTIONS)]);
    let asked = |cx: &mut TestAppContext| {
        drive(cx, &window, |ws, _, cx| {
            let at =
                ws.feature_defs().iter().position(|def| def.id == wormhole_core::config::FeatureId::Project);
            let at = at.expect("in the table");
            let (def, entity) =
                (&ws.feature_defs()[at], ws.feature_entity(at).expect("made with the window"));
            let (tab, facts) = (&def.tabs[0], ws.facts(cx));
            (
                (tab.shown)(&facts, cx),
                (tab.spoken)(entity, &facts, cx),
                ws.programs_approved(ws.sessions()[0].id, cx),
            )
        })
    };
    assert_eq!(asked(cx), (false, "Project".to_string(), false), "no folder");
    drive(cx, &window, |ws, _, cx| go(ws, &dir, cx));
    assert_eq!(asked(cx), (true, "Project".to_string(), false), "a folder, a project not trusted");
    trust(cx, &dir);
    drive(cx, &window, |ws, _, cx| refresh(ws, cx));
    assert_eq!(asked(cx), (true, "Project".to_string(), true), "trusted");
    flag(cx, false);
    assert!(!asked(cx).2, "the feature off");
    flag(cx, true);
    cx.update(|cx| revoke_project(&shown_profile(cx).clone(), &dir, cx));
    cx.run_until_parked();
    assert!(!asked(cx).2, "revoked");
}

/// A window whose table has no feature to ask approves nothing: the check fails closed.
#[gpui_kit::test]
fn a_window_with_nobody_to_ask_trusts_no_folder(cx: &mut TestAppContext) {
    if isolate("a_window_with_nobody_to_ask_trusts_no_folder") {
        return;
    }
    let record = WindowRecord { id: 1, ..Default::default() };
    let window = start_with_table(cx, record, &[]);
    assert!(!read_cx(cx, &window, |ws, cx| ws.programs_approved(ws.sessions()[0].id, cx)));
}

/// Allocations the third draw of a one-terminal window on the Project tab of a trusted project (three
/// actions, two prompts, a layout and a notes switch) may make. Measured: 22,609 allocations (12 MB), the
/// same on every run, and 22,610 (12 MB) before the project's state moved into its entity. The budget is that
/// plus about a hundredth; the tab draws a row for each part the project has, so a cost per action or per
/// frame that grows shows at once.
const PROJECT_FRAME_ALLOCATIONS: u64 = 22_835;

#[gpui_kit::test]
fn a_window_frame_on_project_stays_within_its_allocation_budget(cx: &mut TestAppContext) {
    use crate::perf::allocations::thread_counts;
    if isolate("a_window_frame_on_project_stays_within_its_allocation_budget") {
        return;
    }
    let window = start(cx, 1);
    let dir = project(
        "frame",
        &[
            (
                "actions.json",
                r#"[
                    { "id": "build", "title": "Build", "type": "run", "command": "make", "key": "f9" },
                    { "id": "test", "title": "Test", "type": "run", "command": "make test" },
                    { "id": "say", "title": "Say", "type": "prompt", "text": "hello" }
                ]"#,
            ),
            ("prompts/review.md", "# Review\n\nLook at the diff.\n"),
            ("prompts/plan.md", "# Plan\n\nWrite the steps.\n"),
            ("layout.json", r#"{ "sessions": [ { "name": "web" }, { "name": "api" } ] }"#),
            ("notes.md", "# Notes\n<!-- wormhole:notes -->\n\n## Next steps\n\n## Notes\n"),
            ("config.json", r#"{ "env": { "A": "1" } }"#),
        ],
    );
    trust(cx, &dir);
    drive(cx, &window, |ws, _, cx| {
        go(ws, &dir, cx);
        ws.show_tab(TabId::Project, cx);
    });
    assert_eq!(read(cx, &window, |ws| ws.right_tab()), TabId::Project);
    assert_eq!(status(cx, &dir), Some(TrustStatus::Trusted));
    // The first draws build what a window keeps; the third is a frame.
    draw(cx, &window);
    draw(cx, &window);
    let before = thread_counts();
    draw_fresh(cx, &window);
    let drawn = thread_counts().since(before);
    eprintln!("window frame on Project: {} allocations, {} bytes", drawn.allocations, drawn.bytes);
    assert!(
        drawn.allocations <= PROJECT_FRAME_ALLOCATIONS,
        "a window frame on Project made {} allocations (budget {PROJECT_FRAME_ALLOCATIONS})",
        drawn.allocations
    );
}
