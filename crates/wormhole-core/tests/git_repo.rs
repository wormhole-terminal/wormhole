//! End-to-end checks of the git layer against throwaway repositories.

use std::path::Path;
use std::process::Command;

use wormhole_core::git::{self, ChangeKind, LineKind};

/// Run git in `dir` with a fixed identity (never the user's config).
fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.com", "-c", "init.defaultBranch=main"])
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?} failed");
}

fn write(dir: &Path, path: &str, text: &str) {
    let path = dir.join(path);
    std::fs::create_dir_all(path.parent().expect("has parent")).expect("mkdir");
    std::fs::write(path, text).expect("write");
}

/// An empty repository in `root`. The code under test reads the user's global config, as it
/// should; a global excludes file (`*.txt`, say) would hide the files these tests count, so
/// the repository's own config overrides it.
fn init(root: &Path) {
    git(root, &["init", "-q"]);
    git(root, &["config", "core.excludesFile", "/dev/null"]);
}

/// A repo with one commit on `main` and a feature branch with one more.
fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    init(root);
    write(root, "src/lib.rs", "fn a() {}\nfn b() {}\n");
    write(root, "README.md", "# Demo\n");
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "initial"]);
    git(root, &["checkout", "-q", "-b", "feature"]);
    write(root, "src/new.rs", "pub fn new() {}\n");
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "Add new.rs"]);
    dir
}

#[test]
fn outside_a_repo_is_none() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert_eq!(git::status(dir.path(), None).expect("status"), None);
    assert_eq!(git::repo_root(dir.path()).expect("git answers"), None);
}

#[test]
fn a_compare_branch_that_was_deleted_shows_head_and_is_named() {
    let dir = repo();
    let root = dir.path();
    write(root, "src/lib.rs", "fn a() {}\n");
    git(root, &["branch", "-q", "-D", "main"]);
    let status = git::status(root, Some("main")).expect("status").expect("is a repo");
    assert_eq!(status.compare_missing.as_deref(), Some("main"));
    assert_eq!((status.compare, status.base_rev.as_str()), (None, "HEAD"));
    let files: Vec<_> = status.files.iter().map(|f| (f.path.as_str(), f.kind)).collect();
    assert_eq!(files, vec![("src/lib.rs", ChangeKind::Modified)]);
}

/// `WORMHOLE_GIT_BIN` stands in for git in debug builds, so devctl can show the panels without
/// one. Setting it in this process would change git for every test running beside this one, so
/// the test runs itself again in a child process with it set.
#[test]
fn a_git_that_is_not_installed_is_reported_as_such_not_as_no_repository() {
    const CHILD: &str = "HD_GIT_REPO_TEST_CHILD";
    if !cfg!(debug_assertions) {
        return; // release builds always run the real git
    }
    let dir = repo();
    if std::env::var_os(CHILD).is_some() {
        let err = git::status(dir.path(), None).expect_err("there is no git");
        assert_eq!(git::GitError::of(&err), git::GitError::GitMissing);
        let err = git::repo_root(dir.path()).expect_err("there is no git");
        assert_eq!(git::GitError::of(&err), git::GitError::GitMissing);
        return;
    }
    let name = "a_git_that_is_not_installed_is_reported_as_such_not_as_no_repository";
    let output = Command::new(std::env::current_exe().expect("test binary"))
        .args(["--exact", name, "--nocapture", "--test-threads=1"])
        .env(CHILD, "1")
        .env("WORMHOLE_GIT_BIN", dir.path().join("no-git-here"))
        .output()
        .expect("runs");
    let said = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{said}{}", String::from_utf8_lossy(&output.stderr));
    assert!(said.contains("1 passed"), "the child ran the test: {said}");
}

