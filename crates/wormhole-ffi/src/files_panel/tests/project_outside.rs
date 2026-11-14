//! The approvals changed by another program (the file written with no service of this process in between): the window
//! is told as an event, and refuses the project from then on.

use std::time::{Duration, Instant};

use wormhole_core::project::TrustStore;

use super::{Owner, RUN_TESTS, Repo, service};
use crate::files_panel::listener::testing::Counting;
use crate::files_panel::{ApproveOutcome, OwnerAnswer, ProjectFailure, TrustStatusRecord};

/// Well inside the tab's own slow look (30 s while its folder is watched) and the title row's (a minute).
const TOLD_WITHIN: Duration = Duration::from_secs(10);

fn told(heard: &Counting, times: u32) -> bool {
    let until = Instant::now() + TOLD_WITHIN;
    while heard.count() < times && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(20));
    }
    heard.count() >= times
}

#[cfg(target_os = "macos")]
#[test]
fn a_project_revoked_by_another_program_is_told_to_the_window_and_runs_nothing() {
    let repo = Repo::new(&[("actions.json", RUN_TESTS)]);
    let owner = Owner::answering([OwnerAnswer::Approved]);
    let (data, window) = service(&owner);
    let heard = Counting::new();
    let watch = window.watch(repo.root(), heard.clone());
    window.project(repo.root());
    assert_eq!(window.approve(repo.root()), ApproveOutcome::Approved);
    // The approval tells the tab itself; its own write to the file is then told once more, as any write is.
    assert!(told(&heard, 1), "the approval is told");
    watch.drain();
    assert!(told(&heard, 2), "the approvals' file is followed: the approval's own write is an event");
    watch.drain();
    assert!(window.run_action(repo.root(), "test".into()).is_ok(), "trusted, the action runs");

    let mut outside = TrustStore::load(data.path()).expect("approvals");
    assert!(outside.revoke(&repo.path()), "the approval was on disk");
    outside.save().expect("saved");

    assert!(
        told(&heard, 3),
        "the window is told of a revoke another program wrote, without waiting for its slow look"
    );
    assert_eq!(window.project(repo.root()).expect("project").status, TrustStatusRecord::Untrusted);
    assert!(
        matches!(window.run_action(repo.root(), "test".into()), Err(ProjectFailure::Refused { .. })),
        "and nothing of the project runs"
    );
}
