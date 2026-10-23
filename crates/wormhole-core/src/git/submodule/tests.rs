use std::path::Path;
use std::sync::Arc;

use super::log::{head_of, log_for};
use super::{Checkout, Range, Submodule, SubmoduleLog, annotate_files, short, submodule_log, superproject};
use crate::git::testing::{git, init, repo_with, spawns, write};
use crate::git::{ChangeKind, FileChange, commit_files, log, status};

const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

/// A parent repository with the submodule `libs/lib` (one commit in it), committed.
struct Fixture {
    /// Where `libs/lib` was cloned from.
    _upstream: tempfile::TempDir,
    parent: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let upstream = repo_with(&[("lib.txt", "1\n")]);
        let parent = repo_with(&[("main.txt", "m\n")]);
        let source = upstream.path().to_str().expect("utf-8 path");
        // The file protocol is off by default; only these throwaway repositories need it.
        git(
            parent.path(),
            &["-c", "protocol.file.allow=always", "submodule", "add", "-q", source, "libs/lib"],
        );
        git(parent.path(), &["commit", "-q", "-m", "add the submodule"]);
        Self { _upstream: upstream, parent }
    }

    fn root(&self) -> &Path {
        self.parent.path()
    }

    fn lib(&self) -> std::path::PathBuf {
        self.root().join("libs/lib")
    }

    /// `n` more commits in the submodule's folder.
    fn commit_in_lib(&self, n: usize) {
        for i in 0..n {
            write(
                &self.lib(),
                "lib.txt",
                &format!("change {i} {}\n", head_of(&self.lib()).unwrap_or_default()),
            );
            git(&self.lib(), &["commit", "-q", "-am", &format!("lib change {i}")]);
        }
    }

    /// The submodule's entry in the parent's status, with its commits looked up.
    fn entry(&self) -> FileChange {
        let mut files = status(self.root(), None).expect("status").expect("repo").files;
        annotate_files(self.root(), &mut files);
        files.into_iter().find(|f| f.path == "libs/lib").expect("the submodule is listed")
    }

    fn label(&self) -> String {
        let file = self.entry();
        file.submodule.expect("a submodule").label(file.kind)
    }
}

fn counts(log: &SubmoduleLog) -> (usize, usize) {
    match &log.range {
        Range::Commits { ahead_total, behind_total, .. } => (*ahead_total, *behind_total),
        other => panic!("not a list of commits: {other:?}"),
    }
}

fn sub(old: Option<&str>, new: Option<&str>) -> Submodule {
    Submodule {
        old: old.map(str::to_string),
        new: new.map(str::to_string),
        commit_changed: true,
        removed: false,
        tracked: false,
        untracked: false,
        log: None,
    }
}

fn with_log(mut sub: Submodule, range: Range) -> Submodule {
    sub.log = Some(Arc::new(SubmoduleLog {
        checkout: Checkout::Ready,
        old: sub.old.clone(),
        new: sub.new.clone(),
        range,
    }));
    sub
}

fn listed(ahead: usize, behind: usize) -> Range {
    Range::Commits { ahead: Vec::new(), behind: Vec::new(), ahead_total: ahead, behind_total: behind }
}

#[test]
fn the_row_says_in_plain_words_what_moved() {
    let label = |sub: &Submodule| sub.label(ChangeKind::Modified);
    let moved = |ahead, behind| with_log(sub(Some(A), Some(B)), listed(ahead, behind));
    assert_eq!(label(&moved(2, 0)), "2 new commits");
    assert_eq!(label(&moved(1, 0)), "1 new commit");
    assert_eq!(label(&moved(0, 1)), "1 commit back");
    assert_eq!(label(&moved(0, 3)), "3 commits back");
    assert_eq!(label(&moved(2, 1)), "2 new commits, 1 dropped");
    // Not counted yet, or not countable: the commits are not in the clone.
    assert_eq!(label(&sub(Some(A), Some(B))), "new commits");
    let unfetched = with_log(sub(Some(A), Some(B)), Range::NotFetched { old: false, new: true });
    assert_eq!(label(&unfetched), "new commits");
}

