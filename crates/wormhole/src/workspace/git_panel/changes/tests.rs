use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

// Not `super::*`: that would pull in GPUI's `test` macro and shadow `#[test]`.
use super::{
    ChangeRow, Folded, GitState, Group, Refreshed, StatusKey, build_rows, build_tree, collapse_key,
    display_order, untracked_apart,
};
use crate::workspace::git_panel::job::testing::ManualExecutor;
use wormhole_core::git::{ChangeKind, FileChange, GitError, RepoStatus};

fn change(path: &str, kind: ChangeKind) -> FileChange {
    FileChange { path: path.to_string(), original_path: None, kind, added: None, removed: None, lfs: false }
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
    }
}

fn key(cwd: &str, compare: Option<&str>) -> StatusKey {
    (PathBuf::from(cwd), compare.map(String::from))
}

#[test]
fn tree_groups_by_folder() {
    let files = modified(&["src/a.rs", "src/ui/b.rs", "README.md"]);
    let ids: Vec<usize> = (0..files.len()).collect();
    let tree = build_tree(&files, &ids);
    assert_eq!(tree.files, [2]);
    let src = &tree.dirs["src"];
    assert_eq!(src.files, [0]);
    assert_eq!(src.dirs["ui"].files, [1]);
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
    assert_eq!(session.git.error, Some(GitError::Failed("index.lock exists".into())));
    // The next good answer clears it.
    refresh(&mut session, &mut exec, Ok(Some(status(modified(&["a.rs", "b.rs"]), None))));
    exec.deliver(2, &mut session);
    assert!(session.git.error.is_none());
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
    assert_eq!(session.git.error, Some(GitError::Failed("fatal: index file corrupt".into())));
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
    assert_eq!(session.git.error, Some(GitError::GitMissing));
    assert!(session.git.waits_for_retry());
    // A retry that finds git again brings the list back.
    refresh(&mut session, &mut exec, Ok(Some(status(modified(&["a.rs"]), None))));
    exec.deliver(2, &mut session);
    assert_eq!((paths(&session.git), session.git.error.as_ref()), (vec!["a.rs"], None));
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
    assert_eq!((session.git.status.as_ref(), session.git.error.as_ref()), (None, None));
}

// ------------------------------------------------------------ folders

#[test]
fn folders_fold_per_session_and_per_block() {
    // `src` holds a tracked and an untracked file, so it is listed in both blocks.
    let files = vec![change("src/a.rs", ChangeKind::Modified), change("src/b.rs", ChangeKind::Untracked)];
    let mut set = HashSet::new();
    set.insert(collapse_key(1, Group::Changes, "src"));

    let rows = |session| build_rows(&files, true, true, Folded { set: &set, session }, &HashMap::new());
    let dirs = |rows: Vec<ChangeRow>| -> Vec<(Group, bool)> {
        rows.into_iter()
            .filter_map(|row| match row {
                ChangeRow::Dir { group, collapsed, .. } => Some((group, collapsed)),
                _ => None,
            })
            .collect()
    };
    // In session 1 only the "Changes" copy of `src` is folded...
    assert_eq!(dirs(rows(1)), [(Group::Changes, true), (Group::Untracked, false)]);
    // ...and its file is gone from the list, the untracked one is not.
    let listed = |rows: Vec<ChangeRow>| rows.iter().filter(|r| matches!(r, ChangeRow::File { .. })).count();
    assert_eq!(listed(rows(1)), 1);
    // Session 2 never folded anything.
    assert_eq!(dirs(rows(2)), [(Group::Changes, false), (Group::Untracked, false)]);
    assert_eq!(listed(rows(2)), 2);
}

#[test]
fn a_block_keeps_its_folders_when_the_other_block_is_empty() {
    // Only untracked files: "Untracked" is the first block, but not group 0 of a key.
    let files = vec![change("src/b.rs", ChangeKind::Untracked)];
    let mut set = HashSet::new();
    set.insert(collapse_key(1, Group::Untracked, "src"));
    let rows = build_rows(&files, true, true, Folded { set: &set, session: 1 }, &HashMap::new());
    assert!(rows.iter().any(|r| matches!(r, ChangeRow::Dir { collapsed: true, .. })));
}

#[test]
fn the_diff_steps_through_files_in_the_order_the_list_shows_them() {
    let files = vec![
        change("README.md", ChangeKind::Modified),
        change("src/b.rs", ChangeKind::Modified),
        change("scratch.txt", ChangeKind::Untracked),
        change("docs/a.md", ChangeKind::Modified),
    ];
    let against_head = status(files, None);
    let order = |tree, group| -> Vec<String> {
        display_order(&against_head, tree, group).into_iter().map(|f| f.path).collect()
    };
    // A plain list is the status order.
    assert_eq!(order(false, false), ["README.md", "src/b.rs", "scratch.txt", "docs/a.md"]);
    // Grouped: the tracked block, then the untracked one.
    assert_eq!(order(false, true), ["README.md", "src/b.rs", "docs/a.md", "scratch.txt"]);
    // A tree lists folders before the files beside them, each sorted by name.
    assert_eq!(order(true, false), ["docs/a.md", "src/b.rs", "README.md", "scratch.txt"]);
    assert_eq!(order(true, true), ["docs/a.md", "src/b.rs", "README.md", "scratch.txt"]);
}

/// The bands of the list built for `status` with the "group untracked" setting as given.
fn bands(status: &RepoStatus, tree: bool, setting: bool) -> Vec<(Group, usize)> {
    let none = HashSet::new();
    let grouped = untracked_apart(status, setting);
    build_rows(&status.files, tree, grouped, Folded { set: &none, session: 1 }, &HashMap::new())
        .into_iter()
        .filter_map(|row| match row {
            ChangeRow::Band { group, count, .. } => Some((group, count)),
            _ => None,
        })
        .collect()
}

#[test]
fn against_head_untracked_files_get_a_band_when_the_setting_says_so() {
    let files = vec![change("a.rs", ChangeKind::Modified), change("scratch.txt", ChangeKind::Untracked)];
    let head = status(files, None);
    assert_eq!(bands(&head, false, true), [(Group::Changes, 1), (Group::Untracked, 1)]);
    assert_eq!(bands(&head, false, false), [(Group::All, 2)]);
}

/// Compared with a branch the list is one group, untracked files in it, whatever the setting.
#[test]
fn compared_with_a_branch_there_is_one_group_with_the_untracked_files_in_it() {
    let files = vec![
        change("a.rs", ChangeKind::Modified),
        change("b.rs", ChangeKind::Added),
        change("scratch.txt", ChangeKind::Untracked),
    ];
    let compared = status(files, Some("main"));
    for setting in [true, false] {
        for tree in [false, true] {
            assert_eq!(bands(&compared, tree, setting), [(Group::All, 3)], "setting {setting}, tree {tree}");
        }
    }
    // The diff steps through that one list, in its order.
    let order: Vec<String> = display_order(&compared, false, true).into_iter().map(|f| f.path).collect();
    assert_eq!(order, ["a.rs", "b.rs", "scratch.txt"]);
    // A compare branch that is gone falls back to HEAD (`compare` is cleared): bands again.
    let fell_back = RepoStatus { compare: None, compare_missing: Some("main".into()), ..compared };
    assert_eq!(bands(&fell_back, false, true), [(Group::Changes, 2), (Group::Untracked, 1)]);
}