#[test]
fn status_lists_working_tree_changes() {
    let dir = repo();
    let root = dir.path();
    write(root, "src/lib.rs", "fn a() {}\nfn b2() {}\nfn c() {}\n");
    write(root, "notes.txt", "one\ntwo\n");

    let status = git::status(root, None).expect("status").expect("is a repo");
    assert_eq!(status.branch.as_deref(), Some("feature"));
    let files: Vec<_> = status.files.iter().map(|f| (f.path.as_str(), f.kind, f.added, f.removed)).collect();
    assert_eq!(
        files,
        vec![
            ("notes.txt", ChangeKind::Untracked, Some(2), Some(0)),
            ("src/lib.rs", ChangeKind::Modified, Some(2), Some(1)),
        ]
    );
}

#[test]
fn compare_with_branch_includes_committed_work() {
    let dir = repo();
    let status = git::status(dir.path(), Some("main")).expect("status").expect("is a repo");
    assert_eq!(status.compare.as_deref(), Some("main"));
    let files: Vec<_> = status.files.iter().map(|f| (f.path.as_str(), f.kind)).collect();
    assert_eq!(files, vec![("src/new.rs", ChangeKind::Added)]);
}

#[test]
fn diff_of_working_change_has_highlight_sources() {
    let dir = repo();
    let root = dir.path();
    write(root, "src/lib.rs", "fn a() {}\nfn b2() {}\n");
    let status = git::status(root, None).expect("status").expect("is a repo");
    let file = &status.files[0];
    let spec = git::DiffSpec { old: status.base_rev.clone(), new: None };
    let diff = git::diff(root, file, &spec).expect("diff");

    let kinds: Vec<_> = diff.hunks[0].lines.iter().map(|l| (l.kind, l.text.as_str())).collect();
    assert_eq!(
        kinds,
        vec![
            (LineKind::Context, "fn a() {}"),
            (LineKind::Removed, "fn b() {}"),
            (LineKind::Added, "fn b2() {}")
        ]
    );
    assert_eq!(diff.old_text.as_deref(), Some("fn a() {}\nfn b() {}\n"));
    assert_eq!(diff.new_text.as_deref(), Some("fn a() {}\nfn b2() {}\n"));
}

#[test]
fn history_and_commit_files() {
    let dir = repo();
    let root = dir.path();
    let commits = git::log(root, "HEAD", 10).expect("log");
    let subjects: Vec<_> = commits.iter().map(|c| c.subject.as_str()).collect();
    assert_eq!(subjects, vec!["Add new.rs", "initial"]);
    assert_eq!(commits[1].parent, None);

    let files = git::commit_files(root, &commits[0]).expect("files");
    assert_eq!(files.len(), 1);
    assert_eq!((files[0].path.as_str(), files[0].kind), ("src/new.rs", ChangeKind::Added));

    // The root commit diffs against the empty tree.
    let first = git::commit_files(root, &commits[1]).expect("files");
    assert_eq!(first.len(), 2);
    let diff = git::diff(root, &first[1], &git::commit_spec(&commits[1])).expect("diff");
    assert!(diff.hunks[0].lines.iter().all(|l| l.kind == LineKind::Added));
}

/// A file `.gitattributes` stores with LFS says "LFS" in the list instead of
/// counting its pointer's lines; the others keep their counts. (git-lfs itself isn't needed:
/// the attribute is what marks it.)
#[test]
fn files_stored_with_lfs_are_marked() {
    let dir = repo();
    let root = dir.path();
    write(root, ".gitattributes", "*.mov filter=lfs diff=lfs merge=lfs -text\n");
    let pointer =
        |oid: &str| format!("version https://git-lfs.github.com/spec/v1\noid sha256:{oid}\nsize 12345\n");
    write(root, "assets/demo.mov", &pointer("aaaa"));
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "Add a video"]);
    write(root, "assets/demo.mov", &pointer("bbbb"));
    write(root, "src/lib.rs", "fn a() {}\n");
    let status = git::status(root, None).expect("status").expect("is a repo");
    let lfs: Vec<_> = status.files.iter().map(|f| (f.path.as_str(), f.lfs)).collect();
    assert_eq!(lfs, [("assets/demo.mov", true), ("src/lib.rs", false)]);

    // A commit's files are marked the same way.
    let commits = git::log(root, "HEAD", git::LOG_LIMIT).expect("log");
    let files = git::commit_files(root, &commits[0]).expect("files");
    let lfs: Vec<_> = files.iter().map(|f| (f.path.as_str(), f.lfs)).collect();
    assert_eq!(lfs, [(".gitattributes", false), ("assets/demo.mov", true)]);
}

