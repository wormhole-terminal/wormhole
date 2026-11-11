//! The reads of a terminal's repository, with no git and no thread: the rules of the app's
//! `window/repo/tests.rs` for stale answers, failures and the cadence of the polling, and the
//! attribution of changed files through the whole feed.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::agents::AgentId;
use crate::git::attribution::{Place, Snapshot};
use crate::git::discover::Sighting;
use crate::git::{ChangeKind, FileChange, GitError, RepoStatus, Upstream};
use crate::view::sidebar::Status;
use crate::workspace::{Action, ModelConfig, TerminalFacts, WorkspaceModel};

use super::state::{GitState, Poll, Refreshed, Sight, WATCHED_READ};
use super::{ChangeRules, GitFeed, GitJob};

fn change(path: &str) -> FileChange {
    FileChange {
        path: path.to_string(),
        original_path: None,
        kind: ChangeKind::Modified,
        added: None,
        removed: None,
        lfs: false,
        submodule: None,
    }
}

fn status(files: &[&str]) -> RepoStatus {
    RepoStatus {
        root: PathBuf::from("/repo"),
        branch: Some("main".into()),
        files: files.iter().map(|path| change(path)).collect(),
        base_rev: "HEAD".into(),
        compare: None,
        compare_missing: None,
        dirty: !files.is_empty(),
        upstream: Some(Upstream { name: "origin/main".into(), ahead: 2, behind: 1 }),
        superproject: None,
    }
}

/// A read of `cwd` that answers `result`, delivered while the terminal shows `now`.
fn read(
    git: &mut GitState,
    cwd: &str,
    now: Option<&str>,
    result: Result<Option<RepoStatus>, GitError>,
) -> Refreshed {
    let wanted = git.begin_refresh(PathBuf::from(cwd), 0).expect("a read starts");
    git.finish_refresh(wanted.ticket(), now.map(Path::new), result.map_err(anyhow::Error::from))
}

fn files(git: &GitState) -> Option<usize> {
    git.repo.as_ref().map(|repo| repo.files)
}

// ------------------------------------------------------------ reading

#[test]
fn an_answer_for_a_folder_the_terminal_left_is_dropped_even_if_it_is_the_latest() {
    let mut git = GitState::default();
    let first = git.begin_refresh("/a".into(), 0).expect("starts");
    // The same read again is already running; another folder supersedes it.
    assert!(git.begin_refresh("/a".into(), 1).is_none());
    let second = git.begin_refresh("/b".into(), 2).expect("another folder is another read");
    let late = git.finish_refresh(first.ticket(), Some(Path::new("/b")), Ok(Some(status(&["x"]))));
    assert_eq!(late, Refreshed::Stale, "superseded");
    assert!(!first.still_wanted() && second.still_wanted());
    // The terminal moved on while git ran: the answer is for a place it is not in.
    let gone = git.finish_refresh(second.ticket(), Some(Path::new("/c")), Ok(Some(status(&["x"]))));
    assert_eq!(gone, Refreshed::Stale);
    assert_eq!(files(&git), None, "nothing shown from it");
}

#[test]
fn an_unchanged_status_is_not_a_change_and_a_new_one_is() {
    let mut git = GitState::default();
    assert_eq!(
        read(&mut git, "/repo", Some("/repo"), Ok(Some(status(&["a"])))),
        Refreshed::Applied { changed: true }
    );
    assert_eq!(
        read(&mut git, "/repo", Some("/repo"), Ok(Some(status(&["a"])))),
        Refreshed::Applied { changed: false }
    );
    assert_eq!(
        read(&mut git, "/repo", Some("/repo"), Ok(Some(status(&["a", "b"])))),
        Refreshed::Applied { changed: true }
    );
    assert_eq!(files(&git), Some(2));
    let repo = git.repo.clone().expect("repo");
    assert_eq!((repo.branch.as_deref(), repo.dirty, repo.ahead, repo.behind), (Some("main"), true, 2, 1));
}

#[test]
fn a_failed_read_keeps_the_answer_of_the_same_folder_and_says_so() {
    let mut git = GitState::default();
    read(&mut git, "/repo", Some("/repo"), Ok(Some(status(&["a"]))));
    let hiccup = Err(GitError::Failed("index.lock".into()));
    assert_eq!(read(&mut git, "/repo", Some("/repo"), hiccup), Refreshed::Applied { changed: true });
    assert_eq!(files(&git), Some(1), "a hiccup does not blank the list");
    assert_eq!(git.error(), Some(&GitError::Failed("index.lock".into())));
    // The same failure again says nothing new.
    let again = Err(GitError::Failed("index.lock".into()));
    assert_eq!(read(&mut git, "/repo", Some("/repo"), again), Refreshed::Applied { changed: false });
    // In another folder, the old folder's answer would mislead.
    let other = Err(GitError::Failed("boom".into()));
    read(&mut git, "/elsewhere", Some("/elsewhere"), other);
    assert_eq!(files(&git), None);
    // A recovered git clears the failure.
    read(&mut git, "/elsewhere", Some("/elsewhere"), Ok(Some(status(&["z"]))));
    assert_eq!(git.error(), None);
}

