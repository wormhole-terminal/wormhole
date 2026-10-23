use std::path::PathBuf;

// Not `super::*`: that would pull in GPUI's `test` macro and shadow `#[test]`.
use super::GitState;
use super::state::{Refreshed, StatusKey};
use crate::window::repo::job::testing::ManualExecutor;
use wormhole_core::git::discover::Sighting;
use wormhole_core::git::{ChangeKind, FileChange, GitError, RepoStatus};

fn change(path: &str, kind: ChangeKind) -> FileChange {
    FileChange {
        path: path.to_string(),
        original_path: None,
        kind,
        added: None,
        removed: None,
        lfs: false,
        submodule: None,
    }
}

fn modified(paths: &[&str]) -> Vec<FileChange> {
    paths.iter().map(|p| change(p, ChangeKind::Modified)).collect()
}

fn status(files: Vec<FileChange>, compare: Option<&str>) -> RepoStatus {
    RepoStatus {
        root: PathBuf::from("/repo"),
        branch: Some("main".into()),
        files,
        base_rev: "HEAD".into(),
        compare: compare.map(String::from),
        compare_missing: None,
        dirty: false,
        upstream: None,
        superproject: None,
    }
}

fn key(cwd: &str, compare: Option<&str>) -> StatusKey {
    (PathBuf::from(cwd), compare.map(String::from))
}

// ------------------------------------------------------------ refreshing

/// A session's git state plus what the session shows now (what a result is checked against).
struct Session {
    git: GitState,
    shown: StatusKey,
}

fn refresh(
    session: &mut Session,
    exec: &mut ManualExecutor<Session>,
    result: Result<Option<RepoStatus>, GitError>,
) {
    let key = session.shown.clone();
    let Some(wanted) = session.git.begin_refresh(key) else { return };
    let ticket = wanted.ticket();
    exec.spawn(
        move || result.map_err(anyhow::Error::from),
        move |session: &mut Session, result| {
            let now = session.shown.clone();
            session.git.finish_refresh(ticket, Some(&now), result);
        },
    );
}

fn failed(message: &str) -> GitError {
    GitError::Failed(message.into())
}

fn session() -> Session {
    Session { git: GitState::default(), shown: key("/repo", None) }
}

fn paths(git: &GitState) -> Vec<&str> {
    git.status.iter().flat_map(|s| &s.files).map(|f| f.path.as_str()).collect()
}

#[test]
fn a_refresh_for_another_branch_is_not_swallowed_by_the_one_in_flight() {
    let mut git = GitState::default();
    assert!(git.begin_refresh(key("/repo", None)).is_some());
    // The same request again (a second poll, the Refresh button): the first answer will do.
    assert!(git.begin_refresh(key("/repo", None)).is_none());
    // Picking a branch to compare with is a different request and must start.
    assert!(git.begin_refresh(key("/repo", Some("dev"))).is_some());
    assert!(git.loading());
}

#[test]
fn a_poll_that_was_running_when_the_branch_changed_never_shows_the_old_branch() {
    // The bug: compare switched while a poll for the old one was in flight; the old
    // answer landed and the panel showed HEAD's changes under the new branch's chip.
    for old_lands_last in [true, false] {
        let mut session = session();
        let mut exec = ManualExecutor::default();
        refresh(&mut session, &mut exec, Ok(Some(status(modified(&["head.rs"]), None))));
        session.shown = key("/repo", Some("dev"));
        session.git.status = None;
        refresh(&mut session, &mut exec, Ok(Some(status(modified(&["dev.rs"]), Some("dev")))));

        let order = if old_lands_last { [1, 0] } else { [0, 1] };
        for ix in order {
            exec.deliver(ix, &mut session);
        }
        assert_eq!(paths(&session.git), ["dev.rs"]);
        assert_eq!(session.git.status.as_ref().and_then(|s| s.compare.as_deref()), Some("dev"));
        assert!(!session.git.loading());
    }
}

#[test]
fn an_answer_for_what_is_no_longer_shown_is_dropped_even_if_it_is_the_latest() {
    // The tab code resets `compare` directly, without a new request.
    let mut session = session();
    let mut exec = ManualExecutor::default();
    refresh(&mut session, &mut exec, Ok(Some(status(modified(&["a.rs"]), None))));
    session.shown = key("/repo", Some("dev"));
    exec.deliver(0, &mut session);
    assert!(session.git.status.is_none());
    // The request is over, so the next poll asks for what is shown now.
    assert!(!session.git.loading());
    refresh(&mut session, &mut exec, Ok(Some(status(modified(&["b.rs"]), Some("dev")))));
    exec.deliver(1, &mut session);
    assert_eq!(paths(&session.git), ["b.rs"]);
}