/// When each side is a pointer, the diff is what they point at (sizes and
/// the object), not the pointer's three lines.
#[test]
fn the_diff_of_a_file_stored_with_lfs_is_what_its_pointers_say() {
    let dir = repo();
    let root = dir.path();
    let (old_oid, new_oid) = ("4d".repeat(32), "7e".repeat(32));
    let pointer = |oid: &str, size| {
        format!("version https://git-lfs.github.com/spec/v1\noid sha256:{oid}\nsize {size}\n")
    };
    write(root, ".gitattributes", "*.mov filter=lfs diff=lfs merge=lfs -text\n");
    write(root, "assets/demo.mov", &pointer(&old_oid, 12_300_000));
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "Add a video"]);
    write(root, "assets/demo.mov", &pointer(&new_oid, 12_800_000));
    write(root, "assets/new.mov", &pointer(&new_oid, 5));
    let status = git::status(root, None).expect("status").expect("is a repo");
    let spec = git::DiffSpec { old: status.base_rev.clone(), new: None };
    let file = |path: &str| status.files.iter().find(|f| f.path == path).expect("listed").clone();

    let diff = git::diff(root, &file("assets/demo.mov"), &spec).expect("diff");
    let change = diff.lfs.clone().expect("both sides are pointers");
    assert_eq!(change.old, Some(git::LfsPointer { oid: old_oid, size: 12_300_000 }));
    assert_eq!(change.new, Some(git::LfsPointer { oid: new_oid.clone(), size: 12_800_000 }));
    assert!(diff.hunks.is_empty(), "the pointer lines are not shown");
    assert_eq!(git::sides(&diff, ChangeKind::Modified), (git::Side::Lfs, git::Side::Lfs));
    assert_eq!((diff.old_text, diff.new_text), (None, None));

    let added = git::diff(root, &file("assets/new.mov"), &spec).expect("diff");
    let change = added.lfs.expect("a pointer");
    assert_eq!((change.old, change.new), (None, Some(git::LfsPointer { oid: new_oid, size: 5 })));
}

/// A binary file: no lines, but its size before and after.
#[test]
fn a_binary_diff_says_the_sizes_of_both_sides() {
    let dir = repo();
    let root = dir.path();
    let bytes = |n: usize| (0..n).map(|i| (i % 7) as u8).collect::<Vec<u8>>(); // NULs: binary
    std::fs::write(root.join("icon.png"), bytes(48_200)).expect("write");
    std::fs::write(root.join("gone.png"), bytes(300)).expect("write");
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "Add icons"]);
    std::fs::write(root.join("icon.png"), bytes(52_900)).expect("write");
    std::fs::write(root.join("fresh.png"), bytes(10)).expect("write");
    std::fs::remove_file(root.join("gone.png")).expect("remove");
    let status = git::status(root, None).expect("status").expect("is a repo");
    let spec = git::DiffSpec { old: status.base_rev.clone(), new: None };
    let sizes = |path: &str| {
        let file = status.files.iter().find(|f| f.path == path).expect("listed");
        let diff = git::diff(root, file, &spec).expect("diff");
        assert!(diff.binary && diff.hunks.is_empty(), "{path}");
        diff.sizes
    };
    assert_eq!(sizes("icon.png"), (Some(48_200), Some(52_900)));
    assert_eq!(sizes("fresh.png"), (None, Some(10)));
    assert_eq!(sizes("gone.png"), (Some(300), None));

    // A commit's sides are both read from git.
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "Change icons"]);
    let commit = git::log(root, "HEAD", 1).expect("log").remove(0);
    let files = git::commit_files(root, &commit).expect("files");
    let icon = files.iter().find(|f| f.path == "icon.png").expect("in the commit");
    let diff = git::diff(root, icon, &git::commit_spec(&commit)).expect("diff");
    assert_eq!(diff.sizes, (Some(48_200), Some(52_900)));
}

