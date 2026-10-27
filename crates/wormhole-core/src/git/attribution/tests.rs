use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::snapshot::{Fingerprint, MAX_FILES};
use super::{Attribution, Kind, Place, Snapshot};
use crate::config::SharedChanges;
use crate::git::{ChangeKind, FileChange, RepoStatus};

const AGENT: i64 = 1;
const OTHER_AGENT: i64 = 2;
const SHELL: i64 = 3;

/// A repository at `/repo`, a clock, and a disk to mint distinct fingerprints from: each file made
/// there is another file, so no two are alike.
struct World {
    attribution: Attribution,
    start: Instant,
    root: PathBuf,
    disk: tempfile::TempDir,
    made: usize,
}

impl World {
    fn new() -> Self {
        Self {
            attribution: Attribution::default(),
            start: Instant::now(),
            root: PathBuf::from("/repo"),
            disk: tempfile::tempdir().expect("tempdir"),
            made: 0,
        }
    }

    /// `secs` seconds after the start.
    fn at(&self, secs: impl Into<f64>) -> Instant {
        self.start + Duration::from_secs_f64(secs.into())
    }

    /// A fingerprint nothing else has.
    fn print(&mut self) -> Fingerprint {
        self.made += 1;
        let path = self.disk.path().join(format!("file-{}", self.made));
        std::fs::write(&path, b"x").expect("write");
        Fingerprint::of_path(&path)
    }

    /// The picture of these files, taken at second `secs`.
    fn snapshot(&self, files: &[(&str, Fingerprint)], secs: impl Into<f64>) -> Snapshot {
        let prints = files.iter().map(|&(path, print)| (path.to_string(), print));
        Snapshot::of_prints(&self.root, self.at(secs), prints)
    }

    /// Terminal `id` is in the repository and `working` as of second `secs`.
    fn track(&mut self, id: i64, kind: Kind, working: bool, secs: impl Into<f64>) -> bool {
        let (root, now) = (self.root.clone(), self.at(secs));
        self.attribution.track(id, Place::In(&root), kind, working, now)
    }

    fn observe(&mut self, files: &[(&str, Fingerprint)], secs: impl Into<f64>) -> bool {
        let snapshot = self.snapshot(files, secs);
        self.attribution.observe(snapshot)
    }

    fn changes(&self, id: i64) -> Option<usize> {
        self.attribution.changes_of(id, SharedChanges::Each)
    }
}

fn status(root: &Path, files: usize, compare: Option<&str>) -> RepoStatus {
    RepoStatus {
        root: root.to_path_buf(),
        branch: None,
        files: (0..files).map(|n| FileChange::new(format!("f{n}"), ChangeKind::Modified)).collect(),
        base_rev: "HEAD".into(),
        compare: compare.map(str::to_string),
        compare_missing: None,
        dirty: files > 0,
        upstream: None,
        superproject: None,
    }
}

#[test]
fn the_first_snapshot_is_a_baseline_that_belongs_to_nobody() {
    let mut w = World::new();
    let (a, b) = (w.print(), w.print());
    w.track(AGENT, Kind::Told, true, 1);
    w.track(SHELL, Kind::Plain, true, 1);
    assert!(!w.observe(&[("a", a), ("b", b)], 2));
    assert_eq!((w.changes(AGENT), w.changes(SHELL)), (None, None));
}

#[test]
fn a_working_agent_takes_the_files_that_are_new_or_look_different() {
    let mut w = World::new();
    let (a1, a2, b, c) = (w.print(), w.print(), w.print(), w.print());
    w.observe(&[("a", a1), ("c", c)], 1);
    w.track(AGENT, Kind::Told, true, 2);
    // `a` was changed already and is edited again, `b` is new, `c` is as it was.
    assert!(w.observe(&[("a", a2), ("b", b), ("c", c)], 3));
    assert_eq!(w.changes(AGENT), Some(2));
}

#[test]
fn an_edit_that_leaves_the_count_alone_reports_no_change() {
    let mut w = World::new();
    let (a1, a2) = (w.print(), w.print());
    w.observe(&[], 1);
    w.track(AGENT, Kind::Told, true, 2);
    assert!(w.observe(&[("a", a1)], 3));
    w.track(AGENT, Kind::Told, true, 4);
    assert!(!w.observe(&[("a", a2)], 5));
    assert_eq!(w.changes(AGENT), Some(1));
}