#[test]
fn an_unchanged_status_is_not_a_change() {
    let mut git = GitState::default();
    let k = key("/repo", None);
    let one = || Ok(Some(status(modified(&["a.rs"]), None)));
    let t = git.begin_refresh(k.clone()).expect("started").ticket();
    assert_eq!(git.finish_refresh(t, Some(&k), one()), Refreshed::Applied { changed: true });
    let t = git.begin_refresh(k.clone()).expect("started").ticket();
    assert_eq!(git.finish_refresh(t, Some(&k), one()), Refreshed::Applied { changed: false });
    // A stale ticket is reported as such.
    assert_eq!(git.finish_refresh(t, Some(&k), one()), Refreshed::Stale);
}

#[test]
fn a_failed_refresh_keeps_the_list_of_the_same_folder_and_says_so() {
    let mut session = session();
    let mut exec = ManualExecutor::default();
    refresh(&mut session, &mut exec, Ok(Some(status(modified(&["a.rs", "b.rs"]), None))));
    exec.deliver(0, &mut session);
    refresh(&mut session, &mut exec, Err(failed("index.lock exists")));
    exec.deliver(1, &mut session);
    // A hiccup does not blank the list (and reset its scroll).
    assert_eq!(paths(&session.git), ["a.rs", "b.rs"]);
    assert_eq!(session.git.error().cloned(), Some(GitError::Failed("index.lock exists".into())));
    // The next good answer clears it.
    refresh(&mut session, &mut exec, Ok(Some(status(modified(&["a.rs", "b.rs"]), None))));
    exec.deliver(2, &mut session);
    assert!(session.git.error().is_none());
}

#[test]
fn a_failed_refresh_in_another_folder_does_not_keep_the_old_folders_list() {
    let mut session = session();
    let mut exec = ManualExecutor::default();
    refresh(&mut session, &mut exec, Ok(Some(status(modified(&["a.rs"]), None))));
    exec.deliver(0, &mut session);
    session.shown = key("/elsewhere", None);
    refresh(&mut session, &mut exec, Err(failed("fatal: index file corrupt")));
    exec.deliver(1, &mut session);
    assert!(session.git.status.is_none());
    assert_eq!(session.git.cwd.as_deref(), Some(std::path::Path::new("/elsewhere")));
    assert_eq!(session.git.error().cloned(), Some(GitError::Failed("fatal: index file corrupt".into())));
}

#[test]
fn a_missing_git_clears_the_list_and_stops_the_polling_until_it_answers_again() {
    let mut session = session();
    let mut exec = ManualExecutor::default();
    refresh(&mut session, &mut exec, Ok(Some(status(modified(&["a.rs"]), None))));
    exec.deliver(0, &mut session);
    assert!(!session.git.waits_for_retry());
    refresh(&mut session, &mut exec, Err(GitError::GitMissing));
    exec.deliver(1, &mut session);
    // Not a hiccup: the list would never refresh, so it goes, and git is not asked again by itself.
    assert!(session.git.status.is_none());
    assert_eq!(session.git.error().cloned(), Some(GitError::GitMissing));
    assert!(session.git.waits_for_retry());
    // A retry that finds git again brings the list back.
    refresh(&mut session, &mut exec, Ok(Some(status(modified(&["a.rs"]), None))));
    exec.deliver(2, &mut session);
    assert_eq!((paths(&session.git), session.git.error()), (vec!["a.rs"], None));
    assert!(!session.git.waits_for_retry());
}

#[test]
fn a_failing_git_keeps_polling_so_a_fix_in_the_terminal_shows_up() {
    let mut session = session();
    let mut exec = ManualExecutor::default();
    refresh(&mut session, &mut exec, Err(failed("fatal: detected dubious ownership")));
    exec.deliver(0, &mut session);
    assert!(session.git.status.is_none());
    assert!(!session.git.waits_for_retry());
}

#[test]
fn a_folder_that_stopped_being_a_repository_reads_as_one_that_never_was() {
    let mut session = session();
    let mut exec = ManualExecutor::default();
    refresh(&mut session, &mut exec, Ok(Some(status(modified(&["a.rs"]), None))));
    exec.deliver(0, &mut session);
    refresh(&mut session, &mut exec, Err(GitError::NotARepo));
    exec.deliver(1, &mut session);
    assert_eq!((session.git.status.as_ref(), session.git.error()), (None, None));
}

// ------------------------------------------------------------ folders that are no repository

/// Git answered "not a repository" for `/plain`; returns the state afterwards.
fn absent_git(result: Result<Option<RepoStatus>, GitError>) -> (GitState, Refreshed) {
    let mut git = GitState::default();
    let k = key("/plain", None);
    let t = git.begin_refresh(k.clone()).expect("started").ticket();
    let refreshed = git.finish_refresh(t, Some(&k), result.map_err(anyhow::Error::from));
    (git, refreshed)
}