#[test]
fn branches_put_current_first() {
    let dir = repo();
    assert_eq!(git::branches(dir.path()).expect("branches"), vec!["feature", "main"]);
}

// ---------------------------------------------------------------------------------------------
// Hardening: nothing a repository (or a branch name) says may make our background git run code,
// write a file, or answer for another file.
// ---------------------------------------------------------------------------------------------

/// A script in `dir` that records that it ran by creating `marker`, then prints `output`.
#[cfg(unix)]
fn script(dir: &Path, name: &str, marker: &Path, output: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join(name);
    std::fs::write(&path, format!("#!/bin/sh\ntouch '{}'\n{output}\n", marker.display())).expect("script");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    path
}

fn outside() -> tempfile::TempDir {
    // Not `.tmpXXXX` (the default): a ref name may not have a component that starts with a dot.
    tempfile::Builder::new().prefix("hd-out-").tempdir().expect("tempdir")
}

#[test]
fn a_branch_named_like_an_option_is_never_read_as_one() {
    let dir = repo();
    let root = dir.path();
    let out = outside();
    let pwned = out.path().join("pwned.txt");
    // Legal for git, and `git log --output=<file> ...` would write that file.
    let name = format!("--output={}", pwned.display());
    git(root, &["update-ref", &format!("refs/heads/{name}"), "HEAD"]);
    assert!(git::branches(root).expect("branches").contains(&name));

    let commits = git::log(root, &name, 10).expect("log resolves it as a branch");
    assert_eq!(commits.len(), 2);
    assert!(!pwned.exists(), "git log wrote the file");

    // As the "compare with" branch (merge-base) and as a side of a diff.
    let status = git::status(root, Some(&name)).expect("status").expect("is a repo");
    assert_eq!(status.compare.as_deref(), Some(name.as_str()));
    write(root, "src/lib.rs", "fn a() {}\nfn changed() {}\n");
    let file = git::status(root, None).expect("status").expect("repo").files.remove(0);
    let spec = git::DiffSpec { old: name.clone(), new: None };
    let diff = git::diff(root, &file, &spec).expect("diff");
    assert!(!diff.hunks.is_empty());
    assert!(!pwned.exists(), "git diff wrote the file");
    assert!(std::fs::read_dir(out.path()).expect("dir").next().is_none());
}

#[test]
#[cfg(unix)]
fn a_repositorys_config_cannot_run_programs_during_refreshes() {
    let dir = repo();
    let root = dir.path();
    let tools = outside();
    let monitor = tools.path().join("fsmonitor-ran");
    let textconv = tools.path().join("textconv-ran");
    let external = tools.path().join("external-diff-ran");
    let hook = script(tools.path(), "hook.sh", &monitor, "printf '\\0'");
    let conv = script(tools.path(), "conv.sh", &textconv, "cat \"$1\"");
    let ext = script(tools.path(), "ext.sh", &external, "exit 0");
    write(root, ".gitattributes", "*.conv diff=evil\n*.ext diff=other\n");
    write(root, "x.conv", "one\n");
    write(root, "y.ext", "one\n");
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "drivers"]);
    git(root, &["config", "core.fsmonitor", hook.to_str().expect("utf8")]);
    git(root, &["config", "diff.evil.textconv", conv.to_str().expect("utf8")]);
    git(root, &["config", "diff.other.command", ext.to_str().expect("utf8")]);
    write(root, "x.conv", "one\ntwo\n");
    write(root, "y.ext", "one\ntwo\n");
    write(root, "src/lib.rs", "fn a() {}\nfn b2() {}\n");

    // The setup does what it claims: plain git runs all three.
    git(root, &["status", "--porcelain"]);
    git(root, &["diff", "--", "x.conv", "y.ext"]);
    assert!(monitor.exists() && textconv.exists() && external.exists(), "the trap is not armed");
    for marker in [&monitor, &textconv, &external] {
        std::fs::remove_file(marker).expect("reset");
    }

    let status = git::status(root, None).expect("status").expect("is a repo");
    let spec = git::DiffSpec { old: status.base_rev.clone(), new: None };
    for file in &status.files {
        git::diff(root, file, &spec).expect("diff");
    }
    git::changed_files(root, "HEAD", None).expect("changed files");
    let head = git::log(root, "HEAD", 5).expect("log");
    git::commit_files(root, &head[0]).expect("commit files");
    assert!(!monitor.exists(), "core.fsmonitor ran");
    assert!(!textconv.exists(), "a textconv filter ran");
    assert!(!external.exists(), "an external diff ran");
    // Still the right answer: both files changed by one line, and the diff is the plain one.
    for name in ["x.conv", "y.ext"] {
        let file = status.files.iter().find(|f| f.path == name).expect("listed");
        assert_eq!(file.added, Some(1), "{name}");
        let diff = git::diff(root, file, &spec).expect("diff");
        assert_eq!(diff.hunks.len(), 1, "{name}");
    }
}