#[test]
fn agents_working_at_once_both_take_the_file() {
    let mut w = World::new();
    let a = w.print();
    w.observe(&[], 1);
    w.track(AGENT, Kind::Told, true, 2);
    w.track(OTHER_AGENT, Kind::Told, true, 2);
    w.observe(&[("a", a)], 3);
    assert_eq!((w.changes(AGENT), w.changes(OTHER_AGENT)), (Some(1), Some(1)));
}

#[test]
fn an_agent_takes_the_files_before_a_plain_command_does() {
    let mut w = World::new();
    let a = w.print();
    w.observe(&[], 1);
    w.track(AGENT, Kind::Told, true, 2);
    w.track(SHELL, Kind::Plain, true, 2);
    w.observe(&[("a", a)], 3);
    assert_eq!((w.changes(AGENT), w.changes(SHELL)), (Some(1), None));
}

#[test]
fn a_plain_command_takes_the_files_when_no_agent_worked() {
    let mut w = World::new();
    let a = w.print();
    // The agent worked and stopped before the last picture; the command runs after it.
    w.track(AGENT, Kind::Told, true, 1);
    w.track(AGENT, Kind::Told, false, 2);
    w.observe(&[], 3);
    w.track(SHELL, Kind::Plain, true, 4);
    w.observe(&[("a", a)], 5);
    assert_eq!((w.changes(AGENT), w.changes(SHELL)), (None, Some(1)));
}

#[test]
fn nobody_working_means_an_edit_from_outside() {
    let mut w = World::new();
    let a = w.print();
    w.observe(&[], 1);
    w.track(AGENT, Kind::Told, false, 2);
    w.track(SHELL, Kind::Plain, false, 2);
    assert!(!w.observe(&[("a", a)], 3));
    assert_eq!((w.changes(AGENT), w.changes(SHELL)), (None, None));
}

#[test]
fn a_terminal_that_worked_since_the_last_snapshot_but_has_stopped_still_takes_the_file() {
    let mut w = World::new();
    let a = w.print();
    w.observe(&[], 1);
    w.track(AGENT, Kind::Told, true, 2);
    w.track(AGENT, Kind::Told, false, 3);
    w.observe(&[("a", a)], 4);
    assert_eq!(w.changes(AGENT), Some(1));
}

#[test]
fn work_before_the_last_snapshot_does_not_take_a_later_edit() {
    let mut w = World::new();
    let (a, b) = (w.print(), w.print());
    w.track(AGENT, Kind::Told, true, 1);
    w.track(AGENT, Kind::Told, false, 2);
    w.observe(&[], 3);
    w.observe(&[("a", a)], 4);
    assert_eq!(w.changes(AGENT), None);
    w.observe(&[("a", a), ("b", b)], 5);
    assert_eq!(w.changes(AGENT), None);
}

#[test]
fn a_terminal_elsewhere_takes_nothing() {
    let mut w = World::new();
    let a = w.print();
    w.observe(&[], 1);
    w.attribution.track(AGENT, Place::In(Path::new("/elsewhere")), Kind::Told, true, w.at(2));
    w.observe(&[("a", a)], 3);
    assert_eq!(w.changes(AGENT), None);
    w.attribution.track(AGENT, Place::Outside, Kind::Told, true, w.at(4));
    assert_eq!(w.changes(AGENT), None);
}

#[test]
fn a_file_stops_counting_when_it_leaves_the_status() {
    let mut w = World::new();
    let (a, b) = (w.print(), w.print());
    w.observe(&[], 1);
    w.track(AGENT, Kind::Told, true, 2);
    w.observe(&[("a", a), ("b", b)], 3);
    assert_eq!(w.changes(AGENT), Some(2));
    // `a` was committed.
    assert!(w.observe(&[("b", b)], 4));
    assert_eq!(w.changes(AGENT), Some(1));
    assert!(w.observe(&[], 5));
    assert_eq!(w.changes(AGENT), None);
}

#[test]
fn a_file_taken_again_after_it_left_the_status_is_new() {
    let mut w = World::new();
    let (a1, a2) = (w.print(), w.print());
    w.observe(&[], 1);
    w.track(AGENT, Kind::Told, true, 2);
    w.observe(&[("a", a1)], 3);
    w.track(AGENT, Kind::Told, false, 3.5);
    w.observe(&[], 4);
    w.track(SHELL, Kind::Plain, true, 5);
    w.observe(&[("a", a2)], 6);
    assert_eq!((w.changes(AGENT), w.changes(SHELL)), (None, Some(1)));
}