#[test]
fn a_folder_without_a_repository_answers_once_and_then_changes_nothing() {
    for answer in [Ok(None), Err(GitError::NotARepo)] {
        let (mut git, first) = absent_git(answer);
        assert_eq!(first, Refreshed::Applied { changed: true });
        let plain = std::path::Path::new("/plain");
        assert!(git.answered_for(plain) && git.is_absent_for(plain));
        // Every later answer is the same: nothing to redraw, not even once.
        let k = key("/plain", None);
        for _ in 0..3 {
            let t = git.begin_refresh(k.clone()).expect("started").ticket();
            assert_eq!(git.finish_refresh(t, Some(&k), Ok(None)), Refreshed::Applied { changed: false });
            let t = git.begin_refresh(k.clone()).expect("started").ticket();
            let err = Err(anyhow::Error::from(GitError::NotARepo));
            assert_eq!(git.finish_refresh(t, Some(&k), err), Refreshed::Applied { changed: false });
        }
    }
}

#[test]
fn looking_for_a_git_dir_replaces_polling_git_for_an_absent_repository() {
    let (mut git, _) = absent_git(Ok(None));
    let plain = std::path::Path::new("/plain");
    let elsewhere = std::path::Path::new("/elsewhere");
    assert!(!git.is_absent_for(elsewhere) && !git.begin_probe(elsewhere));
    // One look at a time.
    assert!(git.begin_probe(plain));
    assert!(!git.begin_probe(plain));
    assert_eq!(git.finish_probe(plain, None), Sighting::Unchanged);
    // `git init`: seen, confirmed, then git is asked.
    let root = Some(PathBuf::from("/plain"));
    assert!(git.begin_probe(plain));
    assert_eq!(git.finish_probe(plain, root.clone()), Sighting::LookAgain);
    assert!(git.begin_probe(plain));
    assert_eq!(git.finish_probe(plain, root.clone()), Sighting::AskGit);
    // Git rejects it (a broken .git): the same sighting does not ask again.
    let k = key("/plain", None);
    let t = git.begin_refresh(k.clone()).expect("started").ticket();
    git.finish_refresh(t, Some(&k), Ok(None));
    assert!(git.begin_probe(plain));
    assert_eq!(git.finish_probe(plain, root), Sighting::Unchanged);
}

#[test]
fn the_tick_never_asks_git_about_a_folder_it_knows_is_no_repository() {
    use super::state::GitPoll;
    let plain = std::path::Path::new("/plain");
    // Nothing known yet: ask at once, whatever the tick.
    assert_eq!(GitState::default().poll(Some(plain), 3, true), GitPoll::Refresh);
    let (git, _) = absent_git(Ok(None));
    let polls: Vec<GitPoll> = (0..40).map(|tick| git.poll(Some(plain), tick, true)).collect();
    assert!(polls.iter().all(|p| *p == GitPoll::Probe), "looked at every tick in front");
    let behind = (0..40).filter(|tick| git.poll(Some(plain), *tick, false) == GitPoll::Probe).count();
    assert_eq!(behind, 8);
    for tick in 0..40 {
        assert_ne!(git.poll(Some(plain), tick, tick % 2 == 0), GitPoll::Refresh);
    }
    // Another folder is asked about at once.
    assert_eq!(git.poll(Some(std::path::Path::new("/other")), 1, false), GitPoll::Refresh);
    // A repository keeps its 2 s / 8 s polls.
    let mut repo = GitState::default();
    let k = key("/repo", None);
    let t = repo.begin_refresh(k.clone()).expect("started").ticket();
    repo.finish_refresh(t, Some(&k), Ok(Some(status(modified(&["a.rs"]), None))));
    let repo_dir = std::path::Path::new("/repo");
    let refreshed =
        |front| (0..40).filter(|t| repo.poll(Some(repo_dir), *t, front) == GitPoll::Refresh).count();
    assert_eq!((refreshed(true), refreshed(false)), (8, 2));
}

#[test]
fn a_repository_or_a_failure_ends_the_absence() {
    let (mut git, _) = absent_git(Ok(None));
    let k = key("/plain", None);
    let t = git.begin_refresh(k.clone()).expect("started").ticket();
    git.finish_refresh(t, Some(&k), Ok(Some(status(modified(&["a.rs"]), None))));
    let plain = std::path::Path::new("/plain");
    assert!(!git.is_absent_for(plain));
    // Gone again, and the look starts over from "no .git in sight".
    let t = git.begin_refresh(k.clone()).expect("started").ticket();
    git.finish_refresh(t, Some(&k), Ok(None));
    assert!(git.is_absent_for(plain));
    // A failure (git is busy) is polled like any other, not looked for.
    let t = git.begin_refresh(k.clone()).expect("started").ticket();
    git.finish_refresh(t, Some(&k), Err(anyhow::Error::from(failed("index.lock exists"))));
    assert!(!git.is_absent_for(plain) && !git.begin_probe(plain));
}

#[test]
fn a_folder_git_has_not_answered_for_is_still_being_read() {
    let mut git = GitState::default();
    let plain = std::path::Path::new("/plain");
    assert!(!git.answered_for(plain));
    // A poll running over an answer already shown does not unanswer it.
    let (answered, _) = absent_git(Ok(None));
    git = answered;
    let _poll = git.begin_refresh(key("/plain", None));
    assert!(git.loading() && git.answered_for(plain));
    assert!(!git.answered_for(std::path::Path::new("/other")));
}

// ------------------------------------------------------------ folders