#[test]
fn paths_with_glob_characters_name_exactly_that_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    init(root);
    for name in ["a1.txt", "a[1].txt", "*.txt", "b.txt"] {
        write(root, name, "old\n");
    }
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "files"]);
    write(root, "a1.txt", "old\nin a1\n");
    write(root, "a[1].txt", "old\nin brackets\n");
    write(root, "*.txt", "old\nin star\n");
    write(root, "b.txt", "old\nin b\n");

    let status = git::status(root, None).expect("status").expect("is a repo");
    assert_eq!(status.files.len(), 4);
    let spec = git::DiffSpec { old: status.base_rev.clone(), new: None };
    for file in &status.files {
        let diff = git::diff(root, file, &spec).expect("diff");
        assert_eq!(diff.hunks.len(), 1, "{}: one hunk, from this file only", file.path);
        let added: Vec<_> = diff.hunks[0]
            .lines
            .iter()
            .filter(|l| l.kind == LineKind::Added)
            .map(|l| l.text.as_str())
            .collect();
        let expected = match file.path.as_str() {
            "a1.txt" => "in a1",
            "a[1].txt" => "in brackets",
            "*.txt" => "in star",
            _ => "in b",
        };
        assert_eq!(added, [expected], "{}", file.path);
        assert_eq!((file.added, file.removed), (Some(1), Some(0)), "{}", file.path);
    }
}

#[test]
fn a_status_refresh_leaves_the_index_alone() {
    // An agent may be committing: nothing we run may rewrite (so lock) the index, not even to
    // refresh stale entries. Stale: same content, newer mtime. `git diff` rewrote the index in
    // a few runs in a hundred on a busy machine, so this repeats.
    let dir = repo();
    let root = dir.path();
    let index = root.join(".git/index");
    let modified = || std::fs::metadata(&index).expect("index").modified().expect("mtime");
    let before = modified();
    write(root, "src/lib.rs", "fn a() {}\nfn changed() {}\n"); // a real change to diff
    for round in 0..25 {
        std::thread::sleep(std::time::Duration::from_millis(5));
        write(root, "README.md", "# Demo\n");
        git::status(root, None).expect("status").expect("is a repo");
        git::changed_files(root, "HEAD", None).expect("changed");
        let diff_spec = git::DiffSpec { old: "HEAD".into(), new: None };
        let file = git::status(root, None).expect("status").expect("repo").files.remove(0);
        git::diff(root, &file, &diff_spec).expect("diff");
        assert_eq!(modified(), before, "round {round}: our refresh rewrote the index");
    }

    // The control: without the setting, plain git does refresh it, so the check above could fail.
    std::thread::sleep(std::time::Duration::from_millis(30));
    write(root, "README.md", "# Demo\n");
    git(root, &["status", "--porcelain"]);
    if modified() == before {
        eprintln!("this git did not rewrite the stale index either: the check above proves little here");
    }
}

// ---------------------------------------------------------------------------------------------
// Bounded work: nothing in a repository can make a refresh read gigabytes, wait for a pipe, or
// build a diff the panel cannot show.
// ---------------------------------------------------------------------------------------------

