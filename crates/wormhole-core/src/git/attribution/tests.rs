use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::snapshot::Fingerprint;
use super::{Attribution, Kind, MAX_FILES, Snapshot};
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
    fn at(&self, secs: u64) -> Instant {
        self.start + Duration::from_secs(secs)
    }

    /// A fingerprint nothing else has.
    fn print(&mut self) -> Fingerprint {
        self.made += 1;
        let path = self.disk.path().join(format!("file-{}", self.made));
        std::fs::write(&path, b"x").expect("write");
        Fingerprint::of(&path)
    }

    fn snapshot(&self, files: &[(&str, Fingerprint)]) -> Snapshot {
        Snapshot::of_prints(&self.root, files.iter().map(|&(path, print)| (path.to_string(), print)))
    }

    /// Terminal `id` is in the repository and `working` as of second `secs`.
    fn track(&mut self, id: i64, kind: Kind, working: bool, secs: u64) -> bool {
        let (root, now) = (self.root.clone(), self.at(secs));
        self.attribution.track(id, Some(&root), kind, working, now)
    }

    fn observe(&mut self, files: &[(&str, Fingerprint)], secs: u64) -> bool {
        let (snapshot, asked) = (self.snapshot(files), self.at(secs));
        self.attribution.observe(snapshot, asked)
    }

    fn changes(&self, id: i64) -> Option<usize> {
        self.attribution.changes_of(id)
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
fn an_editing_that_leaves_the_count_alone_reports_no_change() {
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
    w.observe(&[], 5);
    w.track(AGENT, Kind::Told, true, 1);
    w.track(AGENT, Kind::Told, false, 4);
    w.track(SHELL, Kind::Plain, true, 6);
    w.observe(&[("a", a)], 7);
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
    w.attribution.track(AGENT, Some(Path::new("/elsewhere")), Kind::Told, true, w.at(2));
    w.observe(&[("a", a)], 3);
    assert_eq!(w.changes(AGENT), None);
    w.attribution.track(AGENT, None, Kind::Told, true, w.at(4));
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
    w.observe(&[], 4);
    w.track(SHELL, Kind::Plain, true, 5);
    w.track(AGENT, Kind::Told, false, 5);
    w.observe(&[("a", a2)], 6);
    assert_eq!((w.changes(AGENT), w.changes(SHELL)), (None, Some(1)));
}

#[test]
fn a_snapshot_asked_for_before_the_last_one_is_ignored() {
    let mut w = World::new();
    let (a, b) = (w.print(), w.print());
    w.observe(&[], 1);
    w.track(AGENT, Kind::Told, true, 2);
    w.observe(&[("a", a)], 5);
    // Asked for at 4, answered after the one asked for at 5: it would take `a` away again.
    assert!(!w.observe(&[("b", b)], 4));
    assert_eq!(w.changes(AGENT), Some(1));
    // And the baseline is still the one from 5: `b` is new against it.
    w.observe(&[("a", a), ("b", b)], 6);
    assert_eq!(w.changes(AGENT), Some(2));
}

#[test]
fn a_status_against_a_branch_is_not_a_snapshot() {
    let root = PathBuf::from("/repo");
    assert!(Snapshot::take(&status(&root, 3, Some("main"))).is_none());
    assert!(Snapshot::take(&status(&root, 3, None)).is_some());
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
    assert!(w.attribution.track(AGENT, Some(Path::new("/other")), Kind::Told, true, w.at(4)));
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

    let big = Snapshot::take(&status(&w.root, MAX_FILES + 1, None)).expect("a snapshot, without files");
    assert!(w.attribution.observe(big, w.at(4)));
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
    let before = Snapshot::take(&status).expect("a snapshot");
    std::fs::write(repo.path().join("kept"), b"longer").expect("write");
    let after = Snapshot::take(&status).expect("a snapshot");
    let (before, after) = (before.prints.expect("files"), after.prints.expect("files"));
    // The deleted file looks the same both times, the edited one does not.
    let touched: Vec<&str> = after.touched_since(&before).collect();
    assert_eq!(touched, ["kept"]);
}