#[test]
fn what_is_uncommitted_comes_after_the_commits() {
    let mut s = with_log(sub(Some(A), Some(B)), listed(3, 0));
    s.tracked = true;
    assert_eq!(s.label(ChangeKind::Modified), "3 new commits · uncommitted changes");
    s.untracked = true;
    assert_eq!(s.label(ChangeKind::Modified), "3 new commits · uncommitted changes · untracked files");
    let mut dirty = sub(Some(A), Some(A));
    (dirty.commit_changed, dirty.untracked) = (false, true);
    assert_eq!(dirty.label(ChangeKind::Modified), "untracked files");
    dirty.tracked = true;
    assert_eq!(dirty.label(ChangeKind::Modified), "uncommitted changes · untracked files");
    // A conflict keeps its own kind, and the label is the submodule's.
    assert_eq!(s.label(ChangeKind::Conflicted), "3 new commits · uncommitted changes · untracked files");
}

#[test]
fn a_new_a_removed_and_an_unusable_submodule_have_their_own_words() {
    let added = sub(None, Some(B));
    assert_eq!(added.label(ChangeKind::Added), format!("added at {}", short(B)));
    assert_eq!(added.label(ChangeKind::Added), "added at bbbbbbb");
    let mut gone = sub(Some(A), None);
    assert_eq!(gone.label(ChangeKind::Deleted), "folder missing", "the parent still records it");
    gone.removed = true;
    assert_eq!(gone.label(ChangeKind::Deleted), "removed");
    let mut nothing = sub(Some(A), Some(A));
    nothing.commit_changed = false;
    assert_eq!(nothing.label(ChangeKind::Modified), "changed", "never an empty label");

    let mut not_there = sub(Some(A), Some(B));
    for (checkout, words) in
        [(Checkout::NotCheckedOut, "not checked out"), (Checkout::FolderMissing, "folder missing")]
    {
        not_there.log =
            Some(Arc::new(SubmoduleLog { checkout, old: None, new: None, range: Range::Unavailable }));
        assert_eq!(not_there.label(ChangeKind::Modified), words);
        assert!(!not_there.checked_out());
    }
}

#[test]
fn new_commits_in_the_submodule_are_counted_and_listed_newest_first() {
    let fx = Fixture::new();
    assert!(status(fx.root(), None).expect("status").expect("repo").files.is_empty(), "clean to begin with");
    fx.commit_in_lib(2);

    let file = fx.entry();
    let sub = file.submodule.as_ref().expect("a submodule");
    assert!(sub.commit_changed && !sub.tracked && !sub.untracked);
    assert_eq!(fx.label(), "2 new commits");

    let log = submodule_log(fx.root(), &file.path, sub).expect("log");
    assert_eq!(counts(&log), (2, 0));
    let Range::Commits { ahead, behind, .. } = &log.range else { panic!("{:?}", log.range) };
    let subjects: Vec<_> = ahead.iter().map(|c| c.subject.as_str()).collect();
    assert_eq!(subjects, ["lib change 1", "lib change 0"], "newest first");
    assert!(behind.is_empty());
    assert_eq!(ahead[0].author, "Test");
    assert_eq!(Some(ahead[0].sha.as_str()), log.new.as_deref());
}

#[test]
fn a_refresh_costs_no_process_for_a_pair_it_has_seen() {
    let fx = Fixture::new();
    fx.commit_in_lib(1);
    let mut files = status(fx.root(), None).expect("status").expect("repo").files;
    // The first look asks the submodule for its commits (one process); the second costs none, though
    // everything but the commit it is at is read again.
    assert_eq!(spawns(|| annotate_files(fx.root(), &mut files)), 1);
    assert_eq!(spawns(|| annotate_files(fx.root(), &mut files)), 0);
    // Another commit is another pair.
    fx.commit_in_lib(1);
    assert_eq!(spawns(|| annotate_files(fx.root(), &mut files)), 1);
    assert_eq!(counts(files[0].submodule.as_ref().and_then(|s| s.log.as_deref()).expect("log")), (2, 0));
}

#[test]
fn the_commit_is_read_from_packed_refs_too() {
    let fx = Fixture::new();
    fx.commit_in_lib(1);
    let loose = head_of(&fx.lib()).expect("loose ref");
    git(&fx.lib(), &["pack-refs", "--all"]);
    assert_eq!(head_of(&fx.lib()), Some(loose.clone()));
    git(&fx.lib(), &["checkout", "-q", "--detach"]);
    assert_eq!(head_of(&fx.lib()), Some(loose), "a detached HEAD is the id itself");
}

