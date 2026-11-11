use std::path::Path;

use crate::aux_backends::common::test_support::{Wakes, scratch};
use crate::aux_backends::{AuxError, ReviewStatusRecord, TrustDesk};

fn project(root: &Path, config: &str) {
    let dir = root.join("api").join(".wormhole");
    std::fs::create_dir_all(&dir).expect("project folder");
    std::fs::write(dir.join("config.json"), config).expect("config");
}

fn desk(root: &Path, data: &Path, wakes: &std::sync::Arc<Wakes>) -> std::sync::Arc<TrustDesk> {
    TrustDesk::new(
        root.join("api").display().to_string(),
        None,
        Some(data.display().to_string()),
        wakes.clone(),
    )
    .expect("a desk")
}

#[test]
fn a_folder_outside_the_experiment_is_refused() {
    let err = TrustDesk::new("/tmp/x".into(), None, Some("/Users/x/Wormhole".into()), Wakes::new()).err();
    assert!(matches!(err, Some(AuxError::NotIsolated { .. })));
}

#[test]
fn what_a_project_would_run_is_listed_and_approval_is_for_what_was_shown() {
    let (root, data) = scratch();
    project(root.path(), r#"{"shell": "/bin/zsh", "env": {"FOO": "bar"}}"#);
    let wakes = Wakes::new();
    let desk = desk(root.path(), &data, &wakes);

    let review = desk.review();
    assert_eq!((review.name.as_str(), review.directory.as_str()), ("api", ".wormhole"));
    assert_eq!(review.status, ReviewStatusRecord::Untrusted);
    assert!(review.problem.is_none());
    let lines: Vec<String> = review.lines.iter().map(|l| format!("{} {}", l.what, l.runs)).collect();
    assert!(lines.contains(&"shell: /bin/zsh".to_string()), "{lines:?}");
    assert!(lines.contains(&"env: FOO=bar".to_string()), "{lines:?}");

    assert!(desk.approve_reviewed().is_err(), "the review is held before the owner is asked");
    assert!(desk.hold_reviewed());
    desk.approve_reviewed().expect("approved");
    assert!(desk.approve_reviewed().is_err(), "a hold is used once");
    assert!(wakes.count() >= 1);
    assert_eq!(desk.review().status, ReviewStatusRecord::Trusted);
    assert!(data.join("trusted-projects.json").is_file(), "kept in the profile's folder");

    // A `git pull` changes what runs: it reads "changed" until the new content is reviewed and approved.
    project(root.path(), r#"{"shell": "/bin/zsh", "env": {"FOO": "bar", "EVIL": "1"}}"#);
    let review = desk.review();
    assert_eq!(review.status, ReviewStatusRecord::Changed);
    assert!(review.lines.iter().any(|l| l.runs == "EVIL=1"));

    // The approval binds to the request that was shown, not to what is on disk by the time it lands.
    assert!(desk.hold_reviewed());
    project(root.path(), r#"{"shell": "/bin/zsh", "env": {"FOO": "bar", "EVIL": "2"}}"#);
    desk.approve_reviewed().expect("approves the one shown");
    assert_eq!(desk.review().status, ReviewStatusRecord::Changed, "the files moved on since");
}

#[test]
fn nothing_is_approved_without_a_review_and_a_broken_project_has_no_lines() {
    let (root, data) = scratch();
    let wakes = Wakes::new();
    let missing = desk(root.path(), &data, &wakes);
    assert!(!missing.hold_reviewed(), "nothing was shown");
    assert!(missing.approve_reviewed().is_err());
    let review = missing.review();
    assert!(review.problem.is_some(), "no .wormhole folder");
    assert!(missing.approve_reviewed().is_err());

    project(root.path(), "{ not json");
    let broken = desk(root.path(), &data, &wakes);
    let review = broken.review();
    assert!(review.problem.is_some());
    assert!(review.lines.is_empty());
    assert!(broken.approve_reviewed().is_err(), "a project that does not load cannot be trusted");
}

#[test]
fn revoking_takes_the_approval_back() {
    let (root, data) = scratch();
    project(root.path(), r#"{"shell": "/bin/zsh"}"#);
    let desk = desk(root.path(), &data, &Wakes::new());
    desk.review();
    assert!(desk.hold_reviewed());
    desk.approve_reviewed().expect("approved");
    desk.revoke().expect("revoked");
    assert_eq!(desk.review().status, ReviewStatusRecord::Untrusted);
}
