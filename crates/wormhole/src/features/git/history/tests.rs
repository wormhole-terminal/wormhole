use std::path::PathBuf;
use std::rc::Rc;

use super::state::{CommitInfo, DETAILS_KEPT};
use super::{CommitFiles, FILES_KEPT, Fetched, HistoryKey, HistoryRow, HistoryState};
use crate::window::repo::job::testing::ManualExecutor;
use wormhole_core::git::{ChangeKind, Commit, CommitDetails, FileChange, GitError, Signature};

fn commit(sha: &str) -> Commit {
    Commit {
        sha: sha.into(),
        short: sha[..1].into(),
        parent: None,
        merge: false,
        author: "a".into(),
        time: 0,
        subject: format!("commit {sha}"),
    }
}

fn file(path: &str) -> FileChange {
    FileChange {
        path: path.into(),
        original_path: None,
        kind: ChangeKind::Modified,
        added: Some(1),
        removed: Some(0),
        lfs: false,
        submodule: None,
    }
}

fn key(rev: Option<&str>) -> HistoryKey {
    (PathBuf::from("/repo"), rev.map(String::from))
}

fn log_of(shas: &[&str]) -> Fetched {
    Ok(Some((PathBuf::from("/repo"), Ok(shas.iter().map(|s| commit(s)).collect()))))
}

/// A session's history plus what the session shows now (the key a result is checked against).
struct Session {
    history: HistoryState,
    shown: HistoryKey,
}

fn refresh(session: &mut Session, exec: &mut ManualExecutor<Session>, result: Fetched) {
    let key = session.shown.clone();
    let Some(wanted) = session.history.begin_refresh(key) else { return };
    let ticket = wanted.ticket();
    exec.spawn(
        move || result,
        move |session: &mut Session, result| {
            let now = session.shown.clone();
            session.history.finish_refresh(ticket, Some(&now), result);
        },
    );
}

fn shas(history: &HistoryState) -> Vec<&str> {
    history.commits.iter().map(|c| c.sha.as_str()).collect()
}

#[test]
fn the_old_branchs_log_never_lands_under_the_new_one() {
    let mut session = Session { history: HistoryState::for_rev(Some("a".into())), shown: key(Some("a")) };
    let mut exec = ManualExecutor::default();
    refresh(&mut session, &mut exec, log_of(&["a1", "a2"]));
    // The branch is switched before the first log came back: a new state, a new request.
    session.history = HistoryState::for_rev(Some("b".into()));
    session.shown = key(Some("b"));
    refresh(&mut session, &mut exec, log_of(&["b1"]));

    // The old request answers while the new one is still running: it must not end the new one's wait...
    exec.deliver(0, &mut session);
    assert!(session.history.loading());
    assert!(session.history.commits.is_empty());
    // ...and when the new one lands, it is the new one that is shown.
    exec.deliver(1, &mut session);
    assert_eq!(shas(&session.history), ["b1"]);
    assert!(!session.history.loading());
}

#[test]
fn a_slow_old_request_landing_last_is_dropped() {
    let mut session = Session { history: HistoryState::for_rev(Some("a".into())), shown: key(Some("a")) };
    let mut exec = ManualExecutor::default();
    refresh(&mut session, &mut exec, log_of(&["a1"]));
    session.history = HistoryState::for_rev(Some("b".into()));
    session.shown = key(Some("b"));
    refresh(&mut session, &mut exec, log_of(&["b1"]));
    exec.deliver(1, &mut session);
    exec.deliver(0, &mut session);
    assert_eq!(shas(&session.history), ["b1"]);
}

#[test]
fn a_branch_reset_from_outside_drops_the_result_in_flight() {
    // The tab code resets `rev` directly; the result was asked for the old one.
    let mut session = Session { history: HistoryState::for_rev(Some("a".into())), shown: key(Some("a")) };
    let mut exec = ManualExecutor::default();
    refresh(&mut session, &mut exec, log_of(&["a1"]));
    session.history.rev = None;
    session.shown = key(None);
    exec.deliver(0, &mut session);
    assert!(session.history.commits.is_empty());
    // The request is over, so the next poll can ask for what is shown now.
    assert!(!session.history.loading());
    refresh(&mut session, &mut exec, log_of(&["head1"]));
    exec.deliver(1, &mut session);
    assert_eq!(shas(&session.history), ["head1"]);
}

