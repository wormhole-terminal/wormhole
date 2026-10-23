use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;
use std::sync::Arc;

use super::{Built, Check, Observed, Outcome, Stamp, Work, run};
use crate::features::git::diff::rows::{BuildKey, build_rows};
use crate::features::git::diff::{DiffView, Request};
use crate::window::repo::job::testing::ManualExecutor;
use crate::window::repo::job::{Job, Wanted};
use gpui_kit::component::highlighter::HighlightTheme;
use gpui_kit::{TouchPhase, point, px};
use wormhole_core::git::{
    ChangeKind, DiffLimits, DiffLine, DiffSpec, FileChange, FileDiff, Hunk, LineKind, RepoStatus,
};

const KEY: BuildKey = BuildKey { split: true, tab_width: 4, syntax: 1 };
const OTHER_KEY: BuildKey = BuildKey { split: false, tab_width: 4, syntax: 1 };

fn change(path: &str) -> FileChange {
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

fn working_tree(path: &str) -> Request {
    Request {
        root: PathBuf::from("/repo"),
        file: change(path),
        spec: DiffSpec { old: "HEAD".into(), new: None },
        limits: DiffLimits::default(),
    }
}

fn view(request: Request) -> DiffView {
    DiffView::new(1, request, Rc::default(), None)
}

/// What git would answer for `path`: one added line that says whose diff it is.
fn diff_of(path: &str) -> FileDiff {
    let line =
        DiffLine { kind: LineKind::Added, old: None, new: Some(1), text: path.into(), no_newline: false };
    FileDiff { hunks: vec![Hunk { header: "@@".into(), lines: vec![line] }], ..FileDiff::default() }
}

fn built(diff: &FileDiff, key: BuildKey) -> Built {
    let (rows, stats) = build_rows(diff, key, None);
    Built { rows, stats, key }
}

fn loaded(path: &str, stamp: Option<Stamp>) -> Outcome {
    let diff = diff_of(path);
    let built = built(&diff, KEY);
    Outcome::Loaded { diff: Arc::new(diff), stamp, on_disk: stamp.is_some(), built }
}

/// Which file the diff on view holds lines of (the text of its one line).
fn shown(view: &DiffView) -> Option<String> {
    let diff = view.diff.as_ref()?;
    Some(diff.hunks[0].lines[0].text.clone())
}

// ------------------------------------------------------------ stale answers

/// What the workspace holds: the diff on view, replaced by every open.
struct Screen {
    view: Option<DiffView>,
}

/// `open_diff` with the executor injected: a new view asks, its answer is held.
fn open(screen: &mut Screen, exec: &mut ManualExecutor<Screen>, path: &str) {
    let mut view = view(working_tree(path));
    let (wanted, work) = view.begin_load(KEY).expect("a fresh view always starts");
    let ticket = wanted.ticket();
    screen.view = Some(view);
    exec.spawn(
        move || match work {
            Work::Load(request) => loaded(&request.file.path, None),
            Work::Rebuild { .. } => unreachable!("a view opens with a load"),
        },
        move |screen: &mut Screen, outcome| {
            // The workspace applies it to whatever is on view, which must refuse it if it did not ask.
            if let Some(view) = screen.view.as_mut() {
                view.finish(ticket, outcome);
            }
        },
    );
}

#[test]
fn stepping_quickly_never_shows_one_files_lines_under_anothers_name() {
    // The bug: ‹ › opened A then B; A's slower answer landed last and was applied to B's view.
    for a_lands_last in [true, false] {
        let mut screen = Screen { view: None };
        let mut exec = ManualExecutor::default();
        open(&mut screen, &mut exec, "a.rs");
        open(&mut screen, &mut exec, "b.rs");
        let order = if a_lands_last { [1, 0] } else { [0, 1] };
        for ix in order {
            exec.deliver(ix, &mut screen);
        }
        let view = screen.view.as_ref().expect("a diff is open");
        assert_eq!(view.file().path, "b.rs");
        assert_eq!(shown(view).as_deref(), Some("b.rs"), "a_lands_last = {a_lands_last}");
        assert!(!view.loading);
    }
}

#[test]
fn an_answer_for_a_diff_that_was_closed_or_replaced_by_another_session_is_dropped() {
    let mut screen = Screen { view: None };
    let mut exec = ManualExecutor::default();
    open(&mut screen, &mut exec, "a.rs");
    screen.view = None;
    exec.deliver(0, &mut screen);
    assert!(screen.view.is_none());

    // The same file opened again is a new request: the first one's answer is stale.
    open(&mut screen, &mut exec, "a.rs");
    open(&mut screen, &mut exec, "a.rs");
    exec.deliver(1, &mut screen);
    let view = screen.view.as_ref().expect("open");
    assert!(view.loading, "the second view still waits for its own answer");
    exec.deliver(2, &mut screen);
    assert_eq!(shown(screen.view.as_ref().expect("open")).as_deref(), Some("a.rs"));
}

// ------------------------------------------------------------ refresh in place

#[test]
fn a_refresh_keeps_the_rows_and_the_place_until_the_new_ones_arrive() {
    let mut view = view(working_tree("a.rs"));
    let (wanted, _) = view.begin_load(KEY).expect("starts");
    assert!(view.finish(wanted.ticket(), loaded("a.rs", None)));
    view.scroll_x = 120.0;
    let before = view.rows.clone();

    // The file was edited: read again.
    view.request.file.added = Some(2);
    let (wanted, _) = view.begin_load(KEY).expect("a changed entry is a new request");
    assert!(!view.loading, "the old rows stay on screen while the new ones load");
    assert!(Rc::ptr_eq(&before, &view.rows));
    assert!(view.finish(wanted.ticket(), loaded("a.rs", None)));
    assert!(!Rc::ptr_eq(&before, &view.rows));
    assert_eq!(view.scroll_x, 120.0);
}

#[test]
fn a_failed_first_load_shows_the_error_but_a_failed_refresh_does_not_replace_the_diff() {
    let mut first = view(working_tree("a.rs"));
    let (wanted, _) = first.begin_load(KEY).expect("starts");
    assert!(first.finish(wanted.ticket(), failed("fatal: not a repo", None)));
    assert!(!first.loading);
    assert_eq!(first.error.as_deref(), Some("fatal: not a repo"));

    let mut refreshed = view(working_tree("a.rs"));
    let (wanted, _) = refreshed.begin_load(KEY).expect("starts");
    refreshed.finish(wanted.ticket(), loaded("a.rs", None));
    refreshed.request.file.added = Some(5);
    let (wanted, _) = refreshed.begin_load(KEY).expect("starts");
    assert!(refreshed.finish(wanted.ticket(), failed("index.lock", None)), "the header says so");
    assert!(refreshed.error.is_none());
    assert_eq!(refreshed.refresh_error.as_deref(), Some("index.lock"));
    assert_eq!(shown(&refreshed).as_deref(), Some("a.rs"));

    // The next refresh that works clears it.
    refreshed.request.file.added = Some(6);
    let (wanted, _) = refreshed.begin_load(KEY).expect("starts");
    assert!(refreshed.finish(wanted.ticket(), loaded("a.rs", None)));
    assert!(refreshed.refresh_error.is_none());
}

fn failed(message: &str, stamp: Option<Stamp>) -> Outcome {
    Outcome::Failed { message: message.into(), stamp }
}

#[test]
fn a_failing_load_is_not_run_again_at_every_status_poll() {
    // The bug: a failed load left the stamp behind, so every poll saw the file as edited and
    // started the same failing load again.
    for first_load in [true, false] {
        let mut view =
            if first_load { view(working_tree("a.rs")) } else { settled(working_tree("a.rs"), stamp(1)) };
        if !first_load {
            view.request.file.added = Some(2);
        }
        let (wanted, _) = view.begin_load(KEY).expect("starts");
        view.finish(wanted.ticket(), failed("fatal: bad object", stamp(2)));
        let mut now = change("a.rs");
        now.added = view.request.file.added;
        let now = status(vec![now]);
        assert_eq!(view.check(&now, Some(&saw(&view, stamp(2)))), Check::Same, "first_load = {first_load}");
        // An edit since is still worth another try.
        assert!(matches!(view.check(&now, Some(&saw(&view, stamp(3)))), Check::Reload(_)));
    }
}

// ------------------------------------------------------------ too large to load unasked

fn too_large() -> Outcome {
    let diff = FileDiff { too_large: true, ..FileDiff::default() };
    let built = built(&diff, KEY);
    Outcome::Loaded { diff: Arc::new(diff), stamp: None, on_disk: true, built }
}

#[test]
fn a_diff_too_large_to_load_asks_and_loading_it_anyway_lifts_only_the_size_limit() {
    let mut view = view(working_tree("a.rs"));
    view.request.file.added = Some(900_000);
    view.request.file.removed = Some(12);
    assert!(view.begin_load_anyway(KEY).is_none(), "nothing to load anyway yet");
    let (wanted, work) = view.begin_load(KEY).expect("starts");
    let Work::Load(request) = work else { panic!("expected a load") };
    assert_eq!(request.limits, DiffLimits::default());
    assert!(view.finish(wanted.ticket(), too_large()));
    assert!(view.too_large());
    assert!(!view.loading);
    // The header keeps the status's counts instead of +0 −0.
    assert_eq!(view.counts, (900_000, 12));

    let (wanted, work) = view.begin_load_anyway(KEY).expect("the user asked");
    let Work::Load(request) = work else { panic!("expected a load") };
    assert_eq!(request.limits, DiffLimits::LOAD_ANYWAY);
    assert!(view.loading && !view.too_large());
    assert!(view.finish(wanted.ticket(), loaded("a.rs", None)));
    assert_eq!(shown(&view).as_deref(), Some("a.rs"));

    // A refresh of the same file keeps the choice.
    view.request.file.added = Some(900_001);
    let (_, work) = view.begin_load(KEY).expect("starts");
    let Work::Load(request) = work else { panic!("expected a load") };
    assert_eq!(request.limits, DiffLimits::LOAD_ANYWAY);
}

// ------------------------------------------------------------ rebuilding for settings

#[test]
fn rows_are_rebuilt_only_from_a_diff_on_hand_and_never_over_a_load_in_flight() {
    let mut view = view(working_tree("a.rs"));
    assert!(view.begin_rebuild(OTHER_KEY).is_none(), "nothing to rebuild before the first load");
    let (wanted, _) = view.begin_load(KEY).expect("starts");
    view.finish(wanted.ticket(), loaded("a.rs", None));
    assert_eq!(view.built, Some(KEY));

    let (rebuild, work) = view.begin_rebuild(OTHER_KEY).expect("a diff is on hand");
    assert!(view.begin_rebuild(OTHER_KEY).is_none(), "the same rebuild is already running");
    let Work::Rebuild { diff, path } = work else { panic!("expected a rebuild") };
    let outcome = Outcome::Rebuilt(built(&diff, OTHER_KEY));
    assert_eq!(path, "a.rs");
    assert!(view.finish(rebuild.ticket(), outcome));
    assert_eq!(view.built, Some(OTHER_KEY));
    assert!(!view.split_for_test());
}

#[test]
fn a_reload_supersedes_a_rebuild_and_is_not_mistaken_for_it() {
    let mut view = view(working_tree("a.rs"));
    let (wanted, _) = view.begin_load(KEY).expect("starts");
    view.finish(wanted.ticket(), loaded("a.rs", None));

    let (rebuild, work) = view.begin_rebuild(KEY).expect("starts");
    // The same request and key, but a reload: it must start, not wait for the rebuild.
    let (reload, _) = view.begin_load(KEY).expect("a reload is not a rebuild");
    let Work::Rebuild { diff, .. } = work else { panic!("expected a rebuild") };
    assert!(!view.finish(rebuild.ticket(), Outcome::Rebuilt(built(&diff, KEY))), "stale");
    assert!(view.job.is_pending());
    assert!(view.finish(reload.ticket(), loaded("a.rs", None)));
}

// ------------------------------------------------------------ noticing edits

fn status(files: Vec<FileChange>) -> RepoStatus {
    RepoStatus {
        root: PathBuf::from("/repo"),
        branch: Some("main".into()),
        files,
        base_rev: "HEAD".into(),
        compare: None,
        compare_missing: None,
        dirty: false,
        upstream: None,
        superproject: None,
    }
}

fn settled(request: Request, stamp: Option<Stamp>) -> DiffView {
    let mut view = view(request);
    let (wanted, _) = view.begin_load(KEY).expect("starts");
    view.finish(wanted.ticket(), loaded("a.rs", stamp));
    view
}

fn stamp(len: u64) -> Option<Stamp> {
    Some(Stamp { modified: None, len })
}

fn saw(view: &DiffView, stamp: Option<Stamp>) -> Observed {
    Observed { path: view.watched_path().expect("a working-tree diff"), stamp }
}

#[test]
fn an_edit_that_keeps_the_line_counts_is_noticed_by_the_files_stamp() {
    let view = settled(working_tree("a.rs"), stamp(100));
    let now = status(vec![change("a.rs")]);
    assert_eq!(view.check(&now, Some(&saw(&view, stamp(100)))), Check::Same);
    // Same status entry (+1 −0), but the file is not the one the diff was read from.
    assert_eq!(view.check(&now, Some(&saw(&view, stamp(101)))), Check::Reload(change("a.rs")));
    // Without an observation only the status can tell.
    assert_eq!(view.check(&now, None), Check::Same);
}

#[test]
fn a_changed_status_entry_or_base_reloads() {
    let view = settled(working_tree("a.rs"), stamp(1));
    let mut grown = change("a.rs");
    grown.added = Some(7);
    assert_eq!(view.check(&status(vec![grown.clone()]), None), Check::Reload(grown));
    let mut rebased = status(vec![change("a.rs")]);
    rebased.base_rev = "abc123".into();
    assert_eq!(view.check(&rebased, None), Check::Reload(change("a.rs")));
}

#[test]
fn a_file_that_is_no_longer_changed_is_gone_until_it_comes_back() {
    let mut view = settled(working_tree("a.rs"), stamp(1));
    assert_eq!(view.check(&status(vec![change("other.rs")]), None), Check::Gone);
    view.gone = true;
    // Back among the changes: shown again, so it reloads even though nothing else differs.
    assert_eq!(view.check(&status(vec![change("a.rs")]), None), Check::Reload(change("a.rs")));
}

#[test]
fn a_commits_diff_never_reloads_and_neither_does_one_still_loading_or_in_another_repo() {
    let mut commit = working_tree("a.rs");
    commit.spec = DiffSpec { old: "abc^".into(), new: Some("abc".into()) };
    let view = settled(commit, None);
    assert!(view.watched_path().is_none());
    assert_eq!(view.check(&status(vec![]), None), Check::Same);

    let mut loading = view_of_pending();
    assert_eq!(loading.check(&status(vec![change("a.rs")]), None), Check::Same);
    assert!(loading.job.is_pending());
    loading.job = Job::default();

    let elsewhere = settled(working_tree("a.rs"), stamp(1));
    let mut other = status(vec![]);
    other.root = PathBuf::from("/other");
    assert_eq!(elsewhere.check(&other, None), Check::Same);
}

fn view_of_pending() -> DiffView {
    let mut view = view(working_tree("a.rs"));
    let _: Option<(Wanted, Work)> = view.begin_load(KEY);
    view
}

#[test]
fn the_stamp_follows_the_file_on_disk() {
    let dir = temp_dir("stamp");
    let path = dir.join("f.txt");
    std::fs::write(&path, "one").expect("write");
    let first = Observed::of(path.clone());
    assert!(first.stamp.is_some());
    assert_eq!(Observed::of(path.clone()).stamp, first.stamp);
    std::fs::write(&path, "one two").expect("write");
    assert_ne!(Observed::of(path.clone()).stamp, first.stamp);
    std::fs::remove_file(&path).expect("remove");
    assert_eq!(Observed::of(path).stamp, None);
    std::fs::remove_dir_all(dir).ok();
}

// ------------------------------------------------------------ the background half

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hd-diff-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).expect("create dir");
    dir
}