#[test]
fn a_missing_git_clears_the_list_and_waits_for_a_retry() {
    let mut git = GitState::default();
    read(&mut git, "/repo", Some("/repo"), Ok(Some(status(&["a"]))));
    read(&mut git, "/repo", Some("/repo"), Err(GitError::GitMissing));
    assert_eq!(files(&git), None);
    assert!(git.waits_for_retry());
    assert_eq!(
        git.poll(Some(Path::new("/repo")), 1_000_000, false),
        Poll::Idle,
        "the stub would ask to install the tools"
    );
    git.retry();
    assert_eq!(git.poll(Some(Path::new("/repo")), 1_000_000, false), Poll::Refresh);
}

#[test]
fn the_repository_is_not_known_until_git_has_answered_and_survives_a_read_in_flight() {
    let mut git = GitState::default();
    let cwd = Path::new("/repo/src");
    assert_eq!(git.place(Some(cwd)), Place::Unknown);
    read(&mut git, "/repo/src", Some("/repo/src"), Ok(Some(status(&["a"]))));
    assert_eq!(git.place(Some(cwd)), Place::In(Path::new("/repo")));
    // Moved to another folder: not known there yet, but the repository is still claimed.
    let other = Path::new("/other");
    assert_eq!(git.place(Some(other)), Place::Unknown);
    assert_eq!(git.root(), Some(Path::new("/repo")));
    read(&mut git, "/other", Some("/other"), Ok(None));
    assert_eq!(git.place(Some(other)), Place::Outside);
    assert_eq!(git.root(), None);
}

// ------------------------------------------------------------ polling

fn answered(git: &mut GitState, at: u64) {
    let wanted = git.begin_refresh("/repo".into(), at).expect("starts");
    git.finish_refresh(wanted.ticket(), Some(Path::new("/repo")), Ok(Some(status(&["a"]))));
}

#[test]
fn a_repository_is_read_by_its_sight_and_a_watched_one_every_30_seconds() {
    let cwd = Some(Path::new("/repo"));
    let mut git = GitState::default();
    assert_eq!(git.poll(cwd, 0, false), Poll::Refresh, "a folder git has not answered for is asked at once");
    answered(&mut git, 0);
    assert_eq!(
        (git.poll(cwd, 1_199, false), git.poll(cwd, 1_200, false)),
        (Poll::Idle, Poll::Refresh),
        "in front: 1.2 s"
    );
    git.note_sight(Sight::Behind);
    answered(&mut git, 2_000);
    assert_eq!(git.poll(cwd, 9_999, false), Poll::Idle, "behind: 8 s");
    assert_eq!(git.poll(cwd, 10_000, false), Poll::Refresh);
    // Followed through its files: any sight waits for the backstop.
    assert_eq!(git.poll(cwd, 10_000, true), Poll::Idle);
    assert_eq!(git.poll(cwd, 2_000 + WATCHED_READ, true), Poll::Refresh);
    assert_eq!(git.next_due(cwd, 3_000, true), Some(2_000 + WATCHED_READ));
    // A read in flight is not piled on.
    let _in_flight = git.begin_refresh("/repo".into(), 40_000).expect("starts");
    assert_eq!((git.poll(cwd, 80_000, false), git.next_due(cwd, 80_000, false)), (Poll::Idle, None));
}

#[test]
fn a_terminal_seen_another_way_or_whose_repository_changed_is_read_at_once() {
    let cwd = Some(Path::new("/repo"));
    let mut git = GitState::default();
    answered(&mut git, 0);
    git.note_sight(Sight::Unseen);
    assert_eq!(git.poll(cwd, 1, true), Poll::Refresh, "going out of sight pictures the repository once more");
    answered(&mut git, 1);
    assert!(!git.watch_hit(), "nobody sees what git would say");
    assert_eq!(git.poll(cwd, 2, true), Poll::Idle);
    git.note_sight(Sight::Front);
    assert_eq!(git.poll(cwd, 3, true), Poll::Refresh, "coming into sight shows what changed meanwhile");
    answered(&mut git, 3);
    assert!(git.watch_hit());
    assert_eq!((git.poll(cwd, 4, true), git.next_due(cwd, 4, true)), (Poll::Refresh, Some(4)));
}