#[test]
fn a_refresh_that_changes_nothing_reports_no_change() {
    let mut history = HistoryState::default();
    let k = key(None);
    let t = history.begin_refresh(k.clone()).expect("started").ticket();
    assert!(history.finish_refresh(t, Some(&k), log_of(&["a"])));
    let t = history.begin_refresh(k.clone()).expect("started").ticket();
    assert!(!history.finish_refresh(t, Some(&k), log_of(&["a"])));
}

#[test]
fn git_that_cannot_read_the_folder_is_a_problem_not_an_empty_repository() {
    let mut history = loaded();
    let k = key(None);
    let t = history.begin_refresh(k.clone()).expect("started").ticket();
    assert!(history.finish_refresh(t, Some(&k), Err(GitError::GitMissing)));
    assert_eq!((history.root.as_ref(), history.commits.len()), (None, 0));
    assert_eq!(history.problem, Some(GitError::GitMissing));
    assert!(history.waits_for_retry());
    // Found again, with the same log as before: that is still a change, the problem goes.
    let t = history.begin_refresh(k.clone()).expect("started").ticket();
    assert!(history.finish_refresh(t, Some(&k), log_of(&["aaa", "bbb"])));
    assert_eq!((history.problem.as_ref(), history.commits.len()), (None, 2));
    assert!(!history.waits_for_retry());
    // Not a repository is no problem, just nothing to show.
    let t = history.begin_refresh(k.clone()).expect("started").ticket();
    history.finish_refresh(t, Some(&k), Ok(None));
    assert_eq!((history.root.as_ref(), history.problem.as_ref()), (None, None));
}

#[test]
fn a_folder_git_has_not_answered_for_is_not_called_no_repository() {
    // A terminal opened (or switched to) while the History tab shows: nothing has been asked yet,
    // so there is no repository to show, but that is not git saying "not a repository".
    let mut history = HistoryState::default();
    let repo = PathBuf::from("/repo");
    assert!(!history.answered_for(&repo));
    let k = key(None);
    let t = history.begin_refresh(k.clone()).expect("started").ticket();
    assert!(!history.answered_for(&repo), "still asking");
    history.finish_refresh(t, Some(&k), Ok(None));
    assert!(history.answered_for(&repo), "git said: not a repository");
    // Asking again doesn't take the answer back (no "Reading…" flicker on every poll).
    let t = history.begin_refresh(k.clone()).expect("started").ticket();
    assert!(history.answered_for(&repo));
    history.finish_refresh(t, Some(&k), Ok(None));
    // Another folder, or another branch of this one, is a new question.
    assert!(!history.answered_for(&PathBuf::from("/elsewhere")));
    history.rev = Some("main".into());
    assert!(!history.answered_for(&repo));

    // An answer for a folder that is no longer on screen answers nothing.
    let mut history = HistoryState::default();
    let t = history.begin_refresh(k.clone()).expect("started").ticket();
    history.finish_refresh(t, Some(&(PathBuf::from("/elsewhere"), None)), Ok(None));
    assert!(!history.answered_for(&repo));
    assert!(!history.answered_for(&PathBuf::from("/elsewhere")));
}

/// A history of two commits in a repository, as a refresh leaves it.
fn loaded() -> HistoryState {
    let mut history = HistoryState::default();
    let k = key(None);
    let t = history.begin_refresh(k.clone()).expect("started").ticket();
    history.finish_refresh(t, Some(&k), log_of(&["aaa", "bbb"]));
    history
}

/// The palette's commit keeps the branch History shows when the commit is on it, and
/// goes back to HEAD when it isn't; either way the commit is open and its files are fetched.
#[test]
fn a_revealed_commit_keeps_the_branch_only_when_it_is_on_it() {
    let root = PathBuf::from("/repo");
    let mut history = HistoryState::for_rev(Some("feature".into()));
    assert!(history.open(root.clone(), commit("ccc"), true).is_some());
    assert_eq!(history.rev.as_deref(), Some("feature"));
    assert_eq!(history.expanded.as_deref(), Some("ccc"));
    let mut history = HistoryState::for_rev(Some("feature".into()));
    assert!(history.open(root, commit("ddd"), false).is_some());
    assert_eq!(history.rev, None, "HEAD");
    assert_eq!(history.expanded.as_deref(), Some("ddd"));
}