#[test]
fn a_pointer_that_went_back_lists_the_commits_it_dropped() {
    let fx = Fixture::new();
    fx.commit_in_lib(2);
    git(fx.root(), &["add", "libs/lib"]);
    git(fx.root(), &["commit", "-q", "-m", "advance the submodule"]);
    assert!(status(fx.root(), None).expect("status").expect("repo").files.is_empty(), "clean again");

    git(&fx.lib(), &["checkout", "-q", "HEAD~1"]);
    assert_eq!(fx.label(), "1 commit back");
    let file = fx.entry();
    let log = submodule_log(fx.root(), &file.path, file.submodule.as_ref().expect("sub")).expect("log");
    let Range::Commits { ahead, behind, .. } = &log.range else { panic!("{:?}", log.range) };
    assert!(ahead.is_empty());
    assert_eq!(behind.iter().map(|c| c.subject.as_str()).collect::<Vec<_>>(), ["lib change 1"]);
}

#[test]
fn work_inside_the_submodule_is_named_beside_its_commits() {
    let fx = Fixture::new();
    write(&fx.lib(), "lib.txt", "edited\n");
    assert_eq!(fx.label(), "uncommitted changes");
    write(&fx.lib(), "scratch.txt", "x\n");
    assert_eq!(fx.label(), "uncommitted changes · untracked files");
    git(&fx.lib(), &["checkout", "-q", "--", "lib.txt"]);
    assert_eq!(fx.label(), "untracked files");
    git(&fx.lib(), &["add", "scratch.txt"]);
    git(&fx.lib(), &["commit", "-q", "-m", "scratch"]);
    write(&fx.lib(), "lib.txt", "edited again\n");
    assert_eq!(fx.label(), "1 new commit · uncommitted changes");
}

#[test]
fn commits_the_clone_lacks_are_said_so_not_failed() {
    let fx = Fixture::new();
    fx.commit_in_lib(1);
    let ahead = head_of(&fx.lib()).expect("head");
    // The parent now records a commit the submodule's clone does not have (it was pulled, the
    // submodule not fetched).
    let missing = "1234567890123456789012345678901234567890";
    git(fx.root(), &["update-index", "--cacheinfo", &format!("160000,{missing},libs/lib")]);
    git(fx.root(), &["commit", "-q", "-m", "a commit from elsewhere"]);
    let file = fx.entry();
    let sub = file.submodule.as_ref().expect("a submodule");
    assert_eq!((sub.old.as_deref(), sub.commit_changed), (Some(missing), true));
    let log = submodule_log(fx.root(), &file.path, sub).expect("not an error");
    assert_eq!(log.range, Range::NotFetched { old: true, new: false });
    assert_eq!(log.new.as_deref(), Some(ahead.as_str()));
    assert_eq!(fx.label(), "new commits");
}

#[test]
fn a_submodule_that_was_never_checked_out_is_not_asked() {
    let fx = Fixture::new();
    let clone = tempfile::tempdir().expect("tempdir");
    git(clone.path(), &["clone", "-q", fx.root().to_str().expect("utf-8"), "."]);
    assert!(clone.path().join("libs/lib").is_dir(), "git leaves the folder empty");
    let sub = sub(Some(A), Some(B));
    let spawned = spawns(|| {
        let log = submodule_log(clone.path(), "libs/lib", &sub).expect("log");
        assert_eq!((log.checkout, &log.range), (Checkout::NotCheckedOut, &Range::Unavailable));
    });
    assert_eq!(spawned, 0, "git there would answer for the parent");
    let mut files = vec![{
        let mut file = FileChange::new("libs/lib", ChangeKind::Modified);
        file.submodule = Some(sub);
        file
    }];
    annotate_files(clone.path(), &mut files);
    let shown = files[0].submodule.as_ref().expect("sub");
    assert_eq!(shown.label(ChangeKind::Modified), "not checked out");
    assert!(!shown.checked_out());
}

#[test]
fn a_folder_that_was_deleted_is_missing_and_a_link_is_not_followed() {
    let fx = Fixture::new();
    std::fs::remove_dir_all(fx.lib()).expect("rm");
    let entry = fx.entry();
    let sub = entry.submodule.as_ref().expect("still a submodule");
    assert_eq!(entry.kind, ChangeKind::Deleted, "git sees the folder gone");
    assert_eq!(sub.label(entry.kind), "folder missing", "the parent still records the submodule");
    let log = submodule_log(fx.root(), "libs/lib", sub).expect("log");
    assert_eq!(log.checkout, Checkout::FolderMissing);

    // A symlink in the folder's place is somebody else's repository.
    let elsewhere = tempfile::tempdir().expect("tempdir");
    git(elsewhere.path(), &["init", "-q"]);
    crate::platform::fs::symlink(elsewhere.path(), fx.lib()).expect("symlink");
    let log = submodule_log(fx.root(), "libs/lib", sub).expect("log");
    assert_eq!(log.checkout, Checkout::NotCheckedOut);
}