#[test]
fn a_repo_without_commits_diffs_against_the_empty_tree() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    init(root);
    write(root, "staged.txt", "one\ntwo\nthree\n");
    write(root, "loose.txt", "a\nb");
    git(root, &["add", "staged.txt"]);

    let status = git::status(root, None).expect("status").expect("is a repo");
    assert_eq!(status.base_rev, "4b825dc642cb6eb9a060e54bf8d69288fbee4904");
    let files: Vec<_> = status.files.iter().map(|f| (f.path.as_str(), f.kind, f.added, f.removed)).collect();
    assert_eq!(
        files,
        vec![
            ("loose.txt", ChangeKind::Untracked, Some(2), Some(0)),
            ("staged.txt", ChangeKind::Added, Some(3), Some(0)),
        ]
    );
    assert_eq!(git::log(root, "HEAD", 10).expect("log"), vec![]);
}

#[test]
#[cfg(unix)]
fn untracked_fifos_and_big_files_do_not_stall_a_refresh() {
    use std::time::{Duration, Instant};
    let dir = repo();
    let root = dir.path();
    let fifo = root.join("pipe");
    assert!(Command::new("mkfifo").arg(&fifo).status().expect("mkfifo").success());
    std::os::unix::fs::symlink(&fifo, root.join("link-to-pipe")).expect("symlink");
    std::os::unix::fs::symlink("src/lib.rs", root.join("link-to-file")).expect("symlink");
    write(root, "model.bin.txt", &"a line of a very large log\n".repeat(2_000_000)); // 52 MB
    write(root, "small.txt", "one\ntwo\n");

    // The old code opened the link to the pipe and never came back.
    let started = Instant::now();
    let status = git::status(root, None).expect("status").expect("is a repo");
    assert!(started.elapsed() < Duration::from_secs(10), "took {:?}", started.elapsed());
    let counts: Vec<_> = status.files.iter().map(|f| (f.path.as_str(), f.added)).collect();
    assert_eq!(
        counts,
        vec![
            ("link-to-file", Some(1)),
            ("link-to-pipe", Some(1)),
            ("model.bin.txt", None),
            ("small.txt", Some(2))
        ]
    );

    // Opening its diff does not hang either (git itself would wait on the pipe).
    let spec = git::DiffSpec { old: status.base_rev.clone(), new: None };
    let link = &status.files[1];
    let started = Instant::now();
    let diff = git::diff(root, link, &spec).expect("diff of the link");
    assert!(started.elapsed() < Duration::from_secs(10), "took {:?}", started.elapsed());
    assert!(diff.new_file);
    let lines: Vec<_> = diff.hunks[0].lines.iter().map(|l| l.text.as_str()).collect();
    assert_eq!(lines, [fifo.to_str().expect("utf8")]);

    // The big file's diff is "too large", not 52 MB of lines.
    let started = Instant::now();
    let diff = git::diff(root, &status.files[2], &spec).expect("diff of the big file");
    assert!(diff.too_large && diff.hunks.is_empty(), "{} hunks", diff.hunks.len());
    assert!(started.elapsed() < Duration::from_secs(5), "took {:?}", started.elapsed());
}

fn numbered(prefix: &str, lines: usize) -> String {
    (0..lines).map(|i| format!("{prefix} line {i}\n")).collect()
}