#[test]
fn expanding_fetches_the_files_once_however_often_it_is_clicked() {
    let mut history = loaded();
    assert!(history.toggle("aaa").is_some());
    // Collapse and expand again before the files came back: the fetch on its way is awaited.
    assert!(history.toggle("aaa").is_none());
    assert!(history.toggle("aaa").is_none());
    assert_eq!(history.expanded.as_deref(), Some("aaa"));
    history.files_loaded("aaa", Ok(vec![file("x.rs")]));
    assert_eq!(history.rows().iter().filter(|r| matches!(r, HistoryRow::File(_))).count(), 1);
    // Loaded files are kept: opening it again does not fetch.
    history.toggle("aaa");
    assert!(history.toggle("aaa").is_none());
}

#[test]
fn a_git_error_is_shown_but_not_cached_as_an_empty_commit() {
    let mut history = loaded();
    history.toggle("aaa");
    history.files_loaded("aaa", Err("fatal: bad object".into()));
    assert!(matches!(history.files.get("aaa"), Some(CommitFiles::Failed(_))));
    assert!(history.rows().contains(&HistoryRow::FilesError));
    assert!(!history.rows().contains(&HistoryRow::Note("No file changes.")));
    // Closing and opening it again is a retry.
    history.toggle("aaa");
    assert!(!history.files.contains_key("aaa"));
    assert!(history.toggle("aaa").is_some());
}

#[test]
fn a_failure_for_a_commit_that_was_closed_meanwhile_is_not_kept() {
    let mut history = loaded();
    history.toggle("aaa");
    history.toggle("bbb");
    history.files_loaded("aaa", Err("boom".into()));
    assert!(!history.files.contains_key("aaa"), "nothing shows it, so nothing keeps it");
    assert!(history.toggle("aaa").is_some());
}

#[test]
fn a_commit_without_file_changes_is_a_real_answer() {
    let mut history = loaded();
    history.toggle("aaa");
    history.files_loaded("aaa", Ok(Vec::new()));
    assert!(history.rows().contains(&HistoryRow::Note("No file changes.")));
    assert!(matches!(history.files.get("aaa"), Some(CommitFiles::Loaded(f)) if f.is_empty()));
}

#[test]
fn the_files_cache_is_bounded_and_keeps_the_open_commit() {
    let mut history = loaded();
    history.toggle("aaa");
    history.files_loaded("aaa", Ok(vec![file("keep.rs")]));
    for n in 0..FILES_KEPT + 10 {
        history.files_loaded(&format!("other-{n}"), Ok(vec![file("x")]));
    }
    assert!(history.files.len() <= FILES_KEPT + 1);
    assert!(matches!(history.files.get("aaa"), Some(CommitFiles::Loaded(f)) if f[0].path == "keep.rs"));
    assert_eq!(history.loaded("aaa").map(Rc::strong_count), Some(1));
}

fn details(sha: &str) -> CommitDetails {
    let who = Signature { name: "Ada".into(), email: "ada@example.com".into(), time: 0, offset: 0 };
    CommitDetails {
        sha: sha.into(),
        parents: Vec::new(),
        author: who.clone(),
        committer: who,
        refs: Vec::new(),
        message: format!("commit {sha}\n\nbody"),
        message_cut: false,
        co_authors: Vec::new(),
    }
}

#[test]
fn the_open_commit_has_a_details_row_that_changes_as_they_arrive() {
    let mut history = loaded();
    assert!(!history.rows().iter().any(|r| matches!(r, HistoryRow::Details(_))), "nothing is open");
    history.toggle("aaa");
    let before = history.rows();
    let at = before.iter().position(|r| matches!(r, HistoryRow::Details(_))).expect("a row for the details");
    assert_eq!(before[at - 1], HistoryRow::Commit(0), "right under its commit");
    // Asking marks it loading; once read, the row differs, so the list measures it again.
    assert!(history.want_details("aaa"));
    let loading = history.rows();
    assert_ne!(loading[at], before[at]);
    history.details_loaded("aaa", Ok(details("aaa")));
    assert_ne!(history.rows()[at], loading[at]);
    assert_eq!(history.details_of("aaa").map(|d| d.subject().to_string()), Some("commit aaa".into()));
    // A commit that is not open has no such row, and another's details change nothing here.
    let shown = history.rows();
    history.details_loaded("bbb", Ok(details("bbb")));
    assert_eq!(history.rows(), shown);
}

#[test]
fn details_are_read_once_however_often_the_commit_is_opened() {
    let mut history = loaded();
    assert!(history.want_details("aaa"));
    assert!(!history.want_details("aaa"), "the read on its way is awaited");
    history.details_loaded("aaa", Ok(details("aaa")));
    assert!(!history.want_details("aaa"), "a read one is kept");
}

