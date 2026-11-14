//! What the Project tab shows of a folder, and that what is not allowed is not listed.

use super::{Owner, RUN_TESTS, Repo, service};
use crate::files_panel::{
    ApproveOutcome, OwnerAnswer, ProjectEffect, ProjectFailure, TerminalPlace, TrustStatusRecord,
};

#[test]
fn a_folder_without_a_project_has_no_state_and_nothing_to_run() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::create_dir(dir.path().join(".git")).expect("git");
    let (_data, service) = service(&Owner::answering([]));
    assert_eq!(service.project(dir.path().to_string_lossy().into_owned()), None);
    let root = dir.path().to_string_lossy().into_owned();
    assert!(matches!(service.run_action(root.clone(), "x".into()), Err(ProjectFailure::NotFound { .. })));
    assert!(matches!(service.open_layout(root), Err(ProjectFailure::NotFound { .. })));
}

#[test]
fn an_untrusted_project_asks_for_trust_lists_what_it_would_run_and_lists_no_action() {
    let repo = Repo::new(&[
        ("actions.json", RUN_TESTS),
        ("config.json", r#"{"env": {"A": "1"}, "font_size": 14}"#),
        ("layout.json", r#"{"sessions": [{"name": "dev", "run": "make dev"}]}"#),
        ("prompts/review.md", "# Review\nLook at the diff."),
    ]);
    let (_data, service) = service(&Owner::answering([]));
    let state = service.project(repo.root()).expect("a project");
    assert_eq!(state.root, repo.root());
    assert_eq!(state.status, TrustStatusRecord::Untrusted);
    assert!(state.asks_for_trust);
    let runs: Vec<&str> = state.commands.iter().map(|line| line.runs.as_str()).collect();
    assert!(runs.contains(&"cargo test") && runs.contains(&"A=1") && runs.contains(&"make dev"), "{runs:?}");
    assert_eq!(
        state.applies,
        "its settings in config.json, actions in actions.json and terminal layout in layout.json"
    );
    assert!(state.actions.is_empty() && state.layout.is_empty(), "held back until trusted");
    assert_eq!(state.prompts.len(), 1, "prompts only paste: they need no trust");
    assert_eq!(state.prompts[0].title, "Review");
    let hints: Vec<(&str, &str, bool)> =
        state.features.iter().map(|f| (f.key.as_str(), f.hint.as_str(), f.warns)).collect();
    assert!(hints.contains(&("actions", "Needs trust before it takes effect", true)), "{hints:?}");
    assert!(hints.contains(&("notes", "Syncs once the project is trusted", true)), "{hints:?}");
    assert!(hints.contains(&("prompts", "Reusable prompts from the prompts folder", false)), "{hints:?}");
    assert!(!hints.iter().any(|(key, ..)| *key == "theme"), "a file that is not there has no switch");
}

#[test]
fn the_project_is_found_from_a_folder_below_it_and_stops_at_the_repository_root() {
    let repo = Repo::new(&[("actions.json", RUN_TESTS)]);
    std::fs::create_dir_all(repo.path().join("crates/app")).expect("mkdir");
    let (_data, service) = service(&Owner::answering([]));
    let below = repo.path().join("crates/app").to_string_lossy().into_owned();
    assert_eq!(service.project(below).expect("found").root, repo.root());
    // A project above the repository does not leak into a nested checkout.
    let nested = repo.path().join("nested");
    std::fs::create_dir_all(nested.join(".git")).expect("nested repo");
    assert_eq!(service.project(nested.to_string_lossy().into_owned()), None);
}

#[test]
fn a_trusted_project_runs_its_actions_as_effects_and_a_switch_turns_them_off() {
    let repo = Repo::new(&[(
        "actions.json",
        r#"[
            {"id": "test", "title": "Run tests", "type": "run", "command": "cargo test", "key": "cmd-ctrl-t", "cwd": "crates"},
            {"id": "open", "title": "Docs", "type": "open", "url": "https://example.com/docs"},
            {"id": "ask", "title": "Ask", "type": "prompt", "text": "Explain this", "target": "new_session", "submit": true},
            {"id": "all", "title": "All", "type": "chain", "steps": ["test", "open", {"type": "run", "command": "echo hi", "target": "session:dev"}]}
        ]"#,
    )]);
    let owner = Owner::answering([OwnerAnswer::Approved]);
    let (_data, service) = service(&owner);
    service.project(repo.root());
    assert!(matches!(service.run_action(repo.root(), "test".into()), Err(ProjectFailure::Refused { .. })));
    assert_eq!(service.approve(repo.root()), ApproveOutcome::Approved);
    let state = service.project(repo.root()).expect("a project");
    assert_eq!(state.status, TrustStatusRecord::Trusted);
    let titles: Vec<&str> = state.actions.iter().map(|a| a.title.as_str()).collect();
    assert_eq!(titles, ["Run tests", "Docs", "Ask", "All"]);
    assert_eq!(state.actions[0].key.as_deref(), Some("\u{2303}\u{2318}T"));
    assert_eq!(state.actions[0].detail.as_deref(), Some("cargo test"));
    assert_eq!(state.actions[3].detail.as_deref(), Some("3 steps"));

    assert_eq!(
        service.run_action(repo.root(), "test".into()),
        Ok(vec![ProjectEffect::Command { place: TerminalPlace::Current, command: "cargo test".into() }])
    );
    assert_eq!(
        service.run_action(repo.root(), "all".into()).expect("chain"),
        vec![
            ProjectEffect::Command { place: TerminalPlace::Current, command: "cargo test".into() },
            ProjectEffect::OpenUrl { url: "https://example.com/docs".into() },
            ProjectEffect::Command {
                place: TerminalPlace::Named { name: "dev".into(), cwd: Some(repo.root()) },
                command: "echo hi".into()
            },
        ]
    );
    assert_eq!(
        service.run_action(repo.root(), "ask".into()).expect("prompt"),
        vec![ProjectEffect::Paste {
            place: TerminalPlace::New { cwd: Some(repo.root()) },
            text: "Explain this".into(),
            submit: true
        }]
    );
    assert!(matches!(service.run_action(repo.root(), "nope".into()), Err(ProjectFailure::NotFound { .. })));

    service.set_feature(repo.root(), "actions".into(), false).expect("switch");
    let refused = service.run_action(repo.root(), "test".into());
    assert!(
        matches!(&refused, Err(ProjectFailure::Refused { reason }) if reason.contains("switched off")),
        "{refused:?}"
    );
    assert!(service.project(repo.root()).expect("project").actions.is_empty(), "off: not listed either");
    assert_eq!(
        service.set_feature(repo.root(), "nonsense".into(), true),
        Err(ProjectFailure::NotFound { what: "nonsense".into() })
    );
}

#[test]
fn a_projects_settings_go_to_settings_only_while_its_switch_and_trust_allow_them() {
    let repo = Repo::new(&[("config.json", r#"{"font_size": 20}"#)]);
    let (_data, service) = service(&Owner::answering([OwnerAnswer::Approved]));
    let below = repo.root();
    let found = service.settings_overrides(below.clone()).expect("a project");
    assert_eq!((found.dir.as_str(), found.root.as_str()), (".wormhole", repo.root().as_str()));
    assert!(found.note.contains("config.json") && found.note.contains(&found.name), "{}", found.note);
    assert_eq!(found.json, None, "held back until the person trusts the project");
    assert_eq!(service.approve(repo.root()), ApproveOutcome::Approved);
    let json = service.settings_overrides(below.clone()).and_then(|found| found.json).expect("its settings");
    assert!(json.contains("\"font_size\":20"), "{json}");
    // Switched off by the person, it sets nothing; no project at all is none.
    service.set_feature(repo.root(), "settings".into(), false).expect("the switch");
    assert_eq!(service.settings_overrides(below).expect("still a project").json, None);
    let plain = tempfile::tempdir().expect("temp dir");
    std::fs::create_dir(plain.path().join(".git")).expect("git");
    assert_eq!(service.settings_overrides(plain.path().to_string_lossy().into_owned()), None);
}

/// A window-level key asks the core each time: untrusted or switched off, the key is nobody's and the terminal hears it.
#[test]
fn an_actions_key_is_the_projects_only_while_it_is_trusted_and_on() {
    let repo = Repo::new(&[(
        "actions.json",
        r#"[{"id": "test", "title": "Run tests", "type": "run", "command": "cargo test", "key": "cmd-ctrl-t"}]"#,
    )]);
    let (_data, service) = service(&Owner::answering([OwnerAnswer::Approved]));
    let key = || service.action_for_key(repo.root(), "\u{2303}\u{2318}T".into());
    service.project(repo.root());
    assert_eq!(key(), None, "not trusted: the key goes on");
    assert_eq!(service.approve(repo.root()), ApproveOutcome::Approved);
    assert_eq!(key().as_deref(), Some("test"));
    assert_eq!(service.action_for_key(repo.root(), "\u{2318}Y".into()), None, "another key is nobody's");
    service.set_feature(repo.root(), "actions".into(), false).expect("switch");
    assert_eq!(key(), None, "switched off: the key goes on");
    service.revoke(repo.root());
    service.set_feature(repo.root(), "actions".into(), true).expect("switch");
    assert_eq!(key(), None, "trust taken back");
}

#[test]
fn a_project_in_a_folder_whose_name_is_not_text_is_found_and_approved_by_its_key() {
    use std::os::unix::ffi::OsStrExt as _;

    use crate::files_panel::names;

    let parent = tempfile::tempdir().expect("temp dir");
    let folder = parent.path().join(std::ffi::OsStr::from_bytes(b"caf\xe9"));
    // APFS and HFS+ refuse a name that is not UTF-8: on such a disk there is no such folder to read.
    if std::fs::create_dir(&folder).is_err() {
        return;
    }
    std::fs::create_dir(folder.join(".git")).expect("git");
    super::write(&folder.join(".wormhole/config.json"), "{}");
    let (_data, service) = service(&Owner::answering([OwnerAnswer::Approved]));
    let key = names::path_key(&folder);
    let state =
        service.project(key).expect("found by the key of its bytes, not a text that names another folder");
    assert_eq!(
        names::folder(&state.root),
        folder.canonicalize().expect("canonical"),
        "the root is handed back as a key"
    );
    assert_eq!(service.approve(state.root.clone()), ApproveOutcome::Approved);
    let again =
        service.project(names::path_key(&folder.join("sub-that-is-not-there"))).map(|state| state.status);
    assert_eq!(again, None, "a folder that is not there has no project");
    assert_eq!(service.project(state.root).map(|state| state.status), Some(TrustStatusRecord::Trusted));
}

/// Settings and the theme follow the terminal in front by its folder's key. A folder whose name is not text is read
/// by its bytes; read by its name as text it would name a sibling a repository can plant (`caf\u{FFFD}` beside
/// `caf\xE9`), whose settings, once the person trusted that one, would be laid over theirs.
#[test]
fn a_projects_settings_and_theme_are_read_by_the_key_of_a_folder_whose_name_is_not_text() {
    use std::os::unix::ffi::OsStrExt as _;

    use crate::files_panel::names;

    let parent = tempfile::tempdir().expect("temp dir");
    let folder = parent.path().join(std::ffi::OsStr::from_bytes(b"caf\xe9"));
    // APFS and HFS+ refuse a name that is not UTF-8: on such a disk there is no such folder to read.
    if std::fs::create_dir(&folder).is_err() {
        return;
    }
    let planted = parent.path().join("caf\u{FFFD}");
    for (dir, size, theme) in [(&folder, 20, "own"), (&planted, 30, "planted")] {
        std::fs::create_dir_all(dir.join(".git")).expect("git");
        super::write(&dir.join(".wormhole/config.json"), &format!(r#"{{"font_size": {size}}}"#));
        super::write(&dir.join(".wormhole/theme.json"), &format!(r#"{{"name": "{theme}"}}"#));
    }
    let (_data, service) = service(&Owner::answering([OwnerAnswer::Approved, OwnerAnswer::Approved]));
    let planted_root = planted.canonicalize().expect("canonical").to_string_lossy().into_owned();
    assert_eq!(
        service.approve(planted_root),
        ApproveOutcome::Approved,
        "the person trusted the other folder"
    );

    let key = names::path_key(&folder);
    let found = service.settings_overrides(key.clone()).expect("found by the key of its bytes");
    assert_eq!(
        names::folder(&found.root),
        folder.canonicalize().expect("canonical"),
        "the root comes back as a key"
    );
    assert_eq!(
        found.json, None,
        "its own project is not trusted: nothing is laid over the person's settings"
    );
    let theme = service.theme_overlay(key.clone()).expect("its theme, which asks no trust");
    assert!(theme.contains("own") && !theme.contains("planted"), "{theme}");

    // What the drawn folder would have done: its U+FFFD names the planted one, trusted, and its settings.
    let drawn = folder.to_string_lossy().into_owned();
    let wrong =
        service.settings_overrides(drawn).and_then(|found| found.json).expect("the planted one's settings");
    assert!(wrong.contains("\"font_size\":30"), "{wrong}");

    assert_eq!(service.approve(found.root), ApproveOutcome::Approved);
    let json =
        service.settings_overrides(key).and_then(|found| found.json).expect("its own settings, once trusted");
    assert!(json.contains("\"font_size\":20"), "{json}");
}