#[test]
fn a_folder_without_a_repository_is_looked_at_for_a_git_dir_not_polled() {
    let cwd = Some(Path::new("/plain"));
    let mut git = GitState::default();
    read(&mut git, "/plain", Some("/plain"), Ok(None));
    assert!(git.is_absent_for(Path::new("/plain")));
    // Looked at every tick in front, git is never asked.
    assert_eq!(git.poll(cwd, 10_000, false), Poll::Probe);
    assert!(git.begin_probe(Path::new("/plain"), 10_000));
    assert_eq!(git.poll(cwd, 10_400, false), Poll::Idle, "one look at a time");
    assert_eq!(git.finish_probe(Path::new("/plain"), None, 10_000), Sighting::Unchanged);
    assert_eq!((git.poll(cwd, 10_399, false), git.poll(cwd, 10_400, false)), (Poll::Idle, Poll::Probe));
    // A `.git` appears: confirmed by a second look shortly after, and only then is git asked.
    assert!(git.begin_probe(Path::new("/plain"), 11_000));
    let found = Some(PathBuf::from("/plain"));
    assert_eq!(git.finish_probe(Path::new("/plain"), found.clone(), 11_000), Sighting::LookAgain);
    assert_eq!((git.poll(cwd, 11_149, false), git.poll(cwd, 11_150, false)), (Poll::Idle, Poll::Probe));
    assert!(git.begin_probe(Path::new("/plain"), 11_150));
    assert_eq!(git.finish_probe(Path::new("/plain"), found, 11_150), Sighting::AskGit);
    assert_eq!(git.poll(cwd, 11_150, false), Poll::Refresh);
    // Git saying no to a `.git` it rejects does not start the looks again.
    read(&mut git, "/plain", Some("/plain"), Ok(None));
    assert!(git.is_absent_for(Path::new("/plain")));
}

// ------------------------------------------------------------ the whole feed

fn picture(status: &RepoStatus, at: Instant) -> Snapshot {
    Snapshot::take(status, at).expect("a picture")
}

fn answer(feed: &mut GitFeed, jobs: Vec<GitJob>, status: &RepoStatus, at: Instant) -> bool {
    let mut changed = false;
    for job in jobs {
        let GitJob::Read { id, wanted, .. } = job else { continue };
        changed |= feed.finish_read(id, &wanted, Ok(Some(status.clone())), Some(picture(status, at)));
    }
    changed
}

#[test]
fn the_feed_reads_a_folder_once_and_what_a_working_terminal_changed_is_its_count() {
    let start = Instant::now();
    let later = |ms| start + Duration::from_millis(ms);
    let mut feed = GitFeed::new(ChangeRules::default());
    for id in [1, 2] {
        feed.folder(id, Some(Path::new("/repo")));
    }
    let jobs = feed.poll(0);
    assert_eq!(jobs.len(), 2, "each terminal asks, one read each");
    assert!(feed.poll(1).is_empty(), "in flight: never piled up");
    // The first picture of a repository is a baseline: what was changed already belongs to nobody.
    assert!(answer(&mut feed, jobs, &status(&["old.rs"]), later(0)));
    let facts = feed.facts_of(1);
    assert_eq!(
        (facts.repo.as_deref(), facts.branch.as_deref(), facts.dirty),
        (Some(Path::new("/repo")), Some("main"), true)
    );
    assert_eq!((facts.ahead, facts.behind, facts.files, facts.changes), (2, 1, Some(1), None));
    // Terminal 1 is an agent at work; the next picture has a new file: it is terminal 1's.
    let working =
        TerminalFacts { agent: Some(AgentId::Claude), status: Status::Busy, ..TerminalFacts::default() };
    feed.track(1, &working, later(100));
    feed.track(2, &TerminalFacts::default(), later(100));
    let jobs = feed.poll(2_000);
    assert!(answer(&mut feed, jobs, &status(&["old.rs", "new.rs"]), later(200)));
    assert_eq!((feed.facts_of(1).changes, feed.facts_of(2).changes), (Some(1), None));
    assert_eq!(feed.facts_of(1).files, Some(2), "the repository's own count is everyone's");
    assert_eq!(feed.roots(), [PathBuf::from("/repo")]);
    // Closing the terminal takes its count away.
    assert!(feed.forget(1));
}

