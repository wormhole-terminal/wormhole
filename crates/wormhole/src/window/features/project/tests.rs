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

use super::{ProjectHub, approve_project, revoke_project, set_feature_default, set_feature_for};
use crate::app_state::{AppState, OpenWindow, app};
use crate::window::Workspace;
use crate::window::features::notes::testing as notes;
use crate::window::view_tests::{drive, isolate_in, read, read_cx, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

// ---------------------------------------------------------------- where the state is kept

/// The project terminal `ix` is in, as of the window's last look.
fn project_of(window: &OpenWindow, ix: usize, cx: &mut TestAppContext) -> Option<PathBuf> {
    read(cx, window, |ws| ws.sessions[ix].project.clone())
}

/// The window looks for each terminal's project again.
fn refresh(ws: &mut Workspace, cx: &mut gpui_kit::Context<Workspace>) {
    ws.refresh_projects(cx);
}

/// The terminal in front is in `dir`, and the window has looked for its project.
fn go(ws: &mut Workspace, dir: &Path, cx: &mut gpui_kit::Context<Workspace>) {
    let dir = dir.to_path_buf();
    ws.active_terminal().update(cx, |terminal, _| terminal.cwd = Some(dir));
    refresh(ws, cx);
}

/// A click on one of the Project tab's actions (or its key, or a menu line).
fn run_action(
    ws: &mut Workspace,
    id: &str,
    window: &mut gpui_kit::Window,
    cx: &mut gpui_kit::Context<Workspace>,
) {
    ws.run_project_action(id, window, cx);
}

/// The id of the action the key `keys` runs in the project in front, if any.
fn action_for_key(window: &OpenWindow, keys: &str, cx: &mut TestAppContext) -> Option<String> {
    let keystroke = Keystroke::parse(keys).expect("a keystroke");
    read_cx(cx, window, |ws, cx| ws.project_action_for_key(&keystroke, cx))
}

fn prompt_to_note(
    ws: &mut Workspace,
    id: &str,
    window: &mut gpui_kit::Window,
    cx: &mut gpui_kit::Context<Workspace>,
) {
    ws.prompt_to_note(id, window, cx);
}

fn open_layout(ws: &mut Workspace, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Workspace>) {
    ws.open_project_layout(window, cx);
}

/// The environment and shell a terminal started in `dir` gets from its project.
fn spawn_options(
    cx: &mut TestAppContext,
    dir: &Path,
) -> (std::collections::BTreeMap<String, String>, Option<String>) {
    cx.update(|cx| Workspace::project_spawn_options(Some(dir), cx))
}

/// One round of the notes.md sync for the window.
fn sync_notes(ws: &mut Workspace, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Workspace>) {
    ws.sync_project_notes(window, cx);
}

/// What the hub says of the project at `root`.
fn status(cx: &mut TestAppContext, root: &Path) -> Option<TrustStatus> {
    cx.update(|cx| app(cx).hub.projects.get(root).map(|project| project.status))
}

fn with_hub<R>(cx: &mut TestAppContext, f: impl FnOnce(&ProjectHub) -> R) -> R {
    cx.update(|cx| f(&app(cx).hub))
}

fn trust(cx: &mut TestAppContext, root: &Path) {
    cx.update(|cx| {
        let folder = ProjectFolder::open(root).expect("a project");
        crate::app_state::app_mut(cx).hub.trust.trust(&folder);
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
        ws.sessions[0].terminal().update(cx, |t, _| t.cwd = Some(one.clone()));
        ws.sessions[1].terminal().update(cx, |t, _| t.cwd = Some(two.join("src")));
        refresh(ws, cx);
    });
    assert_eq!(project_of(&window, 0, cx).as_deref(), Some(one.as_path()));
    assert_eq!(project_of(&window, 1, cx).as_deref(), Some(two.as_path()), "from a folder inside it");

    drive(cx, &window, |ws, _, cx| {
        ws.sessions[1].terminal().update(cx, |t, _| t.cwd = Some(bare.clone()));
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
    assert_eq!(read(cx, &window, |ws| ws.right_tab), TabId::Project);

    flag(cx, false);
    assert!(!tabs(cx).contains(&TabId::Project), "gone at once");
    drive(cx, &window, |ws, _, cx| refresh(ws, cx));
    assert_eq!(read(cx, &window, |ws| ws.right_tab), TabId::Changes, "and the window's next look leaves it");
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
    let sessions = |cx: &mut TestAppContext| read(cx, &window, |ws| ws.sessions.len());

    drive(cx, &window, |ws, window, cx| run_action(ws, "more", window, cx));
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
    drive(cx, &window, |ws, window, cx| run_action(ws, "more", window, cx));
    assert_eq!(sessions(cx), 2, "a new session was opened for it");
    drive(cx, &window, |ws, window, cx| run_action(ws, "nope", window, cx));
    assert_eq!(sessions(cx), 2, "an id the project doesn't have does nothing");

    cx.update(|cx| set_feature_for(&dir, Feature::Actions, Some(false), cx));
    cx.run_until_parked();
    drive(cx, &window, |ws, window, cx| {
        refresh(ws, cx);
        run_action(ws, "more", window, cx);
    });
    assert_eq!(sessions(cx), 2, "switched off for the project: nothing runs");
    assert_eq!(action_for_key(&window, "f9", cx), None);
    let menu = read_cx(cx, &window, |ws, cx| ws.menu_state(cx).project_actions);
    assert!(menu.is_empty());

    flag(cx, false);
    cx.update(|cx| set_feature_for(&dir, Feature::Actions, None, cx));
    cx.run_until_parked();
    drive(cx, &window, |ws, window, cx| run_action(ws, "more", window, cx));
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

    cx.update(|cx| set_feature_for(&dir, Feature::Prompts, Some(false), cx));
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
        read(cx, &window, |ws| ws.sessions.iter().filter_map(|s| s.custom_name.clone()).collect::<Vec<_>>())
    };

    drive(cx, &window, open_layout);
    assert!(names(cx).is_empty(), "untrusted: nothing opens");

    trust(cx, &dir);
    drive(cx, &window, |ws, _, cx| refresh(ws, cx));
    drive(cx, &window, open_layout);
    assert_eq!(names(cx), ["web", "api"]);
    assert_eq!(read(cx, &window, |ws| ws.active_session().custom_name.clone()).as_deref(), Some("api"));
    drive(cx, &window, open_layout);
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

    cx.update(|cx| set_feature_for(&dir, Feature::Settings, Some(false), cx));
    assert_eq!(spawn_options(cx, &dir), Default::default(), "Settings switched off for the project");
    cx.update(|cx| set_feature_for(&dir, Feature::Settings, None, cx));
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
    cx.update(|cx| set_feature_for(&dir, Feature::Notes, Some(false), cx));
    cx.run_until_parked();
    drive(cx, &window, |ws, window, cx| {
        notes::add_task(ws, 0, "only here", cx);
        sync_notes(ws, window, cx);
    });
    let file = std::fs::read_to_string(dir.join(".wormhole/notes.md")).expect("notes.md");
    assert!(!file.contains("only here"), "{file}");

    cx.update(|cx| set_feature_for(&dir, Feature::Notes, None, cx));
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

    cx.update(|cx| approve_project(&dir, cx));
    cx.run_until_parked();
    assert_eq!(status(cx, &dir), Some(TrustStatus::Untrusted), "the owner cancelled: still not trusted");
    assert!(!with_hub(cx, |hub| hub.approving), "and nothing is waiting");
    assert!(with_hub(cx, |hub| hub.error.is_none()), "a cancel is not an error");

    trust(cx, &dir);
    drive(cx, &window, |ws, _, cx| refresh(ws, cx));
    assert_eq!(status(cx, &dir), Some(TrustStatus::Trusted));
    cx.update(|cx| revoke_project(&dir, cx));
    cx.run_until_parked();
    assert_eq!(status(cx, &dir), Some(TrustStatus::Untrusted), "revoked at once");

    assert!(with_hub(cx, |hub| hub.prefs.default_enabled(Feature::Layout)));
    cx.update(|cx| set_feature_default(Feature::Layout, false, cx));
    assert!(!with_hub(cx, |hub| hub.prefs.default_enabled(Feature::Layout)));
    cx.update(|cx| set_feature_default(Feature::Layout, true, cx));
}

/// Each profile has its own hub: its preferences, its trust and its notes bases. A window of the
/// shown profile reads the shown profile's, and a switch brings the other's back as it was left.
#[gpui_kit::test]
fn each_profile_has_its_own_project_hub(cx: &mut TestAppContext) {
    if isolate("each_profile_has_its_own_project_hub") {
        return;
    }
    let _window = start(cx, 1);
    cx.update(|cx| set_feature_default(Feature::Prompts, false, cx));
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
    cx.update(|cx| set_feature_default(Feature::Layout, false, cx));

    cx.update(|cx| AppState::switch_profile(&ProfileId::default_profile(), cx)).expect("switches back");
    cx.run_until_parked();
    assert!(!with_hub(cx, |hub| hub.prefs.default_enabled(Feature::Prompts)), "the first profile's, as left");
    assert!(with_hub(cx, |hub| hub.prefs.default_enabled(Feature::Layout)), "Work's change stayed in Work");
    assert_eq!(with_hub(cx, |hub| hub.trust.status(&folder)), TrustStatus::Trusted);
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
    assert!(disabled(cx), "no folder");
    let dir = project("menu", &[]);
    drive(cx, &window, |ws, _, cx| go(ws, &dir, cx));
    assert!(!disabled(cx));
    flag(cx, false);
    assert!(disabled(cx), "the feature off");
}
