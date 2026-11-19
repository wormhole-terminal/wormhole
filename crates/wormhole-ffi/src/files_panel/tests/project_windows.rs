//! Two windows of one profile, each with a service of its own over the same data folder (as two windows of the app are):
//! what one changes of the approvals, the switches or a project's notes.md, the other neither undoes nor misses.

use std::fs;
use std::sync::{Arc, Barrier};

use wormhole_core::project::{Feature, ProjectPrefs, TrustStore};

use super::{Owner, RUN_TESTS, Repo, service};
use crate::files_panel::{
    ApproveOutcome, NotesDocRecord, OwnerAnswer, ProjectFailure, ProjectService, TaskDocRecord,
    TrustStatusRecord,
};

fn status(service: &ProjectService, repo: &Repo) -> TrustStatusRecord {
    service.project(repo.root()).expect("a project").status
}

/// The approvals as the file has them, read as a launch would.
fn on_disk(data: &std::path::Path, repo: &Repo) -> TrustStatusRecord {
    let folder = wormhole_core::project::ProjectFolder::open(&repo.path()).expect("project");
    TrustStore::load(data).expect("approvals").status(&folder).into()
}

#[test]
fn a_project_revoked_in_one_window_stays_revoked_when_the_other_approves_another() {
    let p = Repo::new(&[("actions.json", RUN_TESTS)]);
    let q = Repo::new(&[("actions.json", RUN_TESTS)]);
    let owner = Owner::answering([OwnerAnswer::Approved, OwnerAnswer::Approved]);
    let (data, first) = service(&owner);
    let second = ProjectService::in_dir(data.path(), owner.clone());
    first.project(p.root());
    assert_eq!(first.approve(p.root()), ApproveOutcome::Approved);
    // The second window shows P trusted and Q waiting; it reads nothing more before its approval.
    assert_eq!(status(&second, &p), TrustStatusRecord::Trusted);
    assert_eq!(status(&second, &q), TrustStatusRecord::Untrusted);
    first.revoke(p.root());
    assert_eq!(second.approve(q.root()), ApproveOutcome::Approved);

    assert_eq!(
        on_disk(data.path(), &p),
        TrustStatusRecord::Untrusted,
        "the second window's approval did not write P back"
    );
    assert_eq!(on_disk(data.path(), &q), TrustStatusRecord::Trusted);
    for (window, name) in [(&first, "first"), (&second, "second")] {
        assert_eq!(status(window, &p), TrustStatusRecord::Untrusted, "the {name} window reads P revoked");
        assert!(
            matches!(window.run_action(p.root(), "test".into()), Err(ProjectFailure::Refused { .. })),
            "and the {name} window runs nothing of it"
        );
        assert!(window.run_action(q.root(), "test".into()).is_ok(), "Q runs in the {name} window");
    }
}

#[test]
fn a_revoked_project_runs_nothing_and_syncs_no_notes_in_the_window_that_still_shows_it() {
    let repo = Repo::new(&[
        ("actions.json", RUN_TESTS),
        ("notes.md", "# Notes\n\n## Next steps\n\n- [ ] from the repo\n"),
    ]);
    let owner = Owner::answering([OwnerAnswer::Approved]);
    let (data, settings) = service(&owner);
    let window = ProjectService::in_dir(data.path(), owner.clone());
    settings.project(repo.root());
    assert_eq!(settings.approve(repo.root()), ApproveOutcome::Approved);
    // The window shows the project trusted and has merged its terminal once.
    assert!(window.project(repo.root()).expect("project").actions.iter().any(|a| a.id == "test"));
    window.sync_notes(1, repo.root(), notes(&[("mine", false)]), false);
    let before = fs::read_to_string(repo.path().join(".wormhole/notes.md")).expect("notes.md");

    settings.revoke(repo.root());

    // Asked straight after, before its tab read anything again.
    assert!(matches!(window.run_action(repo.root(), "test".into()), Err(ProjectFailure::Refused { .. })));
    let out =
        window.sync_notes(1, repo.root(), notes(&[("mine", false), ("after the revoke", false)]), false);
    assert_eq!(out, Default::default(), "nothing of the file comes in");
    assert_eq!(
        fs::read_to_string(repo.path().join(".wormhole/notes.md")).expect("notes.md"),
        before,
        "nothing is written"
    );
    let state = window.project(repo.root()).expect("project");
    assert_eq!(state.status, TrustStatusRecord::Untrusted);
    assert!(state.actions.is_empty(), "the tab offers none of its actions");
    assert!(
        state.errors.iter().any(|e| e.starts_with("Notes sync: waits for the project to be trusted")),
        "the tab says why the notes wait: {:?}",
        state.errors
    );
}