#[test]
fn a_picture_taken_before_the_last_one_that_came_in_is_ignored() {
    let mut w = World::new();
    let (a, b) = (w.print(), w.print());
    w.observe(&[], 1);
    w.track(AGENT, Kind::Told, true, 2);
    w.observe(&[("a", a)], 5);
    // Taken at 4 but arrived after the one taken at 5: it would take `a` away again.
    assert!(!w.observe(&[("b", b)], 4));
    assert_eq!(w.changes(AGENT), Some(1));
    // And the baseline is still the one from 5: `b` is new against it.
    w.observe(&[("a", a), ("b", b)], 6);
    assert_eq!(w.changes(AGENT), Some(2));
}

#[test]
fn a_status_against_a_branch_is_not_a_snapshot() {
    let root = PathBuf::from("/repo");
    let now = Instant::now();
    assert!(Snapshot::take(&status(&root, 3, Some("main")), now).is_none());
    assert!(Snapshot::take(&status(&root, 3, None), now).is_some());
}

#[test]
fn closing_a_terminal_forgets_its_files() {
    let mut w = World::new();
    let a = w.print();
    w.observe(&[], 1);
    w.track(AGENT, Kind::Told, true, 2);
    w.observe(&[("a", a)], 3);
    assert!(w.attribution.forget(AGENT));
    assert_eq!(w.changes(AGENT), None);
    assert!(!w.attribution.forget(AGENT));
    // Nothing is left of the repository either: the next picture is a baseline again.
    assert!(w.attribution.repos.is_empty());
}

#[test]
fn a_terminal_that_moves_to_another_repository_leaves_its_files_behind() {
    let mut w = World::new();
    let a = w.print();
    w.observe(&[], 1);
    w.track(AGENT, Kind::Told, true, 2);
    w.observe(&[("a", a)], 3);
    assert!(w.attribution.track(AGENT, Place::In(Path::new("/other")), Kind::Told, true, w.at(4)));
    assert_eq!(w.changes(AGENT), None);
}

#[test]
fn a_status_with_too_many_files_says_nothing_and_the_next_is_a_baseline() {
    let mut w = World::new();
    let (a, b) = (w.print(), w.print());
    w.observe(&[], 1);
    w.track(AGENT, Kind::Told, true, 2);
    w.observe(&[("a", a)], 3);
    assert_eq!(w.changes(AGENT), Some(1));

    let big =
        Snapshot::take(&status(&w.root, MAX_FILES + 1, None), w.at(4)).expect("a snapshot, without files");
    assert!(w.attribution.observe(big));
    assert_eq!(w.changes(AGENT), None);
    // What the next picture shows was there before it, as far as anyone can tell.
    assert!(!w.observe(&[("a", a), ("b", b)], 5));
    assert_eq!(w.changes(AGENT), None);
    w.observe(&[("a", a), ("b", b), ("c", a)], 6);
    assert_eq!(w.changes(AGENT), Some(1));
}

#[test]
fn a_snapshot_tells_an_edited_file_from_a_deleted_one() {
    let repo = tempfile::tempdir().expect("tempdir");
    std::fs::write(repo.path().join("kept"), b"one").expect("write");
    let mut status = status(repo.path(), 0, None);
    status.files =
        vec![FileChange::new("kept", ChangeKind::Modified), FileChange::new("gone", ChangeKind::Deleted)];
    let before = Snapshot::take(&status, Instant::now()).expect("a snapshot");
    std::fs::write(repo.path().join("kept"), b"longer").expect("write");
    let after = Snapshot::take(&status, Instant::now()).expect("a snapshot");
    let (before, after) = (before.prints.expect("files"), after.prints.expect("files"));
    // The deleted file looks the same both times, the edited one does not.
    let touched: Vec<&str> = after.touched_since(&before).collect();
    assert_eq!(touched, ["kept"]);
}

