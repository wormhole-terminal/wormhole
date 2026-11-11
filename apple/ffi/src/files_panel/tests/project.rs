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