#[test]
fn what_a_terminal_changed_is_kept_as_rows_and_comes_back_with_the_window() {
    let start = Instant::now();
    let later = |ms| start + Duration::from_millis(ms);
    let mut feed = GitFeed::new(ChangeRules::default());
    feed.folder(1, Some(Path::new("/repo")));
    let jobs = feed.poll(0);
    answer(&mut feed, jobs, &status(&["old.rs"]), later(0));
    let working =
        TerminalFacts { agent: Some(AgentId::Claude), status: Status::Busy, ..TerminalFacts::default() };
    feed.track(1, &working, later(100));
    let jobs = feed.poll(2_000);
    answer(&mut feed, jobs, &status(&["old.rs", "new.rs", "also.rs"]), later(200));
    let rows = feed.claim_rows(&[1, 2]);
    let kept: Vec<_> = rows.iter().map(|r| (r.session_id, r.root.as_str(), r.path.as_str())).collect();
    assert_eq!(
        kept,
        [(1, "/repo", "also.rs"), (1, "/repo", "new.rs")],
        "terminal 1's files, sorted; 2 has none"
    );

    // A window coming back: the terminal has its files again, and a terminal already known keeps what it has.
    let mut back = GitFeed::new(ChangeRules::default());
    assert!(back.restore_claims(rows.clone()));
    assert_eq!(back.claim_rows(&[1]), rows);
    assert_eq!(back.facts_of(1).changes, Some(2));
    assert!(!back.restore_claims(rows), "ignored for a terminal the model knows");
}

#[test]
fn a_plain_terminal_works_by_the_setting() {
    let build = TerminalFacts { command: Some("cargo build".into()), ..TerminalFacts::default() };
    let editor = TerminalFacts { command: Some("vim main.rs".into()), ..TerminalFacts::default() };
    let at_prompt = TerminalFacts::default();
    let by = |plain| GitFeed::new(ChangeRules { plain, ..ChangeRules::default() });
    use crate::config::PlainChanges::{Any, Finishing, None as Never};
    for (plain, expected) in
        [(Finishing, [true, false, false]), (Any, [true, true, false]), (Never, [false; 3])]
    {
        let feed = by(plain);
        let got = [&build, &editor, &at_prompt].map(|facts| feed.working_of(facts));
        assert_eq!(got, expected, "{plain:?}");
    }
}

#[test]
fn the_model_reads_folders_from_the_facts_and_puts_the_answers_back() {
    let mut m = WorkspaceModel::new(ModelConfig::default());
    m.apply(Action::NewTerminal);
    let start = Instant::now();
    assert!(m.git_poll(0, start).is_empty(), "no folder known yet");
    m.update_facts(1, |facts| facts.cwd = Some("/repo".into()));
    let jobs = m.git_poll(1, start);
    let [GitJob::Read { id: 1, wanted, .. }] = jobs.as_slice() else { panic!("{jobs:?}") };
    let events =
        m.git_read(1, wanted, Ok(Some(status(&["a", "b"]))), Some(picture(&status(&["a", "b"]), start)));
    assert_eq!(events, [crate::workspace::Event::Listing]);
    let facts = m.facts(1).expect("facts");
    assert_eq!((facts.branch.as_deref(), facts.dirty, facts.repo_files), (Some("main"), true, Some(2)));
    assert_eq!(m.git_roots(), [PathBuf::from("/repo")]);
    // The feed's own deadline: the next read of a repository nobody follows.
    assert_eq!(m.git_next_due(1_000), Some(1 + 1_200));
    m.git_watched(Path::new("/repo"), true);
    assert_eq!(m.git_next_due(1_000), Some(1 + WATCHED_READ));
    // A terminal that closed is let go of.
    m.apply(Action::Close(1));
    m.git_poll(5_000, start);
    assert!(m.git_roots().is_empty());
}

#[test]
fn rules_set_while_running_decide_the_next_work() {
    let editor = TerminalFacts { command: Some("vim main.rs".into()), ..TerminalFacts::default() };
    let mut feed = GitFeed::new(ChangeRules::default());
    assert!(!feed.working_of(&editor), "an editor is not a command that ends by itself");
    feed.set_rules(ChangeRules { plain: crate::config::PlainChanges::Any, ..ChangeRules::default() });
    assert!(feed.working_of(&editor));

    // The same through the model: a rule that did not change is no action, one that did is kept.
    let mut m = WorkspaceModel::new(ModelConfig::default());
    let rules = ChangeRules { plain: crate::config::PlainChanges::None, ..ChangeRules::default() };
    assert!(m.apply(Action::SetChangeRules(ChangeRules::default())).is_empty());
    m.apply(Action::SetChangeRules(rules));
    assert_eq!(m.cfg.changes, rules);
}