#[test]
fn work_that_stops_between_two_samples_still_takes_its_last_edits() {
    let mut w = World::new();
    let a = w.print();
    w.observe(&[], 1);
    w.track(AGENT, Kind::Told, true, 2);
    // A picture, then the agent stops before the tick after next could have seen it working.
    w.observe(&[], 2.1);
    w.track(AGENT, Kind::Told, false, 2.4);
    assert!(w.observe(&[("a", a)], 4));
    assert_eq!(w.changes(AGENT), Some(1));
    // Once it has stopped it is not working again until a sample says so.
    let b = w.print();
    assert!(!w.observe(&[("a", a), ("b", b)], 6));
}

#[test]
fn pictures_are_ordered_by_when_they_were_taken_not_when_they_arrive() {
    let mut w = World::new();
    let (a, b, c) = (w.print(), w.print(), w.print());
    w.observe(&[], 1);
    w.track(AGENT, Kind::Told, true, 2);
    // Taken at 3, 5 and 4; they arrive in that order.
    w.observe(&[("a", a)], 3);
    w.observe(&[("a", a), ("c", c)], 5);
    assert!(!w.observe(&[("a", a), ("b", b)], 4), "late: ignored whole");
    assert_eq!(w.changes(AGENT), Some(2));
    // The one after is judged against the picture taken at 5, not the late one.
    w.track(AGENT, Kind::Told, true, 5.5);
    w.observe(&[("a", a), ("b", b), ("c", c)], 6);
    assert_eq!(w.changes(AGENT), Some(3));
}

#[test]
fn an_edit_inside_a_submodule_that_was_dirty_already_is_a_touch() {
    let mut w = World::new();
    let dir = w.print();
    w.observe(&[("lib", dir.with_submodule(1))], 1);
    w.track(AGENT, Kind::Told, true, 2);
    // The folder's own stamp and "dirty" are as they were; its checked-out commit is not.
    w.observe(&[("lib", dir.with_submodule(2))], 3);
    assert_eq!(w.changes(AGENT), Some(1));
}

#[test]
fn a_picture_with_nothing_touched_leaves_no_trace() {
    let mut w = World::new();
    let a = w.print();
    w.observe(&[("a", a)], 1);
    w.track(AGENT, Kind::Told, true, 2);
    assert!(!w.observe(&[("a", a)], 3));
    assert!(w.attribution.repos[&w.root].claims.is_empty(), "no empty set for a terminal with nothing");
}

#[test]
fn a_terminal_whose_place_is_not_known_yet_keeps_its_files() {
    let mut w = World::new();
    let a = w.print();
    w.observe(&[], 1);
    w.track(AGENT, Kind::Told, true, 2);
    w.observe(&[("a", a)], 3);
    // Moved to another window: git has not answered there yet.
    assert!(!w.attribution.track(AGENT, Place::Unknown, Kind::Told, true, w.at(4)));
    assert_eq!(w.changes(AGENT), Some(1));
    // It still counts as working while it is not known where.
    let b = w.print();
    w.observe(&[("a", a), ("b", b)], 5);
    assert_eq!(w.changes(AGENT), Some(2));
    // Only an answer that it is in no repository lets them go.
    assert!(w.attribution.track(AGENT, Place::Outside, Kind::Told, true, w.at(6)));
    assert_eq!(w.changes(AGENT), None);
}

#[test]
fn a_terminal_leaving_does_not_forget_a_repository_another_is_in() {
    let mut w = World::new();
    let a = w.print();
    w.observe(&[], 1);
    w.track(AGENT, Kind::Told, true, 2);
    w.track(SHELL, Kind::Plain, false, 2);
    w.observe(&[("a", a)], 3);
    w.attribution.forget(SHELL);
    assert_eq!(w.changes(AGENT), Some(1), "the repository and the agent's file are kept");
}

#[test]
fn a_file_that_several_terminals_changed_counts_for_each_or_for_nobody_as_the_policy_says() {
    let mut w = World::new();
    let (shared, own) = (w.print(), w.print());
    w.observe(&[], 1);
    w.track(AGENT, Kind::Told, true, 2);
    w.track(OTHER_AGENT, Kind::Told, true, 2);
    w.observe(&[("shared", shared)], 3);
    // The first alone after the second stops: its own file is its alone.
    w.track(OTHER_AGENT, Kind::Told, false, 3.5);
    w.observe(&[("shared", shared)], 4);
    w.track(AGENT, Kind::Told, true, 5);
    w.observe(&[("shared", shared), ("own", own)], 6);
    let count = |w: &World, id, policy| w.attribution.changes_of(id, policy);
    assert_eq!(count(&w, AGENT, SharedChanges::Each), Some(2));
    assert_eq!(count(&w, OTHER_AGENT, SharedChanges::Each), Some(1));
    // Switching is instant: the claims are as they were.
    assert_eq!(count(&w, AGENT, SharedChanges::None), Some(1), "only the file nobody else touched");
    assert_eq!(count(&w, OTHER_AGENT, SharedChanges::None), None, "its only file is shared");
    assert_eq!(count(&w, AGENT, SharedChanges::Each), Some(2));
}