/// Run git in `dir` with a fixed identity, never the user's config (their hooks, templates,
/// fsmonitor or filters would run in the test).
fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false"])
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("run git");
    assert!(output.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn a_load_reads_git_stamps_the_file_and_builds_colored_rows() {
    let dir = temp_dir("load");
    git(&dir, &["init", "-q"]);
    std::fs::write(dir.join("a.rs"), "fn a() {}\nfn b() {}\n").expect("write");
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-q", "-m", "one"]);
    std::fs::write(dir.join("a.rs"), "fn a() {}\nfn changed() {}\n").expect("write");

    let request = Request {
        root: dir.clone(),
        file: change("a.rs"),
        spec: DiffSpec { old: "HEAD".into(), new: None },
        limits: DiffLimits::default(),
    };
    let mut job = Job::default();
    let wanted = job.begin(1).expect("starts");
    let theme = HighlightTheme::default_dark();
    let outcome = run(Work::Load(request), KEY, &theme, &wanted);
    let Some(Outcome::Loaded { diff, stamp, built, .. }) = outcome else { panic!("expected a loaded diff") };
    assert!(stamp.is_some(), "a working-tree diff is stamped");
    assert_eq!(diff.hunks.len(), 1);
    assert!(diff.new_text.as_deref().is_some_and(|t| t.contains("changed")));
    assert!(!built.rows.is_empty());
    assert_eq!(built.key, KEY);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn work_nobody_waits_for_any_more_does_not_run() {
    let mut job = Job::default();
    let superseded = job.begin(1).expect("starts");
    job.begin(2).expect("a newer request");
    // A root that does not exist: running would answer `Failed`, not `None`.
    let request = Request {
        root: PathBuf::from("/definitely/not/a/repo"),
        file: change("a.rs"),
        spec: DiffSpec { old: "HEAD".into(), new: None },
        limits: DiffLimits::default(),
    };
    let theme = HighlightTheme::default_dark();
    assert!(run(Work::Load(request.clone()), KEY, &theme, &superseded).is_none());
    let current = job.begin(3).expect("starts");
    assert!(matches!(run(Work::Load(request), KEY, &theme, &current), Some(Outcome::Failed { .. })));
}

// ------------------------------------------------------------ sideways

#[test]
fn a_gesture_moves_the_code_sideways_or_the_rows_up_and_down_never_both() {
    // The bug: the list kept a vertical swipe vertical, but its sideways drift still moved
    // the code, so the view scrolled diagonally.
    let mut view = view(working_tree("a.rs"));
    let vertical = [point(px(1.0), px(12.0)), point(px(4.0), px(9.0)), point(px(5.0), px(7.0))];
    for (ix, delta) in vertical.into_iter().enumerate() {
        let phase = if ix == 0 { TouchPhase::Started } else { TouchPhase::Moved };
        assert_eq!(view.sideways(delta, phase, true), 0.0, "step {ix}");
    }
    view.sideways(point(px(0.0), px(0.0)), TouchPhase::Ended, true);
    // A sideways swipe moves the code, drift and all.
    assert_eq!(view.sideways(point(px(-12.0), px(2.0)), TouchPhase::Started, true), -12.0);
    assert_eq!(view.sideways(point(px(-8.0), px(3.0)), TouchPhase::Moved, true), -8.0);
    // A mouse wheel's notches have no gesture: its sideways part is used as it is.
    assert_eq!(view.sideways(point(px(-20.0), px(0.0)), TouchPhase::Moved, false), -20.0);
}

/// A submodule's "diff" is the commits between its two sides, not git's text: the load answers
/// with them, and there is no file to watch for edits.
#[test]
fn a_submodule_loads_its_commits_not_a_patch() {
    use wormhole_core::git::{Checkout, Range, Submodule};
    let dir = std::env::temp_dir().join(format!("hd-submodule-load-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("libs/lib")).expect("mkdir");
    let mut file = change("libs/lib");
    file.submodule = Some(Submodule {
        old: Some("a".repeat(40)),
        new: Some("b".repeat(40)),
        commit_changed: true,
        removed: false,
        tracked: false,
        untracked: false,
        log: None,
    });
    let request = Request {
        root: dir.clone(),
        file,
        spec: DiffSpec { old: "HEAD".into(), new: None },
        limits: DiffLimits::default(),
    };
    let mut view = view(request.clone());
    assert_eq!(view.watched_path(), None, "a folder is not a file that gets edited");
    let (wanted, work) = view.begin_load(KEY).expect("starts");
    let ticket = wanted.ticket();
    let theme = HighlightTheme::default_dark();
    // The folder has no repository in it: nothing to ask git, and it is said so.
    let outcome = run(work, KEY, &theme, &wanted).expect("answers");
    let Outcome::Submodule(log) = outcome else { panic!("expected the submodule's commits") };
    assert_eq!((log.checkout, &log.range), (Checkout::NotCheckedOut, &Range::Unavailable));
    assert!(view.finish(ticket, Outcome::Submodule(log)));
    assert!(view.submodule.is_some() && view.diff.is_none() && !view.loading);
    std::fs::remove_dir_all(dir).ok();
}