#[test]
fn a_new_submodule_is_added_at_its_commit() {
    let upstream = repo_with(&[("lib.txt", "1\n")]);
    let parent = repo_with(&[("main.txt", "m\n")]);
    let source = upstream.path().to_str().expect("utf-8 path");
    git(parent.path(), &["-c", "protocol.file.allow=always", "submodule", "add", "-q", source, "libs/lib"]);
    let mut files = status(parent.path(), None).expect("status").expect("repo").files;
    annotate_files(parent.path(), &mut files);
    let lib = files.iter().find(|f| f.path == "libs/lib").expect("listed");
    assert_eq!(lib.kind, ChangeKind::Added);
    let id = head_of(&parent.path().join("libs/lib")).expect("head");
    assert_eq!(lib.submodule.as_ref().expect("sub").label(lib.kind), format!("added at {}", short(&id)));
    let log = lib.submodule.as_ref().and_then(|s| s.log.as_deref()).expect("log");
    assert_eq!(log.range, Range::NoBase);
}

#[test]
fn a_removed_submodule_is_still_a_submodule_to_the_list() {
    let fx = Fixture::new();
    git(fx.root(), &["rm", "-q", "-f", "libs/lib"]);
    let entry = fx.entry();
    assert_eq!(entry.kind, ChangeKind::Deleted);
    assert_eq!(entry.submodule.as_ref().map(|s| s.label(entry.kind)).as_deref(), Some("removed"));
}

#[test]
fn a_commit_that_moves_a_submodule_says_from_which_commit_to_which() {
    let fx = Fixture::new();
    fx.commit_in_lib(2);
    let before = log(fx.root(), "HEAD", 1).expect("log").remove(0);
    git(fx.root(), &["add", "libs/lib"]);
    git(fx.root(), &["commit", "-q", "-m", "move the submodule"]);
    let commit = log(fx.root(), "HEAD", 1).expect("log").remove(0);
    assert_eq!(commit.parent.as_deref(), Some(before.sha.as_str()));

    let files = commit_files(fx.root(), &commit).expect("files");
    let lib = files.iter().find(|f| f.path == "libs/lib").expect("listed");
    let sub = lib.submodule.as_ref().expect("a submodule, from the raw diff's modes");
    let (old, new) = (sub.old.clone().expect("old"), sub.new.clone().expect("new"));
    assert_ne!(old, new);
    assert_eq!(sub.label(lib.kind), "2 new commits", "commit_files looked the commits up");
    assert_eq!(sub.log, Some(annotated(fx.root(), lib)));
    let log = sub.log.as_deref().expect("log");
    assert_eq!(counts(log), (2, 0));
    assert_eq!(log.new.as_deref(), Some(new.as_str()));
    assert!(files.iter().all(|f| f.path == "libs/lib"), "only the submodule changed");
}

fn annotated(root: &Path, file: &FileChange) -> Arc<SubmoduleLog> {
    log_for(root, &file.path, file.submodule.as_ref().expect("sub"), false).expect("log").expect("some")
}

#[test]
fn long_lists_are_cut_and_the_rest_counted() {
    let fx = Fixture::new();
    let total = super::COMMITS_SHOWN + 5;
    fx.commit_in_lib(total);
    let file = fx.entry();
    let log = submodule_log(fx.root(), &file.path, file.submodule.as_ref().expect("sub")).expect("log");
    let Range::Commits { ahead, ahead_total, .. } = &log.range else { panic!("{:?}", log.range) };
    assert_eq!((ahead.len(), *ahead_total), (super::COMMITS_SHOWN, total));
    assert_eq!(fx.label(), format!("{total} new commits"));
}

#[test]
fn a_submodule_knows_its_superproject_without_git() {
    let fx = Fixture::new();
    let found = superproject(&fx.lib()).expect("a submodule");
    assert_eq!((found.root.as_path(), found.path.as_str()), (fx.root(), "libs/lib"));
    assert_eq!(found.phrase(), format!("in {}", fx.root().file_name().expect("name").to_string_lossy()));
    assert_eq!(spawns(|| drop(superproject(&fx.lib()))), 0);
    // The status carries it, for the title.
    let status = status(&fx.lib(), None).expect("status").expect("repo");
    // (git names the folder with its links resolved, which a temporary folder on a Mac has.)
    let seen = status.superproject.expect("the status says it too");
    assert_eq!(seen.path, found.path);
    assert_eq!(seen.root.canonicalize().ok(), found.root.canonicalize().ok());
    // The parent itself, and an ordinary repository, belong to nothing.
    assert_eq!(superproject(fx.root()), None);
    assert_eq!(superproject(init().path()), None);
}