#[test]
fn a_switch_turned_off_in_one_window_stays_off_after_the_other_saves() {
    let repo = Repo::new(&[
        ("actions.json", RUN_TESTS),
        ("prompts/review.md", "---\ntitle: Review it\n---\nLook at the diff.\n"),
        ("notes.md", "# Notes\n"),
    ]);
    let owner = Owner::answering([]);
    let (data, first) = service(&owner);
    let second = ProjectService::in_dir(data.path(), owner.clone());
    // Both windows have read the project's switches (all on).
    first.project(repo.root());
    second.project(repo.root());

    first.set_feature(repo.root(), "notes".into(), false).expect("notes off");
    second.set_feature(repo.root(), "prompts".into(), false).expect("prompts off");
    second.set_project_default("theme".into(), false).expect("a default");

    let saved = ProjectPrefs::load(data.path()).expect("prefs");
    assert_eq!(
        saved.project_override(&repo.path(), Feature::Notes),
        Some(false),
        "the second window's saves kept it"
    );
    assert_eq!(saved.project_override(&repo.path(), Feature::Prompts), Some(false));
    for (window, name) in [(&first, "first"), (&second, "second")] {
        let state = window.project(repo.root()).expect("project");
        let on = |key: &str| state.features.iter().find(|f| f.key == key).map(|f| f.on);
        assert_eq!(
            (on("notes"), on("prompts")),
            (Some(false), Some(false)),
            "the {name} window shows both off"
        );
    }
    // Off means off: the second window does not merge the project's notes.md either.
    let before = fs::read_to_string(repo.path().join(".wormhole/notes.md")).expect("notes.md");
    second.sync_notes(2, repo.root(), notes(&[("mine", false)]), false);
    assert_eq!(fs::read_to_string(repo.path().join(".wormhole/notes.md")).expect("notes.md"), before);
}

#[test]
fn two_windows_adding_a_step_each_to_one_projects_notes_keep_both() {
    let repo = Repo::new(&[("notes.md", "# Notes\n\n## Next steps\n\n- [ ] from the repo\n")]);
    let owner = Owner::answering([OwnerAnswer::Approved]);
    let (data, first) = service(&owner);
    let second = ProjectService::in_dir(data.path(), owner.clone());
    first.project(repo.root());
    assert_eq!(first.approve(repo.root()), ApproveOutcome::Approved);

    // Terminal 1 in the first window, terminal 2 in the second, both in the project; each adds a step a round and both
    // merge at the same moment, as their timers may.
    const ROUNDS: usize = 12;
    let both = Barrier::new(2);
    let run = |window: &Arc<ProjectService>, session: i64, name: &str| {
        let mut local = notes(&[]);
        for round in 0..ROUNDS {
            local.tasks.push(TaskDocRecord { text: format!("{name} {round}"), done: false });
            both.wait();
            if let Some(applied) = window.sync_notes(session, repo.root(), local.clone(), false).apply {
                local = applied;
            }
        }
        local
    };
    let (one, two) = std::thread::scope(|scope| {
        let one = scope.spawn(|| run(&first, 1, "first"));
        let two = scope.spawn(|| run(&second, 2, "second"));
        (one.join().expect("first window"), two.join().expect("second window"))
    });
    // Settled: each terminal merges once more and takes what the other wrote.
    let one = first.sync_notes(1, repo.root(), one.clone(), false).apply.unwrap_or(one);
    second.sync_notes(2, repo.root(), two, false);
    first.sync_notes(1, repo.root(), one, false);

    let file = fs::read_to_string(repo.path().join(".wormhole/notes.md")).expect("notes.md");
    for round in 0..ROUNDS {
        for name in ["first", "second"] {
            let step = format!("- [ ] {name} {round}\n");
            assert_eq!(file.matches(&step).count(), 1, "{step:?} is in notes.md once:\n{file}");
        }
    }
    assert_eq!(file.matches("from the repo").count(), 1, "{file}");
}

fn notes(tasks: &[(&str, bool)]) -> NotesDocRecord {
    NotesDocRecord {
        tasks: tasks
            .iter()
            .map(|(text, done)| TaskDocRecord { text: text.to_string(), done: *done })
            .collect(),
        notes: Vec::new(),
    }
}