#[test]
fn details_that_could_not_be_read_are_shown_while_open_and_tried_again_after() {
    let mut history = loaded();
    history.toggle("aaa");
    assert!(history.want_details("aaa"));
    history.details_loaded("aaa", Err("fatal: bad object".into()));
    assert!(matches!(history.details.get("aaa"), Some(CommitInfo::Failed(_))));
    assert!(history.want_details("aaa"), "asking again is a retry");
    // A failure for a commit that was closed meanwhile is not kept.
    history.toggle("aaa");
    history.details_loaded("aaa", Err("boom".into()));
    assert!(!history.details.contains_key("aaa"));
}

#[test]
fn the_details_cache_is_bounded_and_keeps_the_open_commit() {
    let mut history = loaded();
    history.toggle("aaa");
    history.want_details("aaa");
    history.details_loaded("aaa", Ok(details("aaa")));
    for n in 0..DETAILS_KEPT + 10 {
        history.details_loaded(&format!("other-{n}"), Ok(details("x")));
    }
    assert!(history.details.len() <= DETAILS_KEPT + 1);
    assert!(history.details_of("aaa").is_some());
}

#[test]
fn a_changed_log_drops_the_details_but_the_open_ones_are_read_again() {
    let mut history = loaded();
    history.toggle("aaa");
    for sha in ["aaa", "bbb"] {
        history.details_loaded(sha, Ok(details(sha)));
    }
    // Loaded for "bbb" only because nothing waited for it: it is what the cache keeps for a copy.
    assert_eq!(history.details_stale().as_deref(), Some("aaa"));
    assert!(history.details_of("bbb").is_none());
    assert!(history.details_of("aaa").is_some(), "still on screen while it is read again");
    history.toggle("aaa");
    assert_eq!(history.details_stale(), None, "nothing is open, nothing to read again");
    assert!(history.details.is_empty());
}

#[test]
fn a_folder_that_is_no_repository_does_not_redraw_on_every_poll() {
    let mut history = HistoryState::default();
    let k = key(None);
    let t = history.begin_refresh(k.clone()).expect("started").ticket();
    // The first answer replaces "Reading history…".
    assert!(history.finish_refresh(t, Some(&k), Ok(None)));
    for _ in 0..3 {
        let t = history.begin_refresh(k.clone()).expect("started").ticket();
        assert!(!history.finish_refresh(t, Some(&k), Ok(None)));
    }
    // It left a repository: that is a change.
    let t = history.begin_refresh(k.clone()).expect("started").ticket();
    assert!(history.finish_refresh(t, Some(&k), log_of(&["a"])));
    let t = history.begin_refresh(k.clone()).expect("started").ticket();
    assert!(history.finish_refresh(t, Some(&k), Ok(None)));
}

#[test]
fn which_files_are_in_the_working_tree_is_kept_with_their_commit_and_dropped_with_it() {
    let mut history = loaded();
    // Files nobody asked for are not kept.
    history.files_present("aaa", ["x.rs".to_string()].into());
    assert!(history.present.is_empty());
    history.toggle("aaa");
    history.files_loaded("aaa", Ok(vec![file("x.rs"), file("gone.rs")]));
    history.files_present("aaa", ["x.rs".to_string()].into());
    assert_eq!(history.present.get("aaa").map(|p| p.contains("x.rs")), Some(true));
    assert_eq!(history.present.get("aaa").map(|p| p.contains("gone.rs")), Some(false));
    // Pruned with the files, so it never outlives them.
    for n in 0..=FILES_KEPT {
        history.files_loaded(&format!("other-{n}"), Ok(vec![file("x")]));
    }
    history.toggle("aaa");
    history.files_loaded("zzz", Ok(vec![file("y")]));
    assert!(history.present.keys().all(|sha| history.files.contains_key(sha)));
}

#[test]
fn a_commits_submodule_keeps_the_commits_it_moves_between() {
    use wormhole_core::git::Submodule;
    let mut lib = file("libs/lib");
    lib.submodule = Some(Submodule {
        old: Some("a".repeat(40)),
        new: Some("b".repeat(40)),
        commit_changed: true,
        removed: false,
        tracked: false,
        untracked: false,
        log: None,
    });
    let mut history = HistoryState::default();
    history.files.insert("aaa".into(), CommitFiles::Loading);
    history.files_loaded("aaa", Ok(vec![file("a.rs"), lib.clone()]));
    let files = history.loaded("aaa").expect("loaded");
    assert_eq!(files[1].submodule, lib.submodule, "the diff opened from the row asks these two commits");
}