#[test]
fn what_was_kept_survives_the_first_picture_as_a_baseline_and_is_pruned_by_it() {
    let mut w = World::new();
    let (a, b, c) = (w.print(), w.print(), w.print());
    // A terminal comes back with two files it changed; a third repository file is nobody's.
    w.attribution.restore(AGENT, &w.root.clone(), ["a".to_string(), "b".to_string()]);
    assert_eq!(w.changes(AGENT), Some(2), "back at once, before git has been asked");
    assert!(w.attribution.claims_of(AGENT).is_some());
    w.track(AGENT, Kind::Told, true, 1);
    // `b` was committed while the app was closed; `c` was changed by something else meanwhile.
    assert!(w.observe(&[("a", a), ("c", c)], 2), "the count dropped");
    assert_eq!(w.changes(AGENT), Some(1), "pruned to what is still changed, `c` not taken");
    let (_, claimed) = w.attribution.claims_of(AGENT).expect("kept");
    assert!(claimed.contains("a") && !claimed.contains("b") && !claimed.contains("c"));
    // From then on it is attributed as usual.
    w.observe(&[("a", a), ("b", b), ("c", c)], 3);
    assert_eq!(w.changes(AGENT), Some(2), "`a` kept and `b` new; `c` was in the baseline, so it is nobody's");
}

#[test]
fn a_restored_terminal_that_is_somewhere_else_now_lets_go_of_what_was_kept() {
    let mut w = World::new();
    w.attribution.restore(AGENT, &w.root.clone(), ["a".to_string()]);
    assert!(w.attribution.track(AGENT, Place::In(Path::new("/other")), Kind::Told, false, w.at(1)));
    assert_eq!(w.changes(AGENT), None);
    assert!(!w.attribution.repos.contains_key(&w.root), "nobody is in the repository any more");
}

#[test]
fn a_terminals_revision_moves_when_its_files_do_and_only_then() {
    let mut w = World::new();
    let (a, a2) = (w.print(), w.print());
    w.observe(&[], 1);
    w.track(AGENT, Kind::Told, true, 2);
    w.track(OTHER_AGENT, Kind::Told, false, 2);
    let start = w.attribution.revision_of(AGENT);
    w.observe(&[], 3);
    assert_eq!(w.attribution.revision_of(AGENT), start, "nothing touched");
    w.observe(&[("a", a)], 4);
    let first = w.attribution.revision_of(AGENT);
    assert!(first > start);
    assert_eq!(w.attribution.revision_of(OTHER_AGENT), 0, "another terminal's is its own");
    // Same count, other files: committed `a`, edited `b` meanwhile.
    w.observe(&[("b", a2)], 5);
    assert!(w.attribution.revision_of(AGENT) > first, "a different set of files, though as many");
    let before = w.attribution.revision_of(AGENT);
    w.observe(&[("b", a2)], 6);
    assert_eq!(w.attribution.revision_of(AGENT), before, "a picture of the same files");
    assert_eq!(w.attribution.revision_of(99), 0, "a terminal not known");
}

#[test]
fn what_was_kept_is_not_put_back_over_a_terminal_the_model_knows() {
    let mut w = World::new();
    let a = w.print();
    w.observe(&[], 1);
    w.track(AGENT, Kind::Told, true, 2);
    w.observe(&[("a", a)], 3);
    // It moved to another window in this run, which reads what was kept: older than what is here.
    w.attribution.restore(AGENT, &w.root.clone(), ["stale".to_string()]);
    assert_eq!(w.changes(AGENT), Some(1));
    let (_, claimed) = w.attribution.claims_of(AGENT).expect("kept");
    assert!(claimed.contains("a") && !claimed.contains("stale"));
}