#[test]
fn a_huge_diff_is_reported_as_too_large_instead_of_being_built() {
    use std::time::{Duration, Instant};
    let dir = repo();
    let root = dir.path();
    write(root, "generated.txt", &numbered("old", 400_000));
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "generated"]);
    write(root, "generated.txt", &numbered("new", 400_000)); // every line changes: a 12 MB patch
    let status = git::status(root, None).expect("status").expect("is a repo");
    let file = status.files.iter().find(|f| f.path == "generated.txt").expect("listed");
    let spec = git::DiffSpec { old: status.base_rev.clone(), new: None };

    let started = Instant::now();
    let diff = git::diff(root, file, &spec).expect("diff");
    let took = started.elapsed();
    assert!(diff.too_large);
    // Nothing of it is kept but its size, for the card that asks ("This diff is 12.4 MB").
    let patch = Command::new("git")
        .args(["diff", "--no-color", "--no-ext-diff", "--no-textconv", "-M", "HEAD", "--", "generated.txt"])
        .current_dir(root)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("git diff");
    let patch_bytes = Some(patch.stdout.len() as u64);
    assert!(patch_bytes > Some(git::DIFF_ASK_BYTES), "{patch_bytes:?}");
    assert_eq!(diff, git::FileDiff { too_large: true, patch_bytes, ..Default::default() });
    assert!(took < Duration::from_secs(2), "took {took:?}");

    // "Load anyway": all of it, but at 800 000 lines without highlighting.
    let full = git::diff_with_limits(root, file, &spec, &git::DiffLimits::LOAD_ANYWAY).expect("diff");
    assert!(!full.too_large && full.highlight_skipped);
    assert_eq!(full.hunks.len(), 1);
    assert_eq!(full.hunks[0].lines.len(), 800_000);
    assert_eq!((full.new_text, full.old_text), (None, None));
}

#[test]
fn the_size_rule_asks_above_8_mb_and_highlights_up_to_20_000_lines() {
    assert_eq!(git::DIFF_ASK_BYTES, 8 << 20);
    assert_eq!(git::DIFF_HIGHLIGHT_MAX_LINES, 20_000);
    let limits = git::DiffLimits::default();
    assert_eq!(limits.max_patch_bytes, git::DIFF_ASK_BYTES);
    assert_eq!(limits.max_highlight_lines, git::DIFF_HIGHLIGHT_MAX_LINES);
    assert_eq!(git::DiffLimits::LOAD_ANYWAY.max_patch_bytes, u64::MAX);
    assert_eq!(git::DiffLimits::LOAD_ANYWAY.max_highlight_lines, git::DIFF_HIGHLIGHT_MAX_LINES);
}

#[test]
fn a_long_but_small_diff_loads_without_highlighting() {
    let dir = repo();
    let root = dir.path();
    write(root, "list.txt", &"x\n".repeat(30_000));
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "list"]);
    write(root, "list.txt", &"y\n".repeat(30_000)); // 60 000 lines, but only 120 KB
    let status = git::status(root, None).expect("status").expect("is a repo");
    let file = status.files.iter().find(|f| f.path == "list.txt").expect("listed");
    let spec = git::DiffSpec { old: status.base_rev.clone(), new: None };

    let diff = git::diff(root, file, &spec).expect("diff");
    assert!(!diff.too_large, "small enough to load without asking");
    assert!(diff.highlight_skipped, "over 20 000 lines");
    assert_eq!(diff.hunks[0].lines.len(), 60_000);
    assert_eq!((diff.old_text, diff.new_text), (None, None));

    let roomy = git::DiffLimits { max_highlight_lines: 100_000, ..Default::default() };
    let highlighted = git::diff_with_limits(root, file, &spec, &roomy).expect("diff");
    assert!(!highlighted.highlight_skipped);
    assert!(highlighted.old_text.is_some() && highlighted.new_text.is_some());
}

#[test]
fn a_small_change_in_a_big_file_keeps_its_diff_but_not_its_highlighting_text() {
    let dir = repo();
    let root = dir.path();
    let big = numbered("same", 500_000); // 6.5 MB, over the 4 MiB text limit
    write(root, "big.txt", &big);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "big"]);
    write(root, "big.txt", &big.replacen("same line 250000\n", "changed\n", 1));
    let status = git::status(root, None).expect("status").expect("is a repo");
    let file = status.files.iter().find(|f| f.path == "big.txt").expect("listed");
    let spec = git::DiffSpec { old: status.base_rev.clone(), new: None };

    let diff = git::diff(root, file, &spec).expect("diff");
    assert!(!diff.too_large);
    assert_eq!(diff.hunks.len(), 1);
    assert_eq!((diff.old_text, diff.new_text), (None, None));

    let unlimited = git::diff_with_limits(root, file, &spec, &git::DiffLimits::UNLIMITED).expect("diff");
    assert!(unlimited.old_text.is_some() && unlimited.new_text.is_some());

    // The same limit holds for a committed side (`git show`), which used to be unbounded.
    let commit_spec = git::DiffSpec { old: "HEAD".into(), new: Some("HEAD".into()) };
    let same = git::diff(root, file, &commit_spec).expect("diff");
    assert_eq!((same.old_text, same.new_text), (None, None));
}