#[test]
fn nested_and_renamed_submodules_belong_to_the_nearest_parent() {
    let fx = Fixture::new();
    // The submodule gets one of its own.
    let inner = repo_with(&[("inner.txt", "i\n")]);
    let source = inner.path().to_str().expect("utf-8 path");
    git(&fx.lib(), &["-c", "protocol.file.allow=always", "submodule", "add", "-q", source, "deep/inner"]);
    git(&fx.lib(), &["commit", "-q", "-m", "add the inner one"]);
    let found = superproject(&fx.lib().join("deep/inner")).expect("nested");
    assert_eq!((found.root, found.path.as_str()), (fx.lib(), "deep/inner"));

    // Added under another name, `.git/modules/<name>` has a slash in it.
    let other = repo_with(&[("o.txt", "o\n")]);
    let source = other.path().to_str().expect("utf-8 path");
    git(
        fx.root(),
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            "-q",
            "--name",
            "vendor/other",
            source,
            "x/other",
        ],
    );
    let found = superproject(&fx.root().join("x/other")).expect("named with a slash");
    assert_eq!((found.root.as_path(), found.path.as_str()), (fx.root(), "x/other"));
}

#[test]
fn a_worktree_and_a_made_up_git_file_are_not_submodules() {
    let main = repo_with(&[("a.txt", "1\n")]);
    let trees = tempfile::tempdir().expect("tempdir");
    let tree = trees.path().join("feature");
    git(main.path(), &["worktree", "add", "-q", "-b", "feature", tree.to_str().expect("utf-8")]);
    assert_eq!(superproject(&tree), None, "its git dir is under worktrees/");

    let fake = tempfile::tempdir().expect("tempdir");
    write(fake.path(), ".git", "gitdir: /etc\n");
    assert_eq!(superproject(fake.path()), None);
    write(fake.path(), ".git", "gitdir: ../../modules/nothing\n");
    assert_eq!(superproject(fake.path()), None, "no repository above it to belong to");
}

#[test]
fn the_card_says_why_it_lists_nothing_and_what_else_is_in_the_folder() {
    let log = |checkout, range| SubmoduleLog { checkout, old: Some(A.into()), new: Some(B.into()), range };
    let said = |checkout, range| log(checkout, range).nothing_to_list();
    assert_eq!(
        said(Checkout::Ready, Range::NotFetched { old: true, new: false }).as_deref(),
        Some("These commits aren't in the submodule's clone; fetch it to see them.")
    );
    assert!(
        said(Checkout::NotCheckedOut, Range::Unavailable).is_some_and(|s| s.contains("isn't checked out"))
    );
    assert_eq!(
        said(Checkout::FolderMissing, Range::Unavailable).as_deref(),
        Some("The submodule's folder is missing.")
    );
    assert_eq!(said(Checkout::Ready, Range::NoBase).as_deref(), Some("A new submodule, at bbbbbbb."));
    assert_eq!(said(Checkout::Ready, listed(1, 0)), None, "commits are listed, not explained");

    let mut inside = sub(Some(A), Some(A));
    assert_eq!(inside.inside_note(), None);
    inside.commit_changed = false;
    inside.tracked = true;
    assert_eq!(
        inside.inside_note().as_deref(),
        Some("The submodule has uncommitted changes. Open a terminal in it to see them.")
    );
    inside.commit_changed = true;
    inside.untracked = true;
    assert_eq!(
        inside.inside_note().as_deref(),
        Some(
            "The submodule also has uncommitted changes and untracked files. Open a terminal in it to see them."
        )
    );
}

#[test]
fn comparing_with_a_branch_keeps_what_status_found_inside_the_submodule() {
    let fx = Fixture::new();
    git(fx.root(), &["branch", "side"]);
    fx.commit_in_lib(1);
    write(&fx.lib(), "lib.txt", "edited\n");
    write(&fx.lib(), "scratch.txt", "x\n");
    let compared = status(fx.root(), Some("side")).expect("status").expect("repo");
    assert_eq!(compared.compare.as_deref(), Some("side"));
    let lib = compared.files.iter().find(|f| f.path == "libs/lib").expect("listed");
    let sub = lib.submodule.as_ref().expect("a submodule");
    assert!(sub.commit_changed && sub.tracked && sub.untracked, "{sub:?}");
}