/// What a refresh costs in a repo full of untracked files and one huge diff.
/// `cargo test -p wormhole-core --test git_repo refresh_costs -- --ignored --nocapture`
#[test]
#[ignore = "benchmark"]
fn refresh_costs() {
    use std::time::Instant;
    let dir = repo();
    let root = dir.path();
    let body = "let value = compute(input);\n".repeat(300); // 8 KB
    for i in 0..3000 {
        write(root, &format!("untracked/dir{}/file{i}.txt", i % 50), &body);
    }
    for i in 0..20 {
        write(root, &format!("data/big{i}.csv"), &"1,2,3,4,5,6,7,8\n".repeat(190_000)); // 3 MB
    }
    let best = |f: &dyn Fn()| {
        (0..5)
            .map(|_| {
                let t = Instant::now();
                f();
                t.elapsed()
            })
            .min()
            .expect("runs")
    };
    // Files younger than 2 s are counted again every time (a writer may still be at them).
    std::thread::sleep(std::time::Duration::from_millis(2100));
    let t = Instant::now();
    drop(git::status(root, None));
    eprintln!("status (3000 x 8 KB + 20 x 3 MB untracked), first refresh: {:?}", t.elapsed());
    eprintln!("status, next refreshes: {:?}", best(&|| drop(git::status(root, None))));
    eprintln!("status_summary, same repo: {:?}", best(&|| drop(git::status_summary(root, None))));

    write(root, "generated.txt", &numbered("old", 2_000_000)); // ~34 MB
    git(root, &["add", "generated.txt"]);
    git(root, &["commit", "-q", "-m", "generated"]);
    write(root, "generated.txt", &numbered("new", 2_000_000));
    let status = git::status_summary(root, None).expect("status").expect("repo");
    let file = status.files.iter().find(|f| f.path == "generated.txt").expect("listed");
    let spec = git::DiffSpec { old: status.base_rev.clone(), new: None };
    let t = Instant::now();
    let diff = git::diff(root, file, &spec).expect("diff");
    eprintln!(
        "diff of a 34 MB file with every line changed: {:?} (too_large: {})",
        t.elapsed(),
        diff.too_large
    );
}

#[test]
fn a_worktree_names_its_main_repository_and_a_repository_names_none() {
    let dir = repo();
    let root = dir.path();
    let linked = tempfile::tempdir().expect("tempdir");
    let tree = linked.path().join("badge");
    git(root, &["worktree", "add", "-q", tree.to_str().expect("utf-8 path"), "main"]);
    let main = git::worktree_of(&tree).expect("a worktree");
    let real = |p: &Path| std::fs::canonicalize(p).expect("exists");
    assert_eq!(real(&main), real(root));
    assert_eq!(git::worktree_of(root), None, "the main working tree");
    assert_eq!(git::worktree_of(linked.path()), None, "not a repository at all");
}

#[test]
fn a_detached_head_names_its_commit() {
    let dir = repo();
    let root = dir.path();
    git(root, &["checkout", "-q", "--detach", "main"]);
    let status = git::status(root, None).expect("status").expect("a repository");
    assert_eq!(status.branch, None, "detached");
    let head = git::head_commit(root).expect("git runs").expect("a commit");
    assert_eq!(head.subject, "initial");
    assert!(head.short.len() >= 7, "{head:?}");

    let empty = tempfile::tempdir().expect("tempdir");
    init(empty.path());
    assert_eq!(git::head_commit(empty.path()).expect("git runs"), None, "no commit yet");
}
